//! The Play button: install → sync → configure → start → watch.

use crate::accounts;
use crate::state::{AppState, RunningGame};
use anyhow::{anyhow, Context, Result};
use velora_launcher_core::install::{self, InstallSpec};
use velora_launcher_core::launch::{self, LaunchOptions};
use velora_launcher_core::progress::{Event, Reporter, Stage};
use velora_launcher_core::{options, servers_dat, sync};
use velora_shared::InstanceManifest;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Serialize, Clone)]
pub struct StatePayload {
    pub instance_id: String,
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
    let app = app.clone();
    std::thread::spawn(move || {
        let layout = app.state::<AppState>().layout.clone();
        match velora_launcher_core::textures::ensure(&layout) {
            Ok(true) => tracing::info!("extracted item textures from the client jar"),
            Ok(false) => {}
            Err(e) => tracing::warn!("couldn't read textures from the client jar: {e:#}"),
        }
        app.emit("textures://status", velora_launcher_core::textures::status(&layout)).ok();
    });
}

pub fn emit_state(app: &AppHandle, id: &str, state: &str, message: Option<String>) {
    app.emit("launch://state", StatePayload { instance_id: id.into(), run_id: None, state: state.into(), message }).ok();
}

/// Fetch the instance definition, falling back to the last copy we saw so
/// the game still starts without internet.
async fn instance_manifest(state: &AppState, id: &str) -> Result<InstanceManifest> {
    let cache = state.layout.instance_dir(id).join(".scopenet").join("manifest.json");
    let fetched = async {
        let panel = state.panel_url().ok_or_else(|| anyhow!("no panel configured"))?;
        let mut req = state.http.get(format!("{panel}/api/v1/launcher/instances/{id}"));
        if let Some(token) = accounts::panel_token(state) {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await?;
        if resp.status().as_u16() == 404 {
            anyhow::bail!("this instance no longer exists or you don't have access to it");
        }
        Ok::<_, anyhow::Error>(resp.error_for_status()?.json::<InstanceManifest>().await?)
    }
    .await;
    match fetched {
        Ok(m) => {
            std::fs::create_dir_all(cache.parent().unwrap()).ok();
            std::fs::write(&cache, serde_json::to_vec(&m)?).ok();
            Ok(m)
        }
        Err(e) => {
            let bytes = std::fs::read(&cache).map_err(|_| e)?;
            tracing::warn!("panel unreachable, using cached instance manifest");
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

fn reporter(app: &AppHandle) -> Reporter {
    let app = app.clone();
    Arc::new(move |ev: Event| {
        app.emit("launch://progress", ev).ok();
    })
}

/// Install/verify an instance and (when `start`) run the game.
pub async fn run(app: AppHandle, instance_id: String, start: bool, deep: bool) -> Result<()> {
    let state = app.state::<AppState>();
    let report = reporter(&app);
    let activity = crate::telemetry::capture(&state);
    if let Some(recorder) = &activity {
        recorder.background(if deep { "repair_start" } else { "install_start" }, &instance_id);
    }
    emit_state(&app, &instance_id, "preparing", None);

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
        sync::sync(&state.http, &panel, &game_dir, inst.revision, inst.clean_epoch, &manifest.files, spec.concurrency, deep, &report).await?;
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
        emit_state(&app, &instance_id, "idle", Some("Instance is up to date".into()));
        return Ok(());
    }
    let (auth, auth_server) = auth.unwrap();

    // Panel accounts authenticate through the panel's Yggdrasil server via
    // authlib-injector; offline accounts launch without it.
    let agent_args = match &auth_server {
        Some(api) => {
            report(Event::Stage { stage: Stage::Launching, label: "Preparing sign-in".into() });
            let jar =
                velora_launcher_core::authlib::ensure(&state.http, &state.layout, Some(&panel)).await.context("setting up authlib-injector")?;
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
    let mut child = launch::spawn(&cmd, installed.java_major, !close_after).context("couldn't start Java")?;

    if close_after {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        app.exit(0);
        return Ok(());
    }

    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
    let run_id = uuid::Uuid::new_v4().to_string();
    state.game.lock().unwrap().push(RunningGame { run_id: run_id.clone(), instance_id: instance_id.clone(), kill: Some(kill_tx) });
    app.emit(
        "launch://state",
        StatePayload { instance_id: instance_id.clone(), run_id: Some(run_id.clone()), state: "running".into(), message: None },
    )
    .ok();

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
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let app2 = app.clone();
    let id2 = instance_id.clone();
    let run_id2 = run_id.clone();
    tokio::spawn(async move {
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
                        app2.emit("game://log", std::mem::take(&mut batch)).ok();
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
            app2.emit("game://log", batch).ok();
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
            st.settings.write().unwrap().instance_game_options.insert(id2.clone(), preferences);
            st.save_settings().ok();
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
            ExitPayload { instance_id: id2.clone(), run_id: run_id2, code, crashed, tail: tail.into_iter().collect(), crash_report },
        )
        .ok();
        emit_state(&app2, &id2, "idle", None);
        refresh_textures(&app2);
    });
    Ok(())
}

/// Launcher-owned presentation data. Contains no authentication tokens or URLs.
fn write_companion_preferences(game_dir: &std::path::Path, settings: &crate::settings::Settings, branding: &velora_shared::Branding) -> Result<()> {
    let mut prefs = serde_json::to_value(&settings.companion)?;
    prefs["brand"] = serde_json::json!(branding.name);
    prefs["accent"] = serde_json::json!(if settings.theme_mode == "custom" { settings.accent.as_ref().unwrap_or(&branding.colors.accent) } else { &branding.colors.accent });
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
