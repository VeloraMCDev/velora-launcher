//! Compatible host adapter for authority-owned identity textures.
use crate::error::{AppError, AppResult};
use std::path::Path;
pub use velora_auth_core::textures::{path, Kind};
fn host_error(error: velora_auth_core::textures::TextureError) -> AppError {
    match error {
        velora_auth_core::textures::TextureError::BadRequest(message) => AppError::bad_request(message),
        velora_auth_core::textures::TextureError::Io(error) => error.into(),
    }
}
pub fn store(dir: &Path, kind: Kind, bytes: &[u8]) -> AppResult<String> {
    velora_auth_core::textures::store(dir, kind, bytes).map_err(host_error)
}
pub fn render_head(bytes: &[u8], size: u32) -> AppResult<Vec<u8>> {
    velora_auth_core::textures::render_head(bytes, size).map_err(host_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_messages_and_http_status_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let error = store(dir.path(), Kind::Skin, b"not a PNG").unwrap_err();
        assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(error.message, "that isn't a valid PNG image");
    }
    #[test]
    fn filesystem_errors_keep_the_host_io_prefix_and_server_status() {
        let error = host_error(velora_auth_core::textures::TextureError::Io(std::io::Error::other("synthetic failure")));
        assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.message, "io error: synthetic failure");
    }
}
