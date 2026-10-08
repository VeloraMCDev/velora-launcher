//! Game server integration: the SCOPENET Paper plugin and Fabric/Forge mods
//! report to these endpoints with a per-server token, and admins manage the
//! servers (and read the stats they collect) from the panel.

use crate::auth::{self, AdminUser, AuthUser, UserRow};
use crate::error::{AppError, AppResult};
use crate::net;
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::store;
use crate::yggdrasil::{dashed, user_by_uuid};
use axum::extract::{FromRequestParts, Path, Query};
use axum::http::request::Parts;
use axum::http::HeaderMap;
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// How often integrations should call `/sync`.
pub const SYNC_INTERVAL_SECS: i64 = 30;
/// A server that hasn't synced for this long is shown as offline.
const STALE_SECS: i64 = 90;
/// A launcher launch from the same IP within this window counts as "joined
/// through the launcher".
const LAUNCHER_WINDOW_HOURS: i64 = 12;
const EVENT_RETENTION_DAYS: i64 = 60;
const MAX_EVENTS_PER_SYNC: usize = 500;
const MAX_ONLINE: usize = 5000;

use crate::db::now;

/// Same format as `db::now()`, so timestamps compare as strings.
fn ts(t: chrono::DateTime<chrono::Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn ago(d: chrono::Duration) -> String {
    ts(chrono::Utc::now() - d)
}

fn stale_cutoff() -> String {
    ago(chrono::Duration::seconds(STALE_SECS))
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn new_token() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    let body: String = (0..40).map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char).collect();
    format!("sn_{body}")
}

fn clip(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect()
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct ServerRow {
    pub instance_id: String,
    /// Whether this server draws the SCOPENET Map (stored in the old `live_map_enabled` column).
    #[sqlx(rename = "live_map_enabled")]
    pub map_enabled: bool,
    /// Servers with the same non-empty name share player balances and guild banks.
    #[sqlx(default)]
    pub economy_group: String,
    /// The server whose balances this one uses: itself, or the oldest in its economy group.
    /// Only filled in for token-authenticated game server requests.
    #[sqlx(default)]
    #[serde(skip)]
    pub economy_id: i64,
    pub id: i64,
    pub name: String,
    #[serde(skip)]
    pub token_hash: String,
    pub token_hint: String,
    /// `all` (anyone who can connect), `members` (panel accounts) or `groups`.
    pub access: String,
    #[serde(skip)]
    pub allowed_groups: String,
    pub require_launcher: bool,
    pub software: Option<String>,
    pub mc_version: Option<String>,
    pub plugin_version: Option<String>,
    pub online_mode: Option<bool>,
    pub max_players: i64,
    pub online_count: i64,
    pub tps: Option<f64>,
    pub last_seen: Option<String>,
    pub created_at: String,
}

impl ServerRow {
    fn groups(&self) -> Vec<String> {
        serde_json::from_str(&self.allowed_groups).unwrap_or_default()
    }
    fn is_online(&self) -> bool {
        self.last_seen.as_deref().is_some_and(|t| t >= stale_cutoff().as_str())
    }
}

#[derive(Serialize)]
pub struct ServerView {
    #[serde(flatten)]
    server: ServerRow,
    allowed_groups: Vec<String>,
    online: bool,
    players: i64,
}

fn view(server: ServerRow) -> ServerView {
    let online = server.is_online();
    ServerView { allowed_groups: server.groups(), players: if online { server.online_count } else { 0 }, online, server }
}

// ---------------------------------------------------------------------------
// Token-authenticated game server API (/api/server/v1)
// ---------------------------------------------------------------------------

/// The game server behind a valid `Authorization: Bearer sn_…` token.
pub struct GameServer(pub ServerRow);

impl FromRequestParts<AppState> for GameServer {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let state = parts.extensions.get::<AppState>().unwrap_or(state);
        let token = auth::bearer(parts).ok_or_else(|| AppError::unauthorized("missing server token"))?;
        let row: Option<ServerRow> = sqlx::query_as("SELECT * FROM game_servers WHERE token_hash = ?")
            .bind(hash_token(token.trim()))
            .fetch_optional(&state.db)
            .await?;
        let mut server = row.ok_or_else(|| AppError::unauthorized("unknown server token — create one under Servers in the panel"))?;
        server.economy_id = economy_scope(&state.db, server.id).await?;
        Ok(GameServer(server))
    }
}

/// Which server's balances `server_id` uses (see [`ServerRow::economy_group`]).
pub async fn economy_scope(db: &sqlx::SqlitePool, server_id: i64) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT MIN(b.id) FROM game_servers b WHERE b.economy_group <> '' AND b.economy_group = a.economy_group COLLATE NOCASE), a.id)
         FROM game_servers a WHERE a.id = ?",
    )
    .bind(server_id)
    .fetch_optional(db)
    .await?
    .unwrap_or(server_id))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Hello {
    software: Option<String>,
    mc_version: Option<String>,
    plugin_version: Option<String>,
    online_mode: Option<bool>,
    max_players: i64,
}

/// Sent once when the plugin/mod starts: records what the server runs and
/// returns the settings the integration needs.
pub async fn hello(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(h): Json<Hello>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        "UPDATE game_servers SET software = ?, mc_version = ?, plugin_version = ?, online_mode = ?, max_players = ?, online_count = 0, last_seen = ? WHERE id = ?",
    )
    .bind(h.software.as_deref().map(|s| clip(s, 64)))
    .bind(h.mc_version.as_deref().map(|s| clip(s, 32)))
    .bind(h.plugin_version.as_deref().map(|s| clip(s, 32)))
    .bind(h.online_mode)
    .bind(h.max_players.clamp(0, 100_000))
    .bind(now())
    .bind(server.id)
    .execute(&state.db)
    .await?;
    if state.instance_id.is_some() {
        sqlx::query("DELETE FROM server_online WHERE server_id=?").bind(server.id).execute(&state.platform_db).await?;
        sqlx::query("UPDATE game_servers SET last_seen=?,online_count=0 WHERE id=?")
            .bind(now())
            .bind(server.id)
            .execute(&state.platform_db)
            .await?;
    }
    // Fresh start: nobody is online yet.
    sqlx::query("DELETE FROM server_online WHERE server_id = ?").bind(server.id).execute(&state.db).await?;

    let base = net::public_base(&state, &headers).await;
    let branding = store::branding(&state).await?;
    let experience: scopenet_shared::Experience = if let Some(id) = &state.instance_id {
        serde_json::from_str(&store::get_instance(&state, id).await?.experience)?
    } else {
        scopenet_shared::Experience::default()
    };
    Ok(Json(json!({
        "server_id": server.id,
        "name": server.name,
        "brand": branding.name,
        "yggdrasil_url": format!("{base}/api/yggdrasil"),
        "sync_interval_secs": SYNC_INTERVAL_SECS,
        "access": server.access,
        "require_launcher": server.require_launcher,
        "map": json!({ "enabled": server.map_enabled && experience.enabled("maps") }),
        "experience_features": experience.features,
    })))
}

