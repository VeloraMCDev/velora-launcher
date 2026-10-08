//! The protected host must not admit a cached JWT when live authority lookup fails.
mod common;
use common::*;

#[tokio::test]
async fn protected_routes_refuse_cached_sessions_when_identity_store_is_unavailable() {
    let t = setup().await;
    let token = t.login("admin", "supersecret").await;
    assert_eq!(t.call("GET", "/api/admin/users", Some(&token), None).await.0, StatusCode::OK);
    t.db.close().await;
    assert_eq!(t.call("GET", "/api/admin/users", Some(&token), None).await.0, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn game_tokens_and_join_sessions_require_the_live_authority() {
    let t = setup().await;
    let root = "/api/yggdrasil";
    let (status, body) = t
        .call(
            "POST",
            &format!("{root}/authserver/authenticate"),
            None,
            Some(json!({"username":"admin", "password":"supersecret", "clientToken":"synthetic-client"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let access = body["accessToken"].as_str().unwrap();
    let profile = body["selectedProfile"]["id"].as_str().unwrap();
    let validate = json!({"accessToken":access,"clientToken":"synthetic-client"});
    assert_eq!(t.call("POST", &format!("{root}/authserver/validate"), None, Some(validate.clone())).await.0, StatusCode::NO_CONTENT);
    let join = json!({"accessToken":access,"selectedProfile":profile,"serverId":"synthetic-server"});
    assert_eq!(
        t.call("POST", &format!("{root}/sessionserver/session/minecraft/join"), None, Some(join.clone())).await.0,
        StatusCode::NO_CONTENT
    );
    let joined = format!("{root}/sessionserver/session/minecraft/hasJoined?username=admin&serverId=synthetic-server");
    assert_eq!(t.call("GET", &joined, None, None).await.0, StatusCode::OK);
    t.db.close().await;
    for (method, path, body) in [
        ("POST", format!("{root}/authserver/validate"), Some(validate)),
        ("POST", format!("{root}/sessionserver/session/minecraft/join"), Some(join)),
        ("GET", joined, None),
    ] {
        let (status, body) = t.call(method, &path, None, body).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{path}: {body}");
        assert_eq!(body["error"], "InternalServerError");
    }
}
