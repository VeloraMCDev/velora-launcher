use super::{err, Res};
use crate::game;
use crate::state::AppState;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn launch(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Res<String> {
    start_task(app, &state, instance_id, true, false)
}

#[tauri::command]
pub fn repair_instance(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Res<String> {
    start_task(app, &state, instance_id, false, true)
}

fn start_task(app: AppHandle, state: &AppState, instance_id: String, start: bool, deep: bool) -> Res<String> {
    let completion_work = state.shutdown.enter().map_err(|message| message.to_owned())?;
    let lease = state.mutation_lease().map_err(|message| message.to_owned())?;
    if deep && state.game.lock().unwrap().iter().any(|g| g.instance_id == instance_id) {
        return Err("Close this instance's game windows before repairing it".into());
    }
    let worker_app = app.clone();
    let worker_instance = instance_id.clone();
    let (operation_id, worker) = state
        .preparations
        .lock()
        .unwrap()
        .start(instance_id.clone(), |operation_id| {
            let worker = tauri::async_runtime::spawn(async move {
                let _lease = lease;
                game::run(worker_app, worker_instance, operation_id, start, deep).await
            });
            (worker.inner().abort_handle(), worker)
        })
        .map_err(|message| message.to_owned())?;
    tauri::async_runtime::spawn(async move {
        let _work = completion_work;
        let outcome = worker.await;
        let state = app.state::<AppState>();
        let cancelled = matches!(&outcome, Err(tauri::Error::JoinError(error)) if error.is_cancelled())
            && state.preparations.lock().unwrap().is_cancelled(operation_id);
        let failure = match outcome {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(format!("{error:#}")),
            Err(_) if cancelled => None,
            Err(error) => Some(format!("{error:#}")),
        };
        if let Some(message) = &failure {
            crate::telemetry::background(&state, "launch_failed", &instance_id);
            tracing::error!("launch failed: {message}");
        }
        let owned = state.preparations.lock().unwrap().owns(operation_id);
        if owned {
            if let Some(message) = failure {
                game::emit_state(&app, &instance_id, operation_id, "error", Some(message));
            } else if cancelled {
                game::emit_state(&app, &instance_id, operation_id, "idle", Some("Cancelled".into()));
            }
            state.preparations.lock().unwrap().finish(operation_id);
        }
    });
    Ok(operation_id.to_string())
}

#[tauri::command]
pub fn cancel_launch(state: State<'_, AppState>, instance_id: String, operation_id: String) -> Res<()> {
    let operation_id = uuid::Uuid::parse_str(&operation_id).map_err(err)?;
    state.preparations.lock().unwrap().cancel(&instance_id, operation_id);
    Ok(())
}

#[tauri::command]
pub fn kill_game(state: State<'_, AppState>, run_id: String) {
    if let Some(g) = state.game.lock().unwrap().iter_mut().find(|g| g.run_id == run_id) {
        if let Some(kill) = g.kill.take() {
            kill.send(()).ok();
        }
    }
}
