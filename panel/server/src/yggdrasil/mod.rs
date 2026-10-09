//! Compatibility host for the independently maintained Velora authentication router.
pub mod keys;
use crate::auth::UserRow;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, HeaderMap};
use axum::Router;
use velora_shared::{CapeInfo, PlayerProfile};
use serde_json::Value;
use std::sync::Arc;
pub use velora_auth_core::capes::CapeRow;
use velora_auth_http::state::{AuthorityState, HostFuture, HostPorts};
pub use velora_auth_http::{dashed, texture_url, undashed, YggError, ROOT};
struct Host(AppState);
fn owner_error(error: AppError) -> velora_auth_http::error::AppError {
    velora_auth_http::error::AppError { status: error.status, message: error.message }
}
fn host_error(error: velora_auth_http::error::AppError) -> AppError {
    AppError::new(error.status, error.message)
}
impl From<AppError> for YggError {
    fn from(error: AppError) -> Self {
        owner_error(error).into()
    }
}
impl HostPorts for Host {
    fn public_base<'a>(&'a self, headers: &'a HeaderMap) -> HostFuture<'a, String> {
        Box::pin(async move { crate::net::public_base(&self.0, headers).await })
    }
    fn server_name(&self) -> HostFuture<'_, Result<String, velora_auth_http::error::AppError>> {
        Box::pin(async move { crate::store::branding(&self.0).await.map(|branding| branding.name).map_err(owner_error) })
    }
    fn client_ip<'a>(&'a self, parts: &'a mut Parts) -> HostFuture<'a, Result<Option<String>, velora_auth_http::error::AppError>> {
        Box::pin(async move { crate::net::ClientIp::from_request_parts(parts, &self.0).await.map(|ip| ip.0).map_err(owner_error) })
    }
}
pub(crate) fn context(state: &AppState) -> AuthorityState {
    AuthorityState {
        db: state.db.clone(),
        identity_db: state.platform_db.clone(),
        ygg: state.ygg.clone(),
        login_guard: state.login_guard.authority_guard(),
        textures_dir: state.cfg.textures_dir(),
        host: Arc::new(Host(state.clone())),
        implementation_version: env!("CARGO_PKG_VERSION").into(),
    }
}
pub async fn user_by_uuid(state: &AppState, uuid: &str) -> AppResult<Option<UserRow>> {
    velora_auth_http::user_by_uuid(&context(state), uuid).await.map_err(host_error)
}
pub async fn cape_of(state: &AppState, user: &UserRow) -> AppResult<Option<CapeRow>> {
    velora_auth_http::cape_of(&context(state), user).await.map_err(host_error)
}
pub async fn available_capes(state: &AppState, user: &UserRow) -> AppResult<Vec<CapeRow>> {
    velora_auth_http::available_capes(&context(state), user).await.map_err(host_error)
}
pub async fn player_profile(state: &AppState, base: &str, user: &UserRow) -> AppResult<PlayerProfile> {
    let profile = velora_auth_http::player_profile(&context(state), base, user).await.map_err(host_error)?;
    Ok(profile_response(profile))
}
pub(crate) fn profile_response(profile: velora_platform_contracts::PlayerProfile) -> PlayerProfile {
    PlayerProfile {
        uuid: profile.uuid,
        name: profile.name,
        skin_url: profile.skin_url,
        skin_model: profile.skin_model,
        cape: profile.cape.map(|c| CapeInfo { id: c.id, name: c.name, url: c.url }),
        available_capes: profile.available_capes.into_iter().map(|c| CapeInfo { id: c.id, name: c.name, url: c.url }).collect(),
    }
}
pub async fn profile_json(state: &AppState, base: &str, user: &UserRow, signed: bool) -> AppResult<Value> {
    velora_auth_http::profile_json(&context(state), base, user, signed).await.map_err(host_error)
}
pub async fn issue_token(state: &AppState, user_id: i64, client_token: Option<String>) -> AppResult<(String, String)> {
    velora_auth_http::issue_token(&context(state), user_id, client_token).await.map_err(host_error)
}
pub async fn token_user(state: &AppState, access: &str, client: Option<&str>) -> AppResult<Option<UserRow>> {
    velora_auth_http::token_user(&context(state), access, client).await.map_err(host_error)
}
pub async fn set_skin(state: &AppState, user_id: i64, bytes: &[u8], model: &str) -> AppResult<()> {
    velora_auth_http::set_skin(&context(state), user_id, bytes, model).await.map_err(host_error)
}
pub fn routes(state: &AppState) -> Router<AppState> {
    velora_auth_http::routes().with_state(context(state))
}
pub async fn texture_file(
    axum::extract::State(state): axum::extract::State<AppState>,
    path: axum::extract::Path<String>,
) -> axum::response::Response {
    velora_auth_http::texture_file(axum::extract::State(context(&state)), path).await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uuid_forms() {
        assert_eq!(undashed("B50AD385-829D-3141-A216-7E7D7539BA7F"), "b50ad385829d3141a2167e7d7539ba7f");
        assert_eq!(dashed("b50ad385829d3141a2167e7d7539ba7f").as_deref(), Some("b50ad385-829d-3141-a216-7e7d7539ba7f"));
        assert!(dashed("nope").is_none());
    }
}
