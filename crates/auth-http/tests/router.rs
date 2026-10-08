//! Independent authority router fixture: synthetic accounts, keys and host ports only.
#[tokio::test]
async fn first_admin_bootstrap_preserves_existing_credentials_identity_and_disabled_admins_on_restart() {
    let (state, _directory) = admin_fixture().await;
    let first = velora_auth_http::web::bootstrap_admin(&state, "SyntheticAdmin", "synthetic-admin-password").await.unwrap().unwrap();
    let before: (String, String) =
        sqlx::query_as("SELECT uuid,password_hash FROM users WHERE id=?").bind(first).fetch_one(&state.authority.db).await.unwrap();
    sqlx::query("UPDATE users SET status='disabled' WHERE id=?").bind(first).execute(&state.authority.db).await.unwrap();
    assert_eq!(velora_auth_http::web::bootstrap_admin(&state, "ChangedAdmin", "changed-password").await.unwrap(), None);
    let after: (String, String) =
        sqlx::query_as("SELECT uuid,password_hash FROM users WHERE id=?").bind(first).fetch_one(&state.authority.db).await.unwrap();
    assert_eq!(before, after);
    assert!(velora_auth_core::password::verify_password("synthetic-admin-password", &after.1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE role='admin'").fetch_one(&state.authority.db).await.unwrap(),
        1
    );
}

#[tokio::test]
async fn competing_bootstrap_attempts_create_one_admin_and_never_promote_an_existing_player() {
    let (mut state, directory) = admin_fixture().await;
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(directory.path().join("synthetic-bootstrap.sqlite"))
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(4).connect_with(options).await.unwrap();
    velora_auth_core::schema::initialize(&pool).await.unwrap();
    let player = velora_auth_core::identity_store::find_by_id(&state.authority.db, 1).await.unwrap().unwrap();
    velora_auth_core::identity_store::insert(
        &pool,
        velora_auth_core::identity_store::NewAccount {
            username: &player.username,
            password_hash: &player.password_hash,
            email: None,
            role: "member",
            status: "active",
            created_at: &player.created_at,
            uuid: &player.uuid,
        },
    )
    .await
    .unwrap();
    state.authority.db = pool.clone();
    state.authority.identity_db = pool;
    let error = velora_auth_http::web::bootstrap_admin(&state, "ExamplePlayer", "synthetic-admin-password").await.unwrap_err();
    assert_eq!(error.status, StatusCode::CONFLICT);
    let role: String =
        sqlx::query_scalar("SELECT role FROM users WHERE username='ExamplePlayer'").fetch_one(&state.authority.db).await.unwrap();
    assert_eq!(role, "member");
    let (first, second) = tokio::join!(
        velora_auth_http::web::bootstrap_admin(&state, "FirstAdmin", "synthetic-admin-password"),
        velora_auth_http::web::bootstrap_admin(&state, "OtherAdmin", "other-synthetic-password")
    );
    assert_eq!(usize::from(first.unwrap().is_some()) + usize::from(second.unwrap().is_some()), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE role='admin'").fetch_one(&state.authority.db).await.unwrap(),
        1
    );
}
use axum::{
    body::{to_bytes, Body},
    http::{request::Parts, HeaderMap, Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use velora_auth_http::{
    error::AppError,
    state::{AuthorityState, HostFuture, HostPorts},
};
struct SyntheticMutationHost {
    online: bool,
    fail: bool,
}
fn admin_input(value: Value) -> velora_auth_http::admin::UserInput {
    serde_json::from_value(value).unwrap()
}
#[tokio::test]
async fn administrator_cosmetics_preserve_strict_models_private_assignment_wire_fields_and_delete_order() {
    use velora_auth_http::cosmetics;
    let (_, state, _directory) = fixture().await;
    let user = velora_auth_core::identity_store::find_by_name(&state.db, "ExamplePlayer").await.unwrap().unwrap();
    assert_eq!(cosmetics::set_model(&state, user.id, "SLIM").await.unwrap_err().message, "model must be classic or slim");
    assert_eq!(cosmetics::set_model(&state, 9999, "slim").await.unwrap_err().status, StatusCode::NOT_FOUND);
    assert!(cosmetics::delete_skin(&state, 9999).await.unwrap_err().status.is_server_error());
    let slim = cosmetics::set_model(&state, user.id, "slim").await.unwrap();
    assert_eq!(slim.uuid, user.uuid);
    assert_eq!(cosmetics::delete_skin(&state, user.id).await.unwrap().skin_model, "slim");
    sqlx::query("INSERT INTO capes VALUES (1, 'z Staff', 'synthetic-hash', 'private', 'not-json', 'x'), (2, 'A Public', 'other-hash', 'public', '[]', 'x')").execute(&state.db).await.unwrap();
    assert_eq!(cosmetics::set_cape(&state, 9999, Some(9999)).await.unwrap_err().message, "cape not found");
    let assigned = cosmetics::set_cape(&state, user.id, Some(1)).await.unwrap();
    assert_eq!(assigned.cape_id, Some(1));
    let views = cosmetics::list_capes(&state).await.unwrap();
    assert_eq!(views[0].id, 2);
    assert_eq!((views[1].wearers, views[1].url.as_str()), (1, "/textures/synthetic-hash"));
    assert!(views[1].allowed_groups.is_empty());
    let long_name = "x".repeat(80);
    let updated = cosmetics::update_cape(
        &state,
        1,
        serde_json::from_value(json!({"name":long_name, "visibility":"groups", "allowed_groups":["Readers"]})).unwrap(),
    )
    .await
    .unwrap();
    assert!(updated.iter().any(|cape| cape.name.len() == 80), "legacy updates have no create-name length cap");
    sqlx::raw_sql("CREATE TRIGGER synthetic_cape_delete_failure BEFORE DELETE ON capes BEGIN SELECT RAISE(ABORT, 'synthetic cape delete failure'); END;").execute(&state.db).await.unwrap();
    assert!(cosmetics::delete_cape(&state, 1).await.err().unwrap().status.is_server_error());
    assert!(
        velora_auth_core::identity_store::find_by_id(&state.db, user.id).await.unwrap().unwrap().cape_id.is_none(),
        "legacy wearer clearing precedes cape deletion"
    );
    sqlx::raw_sql("DROP TRIGGER synthetic_cape_delete_failure;").execute(&state.db).await.unwrap();
    assert_eq!(cosmetics::delete_cape(&state, 1).await.unwrap().len(), 1);
}
#[tokio::test]
async fn administrator_cape_validation_precedence_and_outage_errors_stay_compatible() {
    use velora_auth_http::cosmetics;
    let (_, state, _directory) = fixture().await;
    assert_eq!(
        cosmetics::create_cape(&state, String::new(), "invalid".into(), "invalid".into(), None).await.err().unwrap().message,
        "give the cape a name (up to 40 characters)"
    );
    assert_eq!(
        cosmetics::create_cape(&state, "Synthetic".into(), "invalid".into(), "invalid".into(), None).await.err().unwrap().message,
        "visibility must be public, groups or private"
    );
    assert_eq!(
        cosmetics::create_cape(&state, "Synthetic".into(), "public".into(), "invalid".into(), None).await.err().unwrap().message,
        "allowed_groups must be a JSON list"
    );
    assert_eq!(
        cosmetics::create_cape(&state, "Synthetic".into(), "public".into(), "[]".into(), None).await.err().unwrap().message,
        "no image uploaded"
    );
    assert_eq!(
        cosmetics::create_cape(&state, "Synthetic".into(), "public".into(), "[]".into(), Some(b"invalid PNG".to_vec()))
            .await
            .err()
            .unwrap()
            .message,
        "that isn't a valid PNG image"
    );
    state.db.close().await;
    assert!(cosmetics::list_capes(&state).await.err().unwrap().status.is_server_error());
}
async fn admin_fixture() -> (velora_auth_http::web::AccountState, tempfile::TempDir) {
    let (_, authority, directory) = fixture().await;
    (
        velora_auth_http::web::AccountState {
            authority,
            tokens: Arc::new(velora_auth_core::tokens::Keys::new(b"synthetic-test-secret")),
            host: Arc::new(SyntheticAccountHost::new(velora_platform_contracts::RegistrationMode::Open)),
        },
        directory,
    )
}
#[tokio::test]
async fn administrator_accounts_keep_group_projection_order_live_admission_and_self_protection() {
    use velora_auth_http::{admin, web};
    let (state, _directory) = admin_fixture().await;
    let db = &state.authority.db;
    admin::create_group(db, serde_json::from_value(json!({"name":" Founders "})).unwrap()).await.unwrap();
    assert_eq!(
        admin::create_group(db, serde_json::from_value(json!({"name":"Founders"})).unwrap()).await.unwrap_err().status,
        StatusCode::CONFLICT
    );
    let me = admin::create_user(&state, admin_input(json!({"username":"Owner", "password":"synthetic-password", "role":"admin"})))
        .await
        .unwrap();
    let pending = admin::create_user(&state, admin_input(json!({"username":" Awaiting ", "password":"synthetic-password", "status":"pending", "groups":["Founders","Missing","Founders"], "email":" waiting@example.invalid "}))).await.unwrap();
    assert_eq!(pending.email.as_deref(), Some("waiting@example.invalid"));
    assert_eq!(admin::list_users(db).await.unwrap()[0].id, pending.id);
    assert_eq!(admin::counts(db).await.unwrap(), (3, 1));
    let groups = admin::list_groups(db).await.unwrap();
    assert_eq!((groups[0].name.as_str(), groups[0].members, groups[0].color.as_str()), ("Founders", 1, "#7c5cff"));
    assert_eq!(
        admin::update_user(&state, &me, me.id, admin_input(json!({"role":"player"}))).await.unwrap_err().message,
        "you can't remove your own admin role"
    );
    assert_eq!(
        admin::update_user(&state, &me, me.id, admin_input(json!({"status":"disabled"}))).await.unwrap_err().message,
        "you can't disable your own account"
    );
    assert_eq!(admin::deletion_target(db, &me, me.id).await.unwrap_err().message, "you can't delete your own account");
    assert_eq!(admin::deletion_target(db, &me, 9999).await.unwrap_err().message, "player not found");
    let updated = admin::update_user(
        &state,
        &me,
        pending.id,
        admin_input(json!({"username":"IgnoredRename", "status":"active", "groups":[], "status_reason":"  "})),
    )
    .await
    .unwrap();
    assert_eq!((updated.username.as_str(), updated.uuid.as_str()), (pending.username.as_str(), pending.uuid.as_str()));
    assert!(updated.status_reason.is_none());
    assert_eq!(admin::list_groups(db).await.unwrap()[0].members, 0);
    assert!(web::login(
        &state,
        velora_platform_contracts::LoginRequest { username: updated.username, password: "synthetic-password".into() }
    )
    .await
    .is_ok());
    admin::delete_group(db, groups[0].id).await.unwrap();
    assert!(admin::list_groups(db).await.unwrap().is_empty());
}
#[tokio::test]
async fn administrator_password_updates_rollback_revocation_and_preserve_legacy_partial_field_order() {
    use velora_auth_http::{admin, web};
    let (state, _directory) = admin_fixture().await;
    let db = &state.authority.db;
    let me = admin::create_user(&state, admin_input(json!({"username":"Owner", "password":"synthetic-password", "role":"admin"})))
        .await
        .unwrap();
    let user = velora_auth_core::identity_store::find_by_name(db, "ExamplePlayer").await.unwrap().unwrap();
    let session = web::signed_in(&state, &user).await.unwrap();
    velora_auth_core::ygg_store::join(db, "synthetic-server", user.id, None, "2999-01-01T00:00:00Z").await.unwrap();
    sqlx::raw_sql("CREATE TRIGGER synthetic_revoke_failure BEFORE DELETE ON ygg_sessions BEGIN SELECT RAISE(ABORT, 'synthetic session failure'); END;").execute(db).await.unwrap();
    assert!(admin::update_user(&state, &me, user.id, admin_input(json!({"password":"replacement-password"})))
        .await
        .unwrap_err()
        .status
        .is_server_error());
    let restored = velora_auth_core::identity_store::find_by_id(db, user.id).await.unwrap().unwrap();
    assert_eq!((restored.password_hash.as_str(), restored.auth_version), (user.password_hash.as_str(), user.auth_version));
    assert!(velora_auth_http::token_user(&state.authority, &session.yggdrasil.as_ref().unwrap().access_token, None)
        .await
        .unwrap()
        .is_some());
    sqlx::raw_sql("DROP TRIGGER synthetic_revoke_failure;").execute(db).await.unwrap();
    let replaced = admin::update_user(
        &state,
        &me,
        user.id,
        admin_input(json!({"password":"replacement-password", "email":"  ", "status":"disabled", "status_reason":" Operator decision "})),
    )
    .await
    .unwrap();
    assert_eq!((replaced.uuid.as_str(), replaced.auth_version), (user.uuid.as_str(), user.auth_version + 1));
    assert!(replaced.email.is_none());
    assert_eq!(replaced.status_reason.as_deref(), Some("Operator decision"));
    assert!(velora_auth_http::token_user(&state.authority, &session.yggdrasil.unwrap().access_token, None).await.unwrap().is_none());
    assert_eq!(
        admin::update_user(&state, &me, user.id, admin_input(json!({"email":" changed@example.invalid ", "role":"invalid"})))
            .await
            .unwrap_err()
            .message,
        "role must be admin or player"
    );
    assert_eq!(
        velora_auth_core::identity_store::find_by_id(db, user.id).await.unwrap().unwrap().email.as_deref(),
        Some("changed@example.invalid")
    );
    db.close().await;
    assert_eq!(
        admin::create_group(db, serde_json::from_value(json!({"name":"Unavailable"})).unwrap()).await.unwrap_err().status,
        StatusCode::CONFLICT,
        "legacy create-group SQL failure mapping is retained"
    );
    assert!(admin::list_users(db).await.unwrap_err().status.is_server_error());
}
impl velora_auth_http::account::MutationHost for SyntheticMutationHost {
    fn online<'a>(&'a self, _: &'a str) -> HostFuture<'a, Result<bool, AppError>> {
        Box::pin(async move { Ok(self.online) })
    }
    fn renamed<'a, 'c>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'c, sqlx::Sqlite>,
        user: &'a velora_auth_core::identity::IdentityRecord,
        name: &'a str,
    ) -> HostFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            sqlx::query("INSERT INTO synthetic_audit (name, user_id) VALUES (?, ?)").bind(name).bind(user.id).execute(&mut **tx).await?;
            if self.fail {
                return Err(AppError::bad_request("synthetic audit failure"));
            }
            Ok(())
        })
    }
}
#[tokio::test]
async fn account_rename_preserves_host_transaction_rollback_and_live_online_admission() {
    use velora_auth_http::{
        account,
        web::{signed_in, AccountState},
    };
    let (_, authority, _directory) = fixture().await;
    sqlx::query("CREATE TABLE synthetic_audit (name TEXT, user_id INTEGER)").execute(&authority.db).await.unwrap();
    let state = AccountState {
        authority,
        tokens: Arc::new(velora_auth_core::tokens::Keys::new(b"synthetic-test-secret")),
        host: Arc::new(SyntheticAccountHost::new(velora_platform_contracts::RegistrationMode::Open)),
    };
    let user = velora_auth_core::identity_store::find_by_name(&state.authority.db, "ExamplePlayer").await.unwrap().unwrap();
    let session = signed_in(&state, &user).await.unwrap();
    let online = SyntheticMutationHost { online: true, fail: false };
    assert_eq!(
        account::set_username(&state, &online, &user, "NewName", "synthetic-password").await.unwrap_err().status,
        StatusCode::CONFLICT
    );
    let failing = SyntheticMutationHost { online: false, fail: true };
    assert_eq!(
        account::set_username(&state, &failing, &user, "NewName", "synthetic-password").await.unwrap_err().message,
        "synthetic audit failure"
    );
    let restored = velora_auth_core::identity_store::find_by_id(&state.authority.db, user.id).await.unwrap().unwrap();
    assert_eq!(
        (restored.username.as_str(), restored.uuid.as_str(), restored.auth_version),
        (user.username.as_str(), user.uuid.as_str(), user.auth_version)
    );
    assert!(velora_auth_http::token_user(&state.authority, &session.yggdrasil.as_ref().unwrap().access_token, None)
        .await
        .unwrap()
        .is_some());
    let audits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM synthetic_audit").fetch_one(&state.authority.db).await.unwrap();
    assert_eq!(audits, 0);
    let ready = SyntheticMutationHost { online: false, fail: false };
    assert_eq!(account::set_username(&state, &ready, &user, "NewName", "wrong").await.unwrap_err().message, "incorrect password");
    let renamed = account::set_username(&state, &ready, &user, " NewName ", "synthetic-password").await.unwrap();
    assert_eq!((renamed.user.username.as_str(), renamed.user.uuid.as_str()), ("NewName", user.uuid.as_str()));
    assert!(velora_auth_http::token_user(&state.authority, &session.yggdrasil.unwrap().access_token, None).await.unwrap().is_none());
    let renamed_user = velora_auth_core::identity_store::find_by_id(&state.authority.db, user.id).await.unwrap().unwrap();
    assert_eq!(renamed_user.auth_version, user.auth_version + 1);
}
#[tokio::test]
async fn account_cosmetic_workflows_preserve_model_normalization_cape_admission_and_missing_avatar() {
    use velora_auth_http::account;
    let (_, state, _directory) = fixture().await;
    let user = velora_auth_core::identity_store::find_by_name(&state.db, "ExamplePlayer").await.unwrap().unwrap();
    let headers = HeaderMap::new();
    assert_eq!(account::set_model(&state, &headers, &user, "slim").await.unwrap().skin_model, "slim");
    assert_eq!(account::set_model(&state, &headers, &user, "SLIM").await.unwrap().skin_model, "classic");
    assert_eq!(account::upload_skin(&state, &headers, &user, b"invalid", "slim").await.unwrap_err().status, StatusCode::BAD_REQUEST);
    assert_eq!(account::set_cape(&state, &headers, &user, Some(999)).await.unwrap_err().status, StatusCode::FORBIDDEN);
    sqlx::query("INSERT INTO capes VALUES (1, 'Synthetic', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'public', '[]', 'x')").execute(&state.db).await.unwrap();
    assert_eq!(account::set_cape(&state, &headers, &user, Some(1)).await.unwrap().cape.unwrap().id, 1);
    assert!(account::set_cape(&state, &headers, &user, None).await.unwrap().cape.is_none());
    let skinless = account::delete_skin(&state, &headers, &user).await.unwrap();
    assert_eq!(skinless.uuid, user.uuid);
    assert_eq!(account::avatar(&state, &user.username, None).await.unwrap_err().status, StatusCode::NOT_FOUND);
    state.db.close().await;
    assert!(account::set_model(&state, &headers, &user, "slim").await.unwrap_err().status.is_server_error());
}
struct SyntheticHost;
impl HostPorts for SyntheticHost {
    fn public_base<'a>(&'a self, _: &'a HeaderMap) -> HostFuture<'a, String> {
        Box::pin(async { "https://panel.example.invalid".into() })
    }
    fn server_name(&self) -> HostFuture<'_, Result<String, AppError>> {
        Box::pin(async { Ok("Velora".into()) })
    }
    fn client_ip<'a>(&'a self, _: &'a mut Parts) -> HostFuture<'a, Result<Option<String>, AppError>> {
        Box::pin(async { Ok(Some("192.0.2.10".into())) })
    }
}
async fn fixture() -> (axum::Router, AuthorityState, tempfile::TempDir) {
    let options = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
    velora_auth_core::schema::initialize(&pool).await.unwrap();
    let hash = velora_auth_core::password::hash_password("synthetic-password").unwrap();
    velora_auth_core::identity_store::insert(
        &pool,
        velora_auth_core::identity_store::NewAccount {
            username: "ExamplePlayer",
            password_hash: &hash,
            email: Some("player@example.invalid"),
            role: "member",
            status: "active",
            created_at: "2026-01-01T00:00:00Z",
            uuid: "01234567-89ab-4def-8123-456789abcdef",
        },
    )
    .await
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let state = AuthorityState {
        db: pool.clone(),
        identity_db: pool,
        ygg: Arc::new(velora_auth_core::keys::Keys::from_private(rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap()).unwrap()),
        login_guard: Arc::new(Default::default()),
        textures_dir: directory.path().into(),
        host: Arc::new(SyntheticHost),
        implementation_version: "fixture-version".into(),
    };
    (velora_auth_http::routes().with_state(state.clone()), state, directory)
}
async fn call(router: &axum::Router, method: &str, path: &str, body: Option<Value>, bearer: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path).header("content-type", "application/json");
    if let Some(bearer) = bearer {
        request = request.header("authorization", format!("Bearer {bearer}"));
    }
    let response =
        router.clone().oneshot(request.body(body.map(|v| Body::from(v.to_string())).unwrap_or_else(Body::empty)).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024).await.unwrap();
    (status, if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() })
}
const Y: &str = "/api/yggdrasil";
#[tokio::test]
async fn generic_router_metadata_login_refresh_join_profile_and_signout() {
    let (router, _, _directory) = fixture().await;
    let (status, meta) = call(&router, "GET", Y, None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(meta["meta"]["serverName"], "Velora");
    assert_eq!(meta["meta"]["implementationVersion"], "fixture-version");
    assert_eq!(meta["skinDomains"], json!(["panel.example.invalid"]));
    let (status, auth) = call(
        &router,
        "POST",
        &format!("{Y}/authserver/authenticate"),
        Some(json!({
        "username":"player@example.invalid", "password":"synthetic-password", "clientToken":"synthetic-client", "requestUser":true })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{auth}");
    assert_eq!(auth["selectedProfile"]["id"], "0123456789ab4def8123456789abcdef");
    assert!(auth["user"]["id"].is_string());
    let token = auth["accessToken"].as_str().unwrap();
    let (status, refreshed) = call(
        &router,
        "POST",
        &format!("{Y}/authserver/refresh"),
        Some(json!({"accessToken":token,"clientToken":"synthetic-client"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{refreshed}");
    assert_ne!(refreshed["accessToken"], token);
    assert_eq!(refreshed["clientToken"], "synthetic-client");
    assert_eq!(
        call(&router, "POST", &format!("{Y}/authserver/validate"), Some(json!({"accessToken":token})), None).await.0,
        StatusCode::FORBIDDEN
    );
    let token = refreshed["accessToken"].as_str().unwrap();
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{Y}/sessionserver/session/minecraft/join"),
            Some(json!({"accessToken":token,"selectedProfile":auth["selectedProfile"]["id"],"serverId":"synthetic-server"})),
            None
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let joined = format!("{Y}/sessionserver/session/minecraft/hasJoined?username=exampleplayer&serverId=synthetic-server&ip=192.0.2.10");
    let (status, profile) = call(&router, "GET", &joined, None, None).await;
    assert_eq!(status, StatusCode::OK, "{profile}");
    assert!(profile["properties"][0]["signature"].is_string());
    assert_eq!(call(&router, "GET", &joined.replace("192.0.2.10", "192.0.2.11"), None, None).await.0, StatusCode::NO_CONTENT);
    let (status, cert) = call(&router, "POST", &format!("{Y}/minecraftservices/player/certificates"), None, Some(token)).await;
    assert_eq!(status, StatusCode::OK, "{cert}");
    assert!(cert["keyPair"]["privateKey"].is_string());
    let (_, cached) = call(&router, "POST", &format!("{Y}/minecraftservices/player/certificates"), None, Some(token)).await;
    assert_eq!(cached, cert);
    assert_eq!(
        call(
            &router,
            "POST",
            &format!("{Y}/authserver/signout"),
            Some(json!({"username":"ExamplePlayer","password":"synthetic-password"})),
            None
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&router, "POST", &format!("{Y}/authserver/validate"), Some(json!({"accessToken":token})), None).await.0,
        StatusCode::FORBIDDEN
    );
}
#[tokio::test]
async fn live_disable_and_pool_outage_keep_protocol_errors_without_cached_admission() {
    let (router, state, _directory) = fixture().await;
    let (_, auth) = call(
        &router,
        "POST",
        &format!("{Y}/authserver/authenticate"),
        Some(json!({"username":"ExamplePlayer","password":"synthetic-password"})),
        None,
    )
    .await;
    let token = auth["accessToken"].as_str().unwrap();
    sqlx::query("UPDATE users SET status='disabled', status_reason='synthetic reason'").execute(&state.db).await.unwrap();
    let (status, error) = call(
        &router,
        "POST",
        &format!("{Y}/authserver/authenticate"),
        Some(json!({"username":"ExamplePlayer","password":"synthetic-password"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(error["error"], "ForbiddenOperationException");
    assert_eq!(error["errorMessage"], "synthetic reason");
    assert_eq!(
        call(&router, "POST", &format!("{Y}/authserver/validate"), Some(json!({"accessToken":token})), None).await.0,
        StatusCode::FORBIDDEN
    );
    state.db.close().await;
    let (status, error) = call(&router, "POST", &format!("{Y}/authserver/validate"), Some(json!({"accessToken":token})), None).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(error["error"], "InternalServerError");
}

struct SyntheticAccountHost {
    enabled: std::sync::atomic::AtomicBool,
    registration: std::sync::Mutex<velora_platform_contracts::RegistrationMode>,
    audit_failure: std::sync::atomic::AtomicBool,
    audit_calls: std::sync::atomic::AtomicUsize,
}
impl SyntheticAccountHost {
    fn new(registration: velora_platform_contracts::RegistrationMode) -> Self {
        Self { enabled: true.into(), registration: registration.into(), audit_failure: false.into(), audit_calls: 0.into() }
    }
}
impl velora_auth_http::web::AccountHost for SyntheticAccountHost {
    fn policy(&self) -> HostFuture<'_, Result<velora_auth_http::web::AccountPolicy, AppError>> {
        Box::pin(async {
            Ok(velora_auth_http::web::AccountPolicy {
                panel_accounts: self.enabled.load(std::sync::atomic::Ordering::SeqCst),
                registration: *self.registration.lock().unwrap(),
            })
        })
    }
    fn check_username<'a>(&'a self, name: &'a str) -> HostFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            if name.eq_ignore_ascii_case("BlockedName") {
                Err(AppError::bad_request("that username is unavailable; choose another"))
            } else {
                Ok(())
            }
        })
    }
    fn record_login<'a>(&'a self, _: &'a velora_auth_core::identity::IdentityRecord) -> HostFuture<'a, Result<(), AppError>> {
        Box::pin(async {
            self.audit_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.audit_failure.load(std::sync::atomic::Ordering::SeqCst) {
                Err(AppError { status: StatusCode::INTERNAL_SERVER_ERROR, message: "synthetic audit outage".into() })
            } else {
                Ok(())
            }
        })
    }
}
#[tokio::test]
async fn launcher_registration_approval_admin_policy_and_audit_order_remain_compatible() {
    use std::sync::atomic::Ordering::SeqCst;
    use velora_auth_http::web;
    use velora_platform_contracts::{LoginRequest, RegisterRequest, RegistrationMode};
    let (_, authority, _directory) = fixture().await;
    let host = Arc::new(SyntheticAccountHost::new(RegistrationMode::Closed));
    let state = web::AccountState {
        authority,
        tokens: Arc::new(velora_auth_core::tokens::Keys::new(b"synthetic-account-secret")),
        host: host.clone(),
    };
    let registration = |name: &str| RegisterRequest {
        username: name.into(),
        password: "synthetic-password".into(),
        email: Some(" new@example.invalid ".into()),
    };
    assert_eq!(web::register(&state, registration("NewPlayer")).await.unwrap_err().status, StatusCode::FORBIDDEN);
    *host.registration.lock().unwrap() = RegistrationMode::Approval;
    assert_eq!(web::register(&state, registration("a")).await.unwrap_err().message, "usernames are 3-16 letters, numbers or underscores");
    assert_eq!(
        web::register(&state, registration("BlockedName")).await.unwrap_err().message,
        "that username is unavailable; choose another"
    );
    let pending = web::register(&state, registration(" NewPlayer ")).await.unwrap();
    assert!(pending.pending);
    assert!(pending.token.is_empty());
    assert!(pending.yggdrasil.is_none());
    assert_eq!(pending.user.username, "NewPlayer");
    assert_eq!(uuid::Uuid::parse_str(&pending.user.uuid).unwrap().get_version_num(), 4);
    let id = pending.user.id;
    let stored = velora_auth_core::identity_store::find_by_id(&state.authority.db, id).await.unwrap().unwrap();
    assert_eq!(stored.email.as_deref(), Some("new@example.invalid"));
    assert_eq!(web::register(&state, registration("newplayer")).await.unwrap_err().message, "that username is taken");
    let login = || LoginRequest { username: " NewPlayer ".into(), password: "synthetic-password".into() };
    host.enabled.store(false, SeqCst);
    assert_eq!(web::login(&state, login()).await.unwrap_err().message, "your account is waiting for an admin to approve it");
    sqlx::query("UPDATE users SET status='active' WHERE id=?").bind(id).execute(&state.authority.db).await.unwrap();
    assert_eq!(web::login(&state, login()).await.unwrap_err().message, "account sign-in is currently disabled");
    sqlx::query("UPDATE users SET role='admin' WHERE id=?").bind(id).execute(&state.authority.db).await.unwrap();
    let admitted = web::login(&state, login()).await.unwrap();
    assert_eq!(admitted.user.uuid, pending.user.uuid);
    assert_eq!(state.tokens.verify(&admitted.token).unwrap().sub, id);
    assert!(crate_token_user(&state.authority, &admitted.yggdrasil.unwrap().access_token).await);
    assert_eq!(host.audit_calls.load(SeqCst), 1);
    assert!(velora_auth_core::identity_store::find_by_id(&state.authority.db, id).await.unwrap().unwrap().last_login.is_some());
    host.audit_failure.store(true, SeqCst);
    let token_count: i64 = sqlx::query_scalar("SELECT count(*) FROM ygg_tokens").fetch_one(&state.authority.db).await.unwrap();
    assert_eq!(web::login(&state, login()).await.unwrap_err().message, "synthetic audit outage");
    let after_count: i64 = sqlx::query_scalar("SELECT count(*) FROM ygg_tokens").fetch_one(&state.authority.db).await.unwrap();
    assert_eq!(after_count, token_count);
}
async fn crate_token_user(state: &AuthorityState, token: &str) -> bool {
    velora_auth_http::token_user(state, token, None).await.unwrap().is_some()
}
#[tokio::test]
async fn launcher_open_registration_disabled_reason_throttle_and_outage_remain_compatible() {
    use velora_auth_http::web;
    use velora_platform_contracts::{LoginRequest, RegisterRequest, RegistrationMode};
    let (_, authority, _directory) = fixture().await;
    let state = web::AccountState {
        authority,
        tokens: Arc::new(velora_auth_core::tokens::Keys::new(b"synthetic-account-secret")),
        host: Arc::new(SyntheticAccountHost::new(RegistrationMode::Open)),
    };
    let signed_in =
        web::register(&state, RegisterRequest { username: "NewPlayer".into(), password: "synthetic-password".into(), email: None })
            .await
            .unwrap();
    assert!(!signed_in.pending);
    assert!(!signed_in.token.is_empty());
    assert!(signed_in.yggdrasil.is_some());
    let login = |password: &str| LoginRequest { username: "NewPlayer".into(), password: password.into() };
    sqlx::query("UPDATE users SET status='disabled',status_reason='synthetic disabled reason' WHERE id=?")
        .bind(signed_in.user.id)
        .execute(&state.authority.db)
        .await
        .unwrap();
    assert_eq!(web::login(&state, login("wrong")).await.unwrap_err().status, StatusCode::UNAUTHORIZED);
    assert_eq!(web::login(&state, login("synthetic-password")).await.unwrap_err().message, "synthetic disabled reason");
    for _ in 0..10 {
        assert_eq!(web::login(&state, login("wrong")).await.unwrap_err().status, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(web::login(&state, login("synthetic-password")).await.unwrap_err().status, StatusCode::TOO_MANY_REQUESTS);
    state.authority.db.close().await;
    let error =
        web::login(&state, LoginRequest { username: "UncachedPlayer".into(), password: "synthetic-password".into() }).await.unwrap_err();
    assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error.message.starts_with("database error:"));
}

struct SyntheticResetHost {
    sent: std::sync::Mutex<Vec<(String, String, String)>>,
    fail: std::sync::atomic::AtomicBool,
    base_calls: std::sync::atomic::AtomicUsize,
}
impl SyntheticResetHost {
    fn new() -> Self {
        Self { sent: Vec::new().into(), fail: false.into(), base_calls: 0.into() }
    }
    fn token(&self) -> String {
        self.sent.lock().unwrap().last().unwrap().2.split("token=").nth(1).unwrap().split_whitespace().next().unwrap().into()
    }
}
impl velora_auth_http::reset::ResetHost for SyntheticResetHost {
    fn email_base(&self) -> HostFuture<'_, Result<String, AppError>> {
        Box::pin(async {
            self.base_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("https://reset.example.invalid".into())
        })
    }
    fn send_email<'a>(&'a self, to: &'a str, subject: &'a str, body: &'a str) -> HostFuture<'a, Result<(), AppError>> {
        Box::pin(async move {
            self.sent.lock().unwrap().push((to.into(), subject.into(), body.into()));
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                Err(AppError { status: StatusCode::BAD_GATEWAY, message: "synthetic email failure".into() })
            } else {
                Ok(())
            }
        })
    }
}
async fn reset_fixture() -> (AuthorityState, tempfile::TempDir) {
    let (_, state, directory) = fixture().await;
    (state, directory)
}
#[tokio::test]
async fn reset_links_are_hashed_throttled_single_use_and_revoke_existing_identity_sessions() {
    use sha2::{Digest, Sha256};
    use std::sync::atomic::Ordering::SeqCst;
    use velora_auth_http::reset;
    let (state, _directory) = reset_fixture().await;
    let host = SyntheticResetHost::new();
    let generic = reset::forgot_password(&state.db, &host, "not-an-email").await.unwrap();
    assert_eq!(host.base_calls.load(SeqCst), 0);
    assert_eq!(reset::forgot_password(&state.db, &host, "missing@example.invalid").await.unwrap(), generic);
    assert!(host.sent.lock().unwrap().is_empty());
    assert_eq!(reset::forgot_password(&state.db, &host, " PLAYER@EXAMPLE.INVALID ").await.unwrap(), generic);
    assert_eq!(reset::forgot_password(&state.db, &host, "player@example.invalid").await.unwrap(), generic);
    assert_eq!(host.sent.lock().unwrap().len(), 1);
    let token = host.token();
    assert_eq!(token.len(), 72);
    let stored_hash: String = sqlx::query_scalar("SELECT token_hash FROM password_resets").fetch_one(&state.db).await.unwrap();
    assert_eq!(stored_hash, hex::encode(Sha256::digest(token.as_bytes())));
    assert_ne!(stored_hash, token);
    let user = velora_auth_core::identity_store::find_by_id(&state.db, 1).await.unwrap().unwrap();
    let keys = velora_auth_core::tokens::Keys::new(b"synthetic-reset-session-secret");
    let old_token = keys
        .issue(velora_auth_core::tokens::Subject { id: user.id, name: &user.username, role: &user.role, auth_version: user.auth_version })
        .unwrap();
    let (old_game, _) = velora_auth_http::issue_token(&state, user.id, None).await.unwrap();
    velora_auth_core::ygg_store::join(&state.db, "synthetic-reset-server", user.id, None, "2000-01-01T00:00:00Z").await.unwrap();
    assert_eq!(reset::reset_password(&state.db, &token, "new-synthetic-password").await.unwrap(), json!({"ok":true}));
    assert_eq!(reset::reset_password(&state.db, &token, "other-synthetic-password").await.unwrap_err().status, StatusCode::BAD_REQUEST);
    let updated = velora_auth_core::identity_store::find_by_id(&state.db, 1).await.unwrap().unwrap();
    assert_eq!(updated.uuid, user.uuid);
    assert_eq!(updated.auth_version, 1);
    assert!(velora_auth_core::password::verify_password("new-synthetic-password", &updated.password_hash));
    assert!(!velora_auth_core::password::verify_password("synthetic-password", &updated.password_hash));
    assert!(velora_auth_http::token_user(&state, &old_game, None).await.unwrap().is_none());
    assert!(velora_auth_core::admission::session_admission(
        keys.verify(&old_token).unwrap().version,
        Some(velora_auth_core::admission::AccountStatus { status: &updated.status, auth_version: updated.auth_version })
    )
    .is_err());
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM ygg_sessions").fetch_one(&state.db).await.unwrap();
    assert_eq!(sessions, 0);
}
#[tokio::test]
async fn reset_email_failure_cleanup_expiry_and_transaction_failure_preserve_retry_behavior() {
    use std::sync::atomic::Ordering::SeqCst;
    use velora_auth_http::reset;
    let (state, _directory) = reset_fixture().await;
    let host = SyntheticResetHost::new();
    host.fail.store(true, SeqCst);
    let generic = reset::forgot_password(&state.db, &host, "player@example.invalid").await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM password_resets").fetch_one(&state.db).await.unwrap();
    assert_eq!(count, 0);
    host.fail.store(false, SeqCst);
    assert_eq!(reset::forgot_password(&state.db, &host, "player@example.invalid").await.unwrap(), generic);
    let token = host.token();
    sqlx::raw_sql("CREATE TRIGGER synthetic_reset_failure BEFORE UPDATE OF password_hash ON users BEGIN SELECT RAISE(ABORT,'synthetic password mutation outage'); END;").execute(&state.db).await.unwrap();
    assert_eq!(
        reset::reset_password(&state.db, &token, "new-synthetic-password").await.unwrap_err().status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let used: Option<String> = sqlx::query_scalar("SELECT used_at FROM password_resets").fetch_one(&state.db).await.unwrap();
    assert!(used.is_none());
    let user = velora_auth_core::identity_store::find_by_id(&state.db, 1).await.unwrap().unwrap();
    assert_eq!(user.auth_version, 0);
    assert!(velora_auth_core::password::verify_password("synthetic-password", &user.password_hash));
    sqlx::query("DROP TRIGGER synthetic_reset_failure").execute(&state.db).await.unwrap();
    assert!(reset::reset_password(&state.db, &token, "new-synthetic-password").await.is_ok());
    reset::forgot_password(&state.db, &host, "player@example.invalid").await.unwrap();
    let next_token = host.token();
    sqlx::query("UPDATE password_resets SET expires_at='2000-01-01T00:00:00Z'").execute(&state.db).await.unwrap();
    assert_eq!(
        reset::reset_password(&state.db, &next_token, "other-synthetic-password").await.unwrap_err().message,
        "this reset link is invalid or expired"
    );
    state.db.close().await;
    assert!(reset::forgot_password(&state.db, &host, "player@example.invalid").await.unwrap_err().status.is_server_error());
    assert_eq!(reset::forgot_password(&state.db, &host, "").await.unwrap(), generic);
}

struct SyntheticDiscordHost {
    profile: std::sync::Mutex<Value>,
    enabled: std::sync::atomic::AtomicBool,
    mode: std::sync::Mutex<velora_platform_contracts::RegistrationMode>,
    exchange_calls: std::sync::atomic::AtomicUsize,
}
impl SyntheticDiscordHost {
    fn new(profile: Value) -> Self {
        Self {
            profile: profile.into(),
            enabled: true.into(),
            mode: velora_platform_contracts::RegistrationMode::Open.into(),
            exchange_calls: 0.into(),
        }
    }
}
impl velora_auth_http::oauth::DiscordHost for SyntheticDiscordHost {
    fn configuration(&self) -> HostFuture<'_, Result<velora_auth_http::oauth::DiscordConfiguration, AppError>> {
        Box::pin(async { Ok(velora_auth_http::oauth::DiscordConfiguration { client_id: "synthetic-client-id".into(), configured: true }) })
    }
    fn public_base(&self) -> HostFuture<'_, Result<String, AppError>> {
        Box::pin(async { Ok("https://login.example.invalid".into()) })
    }
    fn exchange_profile<'a>(&'a self, _: &'a str) -> HostFuture<'a, Result<Value, AppError>> {
        Box::pin(async {
            self.exchange_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.profile.lock().unwrap().clone())
        })
    }
    fn allocation_policy(&self) -> HostFuture<'_, Result<velora_auth_http::oauth::AllocationPolicy, AppError>> {
        Box::pin(async {
            Ok(velora_auth_http::oauth::AllocationPolicy {
                panel_accounts: self.enabled.load(std::sync::atomic::Ordering::SeqCst),
                registration: *self.mode.lock().unwrap(),
                username_blocklist: vec!["BlockedName".into()],
            })
        })
    }
}
async fn oauth_fixture() -> (velora_auth_http::web::AccountState, tempfile::TempDir) {
    let (_, authority, directory) = fixture().await;
    (
        velora_auth_http::web::AccountState {
            authority,
            tokens: Arc::new(velora_auth_core::tokens::Keys::new(b"synthetic-oauth-secret")),
            host: Arc::new(SyntheticAccountHost::new(velora_platform_contracts::RegistrationMode::Open)),
        },
        directory,
    )
}
#[tokio::test]
async fn discord_flow_cancellation_replay_link_ownership_and_login_poll_stay_compatible() {
    use velora_auth_http::oauth;
    let (state, _directory) = oauth_fixture().await;
    let host = SyntheticDiscordHost::new(json!({"id":"synthetic-discord-one","username":"RemoteName","global_name":"Remote Display"}));
    assert_eq!(oauth::start(&state, &host, None, Some("link")).await.unwrap_err().status, StatusCode::UNAUTHORIZED);
    let cancelled = oauth::start(&state, &host, None, None).await.unwrap();
    let token = cancelled["state"].as_str().unwrap();
    assert_eq!(token.len(), 72);
    assert!(cancelled["url"]
        .as_str()
        .unwrap()
        .contains("redirect_uri=https%3A%2F%2Flogin%2Eexample%2Einvalid%2Fapi%2Fv1%2Fauth%2Fdiscord%2Fcallback"));
    assert_eq!(oauth::poll(&state, token).await.unwrap(), json!({"pending":true}));
    assert!(oauth::callback(&state, &host, token, None, true).await.unwrap().contains("cancelled"));
    assert_eq!(host.exchange_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        oauth::callback(&state, &host, token, Some("synthetic-code"), false).await.unwrap_err().message,
        "Discord sign-in expired; try again"
    );
    assert_eq!(oauth::poll(&state, token).await.unwrap_err().message, "error:Discord authorization was cancelled");
    assert!(oauth::poll(&state, token).await.is_err());
    let user = velora_auth_core::identity_store::find_by_id(&state.authority.db, 1).await.unwrap().unwrap();
    let link = oauth::start(&state, &host, Some(&user), Some("link")).await.unwrap();
    let link_token = link["state"].as_str().unwrap();
    oauth::callback(&state, &host, link_token, Some("synthetic-code"), false).await.unwrap();
    assert_eq!(oauth::poll(&state, link_token).await.unwrap(), json!({"linked":true}));
    assert_eq!(oauth::linked(&state, 1).await.unwrap(), json!({"id":"synthetic-discord-one","name":"Remote Display"}));
    let other_id =
        velora_auth_http::web::create_account(&state, "OtherPlayer", "synthetic-password", None, "player", "active").await.unwrap();
    let other = velora_auth_core::identity_store::find_by_id(&state.authority.db, other_id).await.unwrap().unwrap();
    let conflict = oauth::start(&state, &host, Some(&other), Some("link")).await.unwrap();
    assert_eq!(
        oauth::callback(&state, &host, conflict["state"].as_str().unwrap(), Some("synthetic-code"), false).await.unwrap_err().status,
        StatusCode::CONFLICT
    );
    let login = oauth::start(&state, &host, None, None).await.unwrap();
    let login_token = login["state"].as_str().unwrap();
    oauth::callback(&state, &host, login_token, Some("synthetic-code"), false).await.unwrap();
    let response = oauth::poll(&state, login_token).await.unwrap();
    assert_eq!(response["user"]["uuid"], user.uuid);
    assert_eq!(state.tokens.verify(response["token"].as_str().unwrap()).unwrap().sub, user.id);
    assert!(velora_auth_http::token_user(&state.authority, response["yggdrasil"]["access_token"].as_str().unwrap(), None)
        .await
        .unwrap()
        .is_some());
    assert!(oauth::poll(&state, login_token).await.is_err());
    assert_eq!(oauth::unlink(&state, 1).await.unwrap(), json!({"ok":true}));
    assert_eq!(oauth::linked(&state, 1).await.unwrap(), Value::Null);
}
#[tokio::test]
async fn discord_allocation_respects_approval_closed_policy_verified_email_and_live_disabled_status() {
    use std::sync::atomic::Ordering::SeqCst;
    use velora_auth_http::oauth;
    use velora_platform_contracts::RegistrationMode;
    let (state, _directory) = oauth_fixture().await;
    let host = SyntheticDiscordHost::new(
        json!({"id":"synthetic-discord-new","username":"BlockedName","email":"new@example.invalid","verified":true}),
    );
    *host.mode.lock().unwrap() = RegistrationMode::Closed;
    let denied = oauth::start(&state, &host, None, None).await.unwrap();
    assert_eq!(
        oauth::callback(&state, &host, denied["state"].as_str().unwrap(), Some("synthetic-code"), false).await.unwrap_err().status,
        StatusCode::FORBIDDEN
    );
    *host.mode.lock().unwrap() = RegistrationMode::Approval;
    let pending = oauth::start(&state, &host, None, None).await.unwrap();
    let pending_token = pending["state"].as_str().unwrap();
    oauth::callback(&state, &host, pending_token, Some("synthetic-code"), false).await.unwrap();
    assert_eq!(oauth::poll(&state, pending_token).await.unwrap(), json!({"pending":true}));
    let user = velora_auth_core::identity_store::find_by_name(&state.authority.identity_db, "Player").await.unwrap().unwrap();
    assert_eq!(user.email.as_deref(), Some("new@example.invalid"));
    assert_eq!(uuid::Uuid::parse_str(&user.uuid).unwrap().get_version_num(), 4);
    sqlx::query("UPDATE users SET status='disabled' WHERE id=?").bind(user.id).execute(&state.authority.db).await.unwrap();
    let disabled = oauth::start(&state, &host, None, None).await.unwrap();
    assert_eq!(
        oauth::callback(&state, &host, disabled["state"].as_str().unwrap(), Some("synthetic-code"), false).await.unwrap_err().message,
        "this account is disabled"
    );
    sqlx::query("UPDATE users SET status='active' WHERE id=?").bind(user.id).execute(&state.authority.db).await.unwrap();
    host.enabled.store(false, SeqCst);
    let closed = oauth::start(&state, &host, None, None).await.unwrap();
    assert_eq!(
        oauth::callback(&state, &host, closed["state"].as_str().unwrap(), Some("synthetic-code"), false).await.unwrap_err().message,
        "account sign-in is disabled"
    );
    let expired = oauth::start(&state, &host, None, None).await.unwrap();
    sqlx::query("UPDATE oauth_attempts SET expires_at='2000-01-01T00:00:00Z' WHERE state=?")
        .bind(expired["state"].as_str().unwrap())
        .execute(&state.authority.db)
        .await
        .unwrap();
    assert!(oauth::poll(&state, expired["state"].as_str().unwrap()).await.is_err());
    state.authority.db.close().await;
    assert!(oauth::linked(&state, user.id).await.unwrap_err().status.is_server_error());
}
