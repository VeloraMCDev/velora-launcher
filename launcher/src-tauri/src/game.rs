//! The Play button: install → sync → configure → start → watch.

use crate::accounts;
use crate::state::{AppState, RunningGame};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use velora_launcher_core::install::{self, InstallSpec};
use velora_launcher_core::launch::{self, LaunchOptions};
use velora_launcher_core::progress::{Event, Reporter, Stage};
use velora_launcher_core::{options, servers_dat, sync};
use velora_shared::InstanceManifest;

mod manifest_cache;
pub(crate) mod manifest_denials;

#[derive(Serialize, Clone)]
pub struct StatePayload {
    pub instance_id: String,
    pub operation_id: String,
    pub run_id: Option<String>,
    /// "preparing" | "running" | "idle" | "error"
    pub state: String,
    pub message: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ExitPayload {
    pub instance_id: String,
    pub run_id: String,
    pub code: Option<i32>,
    pub crashed: bool,
    pub tail: Vec<String>,
    pub crash_report: Option<String>,
}

/// Copy item textures out of the downloaded client jar (a no-op when they are already there) and tell the UI. Runs off the
/// async runtime because it unzips; safe to call as often as we like.
pub fn refresh_textures(app: &AppHandle) {
    let Ok(work) = app.state::<AppState>().shutdown.enter() else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        let _work = work;
        let layout = app.state::<AppState>().layout.clone();
        match velora_launcher_core::textures::ensure(&layout) {
            Ok(true) => tracing::info!("extracted item textures from the client jar"),
            Ok(false) => {}
            Err(e) => tracing::warn!("couldn't read textures from the client jar: {e:#}"),
        }
        app.emit("textures://status", velora_launcher_core::textures::status(&layout)).ok();
    });
}

#[derive(Serialize, Clone)]
pub struct ProgressPayload {
    pub instance_id: String,
    pub operation_id: String,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Serialize, Clone)]
pub struct LogPayload {
    pub instance_id: String,
    pub operation_id: String,
    pub run_id: String,
    pub lines: Vec<String>,
}

fn run_state(instance_id: &str, operation_id: uuid::Uuid, run_id: &str, state: &str) -> StatePayload {
    StatePayload {
        instance_id: instance_id.into(),
        operation_id: operation_id.to_string(),
        run_id: Some(run_id.into()),
        state: state.into(),
        message: None,
    }
}

pub fn emit_state(app: &AppHandle, id: &str, operation_id: uuid::Uuid, state: &str, message: Option<String>) {
    app.emit(
        "launch://state",
        StatePayload { instance_id: id.into(), operation_id: operation_id.to_string(), run_id: None, state: state.into(), message },
    )
    .ok();
}

/// Fetch the instance definition, falling back to the last copy we saw so
/// the game still starts without internet.
async fn instance_manifest(state: &AppState, id: &str) -> Result<InstanceManifest> {
    let cache = state.layout.instance_dir(id).join(".scopenet").join("manifest.json");
    let observed_denial = state.manifest_denials.observe(&cache);
    let panel = state.panel_url().ok_or_else(|| anyhow!("no panel configured"))?;
    let (authority, token) = {
        let accounts = state.accounts.read().unwrap();
        let account = accounts.active();
        let panel_scope = manifest_cache::Authority::new(&panel, account, false)?;
        let token = account
            .filter(|account| account.kind == "panel")
            .filter(|account| account.panel_url.as_deref().is_some_and(|url| panel_scope.same_panel(url)))
            .and_then(|account| state.secrets.get(&account.id).panel_token);
        (manifest_cache::Authority::new(&panel, account, token.is_some())?, token)
    };
    let fetched = async {
        let client = crate::panel_client::PanelClient::new(state.http.clone(), &panel, token.as_deref())?;
        let resp =
            crate::panel_client::PanelClient::send(client.request(reqwest::Method::GET, &format!("/launcher/instances/{id}"))?).await?;
        let manifest = crate::panel_client::PanelClient::json::<InstanceManifest>(resp, crate::panel_client::ErrorPolicy::Generic).await?;
        anyhow::ensure!(manifest.instance.id == id, "instance manifest identity mismatch");
        Ok::<_, anyhow::Error>(manifest)
    }
    .await;
    match fetched {
        Ok(m) => {
            if state.manifest_denials.refresh(&cache, observed_denial, || manifest_cache::publish(&cache, &authority, id, &m)).is_err() {
                tracing::warn!("instance manifest cache publication failed");
            }
            Ok(m)
        }
        Err(error) => {
            if error
                .downcast_ref::<crate::panel_client::PanelFailure>()
                .is_some_and(|failure| matches!(failure.status, Some(401 | 403 | 404)))
                && state.manifest_denials.deny(&cache, || manifest_cache::invalidate(&cache, &authority)).is_err()
            {
                tracing::warn!("instance manifest cache invalidation failed");
            }
            manifest_cache::recover_with(Err(error), || {
                let manifest = state.manifest_denials.read(&cache, || manifest_cache::read(&cache, &authority, id))?;
                tracing::warn!("network unavailable, using cached instance manifest");
                Ok(manifest)
            })
        }
    }
}

