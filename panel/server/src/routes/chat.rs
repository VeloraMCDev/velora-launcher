//! In-game chat layout, edited in the admin panel and handed to every game server with its sync.
//!
//! A layout is a main template plus small templates for each optional part. A part whose value is empty (no guild, no
//! LuckPerms suffix, …) disappears completely, so there are never stray brackets or double spaces.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::store;

use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ChatSettings {
    /// Turn the layout off to leave chat to the server (or another chat plugin).
    pub enabled: bool,
    /// Placeholders: {guild} {title} {rank_title} {group} {prefix} {name} {suffix} {level} {message}.
    pub format: String,
    /// Used for {guild}; placeholders {tag} and {guild_name}.
    pub guild_format: String,
    /// Used for {title}; placeholder {title}.
    pub title_format: String,
    /// Used for {group}; placeholder {group} (the LuckPerms group's display name).
    pub group_format: String,
    /// Used for {level}; placeholder {level}.
    pub level_format: String,
    /// `**bold**`, `*italic*`, `__underline__`, `~~strike~~` and `` `code` `` in messages.
    pub markdown: bool,
    /// Players with `velora.chat.color` may use & colour codes and &#RRGGBB in their messages.
    pub allow_colors: bool,
}

impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "{guild}{title}{group}{prefix}{name}{suffix}&7: &f{message}".into(),
            guild_format: "&3[{tag}] ".into(),
            title_format: "&6[{title}] ".into(),
            group_format: String::new(),
            level_format: String::new(),
            markdown: true,
            allow_colors: true,
        }
    }
}

const PARTS: &[&str] = &["guild", "title", "rank_title", "group", "prefix", "name", "suffix", "level", "message"];

fn check(s: &ChatSettings) -> AppResult<()> {
    for (label, v) in [("Layout", &s.format), ("Faction", &s.guild_format), ("Title", &s.title_format), ("Group", &s.group_format), ("Level", &s.level_format)] {
        if v.chars().count() > 300 {
            return Err(AppError::bad_request(format!("{label} template is too long (300 characters at most)")));
        }
        if v.contains('\n') || v.contains('\r') {
            return Err(AppError::bad_request(format!("{label} template must be a single line")));
        }
        if v.contains('%') {
            return Err(AppError::bad_request(format!("{label} template can't contain '%'")));
        }
    }
    if !s.format.contains("{message}") {
        return Err(AppError::bad_request("The layout must contain {message}, otherwise nobody could read what players say"));
    }
    if !s.format.contains("{name}") {
        return Err(AppError::bad_request("The layout must contain {name}"));
    }
    // Unknown placeholders are almost always typos; refuse them instead of printing them in chat.
    for t in [&s.format, &s.guild_format, &s.title_format, &s.group_format, &s.level_format] {
        let mut rest = t.as_str();
        while let Some(i) = rest.find('{') {
            let Some(j) = rest[i..].find('}') else { return Err(AppError::bad_request("A placeholder is missing its closing }")) };
            let key = &rest[i + 1..i + j];
            if !PARTS.contains(&key) && !["tag", "guild_name"].contains(&key) {
                return Err(AppError::bad_request(format!("Unknown placeholder {{{key}}}")));
            }
            rest = &rest[i + j + 1..];
        }
    }
    Ok(())
}

pub async fn load(state: &AppState) -> AppResult<ChatSettings> {
    store::kv_get(state, "chat_settings").await
}

pub async fn get(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "settings": load(&state).await?, "defaults": ChatSettings::default() })))
}

pub async fn put(_: AdminUser, State(state): State<AppState>, Json(mut s): Json<ChatSettings>) -> AppResult<Json<Value>> {
    // `%rank_title%` is how the placeholder is written elsewhere (PlaceholderAPI); accept it here too.
    s.format = s.format.replace("%rank_title%", "{rank_title}");
    check(&s)?;
    store::kv_set(&state, "chat_settings", &s).await?;
    Ok(Json(json!({ "ok": true, "settings": s })))
}
