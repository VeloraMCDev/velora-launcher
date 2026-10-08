//! Neutral provider configuration and transport. Identity/gameplay workflows stay with their owners.
pub mod discord;
pub mod mail;
pub mod render;
pub mod settings;
pub mod templates;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    BadRequest,
    BadGateway,
}
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}
impl Error {
    pub(crate) fn request(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::BadRequest, message: message.into() }
    }
    pub(crate) fn gateway(message: impl Into<String>) -> Self {
        Self { kind: ErrorKind::BadGateway, message: message.into() }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
