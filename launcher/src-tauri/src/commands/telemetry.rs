use super::{aerr, Res};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn record_activity(state: State<'_, AppState>, kind: String, instance_id: Option<String>) -> Res<()> {
    crate::telemetry::send(&state, &kind, instance_id.as_deref().unwrap_or("")).await.map_err(aerr)
}
