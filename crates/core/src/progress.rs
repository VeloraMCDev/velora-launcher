use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Preparing,
    Java,
    Game,
    Loader,
    Files,
    Launching,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Stage { stage: Stage, label: String },
    Progress { stage: Stage, done: u64, total: u64, files_done: u32, files_total: u32 },
    Log { line: String },
}

/// Callback the engine uses to report progress. Must be cheap: it is called
/// from download tasks (throttled to ~10 Hz).
pub type Reporter = Arc<dyn Fn(Event) + Send + Sync>;

pub fn noop() -> Reporter {
    Arc::new(|_| {})
}

pub fn stage(r: &Reporter, stage: Stage, label: impl Into<String>) {
    r(Event::Stage { stage, label: label.into() });
}
