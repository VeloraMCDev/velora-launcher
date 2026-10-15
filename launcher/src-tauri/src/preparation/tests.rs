use super::{mutation_lease, PreparationManager};
use std::sync::{Arc, Mutex};
use tokio::sync::{oneshot, Mutex as AsyncMutex, Notify};
use tokio::task::{AbortHandle, JoinHandle};

struct WaitingTask {
    handle: JoinHandle<()>,
    started: oneshot::Receiver<()>,
    release: Arc<Notify>,
}

fn spawn_waiting(_operation_id: uuid::Uuid) -> (AbortHandle, WaitingTask) {
    let (started_sender, started) = oneshot::channel();
    let release = Arc::new(Notify::new());
    let task_release = Arc::clone(&release);
    let handle = tokio::spawn(async move {
        started_sender.send(()).expect("test awaits task startup");
        task_release.notified().await;
    });
    (handle.abort_handle(), WaitingTask { handle, started, release })
}

#[tokio::test]
async fn wrong_instance_cancel_preserves_active_preparation() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();

    assert!(!preparations.cancel("beta", operation_id));
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_busy());
    assert!(!preparations.is_cancelled(operation_id));
    assert!(!task.handle.is_finished());

    task.release.notify_one();
    task.handle.await.unwrap();
    assert!(preparations.finish(operation_id));
}

#[tokio::test]
async fn cancellation_retains_slot_before_join_and_until_finish() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();

    assert!(preparations.cancel("alpha", operation_id));
    assert!(preparations.is_cancelled(operation_id));
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_busy());
    assert!(preparations.start("beta".into(), spawn_waiting).is_err());

    assert!(task.handle.await.unwrap_err().is_cancelled());
    assert!(preparations.is_busy());
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_cancelled(operation_id));
    assert!(preparations.finish(operation_id));
    assert!(!preparations.is_busy());
    assert!(!preparations.owns(operation_id));
    assert!(!preparations.is_cancelled(operation_id));
}

#[tokio::test]
async fn repeated_cancellation_is_a_no_op() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();

    assert!(preparations.cancel("alpha", operation_id));
    assert!(!preparations.cancel("alpha", operation_id));
    assert!(preparations.is_cancelled(operation_id));
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_busy());
    assert!(task.handle.await.unwrap_err().is_cancelled());
    assert!(!preparations.cancel("alpha", operation_id));
    assert!(preparations.finish(operation_id));
}

#[tokio::test]
async fn matching_owner_can_cancel_an_unfinished_task() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();

    assert!(!task.handle.is_finished());
    assert!(preparations.cancel("alpha", operation_id));
    assert!(task.handle.await.unwrap_err().is_cancelled());
    assert!(preparations.is_cancelled(operation_id));
    assert!(preparations.finish(operation_id));
}

#[tokio::test]
async fn premature_finish_preserves_the_unfinished_owner() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();

    assert!(!task.handle.is_finished());
    assert!(!preparations.finish(operation_id));
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_busy());
    assert!(!preparations.is_cancelled(operation_id));

    task.release.notify_one();
    task.handle.await.unwrap();
    assert!(preparations.finish(operation_id));
    assert!(!preparations.is_busy());
}

#[tokio::test]
async fn normal_finish_clears_only_the_current_owner() {
    let mut preparations = PreparationManager::default();
    assert!(!preparations.is_busy());
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();
    task.release.notify_one();
    task.handle.await.unwrap();

    assert!(!preparations.is_cancelled(operation_id));
    assert!(preparations.finish(operation_id));
    assert!(!preparations.is_busy());
    assert!(!preparations.owns(operation_id));
    assert!(!preparations.finish(operation_id));
}

#[tokio::test]
async fn stale_finish_cannot_remove_a_new_owner_for_the_same_instance() {
    let mut preparations = PreparationManager::default();
    let (previous_id, previous_task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    previous_task.started.await.unwrap();
    previous_task.release.notify_one();
    previous_task.handle.await.unwrap();
    assert!(preparations.finish(previous_id));

    let (current_id, current_task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    current_task.started.await.unwrap();
    current_task.release.notify_one();
    current_task.handle.await.unwrap();
    assert_ne!(previous_id, current_id);
    assert!(!preparations.finish(previous_id));
    assert!(!preparations.owns(previous_id));
    assert!(preparations.owns(current_id));
    assert!(preparations.is_busy());
    assert!(!preparations.is_cancelled(current_id));

    assert!(preparations.finish(current_id));
}

#[tokio::test]
async fn busy_admission_does_not_invoke_the_spawn_closure() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();

    let denied = preparations.start::<()>("beta".into(), |_operation_id| panic!("busy admission must not invoke the spawn closure"));
    assert!(denied.is_err());
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_busy());

    task.release.notify_one();
    task.handle.await.unwrap();
    assert!(preparations.finish(operation_id));
}

