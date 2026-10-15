use reqwest::{
    header::{HeaderValue, AUTHORIZATION},
    Client, Method, RequestBuilder, Response, StatusCode, Url,
};
use serde::de::DeserializeOwned;
use std::fmt;

pub const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ERROR_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureKind {
    Network,
    Auth,
    Status,
    Decode,
    Config,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelFailure {
    pub kind: FailureKind,
    pub status: Option<u16>,
    message: &'static str,
}

impl PanelFailure {
    fn new(kind: FailureKind, status: Option<StatusCode>, message: &'static str) -> Self {
        Self { kind, status: status.map(|status| status.as_u16()), message }
    }
}

impl fmt::Display for PanelFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for PanelFailure {}

#[derive(Clone, Copy, Debug)]
pub enum ErrorPolicy {
    Generic,
    Account,
    Fallback(&'static str),
}

#[derive(Clone)]
pub struct PanelClient {
    http: Client,
    base: Url,
    bearer: Option<HeaderValue>,
}

pub(crate) trait PanelRequest {
    async fn panel_send(self) -> Result<Response, PanelFailure>;
}

impl PanelRequest for RequestBuilder {
    async fn panel_send(self) -> Result<Response, PanelFailure> {
        PanelClient::send(self).await
    }
}

impl fmt::Debug for PanelClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("PanelClient").field("authenticated", &self.bearer.is_some()).finish_non_exhaustive()
    }
}

