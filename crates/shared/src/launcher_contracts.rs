use crate::{AuthResponse, PublicUser, YggdrasilTokens};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionsConfig {
    pub discord_enabled: bool,
    pub email_enabled: bool,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordStartResponse {
    pub url: String,
    pub state: String,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConnection {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

pub type DiscordConnectionResponse = Option<DiscordConnection>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordInfoResponse {
    pub invite_url: String,
    pub linked: bool,
    #[serde(default, deserialize_with = "present_nullable", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<Option<String>>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordPendingResponse {
    pub pending: bool,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordLinkedResponse {
    pub linked: bool,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUnlinkResponse {
    pub ok: bool,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordAuthUser {
    pub id: i64,
    pub username: String,
    pub uuid: String,
    pub role: String,
    pub groups: Vec<String>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGameTokens {
    pub access_token: String,
    pub client_token: String,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

fn present_nullable<'de, Deserialization, Payload>(deserializer: Deserialization) -> Result<Option<Option<Payload>>, Deserialization::Error>
where
    Deserialization: Deserializer<'de>,
    Payload: Deserialize<'de>,
{
    Option::<Payload>::deserialize(deserializer).map(Some)
}

fn present_boolean<'de, Deserialization>(deserializer: Deserialization) -> Result<Option<bool>, Deserialization::Error>
where
    Deserialization: Deserializer<'de>,
{
    bool::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordAuthResponse {
    pub token: String,
    pub user: DiscordAuthUser,
    #[serde(default, deserialize_with = "present_boolean", skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(default, deserialize_with = "present_nullable", skip_serializing_if = "Option::is_none")]
    pub yggdrasil: Option<Option<DiscordGameTokens>>,
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl DiscordAuthResponse {
    pub fn into_auth_response(self) -> AuthResponse {
        AuthResponse {
            token: self.token,
            user: PublicUser {
                id: self.user.id,
                username: self.user.username,
                uuid: self.user.uuid,
                role: self.user.role,
                groups: self.user.groups,
            },
            pending: self.pending.unwrap_or(false),
            yggdrasil: self
                .yggdrasil
                .flatten()
                .map(|tokens| YggdrasilTokens { access_token: tokens.access_token, client_token: tokens.client_token }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum DiscordPollResponse {
    Authenticated(DiscordAuthResponse),
    Linked(DiscordLinkedResponse),
    Pending(DiscordPendingResponse),
}

impl<'de> Deserialize<'de> for DiscordPollResponse {
    fn deserialize<Deserialization>(deserializer: Deserialization) -> Result<Self, Deserialization::Error>
    where
        Deserialization: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let object = value.as_object().ok_or_else(|| serde::de::Error::custom("Discord poll response must be an object"))?;
        if object.contains_key("token") {
            return serde_json::from_value(value).map(Self::Authenticated).map_err(serde::de::Error::custom);
        }
        if object.contains_key("linked") {
            let linked: DiscordLinkedResponse = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            if !linked.linked {
                return Err(serde::de::Error::custom("Discord link completion must be true"));
            }
            return Ok(Self::Linked(linked));
        }
        let pending: DiscordPendingResponse = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        if !pending.pending {
            return Err(serde::de::Error::custom("Discord pending response must be true"));
        }
        Ok(Self::Pending(pending))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_consumer_fixtures_preserve_panel_wire_json() {
        let fixtures: Vec<Value> = serde_json::from_str(include_str!("../test-fixtures/launcher-contracts.json")).unwrap();
        for fixture in fixtures {
            let value = fixture["value"].clone();
            let output = match fixture["type"].as_str().unwrap() {
                "config" => serde_json::to_value(serde_json::from_value::<ConnectionsConfig>(value.clone()).unwrap()).unwrap(),
                "start" => serde_json::to_value(serde_json::from_value::<DiscordStartResponse>(value.clone()).unwrap()).unwrap(),
                "connection" => serde_json::to_value(serde_json::from_value::<DiscordConnectionResponse>(value.clone()).unwrap()).unwrap(),
                "info" => serde_json::to_value(serde_json::from_value::<DiscordInfoResponse>(value.clone()).unwrap()).unwrap(),
                "poll" => serde_json::to_value(serde_json::from_value::<DiscordPollResponse>(value.clone()).unwrap()).unwrap(),
                "unlink" => serde_json::to_value(serde_json::from_value::<DiscordUnlinkResponse>(value.clone()).unwrap()).unwrap(),
                "ipc_poll" => continue,
                unexpected => panic!("unknown fixture type: {unexpected}"),
            };
            assert_eq!(output, value);
        }
    }
    use serde::de::DeserializeOwned;
    use serde_json::json;

    fn round_trip<Payload: DeserializeOwned + Serialize>(fixture: Value) {
        let payload: Payload = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(payload).unwrap(), fixture);
    }

    fn auth() -> Value {
        json!({"token":"fixture-panel-session","user":{"id":17,"username":"FixturePlayer",
            "uuid":"00000000-0000-0000-0000-000000000017","role":"player","groups":[]}})
    }

    #[test]
    fn configuration_and_start_preserve_producer_wire_and_extensions() {
        round_trip::<ConnectionsConfig>(json!({"discord_enabled":false,"email_enabled":true}));
        round_trip::<DiscordStartResponse>(json!({"url":"https://example.invalid/authorize","state":"fixture-state"}));
        round_trip::<DiscordStartResponse>(
            json!({"url":"https://example.invalid/authorize","state":"fixture-state","future":{"version":2}}),
        );
    }

    #[test]
    fn connection_preserves_null_and_linked_shapes() {
        round_trip::<DiscordConnectionResponse>(Value::Null);
        round_trip::<DiscordConnectionResponse>(json!({"id":"fixture-discord-id","name":"Fixture Player"}));
        round_trip::<DiscordConnectionResponse>(json!({"id":"fixture-discord-id","name":"Fixture Player","future":7}));
    }

    #[test]
    fn info_and_unlink_preserve_producer_wire() {
        round_trip::<DiscordInfoResponse>(json!({"invite_url":"","linked":false}));
        round_trip::<DiscordInfoResponse>(json!({"invite_url":"","linked":false,"display_name":null}));
        round_trip::<DiscordInfoResponse>(
            json!({"invite_url":"https://example.invalid/invite","linked":true,"display_name":"Fixture Player"}),
        );
        round_trip::<DiscordUnlinkResponse>(json!({"ok":true}));
    }

    #[test]
    fn pending_and_link_poll_preserve_discriminants_and_extensions() {
        round_trip::<DiscordPollResponse>(json!({"pending":true}));
        round_trip::<DiscordPollResponse>(json!({"linked":true}));
        round_trip::<DiscordPollResponse>(json!({"pending":true,"future":[1,2]}));
        round_trip::<DiscordPollResponse>(json!({"linked":true,"future":null}));
    }

    #[test]
    fn legacy_auth_missing_optional_fields_preserves_omission_and_defaults() {
        round_trip::<DiscordPollResponse>(auth());
        let response: DiscordAuthResponse = serde_json::from_value(auth()).unwrap();
        let existing = response.into_auth_response();
        assert!(!existing.pending);
        assert!(existing.yggdrasil.is_none());
        let legacy: AuthResponse = serde_json::from_value(auth()).unwrap();
        assert!(!legacy.pending);
        assert!(legacy.yggdrasil.is_none());
    }

    #[test]
    fn auth_null_and_present_game_tokens_preserve_exact_wire() {
        let mut fixture = auth();
        fixture["pending"] = json!(true);
        fixture["yggdrasil"] = Value::Null;
        round_trip::<DiscordPollResponse>(fixture.clone());
        fixture["pending"] = json!(false);
        fixture["yggdrasil"] = json!({"access_token":"fixture-game-session","client_token":"fixture-client"});
        round_trip::<DiscordPollResponse>(fixture.clone());
        let converted: DiscordAuthResponse = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(converted.into_auth_response()).unwrap(), fixture);
    }

    #[test]
    fn auth_preserves_nested_unknown_extensions() {
        let mut fixture = auth();
        fixture["future"] = json!({"capabilities":[]});
        fixture["user"]["future"] = json!(42);
        fixture["yggdrasil"] = json!({"access_token":"fixture-game-session","client_token":"fixture-client","future":null});
        round_trip::<DiscordPollResponse>(fixture);
    }

    #[test]
    fn malformed_auth_cannot_fall_back_to_pending_or_linked() {
        for fixture in [
            json!({"token":null,"pending":true}),
            json!({"token":17,"linked":true}),
            json!({"token":"fixture-panel-session","pending":true}),
            json!({"token":"fixture-panel-session","user":null,"linked":true}),
        ] {
            assert!(serde_json::from_value::<DiscordPollResponse>(fixture).is_err());
        }
        for field in ["id", "username", "uuid", "role", "groups"] {
            let mut fixture = auth();
            fixture["user"].as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<DiscordPollResponse>(fixture).is_err());
        }
        let mut fixture = auth();
        fixture["yggdrasil"] = json!({"access_token":"fixture-game-session"});
        assert!(serde_json::from_value::<DiscordPollResponse>(fixture).is_err());
    }

    #[test]
    fn malformed_required_fields_are_not_defaulted() {
        for fixture in [json!({}), json!({"url":null,"state":"fixture-state"}), json!({"url":"https://example.invalid","state":3})] {
            assert!(serde_json::from_value::<DiscordStartResponse>(fixture).is_err());
        }
        assert!(serde_json::from_value::<ConnectionsConfig>(json!({"discord_enabled":true})).is_err());
        assert!(serde_json::from_value::<DiscordConnectionResponse>(json!({"id":"fixture-id"})).is_err());
        assert!(serde_json::from_value::<DiscordConnectionResponse>(json!({"id":2,"name":"Fixture"})).is_err());
        for fixture in [Value::Null, json!([]), json!({}), json!({"pending":"true"}), json!({"pending":false}), json!({"linked":false})] {
            assert!(serde_json::from_value::<DiscordPollResponse>(fixture).is_err());
        }
        let mut fixture = auth();
        fixture["pending"] = Value::Null;
        assert!(serde_json::from_value::<DiscordPollResponse>(fixture.clone()).is_err());
        assert!(serde_json::from_value::<AuthResponse>(fixture).is_err());
    }

    #[test]
    fn malformed_auth_user_and_game_token_types_are_rejected() {
        for (field, value) in [("id", json!("17")), ("username", Value::Null), ("groups", json!([17]))] {
            let mut fixture = auth();
            fixture["user"][field] = value;
            assert!(serde_json::from_value::<DiscordPollResponse>(fixture).is_err());
        }
        let mut fixture = auth();
        fixture["yggdrasil"] = json!({"access_token":null,"client_token":"fixture-client"});
        assert!(serde_json::from_value::<DiscordPollResponse>(fixture).is_err());
    }
}