#[derive(Deserialize)]
pub struct LoginCheck {
    uuid: String,
    name: String,
    #[serde(default)]
    ip: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct LoginVerdict {
    allowed: bool,
    /// Shown to the player on the disconnect screen when refused.
    message: Option<String>,
    /// The panel account behind the player, if any.
    account: Option<Value>,
}

impl LoginVerdict {
    fn deny(message: impl Into<String>) -> Self {
        Self { allowed: false, message: Some(message.into()), account: None }
    }
}

/// Decides whether a player may join. Called by the integration before the
/// player is let in.
pub async fn login(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(req): Json<LoginCheck>,
) -> AppResult<Json<LoginVerdict>> {
    let brand = store::branding(&state).await?.name;
    // Names are not proof of identity. Only the UUID authenticated by the
    // server's online-mode session check may select a panel account.
    let user = user_by_uuid(&state, &req.uuid).await?;
    if user.is_none() {
        let reserved: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reserved_usernames WHERE name=?)").bind(&req.name).fetch_one(&state.db).await?;
        if reserved {
            return Ok(Json(LoginVerdict::deny("That player name belongs to another account.")));
        }
    }
    if user.as_ref().is_some_and(|u| !u.username.eq_ignore_ascii_case(&req.name)) {
        return Ok(Json(LoginVerdict::deny("Your player name does not match your account.")));
    }
    if server.require_launcher && req.ip.as_deref().is_none_or(|ip| ip.trim().is_empty()) {
        return Ok(Json(LoginVerdict::deny("Your connection address could not be verified. Please reconnect through the launcher.")));
    }
    Ok(Json(check_login(&state, &server, user.as_ref(), req.ip.as_deref(), &brand).await?))
}

fn canonical_ip(ip: std::net::IpAddr) -> std::net::IpAddr {
    match ip {
        std::net::IpAddr::V4(v4) => std::net::IpAddr::V4(v4),
        std::net::IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                std::net::IpAddr::V4(v4)
            } else {
                std::net::IpAddr::V6(v6)
            }
        }
    }
}

pub fn ips_match(sess_str: &str, req_str: &str) -> bool {
    match (sess_str.trim().parse::<std::net::IpAddr>(), req_str.trim().parse::<std::net::IpAddr>()) {
        (Ok(a), Ok(b)) => canonical_ip(a) == canonical_ip(b),
        _ => false,
    }
}