impl PanelClient {
    pub fn new(http: Client, base: &str, bearer: Option<&str>) -> Result<Self, PanelFailure> {
        let invalid = || PanelFailure::new(FailureKind::Config, None, "invalid panel configuration");
        let base = Self::parse_base(base)?;
        let bearer = bearer
            .map(|token| -> Result<HeaderValue, PanelFailure> {
                let mut header = HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| invalid())?;
                header.set_sensitive(true);
                Ok(header)
            })
            .transpose()?;
        Ok(Self { http, base, bearer })
    }

    pub(crate) fn same_base(first: &str, second: &str) -> bool {
        matches!((Self::parse_base(first), Self::parse_base(second)), (Ok(first), Ok(second)) if first == second)
    }

    fn parse_base(base: &str) -> Result<Url, PanelFailure> {
        let invalid = || PanelFailure::new(FailureKind::Config, None, "invalid panel configuration");
        let mut base = Url::parse(base).map_err(|_| invalid())?;
        if !matches!(base.scheme(), "http" | "https")
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(invalid());
        }
        let path = format!("{}/", base.path().trim_end_matches('/'));
        base.set_path(&path);
        Ok(base)
    }

    pub fn request(&self, method: Method, endpoint: &str) -> Result<RequestBuilder, PanelFailure> {
        let invalid = || PanelFailure::new(FailureKind::Config, None, "invalid panel endpoint");
        if !endpoint.starts_with('/') || endpoint.starts_with("//") || endpoint.contains('\\') || endpoint.contains('#') {
            return Err(invalid());
        }
        let url = self.base.join(&format!("api/v1{endpoint}")).map_err(|_| invalid())?;
        let prefix = format!("{}api/v1/", self.base.path());
        if url.origin() != self.base.origin() || !url.path().starts_with(&prefix) {
            return Err(invalid());
        }
        let request = self.http.request(method, url);
        Ok(match &self.bearer {
            Some(bearer) => request.header(AUTHORIZATION, bearer.clone()),
            None => request,
        })
    }

    pub async fn send(request: RequestBuilder) -> Result<Response, PanelFailure> {
        request.send().await.map_err(|error| {
            if error.is_builder() {
                PanelFailure::new(FailureKind::Config, None, "invalid panel request")
            } else {
                PanelFailure::new(FailureKind::Network, None, "couldn't reach the server")
            }
        })
    }

    pub async fn json<Value: DeserializeOwned>(response: Response, policy: ErrorPolicy) -> Result<Value, PanelFailure> {
        Self::json_with_limit(response, policy, MAX_JSON_BYTES).await
    }

    pub async fn json_with_limit<Value: DeserializeOwned>(
        response: Response,
        policy: ErrorPolicy,
        limit: usize,
    ) -> Result<Value, PanelFailure> {
        let response = Self::success(response, policy).await?;
        let status = response.status();
        let bytes = Self::body(response, limit.min(MAX_JSON_BYTES)).await?;
        serde_json::from_slice(&bytes).map_err(|_| PanelFailure::new(FailureKind::Decode, Some(status), "invalid panel response"))
    }

    pub async fn empty(response: Response, policy: ErrorPolicy) -> Result<(), PanelFailure> {
        Self::success(response, policy).await.map(|_| ())
    }

    pub async fn json_value<Value: DeserializeOwned + serde::Serialize>(
        response: Response,
        policy: ErrorPolicy,
    ) -> Result<serde_json::Value, PanelFailure> {
        let status = response.status();
        let value: Value = Self::json(response, policy).await?;
        serde_json::to_value(value).map_err(|_| PanelFailure::new(FailureKind::Decode, Some(status), "invalid panel response"))
    }

    pub async fn json_fields<Value: DeserializeOwned + serde::Serialize>(
        response: Response,
        policy: ErrorPolicy,
        fields: &[&str],
    ) -> Result<serde_json::Value, PanelFailure> {
        let mut value = Self::json_value::<Value>(response, policy).await?;
        if let Some(object) = value.as_object_mut() {
            object.retain(|key, _| fields.contains(&key.as_str()));
        }
        Ok(value)
    }

    pub async fn json_or_null(response: Response, policy: ErrorPolicy) -> Result<serde_json::Value, PanelFailure> {
        let response = Self::success(response, policy).await?;
        let status = response.status();
        let bytes = Self::body(response, MAX_JSON_BYTES).await?;
        if bytes.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_slice(&bytes).map_err(|_| PanelFailure::new(FailureKind::Decode, Some(status), "invalid panel response"))
    }

    pub async fn success(response: Response, policy: ErrorPolicy) -> Result<Response, PanelFailure> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let kind = if matches!(status.as_u16(), 401 | 403) { FailureKind::Auth } else { FailureKind::Status };
        let mut message = match policy {
            ErrorPolicy::Fallback(fallback) => fallback,
            _ => "panel request failed",
        };
        if !matches!(policy, ErrorPolicy::Generic) {
            if let Ok(bytes) = Self::body(response, MAX_ERROR_BYTES).await {
                if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    let supplied = body.get("error").and_then(|value| value.as_str()).or_else(|| {
                        body.get(if matches!(policy, ErrorPolicy::Account) { "errorMessage" } else { "message" })
                            .and_then(|value| value.as_str())
                    });
                    let safe = supplied.and_then(|supplied| match policy {
                        ErrorPolicy::Account => account_message(supplied),
                        ErrorPolicy::Fallback(_) => public_message(supplied),
                        ErrorPolicy::Generic => None,
                    });
                    if let Some(safe) = safe {
                        message = safe;
                    }
                }
            }
        }
        Err(PanelFailure::new(kind, Some(status), message))
    }

    async fn body(mut response: Response, limit: usize) -> Result<Vec<u8>, PanelFailure> {
        let status = response.status();
        let oversized = || PanelFailure::new(FailureKind::Decode, Some(status), "panel response exceeds size limit");
        if response.content_length().is_some_and(|length| length > limit as u64) {
            return Err(oversized());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| PanelFailure::new(FailureKind::Network, Some(status), "couldn't read the server response"))?
        {
            if chunk.len() > limit.saturating_sub(bytes.len()) {
                return Err(oversized());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

fn account_message(supplied: &str) -> Option<&'static str> {
    const MESSAGES: &[&str] = &[
        "wrong username or password",
        "your account is waiting for an admin to approve it",
        "this account has been disabled",
        "account sign-in is currently disabled",
        "sign-ups are closed — ask an admin for an account",
        "usernames are 3-16 letters, numbers or underscores",
        "passwords need at least 8 characters",
        "that username is taken",
        "that username is taken or reserved",
        "too many failed attempts, try again in a few minutes",
        "Discord sign-in is not configured",
        "invalid Discord flow",
        "sign in before linking Discord",
        "Discord sign-in expired; try again",
        "Discord sign-in was already used",
        "Discord authorization was cancelled",
        "Discord ID missing",
        "link target missing",
        "Discord account is linked to another player",
        "Discord account is not linked; create an account first",
        "could not allocate a username",
        "this account is disabled",
        "account sign-in is disabled",
        "Discord sign-in expired",
        "Discord result was already consumed",
    ];
    MESSAGES.iter().copied().find(|message| *message == supplied)
}

fn public_message(supplied: &str) -> Option<&'static str> {
    const MESSAGES: &[&str] = &[
        "Faction not found",
        "Only members can see their faction's land rules",
        "The server admins manage that rule, so factions can't change it",
        "Rules are on or off",
        "Only the leader, officers and roles that can manage the faction may change land rules",
        "That's you",
        "Player not found",
        "They aren't online on this server right now",
        "Join the server first: a teleport request needs you in game",
        "Write a message first",
        "You can't invite yourself",
        "Your faction role cannot invite players",
        "You have no open faction invitation",
        "You're already in a faction here. Leave it first.",
        "invalid player UUID",
        "unlock not found",
        "Message cannot be empty",
        "You can't message yourself",
        "Messages can be up to 1000 characters",
        "You can only invite friends",
        "Instance not found",
    ];
    MESSAGES.iter().copied().find(|message| *message == supplied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn server(status: u16, body: &str, chunked: bool) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let body = body.to_owned();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
            }
            let response = if chunked {
                format!(
                    "HTTP/1.1 {status} Test\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n",
                    body.len()
                )
            } else {
                format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
            };
            socket.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        (format!("http://{address}/baked/prefix"), task)
    }

    async fn response(status: u16, body: &str, chunked: bool) -> Response {
        let (base, task) = server(status, body, chunked).await;
        let client = PanelClient::new(Client::builder().no_proxy().build().unwrap(), &base, None).unwrap();
        let response = PanelClient::send(client.request(Method::GET, "/fixture").unwrap()).await.unwrap();
        task.await.unwrap();
        response
    }

    #[tokio::test]
    async fn typed_json_preserves_base_path_auth_and_query_encoding() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Fixture {
            count: u32,
        }
        let (base, task) = server(200, "{\"count\":7}", false).await;
        let client = PanelClient::new(Client::builder().no_proxy().build().unwrap(), &base, Some("synthetic-bearer")).unwrap();
        let request = client
            .request(Method::GET, "/fixture?kind=link")
            .unwrap()
            .query(&[("state", "synthetic &+=/?")])
            .header("X-Velora-Instance", "synthetic-instance");
        let value: Fixture = PanelClient::json(PanelClient::send(request).await.unwrap(), ErrorPolicy::Generic).await.unwrap();
        assert_eq!(value, Fixture { count: 7 });
        let captured = task.await.unwrap().to_lowercase();
        assert!(captured.starts_with("get /baked/prefix/api/v1/fixture?kind=link&state=synthetic+%26%2b%3d%2f%3f http/1.1"));
        assert!(captured.contains("authorization: bearer synthetic-bearer"));
        assert!(captured.contains("x-velora-instance: synthetic-instance"));
        assert!(!captured.contains("cache-control:"));
        assert!(!format!("{client:?}").contains("synthetic-bearer"));
        assert!(!format!("{client:?}").contains("baked"));
    }

    #[tokio::test]
    async fn anonymous_empty_and_dynamic_success_are_explicit() {
        let (base, task) = server(204, "", false).await;
        let client = PanelClient::new(Client::builder().no_proxy().build().unwrap(), &base, None).unwrap();
        PanelClient::empty(PanelClient::send(client.request(Method::DELETE, "/fixture").unwrap()).await.unwrap(), ErrorPolicy::Generic)
            .await
            .unwrap();
        assert!(!task.await.unwrap().to_lowercase().contains("authorization:"));
        assert_eq!(PanelClient::json_or_null(response(204, "", false).await, ErrorPolicy::Generic).await.unwrap(), serde_json::Value::Null);
        assert_eq!(
            PanelClient::json_or_null(response(200, "not-json", false).await, ErrorPolicy::Generic).await.unwrap_err().kind,
            FailureKind::Decode
        );
        let dynamic: serde_json::Value =
            PanelClient::json(response(200, "{\"pending\":true}", false).await, ErrorPolicy::Generic).await.unwrap();
        assert_eq!(dynamic["pending"], true);
    }

    #[tokio::test]
    async fn status_auth_and_account_errors_never_echo_untrusted_text() {
        let secret = "https://operator.invalid/private?password=synthetic-password Bearer synthetic-token";
        for status in [400, 401, 403, 500] {
            let body = serde_json::json!({"error": secret}).to_string();
            let failure = PanelClient::empty(response(status, &body, false).await, ErrorPolicy::Account).await.unwrap_err();
            assert_eq!(failure.status, Some(status));
            assert_eq!(failure.kind, if matches!(status, 401 | 403) { FailureKind::Auth } else { FailureKind::Status });
            for sensitive in ["operator.invalid", "synthetic-password", "synthetic-token", "Bearer"] {
                assert!(!format!("{failure} {failure:?}").contains(sensitive));
            }
            assert!(std::error::Error::source(&failure).is_none());
        }
        let failure = PanelClient::empty(response(401, "{\"error\":\"wrong username or password\"}", false).await, ErrorPolicy::Account)
            .await
            .unwrap_err();
        assert_eq!(failure.to_string(), "wrong username or password");
        let failure = PanelClient::empty(response(409, "{\"errorMessage\":\"that username is taken\"}", false).await, ErrorPolicy::Account)
            .await
            .unwrap_err();
        assert_eq!(failure.to_string(), "that username is taken");
        let failure =
            PanelClient::empty(response(500, "not-json", false).await, ErrorPolicy::Fallback("unable to load posts")).await.unwrap_err();
        assert_eq!(failure.to_string(), "unable to load posts");
        let failure = PanelClient::empty(
            response(400, "{\"message\":\"Message cannot be empty\"}", false).await,
            ErrorPolicy::Fallback("unable to send message"),
        )
        .await
        .unwrap_err();
        assert_eq!(failure.to_string(), "Message cannot be empty");
        let body = serde_json::json!({"error": secret}).to_string();
        let failure =
            PanelClient::empty(response(500, &body, false).await, ErrorPolicy::Fallback("unable to send message")).await.unwrap_err();
        assert_eq!(failure.to_string(), "unable to send message");
    }

    #[tokio::test]
    async fn typed_discord_fields_strip_unknown_secrets_and_preserve_nullability() {
        use velora_shared::launcher_contracts::{DiscordConnectionResponse, DiscordInfoResponse, DiscordStartResponse};
        let fixture = serde_json::json!({"url":"https://example.invalid/authorize","state":"synthetic-state","account":{"id":"injected"},"token":"synthetic-injected-token"});
        let actual = PanelClient::json_fields::<DiscordStartResponse>(
            response(200, &fixture.to_string(), false).await,
            ErrorPolicy::Account,
            &["url", "state"],
        )
        .await
        .unwrap();
        assert_eq!(actual, serde_json::json!({"url":"https://example.invalid/authorize","state":"synthetic-state"}));
        let actual = PanelClient::json_fields::<DiscordConnectionResponse>(
            response(200, "null", false).await,
            ErrorPolicy::Account,
            &["id", "name"],
        )
        .await
        .unwrap();
        assert_eq!(actual, serde_json::Value::Null);
        for fixture in
            [serde_json::json!({"invite_url":"","linked":false}), serde_json::json!({"invite_url":"","linked":false,"display_name":null})]
        {
            let mut injected = fixture.clone();
            injected["token"] = serde_json::json!("synthetic-injected-token");
            let actual = PanelClient::json_fields::<DiscordInfoResponse>(
                response(200, &injected.to_string(), false).await,
                ErrorPolicy::Account,
                &["invite_url", "linked", "display_name"],
            )
            .await
            .unwrap();
            assert_eq!(actual, fixture);
        }
        assert_eq!(
            PanelClient::json_fields::<DiscordStartResponse>(
                response(200, "{\"url\":true,\"state\":\"synthetic-state\"}", false).await,
                ErrorPolicy::Account,
                &["url", "state"]
            )
            .await
            .unwrap_err()
            .kind,
            FailureKind::Decode
        );
    }

    #[tokio::test]
    async fn malformed_and_oversized_success_fail_closed() {
        for body in ["synthetic-password not-json", ""] {
            let failure = PanelClient::json::<serde_json::Value>(response(200, body, false).await, ErrorPolicy::Generic).await.unwrap_err();
            assert_eq!(failure.kind, FailureKind::Decode);
            assert!(!format!("{failure:?}").contains("synthetic-password"));
        }
        for chunked in [false, true] {
            let failure =
                PanelClient::json_with_limit::<serde_json::Value>(response(200, "{\"count\":7}", chunked).await, ErrorPolicy::Generic, 4)
                    .await
                    .unwrap_err();
            assert_eq!(failure.message, "panel response exceeds size limit");
        }
        let body = "x".repeat(MAX_ERROR_BYTES + 1);
        let failure = PanelClient::empty(response(403, &body, true).await, ErrorPolicy::Account).await.unwrap_err();
        assert_eq!(failure.kind, FailureKind::Auth);
        assert_eq!(failure.message, "panel request failed");
    }

    #[tokio::test]
    async fn network_and_invalid_configuration_are_redacted() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/private", listener.local_addr().unwrap());
        drop(listener);
        let client = PanelClient::new(Client::builder().no_proxy().build().unwrap(), &base, Some("synthetic-token")).unwrap();
        let failure = PanelClient::send(client.request(Method::GET, "/fixture?password=synthetic-password").unwrap()).await.unwrap_err();
        assert_eq!(failure.kind, FailureKind::Network);
        assert!(!format!("{failure} {failure:?}").contains("private"));
        for base in [
            "not a url",
            "ftp://operator.invalid",
            "https://user:synthetic-password@operator.invalid",
            "https://operator.invalid?token=synthetic-token",
            "https://operator.invalid#fragment",
        ] {
            let failure = PanelClient::new(Client::new(), base, None).unwrap_err();
            assert_eq!(failure.kind, FailureKind::Config);
            assert!(!format!("{failure} {failure:?}").contains("operator"));
        }
        assert!(PanelClient::new(Client::new(), "https://example.invalid", Some("synthetic\npassword")).is_err());
        for endpoint in [
            "https://operator.invalid",
            "//operator.invalid",
            "/../../escape",
            "/%2e%2e/%2e%2e/escape",
            "/fixture#fragment",
            "/fixture\\escape",
        ] {
            assert!(client.request(Method::GET, endpoint).is_err());
        }
    }
}
