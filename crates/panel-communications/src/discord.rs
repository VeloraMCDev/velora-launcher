use crate::{settings::ConnectionsSettings, Error};
use serde_json::Value;

pub async fn profile(client: &reqwest::Client, settings: &ConnectionsSettings, callback: &str, code: &str) -> Result<Value, Error> {
    exchange_at(client, settings, callback, code, "https://discord.com/api/v10/oauth2/token", "https://discord.com/api/v10/users/@me").await
}
async fn exchange_at(
    client: &reqwest::Client,
    settings: &ConnectionsSettings,
    callback: &str,
    code: &str,
    token_url: &str,
    profile_url: &str,
) -> Result<Value, Error> {
    let response = client
        .post(token_url)
        .form(&[
            ("client_id", settings.discord_client_id.as_str()),
            ("client_secret", settings.discord_client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", callback),
        ])
        .send()
        .await
        .map_err(|error| Error::gateway(format!("Discord token exchange failed: {error}")))?;
    if !response.status().is_success() {
        return Err(Error::request("Discord rejected authorization"));
    }
    let token: Value = response.json().await.map_err(|_| Error::request("invalid Discord response"))?;
    let bearer = token["access_token"].as_str().ok_or_else(|| Error::request("Discord token missing"))?;
    let response = client
        .get(profile_url)
        .bearer_auth(bearer)
        .send()
        .await
        .map_err(|error| Error::gateway(format!("Discord profile failed: {error}")))?;
    if !response.status().is_success() {
        return Err(Error::request("Discord profile unavailable"));
    }
    response.json().await.map_err(|_| Error::request("invalid Discord profile"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Bytes,
        extract::State,
        http::{HeaderMap, StatusCode},
        routing::{get, post},
        Router,
    };
    use std::sync::{Arc, Mutex};
    type Requests = Arc<Mutex<Vec<(HeaderMap, String)>>>;
    async fn fixture(
        token_status: StatusCode,
        token_body: &str,
        profile_status: StatusCode,
        profile_body: &str,
    ) -> (String, Requests, tokio::task::JoinHandle<()>) {
        let requests: Requests = Default::default();
        let token_body = token_body.to_owned();
        let profile_body = profile_body.to_owned();
        let app = Router::new()
            .route(
                "/token",
                post(move |State(requests): State<Requests>, headers: HeaderMap, body: Bytes| async move {
                    requests.lock().unwrap().push((headers, String::from_utf8(body.to_vec()).unwrap()));
                    (token_status, token_body)
                }),
            )
            .route(
                "/profile",
                get(move |State(requests): State<Requests>, headers: HeaderMap| async move {
                    requests.lock().unwrap().push((headers, String::new()));
                    (profile_status, profile_body)
                }),
            )
            .with_state(requests.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (base, requests, handle)
    }
    fn settings() -> ConnectionsSettings {
        ConnectionsSettings { discord_client_id: "123".into(), discord_client_secret: "synthetic & secret".into(), ..Default::default() }
    }
    #[tokio::test]
    async fn exchanges_encoded_form_then_bearer_profile_without_forwarding_client_secret() {
        let (base, requests, server) =
            fixture(StatusCode::OK, r#"{"access_token":"synthetic-bearer"}"#, StatusCode::OK, r#"{"id":"456","unknown":{"kept":true}}"#)
                .await;
        let profile = exchange_at(
            &reqwest::Client::new(),
            &settings(),
            "https://example.invalid/callback",
            "code & value",
            &format!("{base}/token"),
            &format!("{base}/profile"),
        )
        .await
        .unwrap();
        assert_eq!(profile, serde_json::json!({"id":"456", "unknown":{"kept":true}}));
        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0].0["content-type"], "application/x-www-form-urlencoded");
        assert!(!captured[0].0.contains_key("authorization"));
        assert_eq!(captured[0].1, "client_id=123&client_secret=synthetic+%26+secret&grant_type=authorization_code&code=code+%26+value&redirect_uri=https%3A%2F%2Fexample.invalid%2Fcallback");
        assert_eq!(captured[1].0["authorization"], "Bearer synthetic-bearer");
        assert!(captured[1].1.is_empty());
        assert!(!format!("{:?}", captured[1].0).contains("synthetic & secret"));
        server.abort();
    }
    #[tokio::test]
    async fn rejected_or_malformed_tokens_stop_before_profile_and_keep_error_precedence() {
        for (status, body, expected) in [
            (StatusCode::UNAUTHORIZED, "not json", "Discord rejected authorization"),
            (StatusCode::OK, "not json", "invalid Discord response"),
            (StatusCode::OK, r#"{"access_token":123}"#, "Discord token missing"),
        ] {
            let (base, requests, server) = fixture(status, body, StatusCode::OK, "{}").await;
            let error =
                exchange_at(&reqwest::Client::new(), &settings(), "callback", "code", &format!("{base}/token"), &format!("{base}/profile"))
                    .await
                    .unwrap_err();
            assert_eq!(error.kind, crate::ErrorKind::BadRequest);
            assert_eq!(error.message, expected);
            assert_eq!(requests.lock().unwrap().len(), 1);
            server.abort();
        }
    }
    #[tokio::test]
    async fn profile_status_and_json_errors_follow_successful_token_exchange() {
        for (status, body, expected) in
            [(StatusCode::FORBIDDEN, "not json", "Discord profile unavailable"), (StatusCode::OK, "not json", "invalid Discord profile")]
        {
            let (base, requests, server) = fixture(StatusCode::OK, r#"{"access_token":"synthetic-bearer"}"#, status, body).await;
            let error =
                exchange_at(&reqwest::Client::new(), &settings(), "callback", "code", &format!("{base}/token"), &format!("{base}/profile"))
                    .await
                    .unwrap_err();
            assert_eq!(error.kind, crate::ErrorKind::BadRequest);
            assert_eq!(error.message, expected);
            assert_eq!(requests.lock().unwrap().len(), 2);
            server.abort();
        }
    }
    #[tokio::test]
    async fn transport_failures_remain_gateway_errors() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(2)).build().unwrap();
        let error = exchange_at(&client, &settings(), "callback", "code", &closed, &closed).await.unwrap_err();
        assert_eq!(error.kind, crate::ErrorKind::BadGateway);
        assert!(error.message.starts_with("Discord token exchange failed:"));
        let (base, requests, server) = fixture(StatusCode::OK, r#"{"access_token":"synthetic-bearer"}"#, StatusCode::OK, "{}").await;
        let error = exchange_at(&client, &settings(), "callback", "code", &format!("{base}/token"), &closed).await.unwrap_err();
        assert_eq!(error.kind, crate::ErrorKind::BadGateway);
        assert!(error.message.starts_with("Discord profile failed:"));
        assert_eq!(requests.lock().unwrap().len(), 1);
        server.abort();
    }
}
