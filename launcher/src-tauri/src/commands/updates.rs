use super::{aerr, Res};
use crate::state::AppState;
use crate::updater;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn check_update(state: State<'_, AppState>) -> Res<Option<updater::UpdateInfo>> {
    updater::check(&state.http, state.panel_url().as_deref()).await.map_err(aerr)
}

#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>, url: String) -> Res<()> {
    let _lease = state.mutation_lease().map_err(|message| message.to_owned())?;
    let update = updater::check(&state.http, state.panel_url().as_deref()).await.map_err(aerr)?.ok_or("No launcher update is available")?;
    if update.url != url {
        return Err("The available release changed. Check for updates again.".into());
    }
    updater::download_and_run(&state.http, &update).await.map_err(aerr)?;
    app.exit(0);
    Ok(())
}
