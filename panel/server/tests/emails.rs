mod common;
use common::*;

#[tokio::test]
async fn template_routes_preserve_distinct_id_rules_validation_order_and_storage_failures() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    let create = json!({"id":" synthetic ","name":"🎮".repeat(80),"subject":"界".repeat(200),"body":"é".repeat(20_000)});
    assert_eq!(t.call("POST", "/api/admin/email-templates", None, Some(create.clone())).await.0, StatusCode::UNAUTHORIZED);
    let (status, created) = t.call("POST", "/api/admin/email-templates", Some(&admin), Some(create.clone())).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["id"], "synthetic");
    let mut duplicate = create.clone();
    duplicate["body"] = json!("x".repeat(20_001));
    assert_eq!(t.call("POST", "/api/admin/email-templates", Some(&admin), Some(duplicate)).await.0, StatusCode::CONFLICT);
    let long_id = "a".repeat(51);
    let mut long = create;
    long["id"] = json!(long_id);
    let (status, error) = t.call("POST", "/api/admin/email-templates", Some(&admin), Some(long.clone())).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "Template ID must be 1-50 alphanumeric characters, underscore or dash");
    // Bulk replacement intentionally retains its different legacy ID-length policy.
    let other = json!({"id":"Case","name":"Synthetic","subject":"{unknown}","body":"Synthetic body"});
    let bulk = json!({"templates":[long, other]});
    assert_eq!(t.call("PUT", "/api/admin/email/templates", Some(&admin), Some(bulk)).await.0, StatusCode::OK);
    let before: String = sqlx::query_scalar("SELECT value FROM kv WHERE key='email_templates'").fetch_one(&t.db).await.unwrap();
    let (status, error) = t.call("PUT", "/api/admin/email-templates/Case", Some(&admin), Some(json!({"name":"界".repeat(81)}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "a template is too long");
    let invalid = json!({"templates":[{"id":"bad id","name":"x".repeat(81)},{"id":"bad id"}]});
    let (status, error) = t.call("PUT", "/api/admin/email/templates", Some(&admin), Some(invalid)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"], "each template needs its own short id (letters, numbers, - and _)");
    let after: String = sqlx::query_scalar("SELECT value FROM kv WHERE key='email_templates'").fetch_one(&t.db).await.unwrap();
    assert_eq!(before, after);
    sqlx::query("CREATE TRIGGER reject_synthetic_email_update BEFORE UPDATE ON kv WHEN NEW.key='email_templates' BEGIN SELECT RAISE(ABORT,'synthetic storage failure'); END")
        .execute(&t.db).await.unwrap();
    assert_eq!(
        t.call("PUT", "/api/admin/email-templates/Case", Some(&admin), Some(json!({"name":"Changed"}))).await.0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let after: String = sqlx::query_scalar("SELECT value FROM kv WHERE key='email_templates'").fetch_one(&t.db).await.unwrap();
    assert_eq!(before, after);
    let (status, fetched) = t.call("GET", &format!("/api/admin/email-templates/{long_id}"), Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["template"]["body"], "é".repeat(20_000));
}

#[tokio::test]
async fn emails_use_templates_audiences_and_opt_outs() {
    let t = setup().await;
    let admin = t.login("admin", "supersecret").await;
    for (n, e) in [("Steve", "steve@example.com"), ("Alex", "alex@example.com"), ("Notch", "")] {
        t.call("POST", "/api/admin/users", Some(&admin), Some(json!({"username": n, "password": "password123", "email": e}))).await;
    }
    let (s, all) = t.call("GET", "/api/admin/email", Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "{all}");
    assert_eq!(all["configured"], false);
    assert!(all["templates"].as_array().unwrap().len() >= 3, "starter templates are offered");

    // Templates save, with sensible limits.
    let bad = json!({"templates": [{"id": "bad id!", "name": "x", "subject": "s", "body": "b"}]});
    assert_eq!(t.call("PUT", "/api/admin/email/templates", Some(&admin), Some(bad)).await.0, StatusCode::BAD_REQUEST);
    let ok = json!({"templates": [{"id": "news", "name": "News", "subject": "Hi {player}", "body": "Hello **{player}**, level {level}\n\n[button: Go]({panel_url})"}]});
    assert_eq!(t.call("PUT", "/api/admin/email/templates", Some(&admin), Some(ok)).await.0, StatusCode::OK);

    // Placeholders fill per player, and the HTML is branded and has an unsubscribe link.
    let (s, p) = t
        .call(
            "POST",
            "/api/admin/email/preview",
            Some(&admin),
            Some(json!({"subject": "Hi {player}", "body": "Hello **{player}**\n\n[button: Go]({panel_url})"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{p}");
    assert_eq!(p["subject"], "Hi admin");
    assert!(p["html"].as_str().unwrap().contains("<strong>admin</strong>") && p["html"].as_str().unwrap().contains("Unsubscribe"));
    assert!(p["text"].as_str().unwrap().contains("/api/v1/email/unsubscribe?u="));

    // Only active players with an email count.
    let count = |a: Value| {
        let (t, admin) = (&t, admin.clone());
        async move { t.call("POST", "/api/admin/email/audience", Some(&admin), Some(json!({"audience": a}))).await.1 }
    };
    assert_eq!(count(json!({"kind": "all"})).await["count"], 2, "Steve and Alex have an address; Notch and admin don't");
    assert_eq!(count(json!({"kind": "players", "names": ["steve", "nobody"]})).await["count"], 1);

    // Opting out through the link removes a player from marketing mail, but not from account notices.
    let steve = t.uuid("Steve").await;
    let token = {
        let secret: String =
            sqlx::query_scalar("SELECT value FROM kv WHERE key = 'email_unsubscribe_secret'").fetch_one(&t.db).await.unwrap();
        let secret = secret.trim_matches('"').to_string();
        use sha2::{Digest, Sha256};
        Sha256::digest(format!("{secret}:{steve}").as_bytes()).iter().take(16).map(|b| format!("{b:02x}")).collect::<String>()
    };
    let (s, _) = t.fetch(&format!("/api/v1/email/unsubscribe?u={steve}&t=wrong")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(count(json!({"kind": "all"})).await["count"], 2, "a wrong token changes nothing");
    t.fetch(&format!("/api/v1/email/unsubscribe?u={steve}&t={token}")).await;
    assert_eq!(count(json!({"kind": "all"})).await["count"], 1);
    let (_, important) =
        t.call("POST", "/api/admin/email/audience", Some(&admin), Some(json!({"audience": {"kind": "all"}, "important": true}))).await;
    assert_eq!(important["count"], 2);

    // Without SMTP configured, a send is queued but each message fails with a clear reason in the log.
    let (s, sent) = t
        .call("POST", "/api/admin/email/send", Some(&admin), Some(json!({"subject": "Hi", "body": "Hello", "audience": {"kind": "all"}})))
        .await;
    assert_eq!(s, StatusCode::OK, "{sent}");
    for _ in 0..40 {
        let (_, all) = t.call("GET", "/api/admin/email", Some(&admin), None).await;
        if all["log"][0]["failed"] == 1 {
            assert!(all["log"][0]["last_error"].as_str().unwrap().contains("Resend"));
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("the send was never logged");
}
