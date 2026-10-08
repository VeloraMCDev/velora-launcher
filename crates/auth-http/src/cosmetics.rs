//! Administrator cosmetic workflows; callers enforce administrator admission.
use crate::{
    error::{AppError, AppResult},
    state::AuthorityState,
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use velora_auth_core::{identity::IdentityRecord, identity_store};
async fn reload(state: &AuthorityState, id: i64) -> AppResult<IdentityRecord> {
    identity_store::find_by_id(&state.db, id).await?.ok_or(sqlx::Error::RowNotFound.into())
}
pub async fn set_skin(state: &AuthorityState, id: i64, bytes: &[u8], model: &str) -> AppResult<IdentityRecord> {
    crate::set_skin(state, id, bytes, model).await?;
    reload(state, id).await
}
pub async fn delete_skin(state: &AuthorityState, id: i64) -> AppResult<IdentityRecord> {
    identity_store::clear_skin(&state.db, id).await?;
    reload(state, id).await
}
pub async fn set_model(state: &AuthorityState, id: i64, model: &str) -> AppResult<IdentityRecord> {
    if !matches!(model, "classic" | "slim") {
        return Err(AppError::bad_request("model must be classic or slim"));
    }
    identity_store::set_skin_model(&state.db, id, model).await?;
    identity_store::find_by_id(&state.db, id)
        .await?
        .ok_or_else(|| AppError { status: StatusCode::NOT_FOUND, message: "user not found".into() })
}
pub async fn set_cape(state: &AuthorityState, id: i64, cape: Option<i64>) -> AppResult<IdentityRecord> {
    if let Some(cape) = cape {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capes WHERE id = ?").bind(cape).fetch_one(&state.db).await?;
        if exists == 0 {
            return Err(AppError { status: StatusCode::NOT_FOUND, message: "cape not found".into() });
        }
    }
    identity_store::set_cape(&state.db, id, cape).await?;
    reload(state, id).await
}
#[derive(Serialize)]
pub struct CapeView {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub visibility: String,
    pub allowed_groups: Vec<String>,
    pub wearers: i64,
    pub created_at: String,
}
pub async fn list_capes(state: &AuthorityState) -> AppResult<Vec<CapeView>> {
    let rows: Vec<velora_auth_core::capes::CapeRow> =
        sqlx::query_as("SELECT * FROM capes ORDER BY name COLLATE NOCASE").fetch_all(&state.db).await?;
    let mut out = Vec::new();
    for cape in rows {
        let wearers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE cape_id = ?").bind(cape.id).fetch_one(&state.db).await?;
        out.push(CapeView {
            id: cape.id,
            name: cape.name,
            url: format!("/textures/{}", cape.hash),
            visibility: cape.visibility,
            allowed_groups: serde_json::from_str(&cape.allowed_groups).unwrap_or_default(),
            wearers,
            created_at: cape.created_at,
        });
    }
    Ok(out)
}
fn check_visibility(visibility: &str) -> AppResult<()> {
    if !matches!(visibility, "public" | "groups" | "private") {
        return Err(AppError::bad_request("visibility must be public, groups or private"));
    }
    Ok(())
}
/// Name has already been trimmed by the caller's multipart extraction.
pub async fn create_cape(
    state: &AuthorityState,
    name: String,
    visibility: String,
    groups_json: String,
    file: Option<Vec<u8>>,
) -> AppResult<Vec<CapeView>> {
    if name.is_empty() || name.len() > 40 {
        return Err(AppError::bad_request("give the cape a name (up to 40 characters)"));
    }
    check_visibility(&visibility)?;
    let groups: Vec<String> =
        serde_json::from_str(&groups_json).map_err(|_| AppError::bad_request("allowed_groups must be a JSON list"))?;
    let bytes = file.ok_or_else(|| AppError::bad_request("no image uploaded"))?;
    let dir = state.textures_dir.clone();
    let hash = tokio::task::spawn_blocking(move || crate::textures::store(&dir, crate::textures::Kind::Cape, &bytes))
        .await
        .map_err(|error| AppError::bad_request(error.to_string()))??;
    sqlx::query("INSERT INTO capes (name, hash, visibility, allowed_groups, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&name)
        .bind(hash)
        .bind(&visibility)
        .bind(serde_json::to_string(&groups)?)
        .bind(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .execute(&state.db)
        .await?;
    list_capes(state).await
}
#[derive(Deserialize)]
pub struct CapeUpdate {
    pub name: String,
    pub visibility: String,
    #[serde(default)]
    pub allowed_groups: Vec<String>,
}
pub async fn update_cape(state: &AuthorityState, id: i64, input: CapeUpdate) -> AppResult<Vec<CapeView>> {
    check_visibility(&input.visibility)?;
    if input.name.trim().is_empty() {
        return Err(AppError::bad_request("the cape needs a name"));
    }
    sqlx::query("UPDATE capes SET name = ?, visibility = ?, allowed_groups = ? WHERE id = ?")
        .bind(input.name.trim())
        .bind(&input.visibility)
        .bind(serde_json::to_string(&input.allowed_groups)?)
        .bind(id)
        .execute(&state.db)
        .await?;
    list_capes(state).await
}
pub async fn delete_cape(state: &AuthorityState, id: i64) -> AppResult<Vec<CapeView>> {
    sqlx::query("UPDATE users SET cape_id = NULL WHERE cape_id = ?").bind(id).execute(&state.db).await?;
    sqlx::query("DELETE FROM capes WHERE id = ?").bind(id).execute(&state.db).await?;
    list_capes(state).await
}
