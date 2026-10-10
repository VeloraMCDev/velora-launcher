//! Discord messages the panel posts: announcements (achievements, new players, new guilds) built from admin-edited templates,
//! and live embeds (server status, active guilds, leaderboards, richest players) that are edited in place on a schedule.
//!
//! Everything is posted by the bot into channels (or, for older setups, through a webhook). Templates use `{placeholders}`; unknown ones are left as typed.

use crate::error::{AppError, AppResult};
use crate::routes::discord;
use crate::state::AppState;
use crate::store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

/// Everything about how one Discord message looks. All text may contain placeholders.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct EmbedStyle {
    pub enabled: bool,
    /// Webhook display name and picture (empty = Velora's).
    pub username: String,
    pub avatar_url: String,
    /// Plain text above the embed. Mentions work here, e.g. `<@&roleid>`.
    pub content: String,
    pub title: String,
    pub url: String,
    pub description: String,
    /// `#rrggbb`
    pub color: String,
    pub thumbnail: String,
    pub image: String,
    pub author_name: String,
    pub author_icon: String,
    pub footer: String,
    pub footer_icon: String,
    /// Show the time the message was made.
    pub timestamp: bool,
    pub fields: Vec<Field>,
}

impl Default for EmbedStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            username: String::new(),
            avatar_url: String::new(),
            content: String::new(),
            title: String::new(),
            url: String::new(),
            description: String::new(),
            color: "#8b6cff".into(),
            thumbnail: String::new(),
            image: String::new(),
            author_name: String::new(),
            author_icon: String::new(),
            footer: String::new(),
            footer_icon: String::new(),
            timestamp: true,
            fields: vec![],
        }
    }
}

fn field(name: &str, value: &str, inline: bool) -> Field {
    Field { name: name.into(), value: value.into(), inline }
}

/// What the three announcements say until an admin changes them.
pub fn default_templates() -> BTreeMap<String, EmbedStyle> {
    let mut m = BTreeMap::new();
    m.insert(
        "achievement".to_string(),
        EmbedStyle {
            title: "🏆 Achievement unlocked".into(),
            description: "**{player}** earned **{achievement}**\n*{achievement_description}*".into(),
            color: "#fcd34d".into(),
            thumbnail: "{avatar}".into(),
            footer: "{category} · {server_name}".into(),
            fields: vec![field("Reward", "+{xp} XP", true), field("Level", "{level}", true), field("Faction", "{guild_line}", true)],
            ..Default::default()
        },
    );
    m.insert(
        "member".to_string(),
        EmbedStyle {
            enabled: false,
            title: "👋 New player".into(),
            description: "Welcome **{player}**! You are player number **{members_total}**.".into(),
            color: "#6ee7b7".into(),
            thumbnail: "{avatar}".into(),
            footer: "{server_name}".into(),
            ..Default::default()
        },
    );
    m.insert(
        "guild".to_string(),
        EmbedStyle {
            title: "🛡️ New faction".into(),
            description: "**{guild}** [{tag}] was founded by **{leader}**.\n{guild_description}".into(),
            color: "#22d3ee".into(),
            thumbnail: "{leader_avatar}".into(),
            footer: "{guilds_total} factions in total · {server_name}".into(),
            fields: vec![field("Leader", "{leader}", true), field("Members", "{members}", true), field("Land", "{claims} chunks", true)],
            ..Default::default()
        },
    );
    m
}

pub async fn templates(state: &AppState) -> AppResult<BTreeMap<String, EmbedStyle>> {
    let mut stored: BTreeMap<String, EmbedStyle> = store::kv_get(state, "discord_templates").await?;
    for (k, v) in default_templates() {
        stored.entry(k).or_insert(v);
    }
    Ok(stored)
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

pub type Vars = BTreeMap<String, String>;

/// Replace `{name}` with its value. Unknown placeholders stay as typed so mistakes are visible.
pub fn fill(text: &str, vars: &Vars) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) if !after[..end].contains(char::is_whitespace) && !after[..end].contains('{') => {
                match vars.get(&after[..end]) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push('{');
                        out.push_str(&after[..end]);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max.saturating_sub(1)).collect::<String>())
    }
}