async fn verify_launcher_ip(state: &AppState, user_id: Option<i64>, req_ip: Option<&str>, since: &str) -> AppResult<bool> {
    let req_ip = match req_ip {
        Some(ip) if !ip.trim().is_empty() => ip.trim(),
        _ => return Ok(false),
    };
    let sessions: Vec<String> = match user_id {
        Some(uid) => {
            sqlx::query_scalar("SELECT ip FROM launcher_sessions WHERE user_id = ? AND created_at >= ? ORDER BY id DESC LIMIT 50")
                .bind(uid)
                .bind(since)
                .fetch_all(&state.db)
                .await?
        }
        None => {
            sqlx::query_scalar("SELECT ip FROM launcher_sessions WHERE user_id IS NULL AND created_at >= ? ORDER BY id DESC LIMIT 50")
                .bind(since)
                .fetch_all(&state.db)
                .await?
        }
    };
    for sess_ip in sessions {
        if ips_match(&sess_ip, req_ip) {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn check_login(
    state: &AppState,
    server: &ServerRow,
    user: Option<&UserRow>,
    ip: Option<&str>,
    brand: &str,
) -> AppResult<LoginVerdict> {
    let Some(user) = user else {
        if server.access == "all" {
            if server.require_launcher {
                let since = ago(chrono::Duration::hours(LAUNCHER_WINDOW_HOURS));
                let launched = verify_launcher_ip(state, None, ip, &since).await?;
                if !launched {
                    return Ok(LoginVerdict::deny(format!("Please join through the {brand} launcher.")));
                }
            }
            return Ok(LoginVerdict { allowed: true, message: None, account: None });
        } else {
            return Ok(LoginVerdict::deny(format!("You need a {brand} account to join this server.\nCreate one in the {brand} launcher.")));
        }
    };
    match user.status.as_str() {
        "active" => {}
        "pending" => return Ok(LoginVerdict::deny(format!("Your {brand} account is still waiting for approval."))),
        _ => {
            let reason = user.status_reason.as_deref().filter(|r| !r.trim().is_empty());
            return Ok(LoginVerdict::deny(match reason {
                Some(r) => format!("Your {brand} account is disabled.\n\n{r}"),
                None => format!("Your {brand} account is disabled."),
            }));
        }
    }
    let groups = auth::user_groups(state, user.id).await?;
    if server.access == "groups" && !user.is_admin() {
        let allowed = server.groups();
        if !groups.iter().any(|g| allowed.iter().any(|a| a.eq_ignore_ascii_case(g))) {
            return Ok(LoginVerdict::deny("You don't have access to this server."));
        }
    }
    if server.require_launcher {
        let since = ago(chrono::Duration::hours(LAUNCHER_WINDOW_HOURS));
        let launched = verify_launcher_ip(state, Some(user.id), ip, &since).await?;
        if !launched {
            return Ok(LoginVerdict::deny(format!("Please join through the {brand} launcher.")));
        }
    }
    Ok(LoginVerdict {
        allowed: true,
        message: None,
        account: Some(json!({ "username": user.username, "uuid": user.uuid, "role": user.role, "groups": groups })),
    })
}

#[derive(Deserialize)]
pub struct OnlinePlayer {
    uuid: String,
    name: String,
}

/// Counters accumulated since the previous sync (deltas, not totals).
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct StatDelta {
    uuid: String,
    name: String,
    playtime_secs: i64,
    joins: i64,
    deaths: i64,
    player_kills: i64,
    mob_kills: i64,
    blocks_broken: i64,
    blocks_placed: i64,
    messages: i64,
}

#[derive(Deserialize)]
pub struct GameEvent {
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    name: Option<String>,
    kind: String,
    #[serde(default)]
    detail: Option<String>,
    /// Epoch millis when it happened (defaults to now).
    #[serde(default)]
    at: Option<i64>,
}

#[derive(Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct FeatureToggles {
    pub leveling_enabled: Option<bool>,
    pub global_xp_multiplier: Option<f64>,
    pub server_xp_multiplier: Option<f64>,
    pub quests_enabled: Option<bool>,
    pub achievements_enabled: Option<bool>,
    pub guilds_enabled: Option<bool>,
    pub land_claiming_enabled: Option<bool>,
    pub social_enabled: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Sync {
    /// Stable across retries. Older integrations may omit it.
    batch_id: Option<String>,
    tps: Option<f64>,
    online: Vec<OnlinePlayer>,
    stats: Vec<StatDelta>,
    events: Vec<GameEvent>,
    features: Option<FeatureToggles>,
}

/// Periodic heartbeat: who's online, TPS, stat deltas and notable events.
/// Answers with players that must be kicked (accounts disabled meanwhile).
pub async fn sync(GameServer(server): GameServer, State(state): State<AppState>, Json(mut s): Json<Sync>) -> AppResult<Json<Value>> {
    let mut casino_on = true;
    let mut economy_on = true;
    if let Some(instance) = &state.instance_id {
        let row = store::get_instance(&state, instance).await?;
        let experience: scopenet_shared::Experience = serde_json::from_str(&row.experience)?;
        casino_on = experience.enabled("casino");
        economy_on = experience.enabled("economy");
        let features = s.features.get_or_insert_with(FeatureToggles::default);
        features.leveling_enabled = Some(experience.enabled("progression") && features.leveling_enabled.unwrap_or(true));
        features.quests_enabled = Some(experience.enabled("quests") && features.quests_enabled.unwrap_or(true));
        features.achievements_enabled = Some(experience.enabled("achievements") && features.achievements_enabled.unwrap_or(true));
        features.guilds_enabled = Some(experience.enabled("guilds") && features.guilds_enabled.unwrap_or(true));
    }
    let at_now = now();
    let mut tx = state.db.begin().await?;
    let progression = crate::progression::load(&mut tx).await?;
    let leveling_on = s.features.as_ref().and_then(|f| f.leveling_enabled).unwrap_or(true);
    let mut xp_before = std::collections::HashMap::new();
    if !leveling_on {
        for uuid in s.stats.iter().filter_map(|d| dashed(&d.uuid)).chain(s.events.iter().filter_map(|e| e.uuid.as_deref().and_then(dashed)))
        {
            if xp_before.contains_key(&uuid) {
                continue;
            }
            let xp: i64 = sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid = ?")
                .bind(&uuid)
                .fetch_optional(&mut *tx)
                .await?
                .unwrap_or(0);
            xp_before.insert(uuid, xp);
        }
    }

    // Record the receipt in the same transaction as the deltas. A response
    // lost after commit can then be retried without counting activity twice.
    let fresh = if let Some(batch) = &s.batch_id {
        if uuid::Uuid::parse_str(batch).is_err() {
            return Err(AppError::bad_request("batch_id must be a UUID"));
        }
        sqlx::query("INSERT OR IGNORE INTO server_sync_receipts (server_id, batch_id, created_at) VALUES (?, ?, ?)")
            .bind(server.id)
            .bind(batch)
            .bind(&at_now)
            .execute(&mut *tx)
            .await?
            .rows_affected()
            > 0
    } else {
        true
    };

    let online: Vec<(String, String)> =
        s.online.iter().take(MAX_ONLINE).filter_map(|p| Some((dashed(&p.uuid)?, clip(&p.name, 16)))).collect();
    sqlx::query("UPDATE game_servers SET last_seen = ?, online_count = ?, tps = ? WHERE id = ?")
        .bind(&at_now)
        .bind(online.len() as i64)
        .bind(s.tps.map(|t| t.clamp(0.0, 1000.0)))
        .bind(server.id)
        .execute(&mut *tx)
        .await?;

    // Keep join times for players that stay online.
    let keep = serde_json::to_string(&online.iter().map(|(u, _)| u).collect::<Vec<_>>()).unwrap_or_default();
    sqlx::query("DELETE FROM server_online WHERE server_id = ? AND uuid NOT IN (SELECT value FROM json_each(?))")
        .bind(server.id)
        .bind(&keep)
        .execute(&mut *tx)
        .await?;
    for (uuid, name) in &online {
        sqlx::query("INSERT INTO server_online (server_id, uuid, name, joined_at) VALUES (?, ?, ?, ?) ON CONFLICT DO UPDATE SET name = excluded.name")
            .bind(server.id)
            .bind(uuid)
            .bind(name)
            .bind(&at_now)
            .execute(&mut *tx)
            .await?;
    }

    for d in s.stats.iter().filter(|_| fresh) {
        let Some(uuid) = dashed(&d.uuid) else { continue };
        let n = |v: i64| v.clamp(0, 1_000_000);
        sqlx::query(
            "INSERT INTO player_stats (server_id, uuid, name, playtime_secs, joins, deaths, player_kills, mob_kills, blocks_broken, blocks_placed, messages, first_seen, last_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (server_id, uuid) DO UPDATE SET
                name = excluded.name,
                playtime_secs = playtime_secs + excluded.playtime_secs,
                joins = joins + excluded.joins,
                deaths = deaths + excluded.deaths,
                player_kills = player_kills + excluded.player_kills,
                mob_kills = mob_kills + excluded.mob_kills,
                blocks_broken = blocks_broken + excluded.blocks_broken,
                blocks_placed = blocks_placed + excluded.blocks_placed,
                messages = messages + excluded.messages,
                last_seen = excluded.last_seen",
        )
        .bind(server.id)
        .bind(&uuid)
        .bind(clip(&d.name, 16))
        .bind(n(d.playtime_secs))
        .bind(n(d.joins))
        .bind(n(d.deaths))
        .bind(n(d.player_kills))
        .bind(n(d.mob_kills))
        .bind(n(d.blocks_broken))
        .bind(n(d.blocks_placed))
        .bind(n(d.messages))
        .bind(&at_now)
        .bind(&at_now)
        .execute(&mut *tx)
        .await?;

        update_progression_for_delta(&mut tx, server.id, &uuid, d, &at_now, s.features.as_ref(), &progression).await?;
    }

    let achievements_on = s.features.as_ref().and_then(|f| f.achievements_enabled).unwrap_or(true);
    let quests_on = s.features.as_ref().and_then(|f| f.quests_enabled).unwrap_or(true);

    let mut casino_notes = Vec::new();
    let mut contract_notes = Vec::new();
    for e in s.events.iter().filter(|_| fresh).take(MAX_EVENTS_PER_SYNC) {
        let kind = clip(&e.kind, 24).to_ascii_lowercase();
        if kind.is_empty() {
            continue;
        }
        let at =
            e.at.and_then(chrono::DateTime::from_timestamp_millis)
                .filter(|t| (chrono::Utc::now() - *t).num_days().abs() < 2)
                .map(ts)
                .unwrap_or_else(|| at_now.clone());
        sqlx::query("INSERT INTO server_events (server_id, uuid, name, kind, detail, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(server.id)
            .bind(e.uuid.as_deref().and_then(dashed))
            .bind(e.name.as_deref().map(|n| clip(n, 16)))
            .bind(&kind)
            .bind(e.detail.as_deref().map(|d| clip(d, 256)))
            .bind(at)
            .execute(&mut *tx)
            .await?;

        // A player killing another player collects any bounties on the victim's head.
        if casino_on && kind == "pvp_kill" {
            if let (Some(killer), Some(victim)) = (e.uuid.as_deref().and_then(dashed), e.detail.as_deref().and_then(dashed)) {
                let killer_name = e.name.as_deref().map(|n| clip(n, 16)).unwrap_or_default();
                casino_notes.extend(super::casino::claim_on_kill(&mut tx, &server, &killer, &killer_name, &victim).await?);
            }
        }

        // Kills count toward the player's kill contracts.
        if economy_on && kind == "action" {
            if let (Some(uuid), Some(detail)) = (e.uuid.as_deref().and_then(dashed), e.detail.as_deref()) {
                if let Some((action, count)) = detail.rsplit_once(" +") {
                    if let Ok(count) = count.parse::<i64>() {
                        let name = e.name.as_deref().map(|n| clip(n, 16)).unwrap_or_default();
                        contract_notes.extend(
                            super::contracts::advance_kills(&mut tx, &server, &uuid, &name, action, count.clamp(0, 1_000_000)).await?,
                        );
                    }
                }
            }
        }

        if quests_on && kind == "action" {
            if let (Some(uuid), Some(detail)) = (e.uuid.as_deref().and_then(dashed), e.detail.as_deref()) {
                if let Some((action, count)) = detail.rsplit_once(" +") {
                    if let Ok(count) = count.parse::<i64>() {
                        crate::routes::quests::advance_action_quests(&mut tx, &uuid, action, count.clamp(0, 1_000_000), &at_now).await?;
                    }
                }
            }
        }

        // Check if event unlocks an achievement (if achievements enabled)
        if achievements_on {
            if let Some(ref euuid) = e.uuid.as_deref().and_then(dashed) {
                let event_ach: Vec<String> =
                    sqlx::query_scalar("SELECT id FROM achievements WHERE requirement_type = 'event' AND requirement_key = ?")
                        .bind(&kind)
                        .fetch_all(&mut *tx)
                        .await?;

                for ach_id in event_ach {
                    sqlx::query("INSERT OR IGNORE INTO user_achievements (user_uuid, achievement_id, unlocked_at) VALUES (?, ?, ?)")
                        .bind(euuid)
                        .bind(ach_id)
                        .bind(&at_now)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }

        if let Some(euuid) = e.uuid.as_deref().and_then(dashed) {
            crate::routes::events::refresh_participant(&mut tx, server.id, &euuid).await?;
        }
    }
    let mut rewarded = std::collections::HashSet::new();
    for d in &s.stats {
        if let Some(uuid) = dashed(&d.uuid) {
            rewarded.insert(uuid);
        }
    }
    for e in &s.events {
        if let Some(uuid) = e.uuid.as_deref().and_then(dashed) {
            rewarded.insert(uuid);
        }
    }
    if leveling_on {
        for uuid in rewarded {
            crate::routes::leveling::grant_rewards(&mut tx, &uuid, &at_now).await?;
        }
    } else {
        // Achievement unlocks remain enabled, but their trigger must not grant
        // progression on a server whose leveling module is disabled.
        for (uuid, xp) in xp_before {
            let level = progression.level_from_xp(xp).0;
            sqlx::query("UPDATE user_levels SET global_xp = ?, global_level = ? WHERE uuid = ?")
                .bind(xp)
                .bind(level)
                .bind(&uuid)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    if state.instance_id.is_some() {
        // Shared social presence contains no XP, balances or instance configuration.
        let mut presence = state.platform_db.begin().await?;
        sqlx::query("UPDATE game_servers SET last_seen=?,online_count=?,tps=? WHERE id=?")
            .bind(&at_now)
            .bind(online.len() as i64)
            .bind(s.tps)
            .bind(server.id)
            .execute(&mut *presence)
            .await?;
        sqlx::query("DELETE FROM server_online WHERE server_id=? AND uuid NOT IN (SELECT value FROM json_each(?))")
            .bind(server.id)
            .bind(&keep)
            .execute(&mut *presence)
            .await?;
        for (uuid, name) in &online {
            sqlx::query(
                "INSERT INTO server_online(server_id,uuid,name,joined_at) VALUES(?,?,?,?) ON CONFLICT DO UPDATE SET name=excluded.name",
            )
            .bind(server.id)
            .bind(uuid)
            .bind(name)
            .bind(&at_now)
            .execute(&mut *presence)
            .await?;
        }
        presence.commit().await?;
    }
    super::casino::send_notes(&state.db, casino_notes).await;
    super::contracts::send_notes(&state.db, contract_notes).await;

    // Occasional housekeeping.
    if rand::thread_rng().gen_ratio(1, 50) {
        let cutoff = ago(chrono::Duration::days(EVENT_RETENTION_DAYS));
        sqlx::query("DELETE FROM server_events WHERE created_at < ?").bind(cutoff).execute(&state.db).await?;
    }

    // Players whose access was revoked while they were online.
    let brand = store::branding(&state).await?.name;
    let mut kick = Vec::new();
    for (uuid, _) in &online {
        let Some(user) = user_by_uuid(&state, uuid).await? else { continue };
        let verdict = check_login(&state, &server, Some(&user), None, &brand).await?;
        if !verdict.allowed {
            kick.push(json!({ "uuid": uuid, "message": verdict.message }));
        }
    }
    // Events for plugins to turn into their own (level-ups, achievements, guild changes).
    let notifications = {
        let mut conn = state.db.acquire().await?;
        super::integrations::take_notifications(&mut conn, server.id).await?
    };
    let chat = super::chat::load(&state).await?;
    let utilities = super::utilities::load(&state).await?;
    let custom_items = super::utilities::custom_items_for_sync(&state).await?;
    Ok(Json(json!({
        "ok": true, "kick": kick, "sync_interval_secs": SYNC_INTERVAL_SECS, "notifications": notifications,
        "chat": chat, "utilities": utilities, "custom_items": custom_items, "content": super::content::content_for_sync(&state).await?,
    })))
}

async fn update_progression_for_delta(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    server_id: i64,
    uuid: &str,
    d: &StatDelta,
    at_now: &str,
    features: Option<&FeatureToggles>,
    cfg: &crate::progression::Progression,
) -> AppResult<()> {
    let leveling_on = features.and_then(|f| f.leveling_enabled).unwrap_or(true);
    let quests_on = features.and_then(|f| f.quests_enabled).unwrap_or(true);
    let achievements_on = features.and_then(|f| f.achievements_enabled).unwrap_or(true);

    // Each game server's own multiplier stacks with the panel-wide one.
    let global_mult = features.and_then(|f| f.global_xp_multiplier).unwrap_or(1.0).max(0.0) * cfg.global_xp_multiplier;
    let server_mult = features.and_then(|f| f.server_xp_multiplier).unwrap_or(1.5).max(0.0) * cfg.server_xp_multiplier;

    // 1. Calculate XP gains: Global XP and Server XP (only if leveling enabled)
    if leveling_on {
        let base_xp = cfg.base_xp(d.playtime_secs, d.player_kills, d.mob_kills, d.blocks_broken, d.blocks_placed, d.messages);

        let delta_xp = (base_xp * global_mult).round() as i64;
        let server_delta_xp = (base_xp * server_mult).round() as i64;

        if delta_xp > 0 {
            sqlx::query(
                "INSERT INTO user_levels (uuid, global_xp, global_level, updated_at)
             VALUES (?, ?, 1, ?)
             ON CONFLICT (uuid) DO UPDATE SET
                global_xp = global_xp + excluded.global_xp,
                updated_at = excluded.updated_at",
            )
            .bind(uuid)
            .bind(delta_xp)
            .bind(at_now)
            .execute(&mut **tx)
            .await?;

            // Recompute global level
            let new_xp: i64 =
                sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid = ?").bind(uuid).fetch_one(&mut **tx).await?;
            let (new_lvl, _, _, _) = cfg.level_from_xp(new_xp);
            sqlx::query("UPDATE user_levels SET global_level = ? WHERE uuid = ?").bind(new_lvl).bind(uuid).execute(&mut **tx).await?;
        }

        if server_delta_xp > 0 {
            sqlx::query(
                "INSERT INTO server_levels (server_id, uuid, server_xp, server_level, updated_at)
             VALUES (?, ?, ?, 1, ?)
             ON CONFLICT (server_id, uuid) DO UPDATE SET
                server_xp = server_xp + excluded.server_xp,
                updated_at = excluded.updated_at",
            )
            .bind(server_id)
            .bind(uuid)
            .bind(server_delta_xp)
            .bind(at_now)
            .execute(&mut **tx)
            .await?;

            // Recompute server level
            let new_s_xp: i64 = sqlx::query_scalar("SELECT server_xp FROM server_levels WHERE server_id = ? AND uuid = ?")
                .bind(server_id)
                .bind(uuid)
                .fetch_one(&mut **tx)
                .await?;
            let (new_s_lvl, _, _, _) = cfg.level_from_xp(new_s_xp);
            sqlx::query("UPDATE server_levels SET server_level = ? WHERE server_id = ? AND uuid = ?")
                .bind(new_s_lvl)
                .bind(server_id)
                .bind(uuid)
                .execute(&mut **tx)
                .await?;
        }
    }

    // 2. Advance active Daily & Weekly Quests (if quests enabled)
    if quests_on {
        let stat_deltas = [
            ("playtime_secs", d.playtime_secs),
            ("joins", d.joins),
            ("player_kills", d.player_kills),
            ("mob_kills", d.mob_kills),
            ("blocks_broken", d.blocks_broken),
            ("blocks_placed", d.blocks_placed),
            ("messages", d.messages),
        ];

        let daily_key = crate::routes::quests::current_daily_key();
        let weekly_key = crate::routes::quests::current_weekly_key();
        // Only the quests this player was given for the period make progress.
        let daily_ids = crate::progression::assigned_quest_ids(&mut **tx, cfg, uuid, "daily", &daily_key).await?;
        let weekly_ids = crate::progression::assigned_quest_ids(&mut **tx, cfg, uuid, "weekly", &weekly_key).await?;

        for (stat_name, amount) in stat_deltas {
            if amount <= 0 {
                continue;
            }

            // Daily quests matching this stat
            let d_quests: Vec<(String, i64)> =
                sqlx::query_as("SELECT id, target_count FROM quests WHERE enabled = 1 AND period = 'daily' AND target_stat = ?")
                    .bind(stat_name)
                    .fetch_all(&mut **tx)
                    .await?;

            for (qid, target) in d_quests.into_iter().filter(|(id, _)| daily_ids.contains(id)) {
                sqlx::query(
                    "INSERT INTO user_quests (user_uuid, quest_id, period_key, progress, completed, claimed, updated_at)
                     VALUES (?, ?, ?, ?, ?, 0, ?)
                     ON CONFLICT (user_uuid, quest_id, period_key) DO UPDATE SET
                        progress = progress + excluded.progress,
                        completed = CASE WHEN (progress + excluded.progress) >= ? THEN 1 ELSE completed END,
                        updated_at = excluded.updated_at",
                )
                .bind(uuid)
                .bind(&qid)
                .bind(&daily_key)
                .bind(amount)
                .bind(if amount >= target { 1 } else { 0 })
                .bind(at_now)
                .bind(target)
                .execute(&mut **tx)
                .await?;
            }

            // Weekly quests matching this stat
            let w_quests: Vec<(String, i64)> =
                sqlx::query_as("SELECT id, target_count FROM quests WHERE enabled = 1 AND period = 'weekly' AND target_stat = ?")
                    .bind(stat_name)
                    .fetch_all(&mut **tx)
                    .await?;

            for (qid, target) in w_quests.into_iter().filter(|(id, _)| weekly_ids.contains(id)) {
                sqlx::query(
                    "INSERT INTO user_quests (user_uuid, quest_id, period_key, progress, completed, claimed, updated_at)
                     VALUES (?, ?, ?, ?, ?, 0, ?)
                     ON CONFLICT (user_uuid, quest_id, period_key) DO UPDATE SET
                        progress = progress + excluded.progress,
                        completed = CASE WHEN (progress + excluded.progress) >= ? THEN 1 ELSE completed END,
                        updated_at = excluded.updated_at",
                )
                .bind(uuid)
                .bind(&qid)
                .bind(&weekly_key)
                .bind(amount)
                .bind(if amount >= target { 1 } else { 0 })
                .bind(at_now)
                .bind(target)
                .execute(&mut **tx)
                .await?;
            }
        }
    }

    // 3. Milestone Achievements Check (if achievements enabled)
    if achievements_on {
        let total_stats: Option<(Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>)> = sqlx::query_as(
            "SELECT SUM(playtime_secs), SUM(player_kills), SUM(mob_kills), SUM(blocks_broken), SUM(blocks_placed), SUM(messages)
             FROM player_stats WHERE uuid = ?",
        )
        .bind(uuid)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some((playtime, pkills, mkills, bbroken, bplaced, msgs)) = total_stats {
            let checks = [
                ("blocks_broken", bbroken.unwrap_or(0)),
                ("blocks_placed", bplaced.unwrap_or(0)),
                ("player_kills", pkills.unwrap_or(0)),
                ("mob_kills", mkills.unwrap_or(0)),
                ("playtime_secs", playtime.unwrap_or(0)),
                ("messages", msgs.unwrap_or(0)),
            ];

            for (key, val) in checks {
                let eligible: Vec<String> = sqlx::query_scalar(
                    "SELECT id FROM achievements WHERE requirement_type = 'stat' AND requirement_key = ? AND requirement_value <= ?",
                )
                .bind(key)
                .bind(val)
                .fetch_all(&mut **tx)
                .await?;

                for ach_id in eligible {
                    sqlx::query("INSERT OR IGNORE INTO user_achievements (user_uuid, achievement_id, unlocked_at) VALUES (?, ?, ?)")
                        .bind(uuid)
                        .bind(ach_id)
                        .bind(at_now)
                        .execute(&mut **tx)
                        .await?;
                }
            }
        }
    }

    // Level milestone achievements
    let global_level: Option<i64> =
        sqlx::query_scalar("SELECT global_level FROM user_levels WHERE uuid = ?").bind(uuid).fetch_optional(&mut **tx).await?;

    if let Some(lvl) = global_level.filter(|_| achievements_on) {
        let lvl_ach: Vec<String> =
            sqlx::query_scalar("SELECT id FROM achievements WHERE requirement_type = 'level' AND requirement_value <= ?")
                .bind(lvl)
                .fetch_all(&mut **tx)
                .await?;

        for ach_id in lvl_ach {
            sqlx::query("INSERT OR IGNORE INTO user_achievements (user_uuid, achievement_id, unlocked_at) VALUES (?, ?, ?)")
                .bind(uuid)
                .bind(ach_id)
                .bind(at_now)
                .execute(&mut **tx)
                .await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Admin API
// ---------------------------------------------------------------------------

pub async fn list(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Vec<ServerView>>> {
    let rows: Vec<ServerRow> = sqlx::query_as("SELECT * FROM game_servers ORDER BY name COLLATE NOCASE").fetch_all(&state.db).await?;
    Ok(Json(rows.into_iter().map(view).collect()))
}

#[derive(Deserialize)]
pub struct ServerInput {
    #[serde(default)]
    instance_id: String,
    name: String,
    #[serde(default = "default_access")]
    access: String,
    #[serde(default)]
    allowed_groups: Vec<String>,
    #[serde(default)]
    require_launcher: bool,
    /// Draw this server on the SCOPENET Map. On unless switched off.
    #[serde(default = "yes")]
    map_enabled: bool,
    #[serde(default)]
    economy_group: String,
}

fn yes() -> bool {
    true
}

fn default_access() -> String {
    "all".into()
}

impl ServerInput {
    fn validate(&self) -> AppResult<()> {
        if self.name.trim().is_empty() || self.name.chars().count() > 48 {
            return Err(AppError::bad_request("give the server a name (up to 48 characters)"));
        }
        if !matches!(self.access.as_str(), "all" | "members" | "groups") {
            return Err(AppError::bad_request("access must be all, members or groups"));
        }
        if self.access == "groups" && self.allowed_groups.is_empty() {
            return Err(AppError::bad_request("pick at least one group"));
        }
        let group = self.economy_group.trim();
        if group.chars().count() > 32 || !group.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')) {
            return Err(AppError::bad_request("economy groups are up to 32 letters, digits, spaces, - or _"));
        }
        Ok(())
    }
}

pub async fn get_server(state: &AppState, id: i64) -> AppResult<ServerRow> {
    sqlx::query_as("SELECT * FROM game_servers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("server not found"))
}

pub async fn create(_: AdminUser, State(state): State<AppState>, Json(mut input): Json<ServerInput>) -> AppResult<Json<Value>> {
    input.validate()?;
    if input.instance_id.is_empty() {
        if let Some(id) = &state.instance_id {
            input.instance_id = id.clone();
        }
    }
    if state.instance_id.as_ref().is_some_and(|id| *id != input.instance_id) {
        return Err(AppError::bad_request("server must belong to the selected instance"));
    }
    let token = new_token();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO game_servers (name, token_hash, token_hint, access, allowed_groups, require_launcher, created_at, instance_id, live_map_enabled, economy_group) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(input.name.trim())
    .bind(hash_token(&token))
    .bind(&token[token.len() - 4..])
    .bind(&input.access)
    .bind(serde_json::to_string(&input.allowed_groups).unwrap_or_else(|_| "[]".into()))
    .bind(input.require_launcher)
    .bind(now())
    .bind(&input.instance_id)
    .bind(input.map_enabled)
    .bind(input.economy_group.trim())
    .fetch_one(&state.platform_db)
    .await?;
    if state.instance_id.is_some() {
        let locator: ServerRow = sqlx::query_as("SELECT * FROM game_servers WHERE id=?").bind(id).fetch_one(&state.platform_db).await?;
        sqlx::query("INSERT INTO game_servers(id,name,token_hash,token_hint,created_at,instance_id,access,allowed_groups,require_launcher,live_map_enabled,economy_group) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
            .bind(id).bind(&locator.name).bind(&locator.token_hash).bind(&locator.token_hint).bind(&locator.created_at).bind(&locator.instance_id)
            .bind(&locator.access).bind(&locator.allowed_groups).bind(locator.require_launcher).bind(locator.map_enabled).bind(&locator.economy_group).execute(&state.db).await?;
    }
    let server = get_server(&state, id).await?;
    Ok(Json(json!({ "server": view(server), "token": token })))
}

pub async fn update(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(mut input): Json<ServerInput>,
) -> AppResult<Json<ServerView>> {
    input.validate()?;
    if input.instance_id.is_empty() {
        if let Some(id) = &state.instance_id {
            input.instance_id = id.clone();
        }
    }
    if state.instance_id.as_ref().is_some_and(|id| *id != input.instance_id) {
        return Err(AppError::bad_request("server must belong to the selected instance"));
    }
    get_server(&state, id).await?;
    sqlx::query("UPDATE game_servers SET name = ?, access = ?, allowed_groups = ?, require_launcher = ?, instance_id = ?, live_map_enabled = ?, economy_group = ? WHERE id = ?")
        .bind(input.name.trim())
        .bind(&input.access)
        .bind(serde_json::to_string(&input.allowed_groups).unwrap_or_else(|_| "[]".into()))
        .bind(input.require_launcher)
        .bind(&input.instance_id)
        .bind(input.map_enabled)
        .bind(input.economy_group.trim())
        .bind(id)
        .execute(&state.db)
        .await?;
    if state.instance_id.is_some() {
        sqlx::query("UPDATE game_servers SET name=?,access=?,allowed_groups=?,require_launcher=?,live_map_enabled=? WHERE id=?")
            .bind(input.name.trim())
            .bind(&input.access)
            .bind(serde_json::to_string(&input.allowed_groups)?)
            .bind(input.require_launcher)
            .bind(input.map_enabled)
            .bind(id)
            .execute(&state.platform_db)
            .await?;
    }
    Ok(Json(view(get_server(&state, id).await?)))
}

pub async fn regenerate_token(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    get_server(&state, id).await?;
    let token = new_token();
    sqlx::query("UPDATE game_servers SET token_hash = ?, token_hint = ? WHERE id = ?")
        .bind(hash_token(&token))
        .bind(&token[token.len() - 4..])
        .bind(id)
        .execute(&state.db)
        .await?;
    if state.instance_id.is_some() {
        sqlx::query("UPDATE game_servers SET token_hash=?,token_hint=? WHERE id=?")
            .bind(hash_token(&token))
            .bind(&token[token.len() - 4..])
            .bind(id)
            .execute(&state.platform_db)
            .await?;
    }
    Ok(Json(json!({ "token": token })))
}

pub async fn remove(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    // If this server holds its group's shared balances, hand them to the next server first.
    let group: String = sqlx::query_scalar("SELECT economy_group FROM game_servers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or_default();
    if !group.is_empty() && economy_scope(&state.db, id).await? == id {
        let next: Option<i64> = sqlx::query_scalar("SELECT MIN(id) FROM game_servers WHERE id <> ? AND economy_group = ? COLLATE NOCASE")
            .bind(id)
            .bind(&group)
            .fetch_one(&state.db)
            .await?;
        if let Some(next) = next {
            for table in ["server_economy", "guild_wallets", "guild_wallet_transactions"] {
                let sql = if table == "guild_wallet_transactions" {
                    format!("UPDATE {table} SET server_id = ? WHERE server_id = ?")
                } else {
                    format!("UPDATE OR IGNORE {table} SET server_id = ? WHERE server_id = ?")
                };
                sqlx::query(&sql).bind(next).bind(id).execute(&state.db).await?;
            }
        }
    }
    state.worldmap.forget_server(id);
    let done = sqlx::query("DELETE FROM game_servers WHERE id = ?").bind(id).execute(&state.db).await?;
    if state.instance_id.is_some() {
        sqlx::query("DELETE FROM game_servers WHERE id=?").bind(id).execute(&state.platform_db).await?;
    }
    if done.rows_affected() == 0 {
        return Err(AppError::not_found("server not found"));
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct StatRow {
    pub server_id: i64,
    pub uuid: String,
    pub name: String,
    pub playtime_secs: i64,
    pub joins: i64,
    pub deaths: i64,
    pub player_kills: i64,
    pub mob_kills: i64,
    pub blocks_broken: i64,
    pub blocks_placed: i64,
    pub messages: i64,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EventRow {
    pub id: i64,
    pub server_id: i64,
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub kind: String,
    pub detail: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct OnlineRow {
    uuid: String,
    name: String,
    joined_at: String,
}

#[derive(Deserialize)]
pub struct DetailQuery {
    #[serde(default = "default_sort")]
    sort: String,
}

fn default_sort() -> String {
    "playtime_secs".into()
}

pub async fn detail(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<DetailQuery>,
) -> AppResult<Json<Value>> {
    let server = get_server(&state, id).await?;
    let online: Vec<OnlineRow> = if server.is_online() {
        sqlx::query_as("SELECT uuid, name, joined_at FROM server_online WHERE server_id = ? ORDER BY joined_at")
            .bind(id)
            .fetch_all(&state.db)
            .await?
    } else {
        Vec::new()
    };
    // Whitelisted so the column name can be interpolated safely.
    let sort = match q.sort.as_str() {
        "joins" | "deaths" | "player_kills" | "mob_kills" | "blocks_broken" | "blocks_placed" | "messages" | "last_seen" => q.sort.as_str(),
        _ => "playtime_secs",
    };
    let leaderboard: Vec<StatRow> =
        sqlx::query_as(&format!("SELECT * FROM player_stats WHERE server_id = ? ORDER BY {sort} DESC LIMIT 100"))
            .bind(id)
            .fetch_all(&state.db)
            .await?;
    let events: Vec<EventRow> =
        sqlx::query_as("SELECT * FROM server_events WHERE server_id = ? ORDER BY id DESC LIMIT 100").bind(id).fetch_all(&state.db).await?;
    let (players, playtime): (i64, Option<i64>) =
        sqlx::query_as("SELECT COUNT(*), SUM(playtime_secs) FROM player_stats WHERE server_id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(Json(json!({
        "server": view(server),
        "online_players": online,
        "leaderboard": leaderboard,
        "events": events,
        "totals": { "players": players, "playtime_secs": playtime.unwrap_or(0) },
    })))
}

/// Per-server stats and recent events for one account (Players page).
pub async fn user_activity(_: AdminUser, State(state): State<AppState>, Path(user_id): Path<i64>) -> AppResult<Json<Value>> {
    let uuid: String = sqlx::query_scalar("SELECT uuid FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    let mut rows: Vec<StatRowNamed> = vec![];
    let mut events: Vec<EventRow> = vec![];
    let mut online = vec![];
    for scope in state.experiences.reporting_states(&state).await? {
        let mut scope_rows: Vec<StatRowNamed> = sqlx::query_as(
        "SELECT s.*, g.name AS server_name FROM player_stats s JOIN game_servers g ON g.id = s.server_id WHERE s.uuid = ? ORDER BY s.last_seen DESC",
    )
    .bind(&uuid)
    .fetch_all(&scope.db)
    .await?;
        let mut scope_events: Vec<EventRow> =
            sqlx::query_as("SELECT * FROM server_events WHERE uuid = ? ORDER BY id DESC LIMIT 50").bind(&uuid).fetch_all(&scope.db).await?;
        let mut scope_online: Vec<(i64, String)> = sqlx::query_as(
            "SELECT g.id, g.name FROM server_online o JOIN game_servers g ON g.id = o.server_id WHERE o.uuid = ? AND g.last_seen >= ?",
        )
        .bind(&uuid)
        .bind(stale_cutoff())
        .fetch_all(&scope.db)
        .await?;
        rows.append(&mut scope_rows);
        events.append(&mut scope_events);
        online.append(&mut scope_online);
    }
    rows.sort_by(|a, b| b.stats.last_seen.cmp(&a.stats.last_seen));
    events.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    events.truncate(50);
    Ok(Json(json!({
        "servers": rows,
        "events": events,
        "online_on": online.into_iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
    })))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct StatRowNamed {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub stats: StatRow,
    pub server_name: String,
}

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default = "default_sort")]
    pub sort: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AggregatedStatRow {
    pub uuid: String,
    pub name: String,
    pub playtime_secs: i64,
    pub joins: i64,
    pub deaths: i64,
    pub player_kills: i64,
    pub mob_kills: i64,
    pub blocks_broken: i64,
    pub blocks_placed: i64,
    pub messages: i64,
    pub first_seen: String,
    pub last_seen: String,
}

/// Public list of servers with status and player counts.
pub async fn public_servers(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let servers: Vec<ServerRow> = sqlx::query_as("SELECT * FROM game_servers ORDER BY name COLLATE NOCASE").fetch_all(&state.db).await?;
    let list: Vec<Value> = servers
        .into_iter()
        .map(|s| {
            let is_online = s.is_online();
            json!({
                "id": s.id,
                "name": s.name,
                "instance_id": s.instance_id,
                "map": s.map_enabled,
                "online": is_online,
                "players_online": if is_online { s.online_count } else { 0 },
                "players_max": s.max_players,
                "software": s.software,
                "mc_version": s.mc_version,
                "tps": s.tps,
                "last_seen": s.last_seen,
            })
        })
        .collect();
    Ok(Json(json!({ "servers": list })))
}

/// Public server-specific leaderboard.
pub async fn public_server_leaderboard(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<LeaderboardQuery>,
) -> AppResult<Json<Value>> {
    let server = get_server(&state, id).await?;
    let sort = match q.sort.as_str() {
        "joins" | "deaths" | "player_kills" | "mob_kills" | "blocks_broken" | "blocks_placed" | "messages" | "last_seen" => q.sort.as_str(),
        _ => "playtime_secs",
    };
    let limit = q.limit.clamp(1, 100);
    let leaderboard: Vec<StatRow> =
        sqlx::query_as(&format!("SELECT * FROM player_stats WHERE server_id = ? ORDER BY {sort} DESC LIMIT {limit}"))
            .bind(id)
            .fetch_all(&state.db)
            .await?;
    let (players, playtime): (i64, Option<i64>) =
        sqlx::query_as("SELECT COUNT(*), SUM(playtime_secs) FROM player_stats WHERE server_id = ?").bind(id).fetch_one(&state.db).await?;
    Ok(Json(json!({
        "server": {
            "id": server.id,
            "name": server.name,
            "online": server.is_online(),
            "players_online": server.online_count,
            "players_max": server.max_players,
        },
        "leaderboard": leaderboard,
        "totals": { "players": players, "playtime_secs": playtime.unwrap_or(0) },
        "sort": sort,
    })))
}

/// Public global leaderboard aggregated across all servers.
pub async fn global_leaderboard(State(state): State<AppState>, Query(q): Query<LeaderboardQuery>) -> AppResult<Json<Value>> {
    let sort = match q.sort.as_str() {
        "joins" | "deaths" | "player_kills" | "mob_kills" | "blocks_broken" | "blocks_placed" | "messages" | "last_seen" => q.sort.as_str(),
        _ => "playtime_secs",
    };
    let limit = q.limit.clamp(1, 100);
    let leaderboard: Vec<AggregatedStatRow> = sqlx::query_as(&format!(
        "SELECT uuid, name,
                SUM(playtime_secs) as playtime_secs,
                SUM(joins) as joins,
                SUM(deaths) as deaths,
                SUM(player_kills) as player_kills,
                SUM(mob_kills) as mob_kills,
                SUM(blocks_broken) as blocks_broken,
                SUM(blocks_placed) as blocks_placed,
                SUM(messages) as messages,
                MIN(first_seen) as first_seen,
                MAX(last_seen) as last_seen
         FROM player_stats
         GROUP BY uuid
         ORDER BY {sort} DESC
         LIMIT {limit}"
    ))
    .fetch_all(&state.db)
    .await?;

    let (players, playtime): (i64, Option<i64>) =
        sqlx::query_as("SELECT COUNT(DISTINCT uuid), SUM(playtime_secs) FROM player_stats").fetch_one(&state.db).await?;

    Ok(Json(json!({
        "leaderboard": leaderboard,
        "totals": { "players": players, "playtime_secs": playtime.unwrap_or(0) },
        "sort": sort,
    })))
}

/// Player's personal stats across servers (authenticated endpoint for launcher).
pub async fn account_player_stats(State(state): State<AppState>, AuthUser(user): AuthUser) -> AppResult<Json<Value>> {
    let rows: Vec<StatRowNamed> = sqlx::query_as(
        "SELECT s.*, g.name AS server_name FROM player_stats s JOIN game_servers g ON g.id = s.server_id WHERE s.uuid = ? ORDER BY s.last_seen DESC",
    )
    .bind(&user.uuid)
    .fetch_all(&state.db)
    .await?;

    let total: Option<AggregatedStatRow> = sqlx::query_as(
        "SELECT uuid, name,
                SUM(playtime_secs) as playtime_secs,
                SUM(joins) as joins,
                SUM(deaths) as deaths,
                SUM(player_kills) as player_kills,
                SUM(mob_kills) as mob_kills,
                SUM(blocks_broken) as blocks_broken,
                SUM(blocks_placed) as blocks_placed,
                SUM(messages) as messages,
                MIN(first_seen) as first_seen,
                MAX(last_seen) as last_seen
         FROM player_stats
         WHERE uuid = ?
         GROUP BY uuid",
    )
    .bind(&user.uuid)
    .fetch_optional(&state.db)
    .await?;

    let events: Vec<EventRow> = sqlx::query_as("SELECT * FROM server_events WHERE uuid = ? ORDER BY id DESC LIMIT 20")
        .bind(&user.uuid)
        .fetch_all(&state.db)
        .await?;

    Ok(Json(json!({
        "username": user.username,
        "uuid": user.uuid,
        "total": total,
        "servers": rows,
        "events": events,
    })))
}

/// Live summary for the dashboard.
pub async fn live_summary(state: &AppState) -> AppResult<Value> {
    let servers: Vec<ServerRow> = sqlx::query_as("SELECT * FROM game_servers ORDER BY name COLLATE NOCASE").fetch_all(&state.db).await?;
    let online_now: i64 = servers.iter().filter(|s| s.is_online()).map(|s| s.online_count).sum();
    Ok(json!({
        "online_now": online_now,
        "servers": servers.into_iter().map(view).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ips_match() {
        // Exact match
        assert!(ips_match("1.2.3.4", "1.2.3.4"));
        assert!(ips_match("203.0.113.9", "203.0.113.9"));

        // Loopback IPv4 & IPv6
        assert!(!ips_match("127.0.0.1", "::1"));
        assert!(!ips_match("::1", "127.0.0.1"));
        assert!(ips_match("127.0.0.1", "127.0.0.1"));

        // IPv4-mapped IPv6
        assert!(ips_match("::ffff:192.168.1.10", "192.168.1.10"));
        assert!(ips_match("192.168.1.10", "::ffff:192.168.1.10"));

        // Docker bridge / private gateway recorded
        assert!(!ips_match("172.18.0.1", "192.168.1.50"));
        assert!(!ips_match("10.0.0.1", "10.0.0.2"));

        // Subnet /24 match
        assert!(!ips_match("203.0.113.5", "203.0.113.9"));

        // Different public networks do not match
        assert!(!ips_match("198.51.100.1", "203.0.113.9"));
        assert!(!ips_match("8.8.8.8", "1.1.1.1"));
    }
}

/// One server's public status, for Discord embeds and the like.
pub struct StatusRow {
    pub id: i64,
    pub name: String,
    pub online: bool,
    pub players: i64,
    pub max_players: i64,
    pub tps: Option<f64>,
    pub software: String,
    pub version: String,
    pub last_seen: Option<String>,
}

pub async fn status_rows(state: &AppState) -> AppResult<Vec<StatusRow>> {
    let rows: Vec<ServerRow> = sqlx::query_as("SELECT * FROM game_servers ORDER BY id").fetch_all(&state.db).await?;
    Ok(rows
        .into_iter()
        .map(|s| {
            let online = s.is_online();
            StatusRow {
                id: s.id,
                name: s.name.clone(),
                online,
                players: if online { s.online_count } else { 0 },
                max_players: s.max_players,
                tps: if online { s.tps } else { None },
                software: s.software.clone().unwrap_or_default(),
                version: s.mc_version.clone().unwrap_or_default(),
                last_seen: s.last_seen.clone(),
            }
        })
        .collect())
}