#[tokio::test]
async fn finished_handle_stays_busy_until_explicit_finalization() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();
    task.release.notify_one();
    task.handle.await.unwrap();

    assert!(preparations.is_busy());
    assert!(preparations.owns(operation_id));
    assert!(preparations
        .start::<()>("beta".into(), |_operation_id| { panic!("finished but unfinalized owner still denies admission") })
        .is_err());
    assert!(preparations.finish(operation_id));

    let (next_id, next_task) = preparations.start("beta".into(), spawn_waiting).unwrap();
    next_task.started.await.unwrap();
    next_task.release.notify_one();
    next_task.handle.await.unwrap();
    assert!(preparations.finish(next_id));
}

#[tokio::test]
async fn empty_and_finished_cancellation_are_no_ops() {
    let mut preparations = PreparationManager::default();
    let unknown_id = uuid::Uuid::new_v4();
    assert!(!preparations.cancel("alpha", unknown_id));
    assert!(!preparations.is_busy());
    assert!(!preparations.owns(unknown_id));
    assert!(!preparations.is_cancelled(unknown_id));
    assert!(!preparations.finish(unknown_id));

    let (operation_id, task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();
    task.release.notify_one();
    task.handle.await.unwrap();
    assert!(!preparations.cancel("alpha", operation_id));
    assert!(!preparations.is_cancelled(operation_id));
    assert!(preparations.is_busy());
    assert!(preparations.owns(operation_id));
    assert!(preparations.finish(operation_id));
}

#[tokio::test]
async fn stale_cancel_cannot_abort_a_replacement_for_the_same_instance() {
    let mut preparations = PreparationManager::default();
    let (previous_id, previous_task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    previous_task.started.await.unwrap();
    assert!(preparations.cancel("alpha", previous_id));
    assert!(previous_task.handle.await.unwrap_err().is_cancelled());
    assert!(preparations.finish(previous_id));

    let (current_id, current_task) = preparations.start("alpha".into(), spawn_waiting).unwrap();
    current_task.started.await.unwrap();
    assert_ne!(previous_id, current_id);
    assert!(!preparations.cancel("alpha", previous_id));
    assert!(!preparations.cancel("alpha", uuid::Uuid::new_v4()));
    assert!(preparations.owns(current_id));
    assert!(!preparations.is_cancelled(current_id));
    assert!(!current_task.handle.is_finished());
    current_task.release.notify_one();
    current_task.handle.await.unwrap();
    assert!(preparations.finish(current_id));
}

#[tokio::test]
async fn global_mutation_lease_excludes_other_resource_users() {
    let preparations = Mutex::new(PreparationManager::default());
    let resource = Arc::new(AsyncMutex::new(()));

    let lease = mutation_lease(&resource, &preparations).unwrap();
    assert!(mutation_lease(&resource, &preparations).is_err());
    assert!(Arc::clone(&resource).try_lock_owned().is_err());
    drop(lease);

    let resource_guard = Arc::clone(&resource).try_lock_owned().unwrap();
    assert!(mutation_lease(&resource, &preparations).is_err());
    drop(resource_guard);
    assert!(mutation_lease(&resource, &preparations).is_ok());
}

#[tokio::test]
async fn busy_registry_denies_mutation_even_after_resource_guard_is_dropped() {
    let preparations = Mutex::new(PreparationManager::default());
    let resource = Arc::new(AsyncMutex::new(()));
    let resource_guard = Arc::clone(&resource).try_lock_owned().unwrap();
    let (operation_id, task) = preparations.lock().unwrap().start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();

    assert!(mutation_lease(&resource, &preparations).is_err());
    drop(resource_guard);
    assert!(mutation_lease(&resource, &preparations).is_err());
    assert!(Arc::clone(&resource).try_lock_owned().is_ok());

    assert!(preparations.lock().unwrap().cancel("alpha", operation_id));
    assert!(task.handle.await.unwrap_err().is_cancelled());
    assert!(mutation_lease(&resource, &preparations).is_err());
    assert!(preparations.lock().unwrap().finish(operation_id));
    assert!(mutation_lease(&resource, &preparations).is_ok());
}

#[tokio::test]
async fn finished_unfinalized_registry_denies_mutation_without_leaking_resource() {
    let preparations = Mutex::new(PreparationManager::default());
    let resource = Arc::new(AsyncMutex::new(()));
    let (operation_id, task) = preparations.lock().unwrap().start("alpha".into(), spawn_waiting).unwrap();
    task.started.await.unwrap();
    task.release.notify_one();
    task.handle.await.unwrap();

    assert!(mutation_lease(&resource, &preparations).is_err());
    assert!(Arc::clone(&resource).try_lock_owned().is_ok());
    assert!(preparations.lock().unwrap().finish(operation_id));
    assert!(mutation_lease(&resource, &preparations).is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_without_active_work_refuses_to_invoke_spawn() {
    let mut preparations = PreparationManager::default();
    preparations.shutdown();
    preparations.shutdown();
    assert!(!preparations.is_busy());
    let result = preparations
        .start("synthetic".into(), |_| -> (AbortHandle, ()) { panic!("shutdown must reject admissions before invoking spawn") });
    assert!(matches!(result, Err("launcher is shutting down")));
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_keeps_cancelled_owner_until_join_and_finish_and_never_reopens() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("synthetic".into(), spawn_waiting).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task.started).await.unwrap().unwrap();

    preparations.shutdown();
    preparations.shutdown();
    assert!(preparations.is_busy());
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_cancelled(operation_id));
    assert!(!preparations.cancel("synthetic", operation_id));
    assert!(!preparations.finish(operation_id));
    assert!(!preparations.finish(uuid::Uuid::new_v4()));
    let result = preparations
        .start("replacement".into(), |_| -> (AbortHandle, ()) { panic!("shutdown must not invoke spawn while cancellation is pending") });
    assert!(matches!(result, Err("launcher is shutting down")));

    assert!(tokio::time::timeout(std::time::Duration::from_secs(5), task.handle).await.unwrap().unwrap_err().is_cancelled());
    assert!(preparations.is_busy());
    assert!(preparations.owns(operation_id));
    assert!(!preparations.finish(uuid::Uuid::new_v4()));
    let result = preparations
        .start("replacement".into(), |_| -> (AbortHandle, ()) { panic!("joined but unfinalized shutdown must not invoke spawn") });
    assert!(matches!(result, Err("launcher is shutting down")));
    assert!(preparations.finish(operation_id));
    assert!(!preparations.finish(operation_id));
    assert!(!preparations.is_busy());
    assert!(!preparations.owns(operation_id));
    assert!(!preparations.is_cancelled(operation_id));
    let result = preparations
        .start("replacement".into(), |_| -> (AbortHandle, ()) { panic!("finishing the owner must not reopen shutdown admissions") });
    assert!(matches!(result, Err("launcher is shutting down")));
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_after_normal_completion_still_refuses_new_spawn() {
    let mut preparations = PreparationManager::default();
    let (operation_id, task) = preparations.start("synthetic".into(), spawn_waiting).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task.started).await.unwrap().unwrap();
    task.release.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(5), task.handle).await.unwrap().unwrap();

    preparations.shutdown();
    assert!(preparations.owns(operation_id));
    assert!(preparations.is_cancelled(operation_id));
    assert!(preparations.finish(operation_id));
    let result = preparations
        .start("replacement".into(), |_| -> (AbortHandle, ()) { panic!("shutdown after completed work must not invoke spawn") });
    assert!(matches!(result, Err("launcher is shutting down")));
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_denies_mutation_lease_before_and_after_owner_finalization() {
    let preparations = Mutex::new(PreparationManager::default());
    let resource = Arc::new(AsyncMutex::new(()));
    let lease = mutation_lease(&resource, &preparations).unwrap();
    let (operation_id, task) = preparations.lock().unwrap().start("synthetic".into(), spawn_waiting).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task.started).await.unwrap().unwrap();

    preparations.lock().unwrap().shutdown();
    assert!(Arc::clone(&resource).try_lock_owned().is_err());
    drop(lease);
    assert!(matches!(mutation_lease(&resource, &preparations), Err("launcher is shutting down")));
    assert!(Arc::clone(&resource).try_lock_owned().is_ok());
    assert!(preparations.lock().unwrap().owns(operation_id));
    assert!(!preparations.lock().unwrap().cancel("synthetic", operation_id));
    assert!(!preparations.lock().unwrap().finish(operation_id));

    assert!(tokio::time::timeout(std::time::Duration::from_secs(5), task.handle).await.unwrap().unwrap_err().is_cancelled());
    assert!(matches!(mutation_lease(&resource, &preparations), Err("launcher is shutting down")));
    assert!(preparations.lock().unwrap().finish(operation_id));
    assert!(matches!(mutation_lease(&resource, &preparations), Err("launcher is shutting down")));
    assert!(Arc::clone(&resource).try_lock_owned().is_ok());
}