fn color(s: &str) -> u32 {
    u32::from_str_radix(s.trim().trim_start_matches('#'), 16).ok().filter(|c| *c <= 0xFFFFFF).unwrap_or(0x8b6cff)
}

fn https(s: String) -> Option<String> {
    let s = s.trim().to_string();
    (s.starts_with("https://") || s.starts_with("http://")).then_some(s)
}

/// The webhook body for a style and its values, kept within Discord's limits.
pub fn render(style: &EmbedStyle, vars: &Vars, extra_description: Option<&str>) -> Value {
    let mut e = Map::new();
    let title = fill(&style.title, vars);
    if !title.trim().is_empty() {
        e.insert("title".into(), json!(cut(&title, 256)));
    }
    if let Some(u) = https(fill(&style.url, vars)) {
        e.insert("url".into(), json!(u));
    }
    let mut desc = fill(&style.description, vars);
    if let Some(extra) = extra_description {
        desc = desc.replace("{rows}", extra);
    }
    if !desc.trim().is_empty() {
        e.insert("description".into(), json!(cut(&desc, 4096)));
    }
    e.insert("color".into(), json!(color(&style.color)));
    if let Some(u) = https(fill(&style.thumbnail, vars)) {
        e.insert("thumbnail".into(), json!({ "url": u }));
    }
    if let Some(u) = https(fill(&style.image, vars)) {
        e.insert("image".into(), json!({ "url": u }));
    }
    let author = fill(&style.author_name, vars);
    if !author.trim().is_empty() {
        let mut a = Map::new();
        a.insert("name".into(), json!(cut(&author, 256)));
        if let Some(u) = https(fill(&style.author_icon, vars)) {
            a.insert("icon_url".into(), json!(u));
        }
        e.insert("author".into(), Value::Object(a));
    }
    let footer = fill(&style.footer, vars);
    if !footer.trim().is_empty() {
        let mut f = Map::new();
        f.insert("text".into(), json!(cut(&footer, 2048)));
        if let Some(u) = https(fill(&style.footer_icon, vars)) {
            f.insert("icon_url".into(), json!(u));
        }
        e.insert("footer".into(), Value::Object(f));
    }
    if style.timestamp {
        e.insert("timestamp".into(), json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)));
    }
    let fields: Vec<Value> = style
        .fields
        .iter()
        .map(|f| (fill(&f.name, vars), fill(&f.value, vars), f.inline))
        .filter(|(n, v, _)| !n.trim().is_empty() && !v.trim().is_empty())
        .take(25)
        .map(|(n, v, i)| json!({ "name": cut(&n, 256), "value": cut(&v, 1024), "inline": i }))
        .collect();
    if !fields.is_empty() {
        e.insert("fields".into(), json!(fields));
    }
    let mut body = Map::new();
    body.insert(
        "username".into(),
        json!(cut(&if style.username.trim().is_empty() { "Velora".to_string() } else { fill(&style.username, vars) }, 80)),
    );
    if let Some(u) = https(fill(&style.avatar_url, vars)) {
        body.insert("avatar_url".into(), json!(u));
    }
    let content = fill(&style.content, vars);
    if !content.trim().is_empty() {
        body.insert("content".into(), json!(cut(&content, 2000)));
    }
    // Mentions only where the admin wrote them in the message text; never from player-supplied names.
    body.insert("allowed_mentions".into(), json!({ "parse": ["roles", "users"] }));
    body.insert("embeds".into(), json!([Value::Object(e)]));
    Value::Object(body)
}

/// Player-supplied text can't ping anyone or break formatting out of its place.
pub fn plain(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect::<String>().replace('@', "@\u{200b}").replace('`', "'").chars().take(120).collect()
}

fn base_url(state: &AppState) -> String {
    state.cfg.public_url.clone().unwrap_or_default().trim_end_matches('/').to_string()
}

pub fn avatar(state: &AppState, uuid: &str) -> String {
    let base = base_url(state);
    if base.is_empty() || uuid.is_empty() {
        String::new()
    } else {
        format!("{base}/api/v1/avatar/{uuid}?size=128")
    }
}

async fn server_name(state: &AppState) -> String {
    store::branding(state).await.map(|b| b.name).unwrap_or_default()
}

