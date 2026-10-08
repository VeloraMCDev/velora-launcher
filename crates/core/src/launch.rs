//! Building the `java ...` command line and starting the game.

use crate::install::Installed;
use crate::paths::Layout;
use crate::rules::{self, Env};
use crate::version::Arg;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GcPreset {
    /// Let the JVM decide.
    Default,
    /// Tuned G1 (Aikar-style) — smooth frame times on most machines.
    #[default]
    G1,
    /// ZGC — lowest pauses, needs Java 17+ and a bit more RAM.
    Zgc,
}

#[derive(Debug, Clone)]
pub struct Auth {
    pub username: String,
    /// Dashed or undashed UUID.
    pub uuid: String,
    pub access_token: String,
    /// "mojang" for panel (Yggdrasil) accounts, "legacy" for offline.
    pub user_type: String,
}

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub auth: Auth,
    /// JVM arguments that must come first (the authlib-injector agent).
    pub agent_args: Vec<String>,
    pub game_dir: PathBuf,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub gc: GcPreset,
    pub extra_jvm_args: Vec<String>,
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: bool,
    /// Join this server right after the game starts.
    pub join_server: Option<(String, u16)>,
    pub version_label: String,
    pub launcher_name: String,
    pub launcher_version: String,
}

/// Split a user-entered argument string, honouring quotes.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;
    for c in s.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"') | (None, '\'') => {
                quote = Some(c);
                has_token = true;
            }
            (None, c) if c.is_whitespace() => {
                if has_token || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if has_token || !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn gc_args(preset: GcPreset, java_major: u32) -> Vec<String> {
    let g1 = [
        "-XX:+UseG1GC",
        "-XX:+ParallelRefProcEnabled",
        "-XX:MaxGCPauseMillis=200",
        "-XX:+UnlockExperimentalVMOptions",
        "-XX:+DisableExplicitGC",
        "-XX:+AlwaysPreTouch",
        "-XX:G1NewSizePercent=30",
        "-XX:G1MaxNewSizePercent=40",
        "-XX:G1HeapRegionSize=8M",
        "-XX:G1ReservePercent=20",
        "-XX:G1HeapWastePercent=5",
        "-XX:G1MixedGCCountTarget=4",
        "-XX:InitiatingHeapOccupancyPercent=15",
        "-XX:G1MixedGCLiveThresholdPercent=90",
        "-XX:G1RSetUpdatingPauseTimePercent=5",
        "-XX:SurvivorRatio=32",
        "-XX:+PerfDisableSharedMem",
        "-XX:MaxTenuringThreshold=1",
    ];
    match preset {
        GcPreset::Default => vec![],
        GcPreset::Zgc if java_major >= 17 => {
            let mut v = vec!["-XX:+UseZGC".to_string()];
            // Generational ZGC: opt-in on 21-22, default (flag obsolete) from 23.
            if (21..23).contains(&java_major) {
                v.push("-XX:+ZGenerational".into());
            }
            v
        }
        _ => g1.iter().map(|s| s.to_string()).collect(),
    }
}

fn substitute(arg: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = String::with_capacity(arg.len());
    let mut rest = arg;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match vars.get(key) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push_str("${");
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn expand(args: &[Arg], env: &Env, vars: &HashMap<&str, String>) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            Arg::Plain(s) => out.push(substitute(s, vars)),
            Arg::Ruled { rules: r, value } => {
                if rules::allowed(r, env) {
                    out.extend(value.values().iter().map(|s| substitute(s, vars)));
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct Command {
    pub java: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

pub fn build(installed: &Installed, layout: &Layout, opts: &LaunchOptions) -> Command {
    let v = &installed.version;
    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath = installed.classpath(layout).iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>().join(sep);

    let quick_play = v.supports_quick_play();
    let env = Env::current()
        .with_feature("has_custom_resolution", opts.resolution.is_some())
        .with_feature("is_quick_play_multiplayer", quick_play && opts.join_server.is_some());

    let uuid = opts.auth.uuid.replace('-', "");
    let (w, h) = opts.resolution.unwrap_or((854, 480));
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", opts.auth.username.clone());
    vars.insert("version_name", v.id.clone());
    vars.insert("game_directory", opts.game_dir.to_string_lossy().into_owned());
    vars.insert("assets_root", installed.assets_root.to_string_lossy().into_owned());
    vars.insert("game_assets", installed.game_assets.to_string_lossy().into_owned());
    vars.insert("assets_index_name", installed.assets_index.clone());
    vars.insert("auth_uuid", uuid.clone());
    vars.insert("auth_access_token", opts.auth.access_token.clone());
    vars.insert("auth_session", format!("token:{}:{}", opts.auth.access_token, uuid));
    vars.insert("clientid", String::new());
    vars.insert("auth_xuid", String::new());
    vars.insert("user_type", opts.auth.user_type.clone());
    vars.insert("user_properties", "{}".into());
    vars.insert("version_type", opts.version_label.clone());
    vars.insert("resolution_width", w.to_string());
    vars.insert("resolution_height", h.to_string());
    vars.insert("natives_directory", installed.natives_dir.to_string_lossy().into_owned());
    vars.insert("launcher_name", opts.launcher_name.clone());
    vars.insert("launcher_version", opts.launcher_version.clone());
    vars.insert("classpath", classpath);
    vars.insert("classpath_separator", sep.into());
    vars.insert("library_directory", layout.libraries().to_string_lossy().into_owned());
    if let Some((host, port)) = &opts.join_server {
        vars.insert("quickPlayMultiplayer", format!("{host}:{port}"));
    }

    let mut args = opts.agent_args.clone();
    args.push(format!("-Xms{}M", opts.memory_min_mb.min(opts.memory_max_mb)));
    args.push(format!("-Xmx{}M", opts.memory_max_mb));
    args.extend(gc_args(opts.gc, installed.java_major));
    args.push("-Dlog4j2.formatMsgNoLookups=true".into());
    if let Some(log) = &installed.logging_arg {
        args.push(log.clone());
    }
    args.extend(opts.extra_jvm_args.iter().cloned());

    match &v.arguments {
        Some(a) if !a.jvm.is_empty() => args.extend(expand(&a.jvm, &env, &vars)),
        _ => {
            args.push(format!("-Djava.library.path={}", vars["natives_directory"]));
            args.push("-cp".into());
            args.push(vars["classpath"].clone());
        }
    }

    args.push(v.main_class.clone().unwrap_or_else(|| "net.minecraft.client.main.Main".into()));

    let mut game_args = match (&v.arguments, &v.minecraft_arguments) {
        (_, Some(legacy)) if v.arguments.as_ref().map(|a| a.game.is_empty()).unwrap_or(true) => {
            legacy.split_whitespace().map(|s| substitute(s, &vars)).collect()
        }
        (Some(a), _) => expand(&a.game, &env, &vars),
        _ => vec![],
    };

    let has = |args: &[String], flag: &str| args.iter().any(|a| a == flag);
    if let Some((w, h)) = opts.resolution {
        if !has(&game_args, "--width") {
            game_args.extend(["--width".into(), w.to_string(), "--height".into(), h.to_string()]);
        }
    }
    if opts.fullscreen && !has(&game_args, "--fullscreen") {
        game_args.push("--fullscreen".into());
    }
    if let Some((host, port)) = &opts.join_server {
        if !quick_play {
            game_args.extend(["--server".into(), host.clone(), "--port".into(), port.to_string()]);
        }
    }
    args.extend(game_args);

    Command { java: installed.java.clone(), args, cwd: opts.game_dir.clone() }
}

/// Java 9+ can read arguments from a file, which sidesteps Windows'
/// 32k command-line limit on huge modpacks.
fn write_argfile(path: &Path, args: &[String]) -> Result<()> {
    let body = args.iter().map(|a| format!("\"{}\"", a.replace('\\', "\\\\").replace('"', "\\\""))).collect::<Vec<_>>().join("\n");
    std::fs::write(path, body)?;
    Ok(())
}

/// Start the game. With `capture` the game's stdout/stderr are piped back to
/// us (for the console); without it they're discarded, which is required
/// when the launcher exits while the game keeps running.
pub fn spawn(cmd: &Command, java_major: u32, capture: bool) -> Result<tokio::process::Child> {
    std::fs::create_dir_all(&cmd.cwd)?;
    let mut proc = tokio::process::Command::new(&cmd.java);
    let total_len: usize = cmd.args.iter().map(|a| a.len() + 3).sum();
    if java_major >= 9 && total_len > 8000 {
        let argfile = cmd.cwd.join(".scopenet").join("launch.args");
        std::fs::create_dir_all(argfile.parent().unwrap())?;
        write_argfile(&argfile, &cmd.args)?;
        proc.arg(format!("@{}", argfile.display()));
    } else {
        proc.args(&cmd.args);
    }
    let out = || if capture { std::process::Stdio::piped() } else { std::process::Stdio::null() };
    proc.current_dir(&cmd.cwd).stdin(std::process::Stdio::null()).stdout(out()).stderr(out());
    #[cfg(windows)]
    proc.creation_flags(0x0800_0000);
    proc.spawn().with_context(|| format!("starting {}", cmd.java.display()))
}

/// Hide the access token when showing the command to the user.
pub fn redact(args: &[String], token: &str) -> Vec<String> {
    if token.len() < 8 {
        return args.to_vec();
    }
    args.iter().map(|a| a.replace(token, "********")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::VersionJson;

    #[test]
    fn splits_quoted_args() {
        assert_eq!(split_args(r#"-Xss2M  -Dfoo="a b" '' -Dx=1"#), vec!["-Xss2M", "-Dfoo=a b", "", "-Dx=1"]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn substitutes_vars() {
        let mut vars = HashMap::new();
        vars.insert("a", "1".to_string());
        assert_eq!(substitute("x${a}y${b}z", &vars), "x1y${b}z");
    }

    #[test]
    fn zgc_falls_back_on_java8() {
        assert!(gc_args(GcPreset::Zgc, 8).contains(&"-XX:+UseG1GC".to_string()));
        assert_eq!(gc_args(GcPreset::Zgc, 21), vec!["-XX:+UseZGC", "-XX:+ZGenerational"]);
        assert_eq!(gc_args(GcPreset::Zgc, 25), vec!["-XX:+UseZGC"]);
    }

    fn installed(json: &str) -> Installed {
        Installed {
            version: serde_json::from_str::<VersionJson>(json).unwrap(),
            java: "java".into(),
            java_major: 17,
            client_jar: "/v/c.jar".into(),
            natives_dir: "/n".into(),
            libraries: vec![],
            assets_root: "/a".into(),
            game_assets: "/a".into(),
            assets_index: "5".into(),
            logging_arg: None,
        }
    }

    fn opts(join: bool) -> LaunchOptions {
        LaunchOptions {
            auth: Auth {
                username: "Steve".into(),
                uuid: "b50ad385-829d-3141-a216-7e7d7539ba7f".into(),
                access_token: "0".into(),
                user_type: "legacy".into(),
            },
            agent_args: vec!["-javaagent:/a.jar=https://p/api/yggdrasil".into()],
            game_dir: "/g".into(),
            memory_min_mb: 1024,
            memory_max_mb: 4096,
            gc: GcPreset::Default,
            extra_jvm_args: vec!["-Dcustom=1".into()],
            resolution: Some((1280, 720)),
            fullscreen: false,
            join_server: join.then(|| ("play.example.net".to_string(), 25565)),
            version_label: "SCOPENET".into(),
            launcher_name: "scopenet".into(),
            launcher_version: "0.1.0".into(),
        }
    }

    #[test]
    fn builds_modern_command_with_quick_play() {
        let json = r#"{"id":"1.20.1","mainClass":"net.minecraft.client.main.Main","arguments":{
          "game":["--username","${auth_player_name}","--uuid","${auth_uuid}","--versionType","${version_type}",
            {"rules":[{"action":"allow","features":{"has_custom_resolution":true}}],"value":["--width","${resolution_width}","--height","${resolution_height}"]},
            {"rules":[{"action":"allow","features":{"is_quick_play_multiplayer":true}}],"value":["--quickPlayMultiplayer","${quickPlayMultiplayer}"]}],
          "jvm":["-Djava.library.path=${natives_directory}","-cp","${classpath}"]}}"#;
        let layout = Layout::new("/root");
        let cmd = build(&installed(json), &layout, &opts(true));
        let a = cmd.args.join(" ");
        assert!(a.starts_with("-javaagent:/a.jar=https://p/api/yggdrasil -Xms1024M -Xmx4096M"), "agent must come first");
        assert!(a.contains("-Dcustom=1"));
        assert!(a.contains("--uuid b50ad385829d3141a2167e7d7539ba7f"));
        assert!(a.contains("--width 1280 --height 720"));
        assert!(a.contains("--quickPlayMultiplayer play.example.net:25565"));
        assert!(!a.contains("--server"));
        assert!(a.contains("--versionType SCOPENET"));
        assert!(a.contains("-Djava.library.path=/n"));
    }

    #[test]
    fn builds_legacy_command_with_server_flag() {
        let json = r#"{"id":"1.12.2","mainClass":"net.minecraft.launchwrapper.Launch",
          "minecraftArguments":"--username ${auth_player_name} --tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker"}"#;
        let cmd = build(&installed(json), &Layout::new("/root"), &opts(true));
        let a = cmd.args.join(" ");
        assert!(a.contains("-Djava.library.path=/n -cp /v/c.jar net.minecraft.launchwrapper.Launch --username Steve"));
        assert!(a.contains("--width 1280 --height 720"));
        assert!(a.ends_with("--server play.example.net --port 25565"));
    }
}
