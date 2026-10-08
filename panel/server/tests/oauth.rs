//! OAuth lifecycle fixtures use cancellation and synthetic stored results; no provider requests occur.
mod common;
use common::*;
async fn configured() -> (TestApp, String) {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let (status, response) = t.call("PUT", "/api/admin/connections", Some(&admin), Some(json!({
        "discord_client_id":"123456789", "discord_client_secret":"synthetic-provider-secret", "discord_bot_token":"", "discord_guild_id":"",
        "resend_api_key":"", "sender_email":"", "sender_name":""
    }))).await;
    assert_eq!(status, StatusCode::OK, "{response}");
    (t, admin)
}
#[tokio::test]
async fn discord_start_link_cancel_and_poll_keep_request_authorization_and_one_use_behavior() {
    let (t, admin) = configured().await;
    let (status, error) = t.call("GET", "/api/v1/auth/discord/start?kind=link", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error["error"], "sign in before linking Discord");
    let (status, started) = t.call("GET", "/api/v1/auth/discord/start?kind=link", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let state = started["state"].as_str().unwrap();
    assert_eq!(state.len(), 72);
    assert!(started["url"].as_str().unwrap().contains("client_id=123456789"));
    assert!(started["url"].as_str().unwrap().contains("redirect_uri=https%3A%2F%2Fpanel%2Etest%2Fapi%2Fv1%2Fauth%2Fdiscord%2Fcallback"));
    let (_, pending) = t.call("GET", &format!("/api/v1/auth/discord/poll?state={state}"), None, None).await;
    assert_eq!(pending, json!({"pending":true}));
    let (status, text) = t.call("GET", &format!("/api/v1/auth/discord/callback?state={state}&error=access_denied"), None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(text.as_str().unwrap().contains("cancelled"));
    let (status, error) = t.call("GET", &format!("/api/v1/auth/discord/callback?state={state}&error=access_denied"), None, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "Discord sign-in expired; try again");
    let (status, error) = t.call("GET", &format!("/api/v1/auth/discord/poll?state={state}"), None, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "error:Discord authorization was cancelled");
    assert_eq!(t.call("GET", &format!("/api/v1/auth/discord/poll?state={state}"), None, None).await.0, StatusCode::BAD_REQUEST);
}
#[tokio::test]
async fn discord_expiry_stored_result_and_unlink_preserve_identity_and_other_provider_rows() {
    let (t, admin) = configured().await;
    let (_, me) = t.call("GET", "/api/v1/auth/me", Some(&admin), None).await;
    let id = me["id"].as_i64().unwrap();
    sqlx::query("INSERT INTO account_connections(user_id,provider,provider_id,display_name,created_at) VALUES(?,'discord','synthetic-discord-id','Synthetic Display','fixture'),(?,'synthetic-other-provider','other-id','Other Display','fixture')")
        .bind(id).bind(id).execute(&t.db).await.unwrap();
    let (status, linked) = t.call("GET", "/api/v1/account/connections/discord", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(linked, json!({"id":"synthetic-discord-id","name":"Synthetic Display"}));
    assert_eq!(t.call("DELETE", "/api/v1/account/connections/discord", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(t.call("DELETE", "/api/v1/account/connections/discord", Some(&admin), None).await.0, StatusCode::OK);
    assert_eq!(t.call("GET", "/api/v1/account/connections/discord", Some(&admin), None).await.1, Value::Null);
    let other: i64 = sqlx::query_scalar("SELECT count(*) FROM account_connections WHERE provider='synthetic-other-provider'")
        .fetch_one(&t.db)
        .await
        .unwrap();
    assert_eq!(other, 1);
    let (_, attempt) = t.call("GET", "/api/v1/auth/discord/start", None, None).await;
    let state = attempt["state"].as_str().unwrap();
    let result = json!({"linked":true});
    sqlx::query("UPDATE oauth_attempts SET result=? WHERE state=?").bind(result.to_string()).bind(state).execute(&t.db).await.unwrap();
    assert_eq!(t.call("GET", &format!("/api/v1/auth/discord/poll?state={state}"), None, None).await.1, result);
    assert_eq!(t.call("GET", &format!("/api/v1/auth/discord/poll?state={state}"), None, None).await.0, StatusCode::BAD_REQUEST);
    let (_, expired) = t.call("GET", "/api/v1/auth/discord/start", None, None).await;
    let state = expired["state"].as_str().unwrap();
    sqlx::query("UPDATE oauth_attempts SET expires_at='2000-01-01T00:00:00Z' WHERE state=?").bind(state).execute(&t.db).await.unwrap();
    assert_eq!(t.call("GET", &format!("/api/v1/auth/discord/poll?state={state}"), None, None).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(t.uuid("admin").await, me["uuid"].as_str().unwrap());
}
