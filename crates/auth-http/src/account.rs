//! Player account mutations. Host-owned side effects share the authority transaction.
use crate::{
    error::{AppError, AppResult},
    state::{AuthorityState, HostFuture},
    web::AccountState,
};
use axum::http::{HeaderMap, StatusCode};
use sqlx::{Sqlite, Transaction};
use velora_auth_core::{identity::IdentityRecord, identity_store, password};
use velora_platform_contracts::{AuthResponse, PlayerProfile};

pub trait MutationHost: Send + Sync {
    fn online<'a>(&'a self, uuid: &'a str) -> HostFuture<'a, AppResult<bool>>;
    fn renamed<'a, 'c>(
        &'a self,
        tx: &'a mut Transaction<'c, Sqlite>,
        user: &'a IdentityRecord,
        name: &'a str,
    ) -> HostFuture<'a, AppResult<()>>;
}
pub async fn set_username(
    state: &AccountState,
    host: &dyn MutationHost,
    user: &IdentityRecord,
    username: &str,
    supplied_password: &str,
) -> AppResult<AuthResponse> {
    let name = username.trim();
    if !velora_platform_contracts::valid_username(name) {
        return Err(AppError::bad_request("usernames are 3–16 letters, numbers or underscores"));
    }
    state.host.check_username(name).await?;
    state
        .authority
        .login_guard
        .check(&user.username)
        .map_err(|message| AppError { status: StatusCode::TOO_MANY_REQUESTS, message: message.into() })?;
    if !password::verify_password(supplied_password, &user.password_hash) {
        state.authority.login_guard.fail(&user.username);
        return Err(AppError { status: StatusCode::UNAUTHORIZED, message: "incorrect password".into() });
    }
    state.authority.login_guard.succeed(&user.username);
    if host.online(&user.uuid).await? {
        return Err(AppError {
            status: StatusCode::CONFLICT,
            message: "disconnect from your servers before changing your username".into(),
        });
    }
    let mut tx = state.authority.db.begin().await?;
    identity_store::rename_in_transaction(&mut tx, user.id, name).await.map_err(|error| match error {
        sqlx::Error::Database(d) if d.message().contains("UNIQUE") || d.message().contains("username reserved") => {
            AppError { status: StatusCode::CONFLICT, message: "that username is taken or reserved".into() }
        }
        error => error.into(),
    })?;
    identity_store::revoke_game_sessions_in_transaction(&mut tx, user.id).await?;
    host.renamed(&mut tx, user, name).await?;
    tx.commit().await?;
    crate::web::signed_in(state, &reload(&state.authority, user.id).await?).await
}
async fn reload(state: &AuthorityState, id: i64) -> AppResult<IdentityRecord> {
    identity_store::find_by_id(&state.db, id).await?.ok_or(sqlx::Error::RowNotFound.into())
}
pub async fn profile(state: &AuthorityState, headers: &HeaderMap, user: &IdentityRecord) -> AppResult<PlayerProfile> {
    let base = state.host.public_base(headers).await;
    crate::player_profile(state, &base, user).await
}
async fn reloaded_profile(state: &AuthorityState, headers: &HeaderMap, id: i64) -> AppResult<PlayerProfile> {
    // Preserve the host's origin lookup before account reload.
    let base = state.host.public_base(headers).await;
    crate::player_profile(state, &base, &reload(state, id).await?).await
}
pub async fn upload_skin(
    state: &AuthorityState,
    headers: &HeaderMap,
    user: &IdentityRecord,
    bytes: &[u8],
    model: &str,
) -> AppResult<PlayerProfile> {
    crate::set_skin(state, user.id, bytes, model).await?;
    reloaded_profile(state, headers, user.id).await
}
pub async fn set_model(state: &AuthorityState, headers: &HeaderMap, user: &IdentityRecord, model: &str) -> AppResult<PlayerProfile> {
    identity_store::set_skin_model(&state.db, user.id, if model == "slim" { "slim" } else { "classic" }).await?;
    reloaded_profile(state, headers, user.id).await
}
pub async fn delete_skin(state: &AuthorityState, headers: &HeaderMap, user: &IdentityRecord) -> AppResult<PlayerProfile> {
    identity_store::clear_skin(&state.db, user.id).await?;
    reloaded_profile(state, headers, user.id).await
}
pub async fn set_cape(state: &AuthorityState, headers: &HeaderMap, user: &IdentityRecord, id: Option<i64>) -> AppResult<PlayerProfile> {
    if let Some(id) = id {
        if !crate::available_capes(state, user).await?.iter().any(|cape| cape.id == id) {
            return Err(AppError { status: StatusCode::FORBIDDEN, message: "that cape isn't available to you".into() });
        }
    }
    identity_store::set_cape(&state.db, user.id, id).await?;
    reloaded_profile(state, headers, user.id).await
}
pub async fn avatar(state: &AuthorityState, id: &str, size: Option<u32>) -> AppResult<Vec<u8>> {
    let user = match crate::user_by_uuid(state, id).await? {
        Some(user) => Some(user),
        None => identity_store::find_by_name(&state.identity_db, id).await?,
    };
    let no_skin = || AppError { status: StatusCode::NOT_FOUND, message: "no skin".into() };
    let hash = user.and_then(|user| user.skin_hash).ok_or_else(no_skin)?;
    let path = velora_auth_core::textures::path(&state.textures_dir, &hash).ok_or_else(no_skin)?;
    let bytes = tokio::fs::read(path).await.map_err(|_| no_skin())?;
    tokio::task::spawn_blocking(move || velora_auth_core::textures::render_head(&bytes, size.unwrap_or(64)))
        .await
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .map_err(|error| match error {
            velora_auth_core::textures::TextureError::BadRequest(message) => AppError::bad_request(message),
            velora_auth_core::textures::TextureError::Io(error) => error.into(),
        })
}
