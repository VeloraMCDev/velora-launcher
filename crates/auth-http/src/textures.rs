use crate::error::{AppError, AppResult};
use std::path::Path;
pub use velora_auth_core::textures::{path, Kind};
pub fn store(dir: &Path, kind: Kind, bytes: &[u8]) -> AppResult<String> {
    velora_auth_core::textures::store(dir, kind, bytes).map_err(|error| match error {
        velora_auth_core::textures::TextureError::BadRequest(message) => AppError::bad_request(message),
        velora_auth_core::textures::TextureError::Io(error) => error.into(),
    })
}
