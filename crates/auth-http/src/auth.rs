use crate::{error::AppResult, state::AuthorityState};
pub use velora_auth_core::identity::IdentityRecord as UserRow;
pub use velora_auth_core::password::verify_password;
pub async fn user_groups(state: &AuthorityState, user_id: i64) -> AppResult<Vec<String>> {
    Ok(velora_auth_core::identity_store::groups(&state.identity_db, user_id).await?)
}
pub async fn find_user_by_name(state: &AuthorityState, name: &str) -> AppResult<Option<UserRow>> {
    Ok(velora_auth_core::identity_store::find_by_name(&state.identity_db, name).await?)
}