fn reporter(app: &AppHandle, instance_id: &str, operation_id: uuid::Uuid) -> Reporter {
    let app = app.clone();
    let instance_id = instance_id.to_owned();
    Arc::new(move |ev: Event| {
        app.emit(
            "launch://progress",
            ProgressPayload { instance_id: instance_id.clone(), operation_id: operation_id.to_string(), event: ev },
        )
        .ok();
    })
}

/// Install/verify an instance and (when `start`) run the game.
pub async fn run(app: AppHandle, instance_id: String, operation_id: uuid::Uuid, start: bool, deep: bool) -> Result<()> {
    let state = app.state::<AppState>();
    let report = reporter(&app, &instance_id, operation_id);
    let activity = crate::telemetry::capture(&state);
    if let Some(recorder) = &activity {
        recorder.background(if deep { "repair_start" } else { "install_start" }, &instance_id);
    }
    emit_state(&app, &instance_id, operation_id, "preparing", None);

    let account = state.accounts.read().unwrap().active().cloned();
    let account = match (start, account) {
        (true, None) => anyhow::bail!("sign in first"),
        (_, a) => a,
    };
    let auth = match &account {
        Some(a) if start => Some(accounts::game_auth(&state, a).await?),
        _ => None,
    };

    let manifest = instance_manifest(&state, &instance_id).await?;
    let inst = &manifest.instance;
    let settings = state.settings.read().unwrap().clone();
    let advanced = state.manifest.read().unwrap().as_ref().map(|m| m.branding.features.allow_advanced_java).unwrap_or(true);
    let ov = settings.instance_overrides.get(&instance_id).cloned().unwrap_or_default();
    let game_dir = state.layout.instance_dir(&instance_id);
    let panel = state.panel_url().unwrap_or_default();

    let java_override = if advanced {
        ov.java_path.clone().or(settings.java_path.clone()).filter(|p| !p.trim().is_empty()).map(PathBuf::from)
    } else {
        None
    };

    let spec = InstallSpec {
        mc_version: inst.mc_version.clone(),
        loader: inst.loader,
        loader_version: inst.loader_version.clone(),
        java_override,
        game_dir: game_dir.clone(),
        concurrency: settings.concurrent_downloads.clamp(1, 32) as usize,
        deep_verify: deep,
    };
    let installed = install::install(&state.http, &state.layout, &spec, &report).await?;
    // The client jar is on disk now; the item icons can switch to the real textures while the game starts.
    refresh_textures(&app);
    let already_running = state.game.lock().unwrap().iter().any(|g| g.instance_id == instance_id);
    // The on-disk file may be newer when the launcher was closed before Minecraft.
    let current_options = options::read_vanilla_preferences(&game_dir).unwrap_or_default();
    let mut saved_options = if current_options.is_empty() {
        settings.instance_game_options.get(&instance_id).cloned().unwrap_or_default()
    } else {
        current_options
    };
    if let Some(fov) = settings.instance_game_options.get(&instance_id).and_then(|o| o.get("fov")) {
        saved_options.insert("fov".into(), fov.clone());
    }
    // Do not rewrite pack files or options while another copy is using this directory.
    if !already_running {
        sync::sync(&state.http, &panel, &game_dir, inst.revision, inst.clean_epoch, &manifest.files, spec.concurrency, deep, &report)
            .await?;
        options::apply(&game_dir, &mut saved_options)?;
        if inst.mc_version == "26.3" && inst.loader == velora_shared::Loader::Fabric {
            let branding = state.manifest.read().unwrap().as_ref().map(|m| m.branding.clone()).unwrap_or_default();
            write_companion_preferences(&game_dir, &settings, &branding)?;
        }
    }

    if let Some(server) = &inst.server {
        if server.inject && !server.address.is_empty() {
            let name = if server.name.is_empty() { inst.name.clone() } else { server.name.clone() };
            servers_dat::upsert(&game_dir, &name, &servers_dat::format_address(&server.address, server.port)).ok();
        }
    }
    if !already_running && settings.vanilla_controls_enabled {
        let mut controls = std::collections::BTreeMap::new();
        controls.insert("autoJump".to_string(), settings.auto_jump.to_string());
        controls.insert("mouseSensitivity".to_string(), settings.sensitivity.clamp(0.0, 1.0).to_string());
        options::apply(&game_dir, &mut controls)?;
    }
    if !already_running && settings.keybinds_enabled && installed.version.is_modern() && !settings.keybinds.is_empty() {
        options::apply_keybinds(&game_dir, &settings.keybinds).ok();
    }

    if let Some(recorder) = &activity {
        recorder.background(if deep { "repair_complete" } else { "install_complete" }, &instance_id);
    }
    if !start {
        emit_state(&app, &instance_id, operation_id, "idle", Some("Instance is up to date".into()));
        return Ok(());
    }
    let (auth, auth_server) = auth.unwrap();

    // Panel accounts authenticate through the panel's Yggdrasil server via
    // authlib-injector; offline accounts launch without it.
    let agent_args = match &auth_server {
        Some(api) => {
            report(Event::Stage { stage: Stage::Launching, label: "Preparing sign-in".into() });
            let jar = velora_launcher_core::authlib::ensure(&state.http, &state.layout, Some(&panel))
                .await
                .context("setting up authlib-injector")?;
            let prefetched = velora_launcher_core::authlib::prefetch_metadata(&state.http, api).await.ok();
            velora_launcher_core::authlib::jvm_args(&jar, api, prefetched.as_deref())
        }
        None => vec![],
    };

    report(Event::Stage { stage: Stage::Launching, label: "Starting Minecraft".into() });
    let pick = |a: u32, b: u32, c: u32| {
        if a > 0 {
            a
        } else if b > 0 {
            b
        } else {
            c
        }
    };
    let max = pick(ov.memory_max_mb, settings.memory_max_mb, inst.memory.max_mb).max(512);
    let min = pick(ov.memory_min_mb, settings.memory_min_mb, inst.memory.min_mb).min(max);
    let mut jvm = launch::split_args(&inst.jvm_args);
    if advanced {
        jvm.extend(launch::split_args(&settings.jvm_args));
        jvm.extend(launch::split_args(&ov.jvm_args));
    }
    let branding = state.manifest.read().unwrap().as_ref().map(|m| m.branding.clone()).unwrap_or_default();
    let opts = LaunchOptions {
        auth: auth.clone(),
        agent_args,
        game_dir: game_dir.clone(),
        memory_min_mb: min,
        memory_max_mb: max,
        gc: settings.gc,
        extra_jvm_args: jvm,
        resolution: settings.custom_resolution.then_some((settings.width, settings.height)),
        fullscreen: settings.fullscreen,
        join_server: inst.server.as_ref().filter(|s| s.auto_join && !s.address.is_empty()).map(|s| (s.address.clone(), s.port)),
        version_label: if branding.version_label.is_empty() { branding.name.clone() } else { branding.version_label.clone() },
        launcher_name: "Velora".into(),
        launcher_version: env!("CARGO_PKG_VERSION").into(),
    };
    let cmd = launch::build(&installed, &state.layout, &opts);
    tracing::info!("launching: {} {}", cmd.java.display(), launch::redact(&cmd.args, &auth.access_token).join(" "));

    let close_after = settings.after_launch == "close";
    // Register the authenticated launch before Quick Play can reach the server.
    if let Some(recorder) = &activity {
        recorder.send("launch", &instance_id).await?;
    }
    let monitor_work = if close_after { None } else { Some(state.shutdown.enter().map_err(anyhow::Error::msg)?) };
    if state.shutdown.is_closing() {
        return Err(anyhow!("launcher is shutting down"));
    }
    let mut child = launch::spawn(&cmd, installed.java_major, !close_after).context("couldn't start Java")?;

    if close_after {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        app.exit(0);
        return Ok(());
    }

    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
    let run_id = uuid::Uuid::new_v4().to_string();
    state.game.lock().unwrap().push(RunningGame { run_id: run_id.clone(), instance_id: instance_id.clone(), kill: Some(kill_tx) });
    app.emit("launch://state", run_state(&instance_id, operation_id, &run_id, "running")).ok();

    if let Some(window) = app.get_webview_window("main") {
        match settings.after_launch.as_str() {
            "hide" => {
                window.hide().ok();
            }
            "minimize" => {
                window.minimize().ok();
            }
            _ => {}
        }
    }

    // Stream logs to the UI in small batches (cheap IPC), keep a tail for crash reports.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    for stream in [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let Ok(work) = state.shutdown.enter() else { continue };
        let tx = tx.clone();
        let task = tokio::spawn(async move {
            let _work = work;
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        state.shutdown.track_cancellable(task.abort_handle());
    }
    drop(tx);

    let app2 = app.clone();
    let id2 = instance_id.clone();
    let run_id2 = run_id.clone();
    let task = tokio::spawn(async move {
        let _work = monitor_work;
        let mut tail: VecDeque<String> = VecDeque::with_capacity(200);
        let mut batch: Vec<String> = Vec::new();
        let mut kill_rx = kill_rx;
        let mut tick = tokio::time::interval(Duration::from_millis(150));
        let mut streams_open = true;
        let mut kill_armed = true;
        let mut killed = false;
        let status = loop {
            tokio::select! {
                line = rx.recv(), if streams_open => match line {
                    Some(l) => {
                        if tail.len() == 200 { tail.pop_front(); }
                        tail.push_back(l.clone());
                        batch.push(l);
                    }
                    None => streams_open = false,
                },
                _ = tick.tick() => {
                    if !batch.is_empty() {
                        app2.emit("game://log", LogPayload { instance_id: id2.clone(), operation_id: operation_id.to_string(), run_id: run_id2.clone(), lines: std::mem::take(&mut batch) }).ok();
                    }
                },
                res = &mut kill_rx, if kill_armed => {
                    kill_armed = false;
                    if res.is_ok() {
                        killed = true;
                        child.kill().await.ok();
                    }
                },
                status = child.wait() => break status.ok(),
            }
        };
        // Drain whatever is left.
        while let Ok(l) = rx.try_recv() {
            tail.push_back(l.clone());
            batch.push(l);
        }
        if !batch.is_empty() {
            app2.emit(
                "game://log",
                LogPayload { instance_id: id2.clone(), operation_id: operation_id.to_string(), run_id: run_id2.clone(), lines: batch },
            )
            .ok();
        }
        let code = status.and_then(|s| s.code());
        let crashed = !killed && code.is_some_and(|c| c != 0);
        let crash_report = if crashed { newest_crash_report(&app2, &id2) } else { None };
        let st = app2.state::<AppState>();
        crate::telemetry::background(
            &st,
            if killed {
                "game_killed"
            } else if crashed {
                "game_crash"
            } else {
                "game_exit"
            },
            &id2,
        );
        let preferences = options::read_vanilla_preferences(&st.layout.instance_dir(&id2)).unwrap_or_default();
        if !preferences.is_empty() {
            if let Err(error) = st.update_settings(|settings| {
                settings.instance_game_options.insert(id2.clone(), preferences);
            }) {
                tracing::warn!("could not persist captured game preferences: {error:#}");
            }
        }
        st.game.lock().unwrap().retain(|g| g.run_id != run_id2);
        if st.game.lock().unwrap().is_empty() {
            if let Some(w) = app2.get_webview_window("main") {
                w.show().ok();
                w.unminimize().ok();
                if crashed {
                    w.set_focus().ok();
                }
            }
        }
        app2.emit(
            "game://exit",
            ExitPayload {
                instance_id: id2.clone(),
                run_id: run_id2.clone(),
                code,
                crashed,
                tail: tail.into_iter().collect(),
                crash_report,
            },
        )
        .ok();
        app2.emit("launch://state", run_state(&id2, operation_id, &run_id2, "idle")).ok();
        refresh_textures(&app2);
    });
    state.shutdown.track_cancellable(task.abort_handle());
    Ok(())
}

/// Launcher-owned presentation data. Contains no authentication tokens or URLs.
fn write_companion_preferences(
    game_dir: &std::path::Path,
    settings: &crate::settings::Settings,
    branding: &velora_shared::Branding,
) -> Result<()> {
    let mut prefs = serde_json::to_value(&settings.companion)?;
    prefs["brand"] = serde_json::json!(branding.name);
    prefs["accent"] = serde_json::json!(if settings.theme_mode == "custom" {
        settings.accent.as_ref().unwrap_or(&branding.colors.accent)
    } else {
        &branding.colors.accent
    });
    prefs["reduceMotion"] = serde_json::json!(settings.reduce_motion);
    let config = game_dir.join("config");
    std::fs::create_dir_all(&config)?;
    let temp = config.join("scopenet-companion.json.tmp");
    std::fs::write(&temp, serde_json::to_vec_pretty(&prefs)?)?;
    std::fs::rename(temp, config.join("scopenet-companion.json"))?;
    Ok(())
}

fn newest_crash_report(app: &AppHandle, id: &str) -> Option<String> {
    let dir = app.state::<AppState>().layout.instance_dir(id).join("crash-reports");
    let newest = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)?;
    // Only if it was written in the last few minutes (i.e. by this session).
    (newest.0.elapsed().ok()? < Duration::from_secs(300)).then(|| newest.1.to_string_lossy().into_owned())
}

