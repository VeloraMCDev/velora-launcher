use super::{aerr, err, Res};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use velora_launcher_core::ping::ServerStatus;

#[derive(Serialize)]
pub struct InstanceLocal {
    installed: bool,
    revision: i64,
    size: u64,
}

#[tauri::command]
pub async fn instance_local(state: State<'_, AppState>, instance_id: String) -> Res<InstanceLocal> {
    let work = state.shutdown.enter().map_err(|message| message.to_owned())?;
    let dir = state.layout.instance_dir(&instance_id);
    let s = velora_launcher_core::sync::load_state(&dir);
    let size = tokio::task::spawn_blocking(move || {
        let _work = work;
        velora_launcher_core::sys::dir_size(&dir)
    })
    .await
    .unwrap_or(0);
    Ok(InstanceLocal { installed: s.revision > 0, revision: s.revision, size })
}

#[tauri::command]
pub async fn ping_server(host: String, port: u16) -> Res<ServerStatus> {
    velora_launcher_core::ping::ping(&host, port).await.map_err(aerr)
}

#[tauri::command]
pub fn open_folder(app: AppHandle, state: State<'_, AppState>, kind: String, instance_id: Option<String>) -> Res<()> {
    let _work = state.shutdown.enter().map_err(|message| message.to_owned())?;
    let base = match &instance_id {
        Some(id) => state.layout.instance_dir(id),
        None => state.data_dir.clone(),
    };
    let path = match kind.as_str() {
        "instance" | "data" => base,
        "logs" | "screenshots" | "mods" | "resourcepacks" | "shaderpacks" | "saves" | "crash-reports" => base.join(&kind),
        _ => return Err("unknown folder".into()),
    };
    std::fs::create_dir_all(&path).map_err(err)?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
pub fn open_file(app: AppHandle, path: String) -> Res<()> {
    app.opener().open_path(path, None::<&str>).map_err(err)
}

#[derive(Serialize)]
pub struct Storage {
    shared: u64,
    runtimes: u64,
    instances: Vec<(String, u64)>,
}

#[tauri::command]
pub async fn storage_info(state: State<'_, AppState>) -> Res<Storage> {
    let work = state.shutdown.enter().map_err(|message| message.to_owned())?;
    let layout = state.layout.clone();
    tokio::task::spawn_blocking(move || {
        let _work = work;
        use velora_launcher_core::sys::dir_size;
        let shared = dir_size(&layout.libraries()) + dir_size(&layout.assets()) + dir_size(&layout.versions());
        let runtimes = dir_size(&layout.runtimes());
        let instances = std::fs::read_dir(layout.instances())
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| (e.file_name().to_string_lossy().into_owned(), dir_size(&e.path()))).collect())
            .unwrap_or_default();
        Storage { shared, runtimes, instances }
    })
    .await
    .map_err(err)
}

#[tauri::command]
pub async fn delete_instance_data(state: State<'_, AppState>, instance_id: String) -> Res<()> {
    let lease = state.mutation_lease().map_err(|message| message.to_owned())?;
    if state.game.lock().unwrap().iter().any(|g| g.instance_id == instance_id) {
        return Err("close the game first".into());
    }
    let dir = state.layout.instance_dir(&instance_id);
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        std::fs::remove_dir_all(dir).or_else(|error| if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) })
    })
    .await
    .map_err(err)?
    .map_err(err)
}

#[tauri::command]
pub async fn clear_cache(state: State<'_, AppState>) -> Res<()> {
    let lease = state.mutation_lease().map_err(|message| message.to_owned())?;
    let cache = state.layout.cache();
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        std::fs::remove_dir_all(cache).or_else(|error| if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) })
    })
    .await
    .map_err(err)?
    .map_err(err)
}

#[tauri::command]
pub async fn detect_java(path: String) -> Option<u32> {
    velora_launcher_core::java::detect_major(std::path::Path::new(&path)).await
}
