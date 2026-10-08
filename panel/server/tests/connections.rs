//! Synthetic connection settings and guarded delivery fixtures; no provider traffic.
mod common;
use common::*;

#[tokio::test]
async fn connection_settings_keep_authorization_masking_secret_updates_and_validation_atomicity() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let input = json!({"discord_client_id":" 123 ", "discord_client_secret":"synthetic-client-secret",
        "discord_bot_token":"synthetic-bot-secret", "discord_guild_id":" 456 ", "resend_api_key":"synthetic-mail-secret",
        "sender_email":" sender@example.invalid ", "sender_name":" Velora "});
    assert_eq!(t.call("PUT", "/api/admin/connections", None, Some(input.clone())).await.0, StatusCode::UNAUTHORIZED);
    let (status, masked) = t.call("PUT", "/api/admin/connections", Some(&admin), Some(input.clone())).await;
    assert_eq!(status, StatusCode::OK, "{masked}");
    assert_eq!(
        masked,
        json!({"discord_client_id":"123","discord_client_secret_set":true,"discord_bot_token_set":true,
        "discord_guild_id":"456","resend_api_key_set":true,"sender_email":"sender@example.invalid","sender_name":"Velora",
        "discord_enabled":true,"email_enabled":true})
    );
    for secret in ["synthetic-client-secret", "synthetic-bot-secret", "synthetic-mail-secret"] {
        assert!(!masked.to_string().contains(secret));
    }
    let before: String = sqlx::query_scalar("SELECT value FROM kv WHERE key='connections_settings'").fetch_one(&t.db).await.unwrap();
    let mut invalid = input.clone();
    invalid["sender_email"] = json!("invalid");
    invalid["discord_client_id"] = json!("invalid");
    let (status, error) = t.call("PUT", "/api/admin/connections", Some(&admin), Some(invalid)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "enter a valid sender email address");
    let after: String = sqlx::query_scalar("SELECT value FROM kv WHERE key='connections_settings'").fetch_one(&t.db).await.unwrap();
    assert_eq!(before, after);
    let mut update = input;
    update["discord_client_secret"] = json!(" ");
    update["discord_bot_token"] = json!(" - ");
    update["resend_api_key"] = json!("-");
    let (status, masked) = t.call("PUT", "/api/admin/connections", Some(&admin), Some(update)).await;
    assert_eq!(status, StatusCode::OK, "{masked}");
    assert_eq!(masked["discord_client_secret_set"], true);
    assert_eq!(masked["discord_bot_token_set"], false);
    assert_eq!(masked["email_enabled"], false);
    let stored: String = sqlx::query_scalar("SELECT value FROM kv WHERE key='connections_settings'").fetch_one(&t.db).await.unwrap();
    let stored: Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored["discord_client_secret"], "synthetic-client-secret");
    assert_eq!(stored["resend_api_key"], "");
    assert_eq!(
        t.call("POST", "/api/admin/connections/test-email", None, Some(json!({"email":"invalid"}))).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, error) = t.call("POST", "/api/admin/connections/test-email", Some(&admin), Some(json!({"email":"invalid"}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    assert_eq!(error["error"], "configure Resend SMTP and a sender email in Settings first");
}