/// Values common to every message.
async fn common(state: &AppState) -> Vars {
    let mut v = Vars::new();
    v.insert("server_name".into(), server_name(state).await);
    v.insert("panel".into(), base_url(state));
    v.insert("time".into(), format!("<t:{}:f>", chrono::Utc::now().timestamp()));
    v.insert("updated".into(), format!("<t:{}:R>", chrono::Utc::now().timestamp()));
    v
}

pub const ANNOUNCEMENT_PLACEHOLDERS: [(&str, &[&str]); 3] = [
    (
        "achievement",
        &[
            "player",
            "uuid",
            "avatar",
            "achievement",
            "achievement_description",
            "xp",
            "category",
            "level",
            "rank_title",
            "guild",
            "guild_tag",
            "guild_line",
            "server_name",
            "panel",
            "time",
        ],
    ),
    ("member", &["player", "uuid", "avatar", "members_total", "server_name", "panel", "time"]),
    (
        "guild",
        &[
            "guild",
            "tag",
            "leader",
            "leader_avatar",
            "guild_description",
            "members",
            "claims",
            "guilds_total",
            "server_name",
            "panel",
            "time",
        ],
    ),
];

pub async fn player_vars(state: &AppState, uuid: &str, name: &str) -> Vars {
    let mut v = common(state).await;
    v.insert("player".into(), plain(name));
    v.insert("uuid".into(), uuid.to_string());
    v.insert("avatar".into(), avatar(state, uuid));
    let level: Option<(i64, Option<String>)> = sqlx::query_as("SELECT global_level, title FROM user_levels WHERE uuid = ?")
        .bind(uuid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();
    let (l, t) = level.unwrap_or((1, None));
    v.insert("level".into(), l.to_string());
    v.insert("rank_title".into(), plain(&t.unwrap_or_default()));
    let guild: Option<(String, String)> =
        sqlx::query_as("SELECT g.name, g.tag FROM guild_members m JOIN guilds g ON g.id = m.guild_id WHERE m.uuid = ?")
            .bind(uuid)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();
    let (gn, gt) = guild.unwrap_or_default();
    v.insert("guild".into(), plain(&gn));
    v.insert("guild_tag".into(), plain(&gt));
    v.insert("guild_line".into(), if gn.is_empty() { "No faction".into() } else { format!("[{}] {}", plain(&gt), plain(&gn)) });
    v
}

pub async fn guild_vars(state: &AppState, guild_id: &str) -> Vars {
    let mut v = common(state).await;
    let row: Option<(String, String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT g.name, g.tag, g.description, g.leader_uuid, (SELECT COUNT(*) FROM guild_members WHERE guild_id = g.id), (SELECT COUNT(*) FROM guild_claims WHERE guild_id = g.id) FROM guilds g WHERE g.id = ?",
    )
    .bind(guild_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();
    let (name, tag, desc, leader_uuid, members, claims) = row.unwrap_or_default();
    let leader: String = sqlx::query_scalar("SELECT username FROM users WHERE uuid = ?")
        .bind(&leader_uuid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM guilds").fetch_one(&state.db).await.unwrap_or(0);
    v.insert("guild".into(), plain(&name));
    v.insert("tag".into(), plain(&tag));
    v.insert("guild_description".into(), plain(&desc));
    v.insert("leader".into(), plain(&leader));
    v.insert("leader_avatar".into(), avatar(state, &leader_uuid));
    v.insert("members".into(), members.to_string());
    v.insert("claims".into(), claims.to_string());
    v.insert("guilds_total".into(), total.to_string());
    v
}

/// Example values, for previews and test messages.
pub async fn sample_vars(state: &AppState, kind: &str) -> Vars {
    let mut v = common(state).await;
    let me = state.cfg.admin_username.clone();
    let pairs: Vec<(&str, String)> = match kind {
        "achievement" => vec![
            ("player", me),
            ("uuid", "00000000-0000-0000-0000-000000000000".into()),
            ("avatar", String::new()),
            ("achievement", "Deep Diver".into()),
            ("achievement_description", "Mine 5,000 blocks below Y 0".into()),
            ("xp", "250".into()),
            ("category", "Mining".into()),
            ("level", "14".into()),
            ("rank_title", "Veteran".into()),
            ("guild", "Iron Wolves".into()),
            ("guild_tag", "IRON".into()),
            ("guild_line", "[IRON] Iron Wolves".into()),
        ],
        "member" => vec![("player", me), ("uuid", String::new()), ("avatar", String::new()), ("members_total", "128".into())],
        _ => vec![
            ("guild", "Iron Wolves".into()),
            ("tag", "IRON".into()),
            ("leader", me),
            ("leader_avatar", String::new()),
            ("guild_description", "Defenders of the realm.".into()),
            ("members", "1".into()),
            ("claims", "0".into()),
            ("guilds_total", "12".into()),
        ],
    };
    for (k, val) in pairs {
        v.insert(k.into(), val);
    }
    v
}

// ---------------------------------------------------------------------------
// Sending
// ---------------------------------------------------------------------------

/// Deliver one message to the destination (the bot into a channel, or a webhook for older setups).
pub async fn post(state: &AppState, dest: &discord::Dest, body: &Value) -> AppResult<()> {
    dest.send(state, body).await.map(|_| ())
}

pub async fn announce_event(state: &AppState, kind: &str, vars: &Vars) -> AppResult<bool> {
    let Some(dest) = discord::Dest::resolve(state, "", "").await? else {
        return Ok(false);
    };
    let style = templates(state).await?.remove(kind).unwrap_or_default();
    if !style.enabled {
        return Ok(false);
    }
    post(state, &dest, &render(&style, vars, None)).await?;
    Ok(true)
}

// ---------------------------------------------------------------------------
// Live embeds
// ---------------------------------------------------------------------------

pub const LIVE_KINDS: [&str; 4] = ["status", "guilds", "leaderboard", "baltop"];

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct LiveConfig {
    pub style: EmbedStyle,
    /// The channel the bot posts this board in. Empty = the announcement channel.
    pub channel_id: String,
    /// Older setups: post through this webhook instead.
    pub webhook_url: String,
    /// Seconds between updates (at least 60).
    pub interval_secs: u64,
    /// Rows shown (leaderboards, guilds).
    pub limit: u32,
    /// leaderboard: `playtime`, `level`, `kills`, `blocks`.
    pub sort: String,
    /// baltop: the economy of this server (0 = the first one).
    pub server_id: i64,
    /// One line per row; `{rows}` in the description is replaced by them.
    pub row: String,
    #[serde(skip_serializing)]
    pub message_id: String,
}

impl Default for LiveConfig {
    fn default() -> Self {
        Self {
            style: EmbedStyle::default(),
            channel_id: String::new(),
            webhook_url: String::new(),
            interval_secs: 120,
            limit: 10,
            sort: "playtime".into(),
            server_id: 0,
            row: String::new(),
            message_id: String::new(),
        }
    }
}

pub fn default_live() -> BTreeMap<String, LiveConfig> {
    let mut m = BTreeMap::new();
    m.insert(
        "status".to_string(),
        LiveConfig {
            style: EmbedStyle {
                enabled: false,
                title: "🟢 Server status".into(),
                description: "{rows}\n\n**{players_online}** players online across **{servers_online}/{servers_total}** servers".into(),
                color: "#34d399".into(),
                footer: "{server_name} · updated".into(),
                ..Default::default()
            },
            row: "{status_icon} **{server}** — {players}/{max} players · TPS {tps} · {version}".into(),
            ..Default::default()
        },
    );
    m.insert(
        "guilds".to_string(),
        LiveConfig {
            style: EmbedStyle {
                enabled: false,
                title: "🛡️ Active factions".into(),
                description: "{rows}\n\n{guilds_total} factions · {claims_total} claimed chunks".into(),
                color: "#22d3ee".into(),
                footer: "{server_name}".into(),
                ..Default::default()
            },
            row: "**{rank}. [{tag}] {guild}** — level {level} · {members} members · {claims} chunks · led by {leader}".into(),
            ..Default::default()
        },
    );
    m.insert(
        "leaderboard".to_string(),
        LiveConfig {
            style: EmbedStyle {
                enabled: false,
                title: "🏆 Leaderboard — {sort_label}".into(),
                description: "{rows}".into(),
                color: "#fcd34d".into(),
                footer: "{server_name}".into(),
                ..Default::default()
            },
            row: "`{rank}.` **{player}** — {value}".into(),
            ..Default::default()
        },
    );
    m.insert(
        "baltop".to_string(),
        LiveConfig {
            style: EmbedStyle {
                enabled: false,
                title: "💰 Richest players".into(),
                description: "{rows}\n\nTotal in circulation: **{total}**".into(),
                color: "#f59e0b".into(),
                footer: "{server_name}".into(),
                ..Default::default()
            },
            row: "`{rank}.` **{player}** — {balance}".into(),
            ..Default::default()
        },
    );
    m
}

pub async fn live_configs(state: &AppState) -> AppResult<BTreeMap<String, LiveConfig>> {
    let mut stored: BTreeMap<String, LiveConfig> = store::kv_get(state, "discord_live").await?;
    for (k, v) in default_live() {
        stored.entry(k).or_insert(v);
    }
    Ok(stored)
}

pub async fn save_live(state: &AppState, configs: &BTreeMap<String, LiveConfig>) -> AppResult<()> {
    // message_id is runtime state; keep it out of the JSON the admin edits but in storage.
    let mut stored = serde_json::Map::new();
    for (k, c) in configs {
        let mut v = serde_json::to_value(c)?;
        v["message_id"] = json!(c.message_id);
        stored.insert(k.clone(), v);
    }
    store::kv_set(state, "discord_live", &Value::Object(stored)).await
}

fn money(v: f64) -> String {
    let whole = v.round() as i64;
    let s = whole.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("${}{}", if whole < 0 { "-" } else { "" }, out.chars().rev().collect::<String>())
}

fn hours(secs: i64) -> String {
    if secs >= 3600 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}m", secs / 60)
    }
}

/// The embed body for one live kind, built from current data.
pub async fn build_live(state: &AppState, kind: &str, cfg: &LiveConfig) -> AppResult<Value> {
    let mut vars = common(state).await;
    let mut rows: Vec<String> = Vec::new();
    let default_row = default_live().remove(kind).map(|c| c.row).unwrap_or_default();
    let row_tpl = if cfg.row.trim().is_empty() { default_row } else { cfg.row.clone() };
    let limit = cfg.limit.clamp(1, 25) as i64;
    match kind {
        "status" => {
            let list = crate::routes::servers::status_rows(state).await?;
            vars.insert("servers_total".into(), list.len().to_string());
            vars.insert("servers_online".into(), list.iter().filter(|s| s.online).count().to_string());
            vars.insert("players_online".into(), list.iter().map(|s| s.players).sum::<i64>().to_string());
            vars.insert("players_max".into(), list.iter().map(|s| s.max_players).sum::<i64>().to_string());
            for s in list {
                let mut r = vars.clone();
                r.insert("status_icon".into(), if s.online { "🟢" } else { "🔴" }.into());
                r.insert("server".into(), plain(&s.name));
                r.insert("status".into(), if s.online { "Online" } else { "Offline" }.into());
                r.insert("players".into(), s.players.to_string());
                r.insert("max".into(), if s.max_players > 0 { s.max_players.to_string() } else { "–".into() });
                r.insert("tps".into(), s.tps.map(|t| format!("{t:.1}")).unwrap_or_else(|| "–".into()));
                r.insert(
                    "version".into(),
                    plain(&if s.version.is_empty() { s.software.clone() } else { format!("{} {}", s.software, s.version) }),
                );
                r.insert(
                    "last_seen".into(),
                    s.last_seen
                        .as_deref()
                        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                        .map(|t| format!("<t:{}:R>", t.timestamp()))
                        .unwrap_or_else(|| "never".into()),
                );
                rows.push(fill(&row_tpl, &r));
            }
        }
        "guilds" => {
            let list: Vec<(String, String, i64, i64, i64, String, String)> = sqlx::query_as(
                "SELECT g.name, g.tag, g.level, (SELECT COUNT(*) FROM guild_members WHERE guild_id = g.id), (SELECT COUNT(*) FROM guild_claims WHERE guild_id = g.id), COALESCE(u.username, ''), g.leader_uuid
                 FROM guilds g LEFT JOIN users u ON u.uuid = g.leader_uuid ORDER BY g.level DESC, g.xp DESC, 4 DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&state.db)
            .await?;
            let totals: (i64, i64) =
                sqlx::query_as("SELECT (SELECT COUNT(*) FROM guilds), (SELECT COUNT(*) FROM guild_claims)").fetch_one(&state.db).await?;
            vars.insert("guilds_total".into(), totals.0.to_string());
            vars.insert("claims_total".into(), totals.1.to_string());
            for (i, (name, tag, level, members, claims, leader, leader_uuid)) in list.into_iter().enumerate() {
                let mut r = vars.clone();
                for (k, v) in [
                    ("rank", (i + 1).to_string()),
                    ("guild", plain(&name)),
                    ("tag", plain(&tag)),
                    ("level", level.to_string()),
                    ("members", members.to_string()),
                    ("claims", claims.to_string()),
                    ("leader", plain(&leader)),
                    ("leader_avatar", avatar(state, &leader_uuid)),
                ] {
                    r.insert(k.into(), v);
                }
                rows.push(fill(&row_tpl, &r));
            }
        }
        "leaderboard" => {
            let (sort_label, order, unit) = match cfg.sort.as_str() {
                "level" => ("Level", "COALESCE(l.global_xp, 0)", "level"),
                "kills" => ("Player kills", "kills", "kills"),
                "blocks" => ("Blocks broken", "blocks", "blocks"),
                _ => ("Playtime", "playtime", "playtime"),
            };
            vars.insert("sort_label".into(), sort_label.into());
            let sql = format!(
                "SELECT s.uuid, COALESCE(u.username, MAX(s.name)), SUM(s.playtime_secs) AS playtime, SUM(s.player_kills) AS kills, SUM(s.blocks_broken) AS blocks, COALESCE(l.global_level, 1), SUM(s.deaths), SUM(s.mob_kills)
                 FROM player_stats s LEFT JOIN users u ON u.uuid = s.uuid LEFT JOIN user_levels l ON l.uuid = s.uuid GROUP BY s.uuid ORDER BY {order} DESC LIMIT ?"
            );
            let list: Vec<(String, String, i64, i64, i64, i64, i64, i64)> = sqlx::query_as(&sql).bind(limit).fetch_all(&state.db).await?;
            for (i, (uuid, name, playtime, kills, blocks, level, deaths, mobs)) in list.into_iter().enumerate() {
                let value = match unit {
                    "level" => format!("level {level}"),
                    "kills" => format!("{kills} kills"),
                    "blocks" => format!("{blocks} blocks"),
                    _ => hours(playtime),
                };
                let mut r = vars.clone();
                for (k, v) in [
                    ("rank", (i + 1).to_string()),
                    ("player", plain(&name)),
                    ("avatar", avatar(state, &uuid)),
                    ("value", value),
                    ("playtime", hours(playtime)),
                    ("kills", kills.to_string()),
                    ("blocks", blocks.to_string()),
                    ("level", level.to_string()),
                    ("deaths", deaths.to_string()),
                    ("mob_kills", mobs.to_string()),
                ] {
                    r.insert(k.into(), v);
                }
                rows.push(fill(&row_tpl, &r));
            }
        }
        _ => {
            let sid: i64 = if cfg.server_id > 0 {
                cfg.server_id
            } else {
                sqlx::query_scalar("SELECT COALESCE(MIN(id), 0) FROM game_servers").fetch_one(&state.db).await?
            };
            let economy = crate::routes::servers::economy_scope(&state.db, sid).await?;
            let list: Vec<(String, String, f64)> =
                sqlx::query_as("SELECT uuid, username, balance FROM server_economy WHERE server_id = ? ORDER BY balance DESC LIMIT ?")
                    .bind(economy)
                    .bind(limit)
                    .fetch_all(&state.db)
                    .await?;
            let total: f64 = sqlx::query_scalar("SELECT COALESCE(SUM(balance), 0) FROM server_economy WHERE server_id = ?")
                .bind(economy)
                .fetch_one(&state.db)
                .await?;
            vars.insert("total".into(), money(total));
            for (i, (uuid, name, balance)) in list.into_iter().enumerate() {
                let mut r = vars.clone();
                for (k, v) in
                    [("rank", (i + 1).to_string()), ("player", plain(&name)), ("avatar", avatar(state, &uuid)), ("balance", money(balance))]
                {
                    r.insert(k.into(), v);
                }
                rows.push(fill(&row_tpl, &r));
            }
        }
    }
    let rows_text = if rows.is_empty() { "*Nothing to show yet.*".to_string() } else { rows.join("\n") };
    Ok(render(&cfg.style, &vars, Some(&rows_text)))
}

/// Post or update one live embed. Returns the stored message id.
pub async fn refresh_live(state: &AppState, kind: &str, force_new: bool) -> AppResult<()> {
    let mut all = live_configs(state).await?;
    let Some(cfg) = all.get(kind).cloned() else { return Err(AppError::bad_request("unknown embed")) };
    let Some(dest) = discord::Dest::resolve(state, &cfg.channel_id, &cfg.webhook_url).await? else {
        return Err(AppError::bad_request("set the announcement channel first (Settings → Discord, or this board's own channel ID)"));
    };
    let body = build_live(state, kind, &cfg).await?;
    let mut id = if force_new { String::new() } else { cfg.message_id.clone() };
    if !id.is_empty() && !dest.edit(state, &id, &body).await? {
        id.clear(); // the message was deleted in Discord (or was posted somewhere else): post a fresh one
    }
    if id.is_empty() {
        id = dest.send(state, &body).await?;
    }
    if let Some(c) = all.get_mut(kind) {
        c.message_id = id;
    }
    save_live(state, &all).await
}

/// Called every half minute: refresh the embeds that are due.
pub async fn tick_live(state: &AppState, last: &mut BTreeMap<String, std::time::Instant>) {
    let Ok(all) = live_configs(state).await else { return };
    for (kind, cfg) in all {
        if !cfg.style.enabled {
            continue;
        }
        let every = Duration::from_secs(cfg.interval_secs.max(60));
        if last.get(&kind).is_some_and(|t| t.elapsed() < every) {
            continue;
        }
        last.insert(kind.clone(), std::time::Instant::now());
        if let Err(e) = refresh_live(state, &kind, false).await {
            tracing::debug!("live embed {kind}: {}", e.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled_and_unknown_ones_kept() {
        let mut v = Vars::new();
        v.insert("player".into(), "Steve".into());
        assert_eq!(fill("Hi {player}, {nope} and {a b} {", &v), "Hi Steve, {nope} and {a b} {");
        assert_eq!(fill("{player}{player}", &v), "SteveSteve");
    }

    #[test]
    fn embeds_stay_inside_discords_limits_and_drop_empties() {
        let mut v = Vars::new();
        v.insert("long".into(), "x".repeat(5000));
        v.insert("img".into(), "javascript:alert(1)".into());
        let style = EmbedStyle {
            title: "{long}".into(),
            description: "{long}".into(),
            thumbnail: "{img}".into(),
            color: "nonsense".into(),
            fields: vec![field("", "empty name", false), field("ok", "{long}", true)],
            ..Default::default()
        };
        let body = render(&style, &v, None);
        let e = &body["embeds"][0];
        assert!(e["title"].as_str().unwrap().chars().count() <= 256);
        assert!(e["description"].as_str().unwrap().chars().count() <= 4096);
        assert!(e.get("thumbnail").is_none(), "only web addresses become images");
        assert_eq!(e["color"], 0x8b6cff);
        assert_eq!(e["fields"].as_array().unwrap().len(), 1);
        assert!(e["fields"][0]["value"].as_str().unwrap().chars().count() <= 1024);
    }

    #[test]
    fn player_text_cannot_ping() {
        assert_eq!(plain("@everyone `x`"), "@\u{200b}everyone 'x'");
    }

    #[test]
    fn money_and_hours_read_well() {
        assert_eq!(money(1234567.4), "$1,234,567");
        assert_eq!(hours(3700), "1h 1m");
        assert_eq!(hours(300), "5m");
    }
}
