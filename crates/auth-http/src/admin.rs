//! Authority administrator account/group operations; callers enforce administrator admission.
use crate::{
    error::{AppError, AppResult},
    web::AccountState,
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use velora_auth_core::{identity::IdentityRecord, identity_store, password};

#[derive(Deserialize)]
pub struct UserInput {
    pub username: Option<String>,
    pub password: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub status_reason: Option<String>,
    pub groups: Option<Vec<String>>,
}
fn check_role(role: &str) -> AppResult<()> {
    if !matches!(role, "admin" | "player") {
        return Err(AppError::bad_request("role must be admin or player"));
    }
    Ok(())
}
fn check_status(status: &str) -> AppResult<()> {
    if !matches!(status, "active" | "pending" | "disabled") {
        return Err(AppError::bad_request("status must be active, pending or disabled"));
    }
    Ok(())
}
async fn set_groups(db: &SqlitePool, user_id: i64, groups: &[String]) -> AppResult<()> {
    // Retain legacy ordering and partial-update semantics; unknown names are ignored.
    sqlx::query("DELETE FROM user_groups WHERE user_id = ?").bind(user_id).execute(db).await?;
    for group in groups {
        sqlx::query("INSERT OR IGNORE INTO user_groups (user_id, group_id) SELECT ?, id FROM groups WHERE name = ?")
            .bind(user_id)
            .bind(group)
            .execute(db)
            .await?;
    }
    Ok(())
}
pub async fn list_users(db: &SqlitePool) -> AppResult<Vec<IdentityRecord>> {
    Ok(sqlx::query_as("SELECT * FROM users ORDER BY status = 'pending' DESC, username COLLATE NOCASE").fetch_all(db).await?)
}
pub async fn counts(db: &SqlitePool) -> AppResult<(i64, i64)> {
    let users = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(db).await?;
    let pending = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = 'pending'").fetch_one(db).await?;
    Ok((users, pending))
}
pub async fn create_user(state: &AccountState, input: UserInput) -> AppResult<IdentityRecord> {
    let username = input.username.as_deref().map(str::trim).unwrap_or_default();
    if !velora_platform_contracts::valid_username(username) {
        return Err(AppError::bad_request("usernames are 3-16 letters, numbers or underscores"));
    }
    let supplied_password = input.password.unwrap_or_default();
    password::validate_password(&supplied_password).map_err(AppError::bad_request)?;
    let role = input.role.unwrap_or_else(|| "player".into());
    check_role(&role)?;
    let status = input.status.unwrap_or_else(|| "active".into());
    check_status(&status)?;
    let id = crate::web::create_account(state, username, &supplied_password, input.email.as_deref(), &role, &status).await?;
    if let Some(groups) = input.groups {
        set_groups(&state.authority.db, id, &groups).await?;
    }
    Ok(identity_store::find_by_id(&state.authority.db, id).await?.ok_or(sqlx::Error::RowNotFound)?)
}
pub async fn update_user(state: &AccountState, me: &IdentityRecord, id: i64, input: UserInput) -> AppResult<IdentityRecord> {
    let db = &state.authority.db;
    let user = identity_store::find_by_id(db, id)
        .await?
        .ok_or_else(|| AppError { status: StatusCode::NOT_FOUND, message: "user not found".into() })?;
    if let Some(supplied_password) = input.password.filter(|password| !password.is_empty()) {
        password::validate_password(&supplied_password).map_err(AppError::bad_request)?;
        let mut tx = db.begin().await?;
        sqlx::query("UPDATE users SET password_hash = ?, auth_version = auth_version + 1 WHERE id = ?")
            .bind(
                password::hash_password(&supplied_password)
                    .map_err(|message| AppError { status: StatusCode::INTERNAL_SERVER_ERROR, message })?,
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
        identity_store::revoke_game_sessions_in_transaction(&mut tx, id).await?;
        tx.commit().await?;
    }
    if let Some(email) = input.email {
        sqlx::query("UPDATE users SET email = ? WHERE id = ?")
            .bind(Some(email.trim()).filter(|email| !email.is_empty()))
            .bind(id)
            .execute(db)
            .await?;
    }
    if let Some(role) = input.role {
        check_role(&role)?;
        if me.id == id && role != "admin" {
            return Err(AppError::bad_request("you can't remove your own admin role"));
        }
        sqlx::query("UPDATE users SET role = ? WHERE id = ?").bind(role).bind(id).execute(db).await?;
    }
    if let Some(reason) = input.status_reason {
        sqlx::query("UPDATE users SET status_reason = ? WHERE id = ?")
            .bind(Some(reason.trim()).filter(|reason| !reason.is_empty()))
            .bind(id)
            .execute(db)
            .await?;
    }
    if let Some(status) = input.status {
        check_status(&status)?;
        if me.id == id && status != "active" {
            return Err(AppError::bad_request("you can't disable your own account"));
        }
        sqlx::query("UPDATE users SET status = ? WHERE id = ?").bind(status).bind(id).execute(db).await?;
    }
    if let Some(groups) = input.groups {
        set_groups(db, user.id, &groups).await?;
    }
    Ok(identity_store::find_by_id(db, id).await?.ok_or(sqlx::Error::RowNotFound)?)
}
pub async fn deletion_target(db: &SqlitePool, me: &IdentityRecord, id: i64) -> AppResult<IdentityRecord> {
    if me.id == id {
        return Err(AppError::bad_request("you can't delete your own account"));
    }
    identity_store::find_by_id(db, id).await?.ok_or_else(|| AppError { status: StatusCode::NOT_FOUND, message: "player not found".into() })
}
#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Group {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    #[sqlx(default)]
    pub members: i64,
    #[serde(default)]
    #[sqlx(default)]
    pub luckperms_group: String,
    #[serde(default)]
    #[sqlx(default)]
    pub discord_role: String,
}
fn default_color() -> String {
    "#7c5cff".into()
}
pub async fn list_groups(db: &SqlitePool) -> AppResult<Vec<Group>> {
    Ok(sqlx::query_as("SELECT g.id, g.name, g.color, (SELECT COUNT(*) FROM user_groups ug WHERE ug.group_id = g.id) AS members, g.luckperms_group, g.discord_role FROM groups g ORDER BY g.name").fetch_all(db).await?)
}
pub async fn create_group(db: &SqlitePool, group: Group) -> AppResult<()> {
    let name = group.name.trim();
    if name.is_empty() || name.len() > 32 {
        return Err(AppError::bad_request("group names are 1-32 characters"));
    }
    sqlx::query("INSERT INTO groups (name, color) VALUES (?, ?)")
        .bind(name)
        .bind(&group.color)
        .execute(db)
        .await
        .map_err(|_| AppError { status: StatusCode::CONFLICT, message: "a group with that name exists".into() })?;
    Ok(())
}
pub async fn delete_group(db: &SqlitePool, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM groups WHERE id = ?").bind(id).execute(db).await?;
    Ok(())
}
