use crate::Error;
use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// No Debug: serialization is for protected operator storage, never a public response.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionsSettings {
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_bot_token: String,
    pub discord_guild_id: String,
    pub resend_api_key: String,
    pub sender_email: String,
    pub sender_name: String,
}
#[derive(Deserialize)]
pub struct SettingsInput {
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_bot_token: String,
    pub discord_guild_id: String,
    pub resend_api_key: String,
    pub sender_email: String,
    pub sender_name: String,
}
fn secret(input: &str, previous: &str) -> String {
    match input.trim() {
        "" => previous.to_string(),
        "-" => String::new(),
        value => value.to_string(),
    }
}
pub fn updated(old: &ConnectionsSettings, input: SettingsInput) -> Result<ConnectionsSettings, Error> {
    let next = ConnectionsSettings {
        discord_client_id: input.discord_client_id.trim().into(),
        discord_client_secret: secret(&input.discord_client_secret, &old.discord_client_secret),
        discord_bot_token: secret(&input.discord_bot_token, &old.discord_bot_token),
        discord_guild_id: input.discord_guild_id.trim().into(),
        resend_api_key: secret(&input.resend_api_key, &old.resend_api_key),
        sender_email: input.sender_email.trim().into(),
        sender_name: input.sender_name.trim().into(),
    };
    if !next.sender_email.is_empty() && next.sender_email.parse::<Mailbox>().is_err() {
        return Err(Error::request("enter a valid sender email address"));
    }
    if !next.discord_client_id.is_empty() && !next.discord_client_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::request("Discord client ID must contain digits only"));
    }
    if !next.discord_guild_id.is_empty() && !next.discord_guild_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::request("Discord server ID must contain digits only"));
    }
    Ok(next)
}
pub fn public_config(settings: &ConnectionsSettings) -> Value {
    json!({"discord_enabled":!settings.discord_client_id.is_empty() && !settings.discord_client_secret.is_empty(),
        "email_enabled":!settings.resend_api_key.is_empty() && !settings.sender_email.is_empty()})
}
pub fn masked(settings: &ConnectionsSettings) -> Value {
    let enabled = public_config(settings);
    json!({"discord_client_id":settings.discord_client_id,
        "discord_client_secret_set":!settings.discord_client_secret.is_empty(),
        "discord_bot_token_set":!settings.discord_bot_token.is_empty(),
        "discord_guild_id":settings.discord_guild_id,
        "resend_api_key_set":!settings.resend_api_key.is_empty(),
        "sender_email":settings.sender_email,"sender_name":settings.sender_name,
        "discord_enabled":enabled["discord_enabled"],"email_enabled":enabled["email_enabled"]})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_storage_defaults_and_masked_wire_shape_preserve_secrets_and_flags() {
        let settings: ConnectionsSettings =
            serde_json::from_value(json!({"discord_client_id":"123", "discord_client_secret":"synthetic-client-key",
            "discord_bot_token":"synthetic-bot-key", "resend_api_key":"synthetic-mail-key", "sender_email":"sender@example.invalid"}))
            .unwrap();
        let storage = serde_json::to_value(&settings).unwrap();
        assert_eq!(storage["discord_client_secret"], "synthetic-client-key");
        assert_eq!(storage["sender_name"], "");
        let response = masked(&settings);
        assert_eq!(
            response,
            json!({"discord_client_id":"123","discord_client_secret_set":true,"discord_bot_token_set":true,
            "discord_guild_id":"","resend_api_key_set":true,"sender_email":"sender@example.invalid","sender_name":"","discord_enabled":true,"email_enabled":true})
        );
        for key in ["synthetic-client-key", "synthetic-bot-key", "synthetic-mail-key"] {
            assert!(!response.to_string().contains(key));
        }
        assert_eq!(public_config(&ConnectionsSettings::default()), json!({"discord_enabled":false,"email_enabled":false}));
    }
    fn input(client: &str, guild: &str, email: &str) -> SettingsInput {
        SettingsInput {
            discord_client_id: client.into(),
            discord_client_secret: " ".into(),
            discord_bot_token: " - ".into(),
            discord_guild_id: guild.into(),
            resend_api_key: " replacement-key ".into(),
            sender_email: email.into(),
            sender_name: " Velora ".into(),
        }
    }
    #[test]
    fn secret_update_policy_and_validation_precedence_keep_original_behavior() {
        let old = ConnectionsSettings {
            discord_client_secret: "original-key".into(),
            discord_bot_token: "old-bot-key".into(),
            ..Default::default()
        };
        let next = updated(&old, input(" 123 ", " 456 ", " sender@example.invalid ")).unwrap();
        assert_eq!((&next.discord_client_id, &next.discord_guild_id, &next.sender_name), (&"123".into(), &"456".into(), &"Velora".into()));
        assert_eq!(next.discord_client_secret, "original-key");
        assert!(next.discord_bot_token.is_empty());
        assert_eq!(next.resend_api_key, "replacement-key");
        assert_eq!(updated(&old, input("bad", "bad", "invalid")).err().unwrap().message, "enter a valid sender email address");
        assert_eq!(updated(&old, input("１２３", "bad", "")).err().unwrap().message, "Discord client ID must contain digits only");
        assert_eq!(updated(&old, input("123", "bad", "")).err().unwrap().message, "Discord server ID must contain digits only");
        assert!(updated(&old, input("", "", "")).is_ok());
        assert_eq!(old.discord_client_secret, "original-key");
    }
}
