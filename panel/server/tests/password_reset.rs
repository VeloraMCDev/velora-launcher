//! Password recovery uses synthetic stored tokens; these fixtures never send email.
mod common;
use common::*;
use sha2::{Digest, Sha256};
const TOKEN: &str = "synthetic-reset-token-not-a-credential";
async fn prepared() -> (TestApp, Value) {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (status, user) = t
        .call(
            "POST",
            "/api/admin/users",
            Some(&admin),
            Some(json!({"username":"ResetPlayer", "password":"old-password123", "email":"reset@example.invalid"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, signed_in) =
        t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"ResetPlayer", "password":"old-password123"}))).await;
    sqlx::query("INSERT INTO password_resets(token_hash,user_id,expires_at) VALUES(?,?,?)")
        .bind(hex::encode(Sha256::digest(TOKEN.as_bytes())))
        .bind(user["id"].as_i64().unwrap())
        .bind("2099-01-01T00:00:00Z")
        .execute(&t.db)
        .await
        .unwrap();
    (t, signed_in)
}
async fn reset(t: &TestApp, token: &str) -> (StatusCode, Value) {
    t.call("POST", "/api/v1/auth/reset-password", None, Some(json!({"token":token,"password":"new-password123"}))).await
}
#[tokio::test]
async fn reset_is_single_use_preserves_uuid_and_revokes_web_game_and_join_sessions() {
    let (t, previous) = prepared().await;
    let web = previous["token"].as_str().unwrap();
    let game = previous["yggdrasil"]["access_token"].as_str().unwrap();
    let uuid = previous["user"]["uuid"].as_str().unwrap();
    let (status, _) = t
        .call(
            "POST",
            "/api/yggdrasil/sessionserver/session/minecraft/join",
            None,
            Some(json!({"accessToken":game,"selectedProfile":uuid.replace('-',""),"serverId":"reset-fixture"})),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(reset(&t, "invalid-synthetic-token").await.0, StatusCode::BAD_REQUEST);
    let (status, response) = reset(&t, TOKEN).await;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response, json!({"ok":true}));
    assert_eq!(reset(&t, TOKEN).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(web), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        t.call("POST", "/api/yggdrasil/authserver/validate", None, Some(json!({"accessToken":game}))).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        t.call("GET", "/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=ResetPlayer&serverId=reset-fixture", None, None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"ResetPlayer","password":"old-password123"}))).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, next) =
        t.call("POST", "/api/v1/auth/login", None, Some(json!({"username":"ResetPlayer","password":"new-password123"}))).await;
    assert_eq!(status, StatusCode::OK, "{next}");
    assert_eq!(next["user"]["uuid"], uuid);
}
#[tokio::test]
async fn password_mutation_failure_leaves_reset_retriable_and_email_configuration_errors_stay_compatible() {
    let (t, previous) = prepared().await;
    let web = previous["token"].as_str().unwrap();
    let game = previous["yggdrasil"]["access_token"].as_str().unwrap();
    sqlx::raw_sql("CREATE TRIGGER synthetic_reset_failure BEFORE UPDATE OF password_hash ON users BEGIN SELECT RAISE(ABORT,'synthetic reset outage'); END;").execute(&t.db).await.unwrap();
    assert_eq!(reset(&t, TOKEN).await.0, StatusCode::INTERNAL_SERVER_ERROR);
    let used: Option<String> = sqlx::query_scalar("SELECT used_at FROM password_resets").fetch_one(&t.db).await.unwrap();
    assert!(used.is_none());
    assert_eq!(t.call("GET", "/api/v1/auth/me", Some(web), None).await.0, StatusCode::OK);
    assert_eq!(
        t.call("POST", "/api/yggdrasil/authserver/validate", None, Some(json!({"accessToken":game}))).await.0,
        StatusCode::NO_CONTENT
    );
    sqlx::query("DROP TRIGGER synthetic_reset_failure").execute(&t.db).await.unwrap();
    assert_eq!(reset(&t, TOKEN).await.0, StatusCode::OK);
    let (status, generic) = t.call("POST", "/api/v1/auth/forgot-password", None, Some(json!({"email":"not-an-email"}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(generic["ok"], true);
    let (status, error) = t.call("POST", "/api/v1/auth/forgot-password", None, Some(json!({"email":"missing@example.invalid"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "password reset email is not configured");
}
