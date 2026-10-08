//! Admin editing of Discord message templates and live embeds, with previews built from real data.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::embeds::{self, EmbedStyle, LiveConfig};
use crate::error::{AppError, AppResult};
use crate::routes::discord;
use crate::state::AppState;
use crate::store;
use axum::extract::{Path};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn check_style(s: &EmbedStyle) -> AppResult<()> {
    let too_long = |t: &str, n: usize| t.chars().count() > n;
    if too_long(&s.title, 600)
        || too_long(&s.description, 4000)
        || too_long(&s.footer, 600)
        || too_long(&s.content, 2000)
        || s.fields.len() > 25
    {
        return Err(AppError::bad_request("a Discord message part is too long (title 256, text 4096, footer 2048, 25 fields)"));
    }
    for url in [&s.url, &s.thumbnail, &s.image, &s.avatar_url, &s.author_icon, &s.footer_icon] {
        let u = url.trim();
        if !u.is_empty() && !u.starts_with('{') && !(u.starts_with("https://") || u.starts_with("http://")) {
            return Err(AppError::bad_request("image and link addresses must start with https:// (or be a placeholder like {avatar})"));
        }
    }
    Ok(())
}

pub async fn get_all(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let templates = embeds::templates(&state).await?;
    let live = embeds::live_configs(&state).await?;
    let mut live_out = serde_json::Map::new();
    for (k, c) in &live {
        let mut v = serde_json::to_value(c)?;
        v["posted"] = json!(!c.message_id.is_empty());
        v["webhook_set"] = json!(!c.webhook_url.is_empty());
        v["webhook_url"] = json!("");
        v["channel_id"] = json!(c.channel_id);
        live_out.insert(k.clone(), v);
    }
    let main = discord::load(&state).await?;
    let destination_set = !main.webhook_url.is_empty() || !main.channel_id.is_empty();
    let placeholders: BTreeMap<&str, Vec<&str>> = embeds::ANNOUNCEMENT_PLACEHOLDERS.iter().map(|(k, v)| (*k, v.to_vec())).collect();
    Ok(Json(json!({
        "templates": templates,
        "live": live_out,
        "placeholders": placeholders,
        "live_placeholders": {
            "status": ["rows", "servers_online", "servers_total", "players_online", "players_max", "server_name", "updated", "time"],
            "status_row": ["status_icon", "server", "status", "players", "max", "tps", "version", "last_seen"],
            "guilds": ["rows", "guilds_total", "claims_total", "server_name", "updated", "time"],
            "guilds_row": ["rank", "guild", "tag", "level", "members", "claims", "leader", "leader_avatar"],
            "leaderboard": ["rows", "sort_label", "server_name", "updated", "time"],
            "leaderboard_row": ["rank", "player", "avatar", "value", "playtime", "kills", "blocks", "level", "deaths", "mob_kills"],
            "baltop": ["rows", "total", "server_name", "updated", "time"],
            "baltop_row": ["rank", "player", "avatar", "balance"],
        },
        "webhook_set": destination_set,
        "channel_id": main.channel_id,
    })))
}

#[derive(Deserialize)]
pub struct TemplatesBody {
    templates: BTreeMap<String, EmbedStyle>,
}

pub async fn put_templates(_: AdminUser, State(state): State<AppState>, Json(b): Json<TemplatesBody>) -> AppResult<Json<Value>> {
    let known = embeds::default_templates();
    let mut next = BTreeMap::new();
    for (k, v) in b.templates {
        if !known.contains_key(&k) {
            return Err(AppError::bad_request("unknown message"));
        }
        check_style(&v)?;
        next.insert(k, v);
    }
    store::kv_set(&state, "discord_templates", &next).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct LiveBody {
    live: BTreeMap<String, LiveConfig>,
}

pub async fn put_live(_: AdminUser, State(state): State<AppState>, Json(b): Json<LiveBody>) -> AppResult<Json<Value>> {
    let mut all = embeds::live_configs(&state).await?;
    for (k, mut v) in b.live {
        let Some(old) = all.get(&k) else { return Err(AppError::bad_request("unknown embed")) };
        check_style(&v.style)?;
        v.channel_id = v.channel_id.trim().to_string();
        if !discord::valid_channel_id(&v.channel_id) {
            return Err(AppError::bad_request(
                "a channel ID is a long number: right click the channel in Discord (Developer Mode on) and choose Copy Channel ID",
            ));
        }
        let hook = v.webhook_url.trim().to_string();
        v.webhook_url = match hook.as_str() {
            "" => old.webhook_url.clone(), // blank keeps the saved one; "-" clears it
            "-" => String::new(),
            h if discord::valid_webhook(h) => h.to_string(),
            _ => return Err(AppError::bad_request("that is not a Discord webhook address")),
        };
        v.interval_secs = v.interval_secs.clamp(60, 86_400);
        v.limit = v.limit.clamp(1, 25);
        if !["playtime", "level", "kills", "blocks"].contains(&v.sort.as_str()) {
            v.sort = "playtime".into();
        }
        v.message_id = old.message_id.clone();
        all.insert(k, v);
    }
    embeds::save_live(&state, &all).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct PreviewBody {
    /// `achievement`, `member`, `guild` or a live kind
    kind: String,
    style: Option<EmbedStyle>,
    live: Option<LiveConfig>,
}

/// What the message would look like right now, as the exact JSON Discord receives.
pub async fn preview(_: AdminUser, State(state): State<AppState>, Json(b): Json<PreviewBody>) -> AppResult<Json<Value>> {
    if embeds::LIVE_KINDS.contains(&b.kind.as_str()) {
        let cfg = b.live.ok_or_else(|| AppError::bad_request("missing embed settings"))?;
        check_style(&cfg.style)?;
        return Ok(Json(embeds::build_live(&state, &b.kind, &cfg).await?));
    }
    let style = b.style.ok_or_else(|| AppError::bad_request("missing message settings"))?;
    check_style(&style)?;
    if !embeds::default_templates().contains_key(&b.kind) {
        return Err(AppError::bad_request("unknown message"));
    }
    let vars = embeds::sample_vars(&state, &b.kind).await;
    Ok(Json(embeds::render(&style, &vars, None)))
}

/// Send the saved message to Discord now (a sample announcement, or a live embed refresh).
pub async fn send_now(_: AdminUser, State(state): State<AppState>, Path(kind): Path<String>) -> AppResult<Json<Value>> {
    if embeds::LIVE_KINDS.contains(&kind.as_str()) {
        embeds::refresh_live(&state, &kind, false).await?;
        return Ok(Json(json!({ "ok": true })));
    }
    let templates = embeds::templates(&state).await?;
    let style = templates.get(&kind).ok_or_else(|| AppError::bad_request("unknown message"))?;
    let Some(dest) = discord::Dest::resolve(&state, "", "").await? else {
        return Err(AppError::bad_request("set the announcement channel under Settings → Discord first"));
    };
    let vars = embeds::sample_vars(&state, &kind).await;
    embeds::post(&state, &dest, &embeds::render(style, &vars, None)).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Forget the posted message so the next update creates a new one (use after moving the embed to another channel).
pub async fn repost(_: AdminUser, State(state): State<AppState>, Path(kind): Path<String>) -> AppResult<Json<Value>> {
    if !embeds::LIVE_KINDS.contains(&kind.as_str()) {
        return Err(AppError::bad_request("unknown embed"));
    }
    embeds::refresh_live(&state, &kind, true).await?;
    Ok(Json(json!({ "ok": true })))
}