#[cfg(test)]
mod operation_regression_tests {
    use super::{run_state, Event, ExitPayload, LogPayload, ProgressPayload, Stage, StatePayload};
    use crate::state::RunningGame;

    fn idle_payload() -> StatePayload {
        run_state("synthetic-alpha", uuid::Uuid::nil(), "synthetic-run-1", "idle")
    }

    #[test]
    fn operation_exit_payload_preserves_run_identity() {
        let payload = ExitPayload {
            instance_id: "synthetic-alpha".into(),
            run_id: "synthetic-run-1".into(),
            code: Some(0),
            crashed: false,
            tail: vec!["Synthetic game shutdown".into()],
            crash_report: None,
        };
        let encoded = serde_json::to_value(payload).unwrap();
        assert_eq!(encoded["instance_id"], "synthetic-alpha");
        assert_eq!(encoded["run_id"], "synthetic-run-1");
        assert_eq!(encoded["code"], 0);
        assert_eq!(encoded["crashed"], false);
    }

    #[test]
    fn operation_progress_payload_preserves_core_fields_and_owner_identity() {
        let encoded = serde_json::to_value(ProgressPayload {
            instance_id: "synthetic-alpha".into(),
            operation_id: uuid::Uuid::nil().to_string(),
            event: Event::Progress { stage: Stage::Files, done: 512, total: 1024, files_done: 1, files_total: 2 },
        })
        .unwrap();
        assert_eq!(encoded["type"], "progress");
        assert_eq!(encoded["stage"], "files");
        assert_eq!(encoded["done"], 512);
        assert_eq!(encoded["instance_id"], "synthetic-alpha");
        assert_eq!(encoded["operation_id"], uuid::Uuid::nil().to_string());
    }

