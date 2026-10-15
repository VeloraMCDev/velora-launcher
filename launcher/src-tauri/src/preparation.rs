use std::sync::{Arc, Mutex};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio::task::AbortHandle;
use uuid::Uuid;

struct Preparation {
    id: Uuid,
    instance_id: String,
    abort: AbortHandle,
    cancellation_requested: bool,
}

#[derive(Default)]
pub struct PreparationManager {
    active: Option<Preparation>,
    closing: bool,
}

impl PreparationManager {
    pub fn is_busy(&self) -> bool {
        self.active.is_some()
    }

    pub fn start<Task>(
        &mut self,
        instance_id: String,
        spawn: impl FnOnce(Uuid) -> (AbortHandle, Task),
    ) -> Result<(Uuid, Task), &'static str> {
        if self.closing {
            return Err("launcher is shutting down");
        }
        if self.is_busy() {
            return Err("already preparing an instance");
        }
        let id = Uuid::new_v4();
        let (abort, task) = spawn(id);
        self.active = Some(Preparation { id, instance_id, abort, cancellation_requested: false });
        Ok((id, task))
    }

    pub fn cancel(&mut self, instance_id: &str, operation_id: Uuid) -> bool {
        let Some(active) = &mut self.active else { return false };
        if active.instance_id != instance_id || active.id != operation_id || active.cancellation_requested || active.abort.is_finished() {
            return false;
        }
        active.cancellation_requested = true;
        active.abort.abort();
        true
    }

    pub fn shutdown(&mut self) {
        self.closing = true;
        if let Some(active) = &mut self.active {
            active.cancellation_requested = true;
            active.abort.abort();
        }
    }

    pub fn owns(&self, id: Uuid) -> bool {
        self.active.as_ref().is_some_and(|active| active.id == id)
    }

    pub fn is_cancelled(&self, id: Uuid) -> bool {
        self.active.as_ref().is_some_and(|active| active.id == id && active.cancellation_requested)
    }

    pub fn finish(&mut self, id: Uuid) -> bool {
        if !self.active.as_ref().is_some_and(|active| active.id == id && active.abort.is_finished()) {
            return false;
        }
        self.active = None;
        true
    }
}

pub fn mutation_lease(
    resource: &Arc<AsyncMutex<()>>,
    preparations: &Mutex<PreparationManager>,
) -> Result<OwnedMutexGuard<()>, &'static str> {
    let lease = resource.clone().try_lock_owned().map_err(|_| "another launcher operation is in progress")?;
    let preparations = preparations.lock().unwrap();
    if preparations.closing {
        return Err("launcher is shutting down");
    }
    if preparations.is_busy() {
        return Err("already preparing an instance");
    }
    Ok(lease)
}

#[cfg(test)]
mod tests;
