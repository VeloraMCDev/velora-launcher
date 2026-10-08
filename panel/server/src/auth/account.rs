//! Local host ports and explicit legacy/SDK wire bridges for authority account workflows.
use crate::{
    auth::UserRow,
    error::{AppError, AppResult},
    state::AppState,
};
use std::sync::Arc;
use velora_auth_http::{
    state::HostFuture,
    web::{AccountHost, AccountPolicy, AccountState},
};
use velora_platform_contracts as sdk;
struct Host(AppState);
pub(crate) fn owner_error(error: AppError) -> velora_auth_http::error::AppError {
    velora_auth_http::error::AppError { status: error.status, message: error.message }
}
pub(crate) fn host_error(error: velora_auth_http::error::AppError) -> AppError {
    AppError::new(error.status, error.message)
}
impl AccountHost for Host {
    fn policy(&self) -> HostFuture<'_, Result<AccountPolicy, velora_auth_http::error::AppError>> {
        Box::pin(async move {
            let settings = crate::store::settings(&self.0).await.map_err(owner_error)?;
            let registration = registration(settings.auth.registration);
            Ok(AccountPolicy { panel_accounts: settings.auth.panel_accounts, registration })
        })
    }
    fn check_username<'a>(&'a self, name: &'a str) -> HostFuture<'a, Result<(), velora_auth_http::error::AppError>> {
        Box::pin(async move { crate::store::check_username(&self.0, name).await.map_err(owner_error) })
    }
    fn record_login<'a>(&'a self, user: &'a UserRow) -> HostFuture<'a, Result<(), velora_auth_http::error::AppError>> {
        Box::pin(async move { crate::routes::activity::record(&self.0, user, "auth", "login", None).await.map_err(owner_error) })
    }
}
pub(crate) fn context(state: &AppState) -> AccountState {
    AccountState { authority: crate::yggdrasil::context(state), tokens: state.keys.authority_keys(), host: Arc::new(Host(state.clone())) }
}
pub(crate) fn response(response: sdk::AuthResponse) -> scopenet_shared::AuthResponse {
    scopenet_shared::AuthResponse {
        token: response.token,
        pending: response.pending,
        user: scopenet_shared::PublicUser {
            id: response.user.id,
            username: response.user.username,
            uuid: response.user.uuid,
            role: response.user.role,
            groups: response.user.groups,
        },
        yggdrasil: response
            .yggdrasil
            .map(|tokens| scopenet_shared::YggdrasilTokens { access_token: tokens.access_token, client_token: tokens.client_token }),
    }
}
pub(crate) async fn login(state: &AppState, request: scopenet_shared::LoginRequest) -> AppResult<scopenet_shared::AuthResponse> {
    velora_auth_http::web::login(&context(state), sdk::LoginRequest { username: request.username, password: request.password })
        .await
        .map(response)
        .map_err(host_error)
}
pub(crate) async fn register(state: &AppState, request: scopenet_shared::RegisterRequest) -> AppResult<scopenet_shared::AuthResponse> {
    velora_auth_http::web::register(
        &context(state),
        sdk::RegisterRequest { username: request.username, password: request.password, email: request.email },
    )
    .await
    .map(response)
    .map_err(host_error)
}

pub(crate) fn registration(mode: scopenet_shared::RegistrationMode) -> sdk::RegistrationMode {
    match mode {
        scopenet_shared::RegistrationMode::Closed => sdk::RegistrationMode::Closed,
        scopenet_shared::RegistrationMode::Open => sdk::RegistrationMode::Open,
        scopenet_shared::RegistrationMode::Approval => sdk::RegistrationMode::Approval,
    }
}
pub(crate) async fn create_account(
    state: &AppState,
    username: &str,
    password: &str,
    email: Option<&str>,
    role: &str,
    status: &str,
) -> AppResult<i64> {
    velora_auth_http::web::create_account(&context(state), username, password, email, role, status).await.map_err(host_error)
}