    #[test]
    fn operation_old_exit_removes_only_its_run_and_idle_identifies_owner() {
        let mut games = vec![
            RunningGame { run_id: "synthetic-run-1".into(), instance_id: "synthetic-alpha".into(), kill: None },
            RunningGame { run_id: "synthetic-run-2".into(), instance_id: "synthetic-alpha".into(), kill: None },
        ];
        games.retain(|game| game.run_id != "synthetic-run-1");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].run_id, "synthetic-run-2");
        let encoded = serde_json::to_value(idle_payload()).unwrap();
        assert_eq!(encoded["state"], "idle");
        assert_eq!(encoded["run_id"], "synthetic-run-1");
        assert_eq!(encoded["operation_id"], uuid::Uuid::nil().to_string());
    }

    #[test]
    fn operation_terminal_state_should_identify_the_finished_run() {
        let encoded = serde_json::to_value(idle_payload()).unwrap();
        assert_eq!(encoded["run_id"], "synthetic-run-1", "a delayed exit must be distinguishable from a newer run");
    }

    #[test]
    fn operation_log_payload_identifies_instance_operation_and_run() {
        let encoded = serde_json::to_value(LogPayload {
            instance_id: "synthetic-alpha".into(),
            operation_id: uuid::Uuid::nil().to_string(),
            run_id: "synthetic-run-1".into(),
            lines: vec!["synthetic log".into()],
        })
        .unwrap();
        assert_eq!(encoded["run_id"], "synthetic-run-1");
        assert_eq!(encoded["instance_id"], "synthetic-alpha");
        assert_eq!(encoded["operation_id"], uuid::Uuid::nil().to_string());
        assert_eq!(encoded["lines"][0], "synthetic log");
    }
}
