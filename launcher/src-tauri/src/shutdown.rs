use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio::task::AbortHandle;

#[derive(Default)]
struct DrainState {
    closing: bool,
    ready: bool,
    active: usize,
    cancellable: Vec<AbortHandle>,
}

#[derive(Default)]
pub struct Shutdown {
    state: Mutex<DrainState>,
    changed: Notify,
}

pub struct WorkPermit {
    owner: Arc<Shutdown>,
}

impl Shutdown {
    pub fn enter(self: &Arc<Self>) -> Result<WorkPermit, &'static str> {
        let mut state = self.state.lock().unwrap();
        if state.closing {
            return Err("launcher is shutting down");
        }
        state.active += 1;
        Ok(WorkPermit { owner: Arc::clone(self) })
    }

    pub fn track_cancellable(&self, task: AbortHandle) {
        let mut state = self.state.lock().unwrap();
        if state.closing {
            task.abort();
        } else {
            state.cancellable.retain(|handle| !handle.is_finished());
            state.cancellable.push(task);
        }
    }

    pub fn begin(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.closing {
            return false;
        }
        state.closing = true;
        for task in state.cancellable.drain(..) {
            task.abort();
        }
        true
    }

    pub fn is_closing(&self) -> bool {
        self.state.lock().unwrap().closing
    }

    pub fn is_ready(&self) -> bool {
        self.state.lock().unwrap().ready
    }

    pub fn mark_ready(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if !state.closing || state.active != 0 {
            return false;
        }
        state.ready = true;
        true
    }

    pub async fn wait_idle(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.state.lock().unwrap().active == 0 {
                return;
            }
            changed.await;
        }
    }
}

impl Drop for WorkPermit {
    fn drop(&mut self) {
        let mut state = self.owner.state.lock().unwrap();
        state.active -= 1;
        drop(state);
        self.owner.changed.notify_waiters();
    }
}

#[cfg(test)]
mod tests;
