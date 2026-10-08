#[derive(Debug)]
pub struct PackError {
    pub status: u16,
    pub message: String,
}
pub type PackResult<T> = Result<T, PackError>;
impl PackError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self { status: 400, message: message.into() }
    }
}
impl From<anyhow::Error> for PackError {
    fn from(e: anyhow::Error) -> Self {
        Self { status: 502, message: format!("{e:#}") }
    }
}
impl From<sqlx::Error> for PackError {
    fn from(e: sqlx::Error) -> Self {
        Self { status: 500, message: format!("database error: {e}") }
    }
}
impl From<std::io::Error> for PackError {
    fn from(e: std::io::Error) -> Self {
        Self { status: 500, message: format!("io error: {e}") }
    }
}
impl From<serde_json::Error> for PackError {
    fn from(e: serde_json::Error) -> Self {
        Self::bad_request(format!("invalid JSON: {e}"))
    }
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}
impl std::error::Error for PackError {}
