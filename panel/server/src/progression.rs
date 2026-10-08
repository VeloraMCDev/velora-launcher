//! Admin-tunable progression: quest limits and rotation, XP rates, the level
//! curve and manual XP/level adjustments.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqliteConnection;

/// Hard ceiling so a bad curve can never make level lookups spin.
pub const LEVEL_CEILING: i64 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct XpRates {
    /// XP per hour spent online.
    pub playtime_per_hour: f64,
    pub player_kill: f64,
    pub mob_kill: f64,
    pub block_broken: f64,
    pub block_placed: f64,
    pub message: f64,
}

impl Default for XpRates {
    // Matches the original hard-coded formula: 1 XP per 6 s, 50/15 per kill,
    // 1 XP per 5 blocks, 2 per chat message.
    fn default() -> Self {
        Self { playtime_per_hour: 600.0, player_kill: 50.0, mob_kill: 15.0, block_broken: 0.2, block_placed: 0.2, message: 2.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Progression {
    /// Daily quests each player is given per day. 0 hands out every enabled quest.
    pub daily_quest_limit: u32,
    /// Weekly quests each player is given per week. 0 hands out every enabled quest.
    pub weekly_quest_limit: u32,
    /// `per_player` (each player draws their own set) or `shared` (everyone gets the same set).
    pub quest_rotation: String,
    /// XP needed for level L is `round(level_base * L^level_exponent)`.
    pub level_base: f64,
    pub level_exponent: f64,
    /// Highest reachable level. 0 means no cap.
    pub max_level: i64,
    /// Panel-side multipliers, applied on top of each game server's own.
    pub global_xp_multiplier: f64,
    pub server_xp_multiplier: f64,
    pub xp_rates: XpRates,
    /// Economy and guild rules that apply on every server.
    pub rules: Rules,
}

/// Network-wide limits an admin can change without touching a server. 0 means "no limit" where it says so.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Rules {
    /// Balance a player starts with on each server economy.
    pub starting_balance: f64,
    /// Chunks a new guild may claim.
    pub guild_base_claims: i64,
    /// Extra claimable chunks for every member beyond the leader.
    pub guild_claims_per_member: i64,
    /// Extra claimable chunks for every guild level above 1.
    pub guild_claims_per_level: i64,
    /// Most members a guild may have (0 = no limit).
    pub guild_max_members: i64,
    /// Most listings one player may have on a server's market (0 = no limit).
    pub market_max_listings: i64,
    /// Money paid by default for quests, achievements and level-ups that have no reward of their own.
    pub auto_money: AutoMoney,
}

/// How much money a quest, achievement or level pays when the admin has not set an amount, scaled by how hard it is.
/// Every amount is a flat number ending in 5 or 0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AutoMoney {
    pub enabled: bool,
    /// Multiplies every default (1.0 = as designed).
    pub scale: f64,
    /// Money per XP of a daily quest, a weekly quest and any other quest (XP is the difficulty the quest was given).
    pub quest_daily: f64,
    pub quest_weekly: f64,
    pub quest_other: f64,
    /// Money per XP of an achievement (earned once, so worth a little more than a quest).
    pub achievement: f64,
    /// Reaching level L pays `level_base * L ^ level_exponent`.
    pub level_base: f64,
    pub level_exponent: f64,
    /// A rank milestone (a level reward such as a title) adds this many times the level's money.
    pub milestone: f64,
}

impl Default for AutoMoney {
    fn default() -> Self {
        Self {
            enabled: true,
            scale: 1.0,
            quest_daily: 0.35,
            quest_weekly: 0.5,
            quest_other: 0.4,
            achievement: 0.6,
            level_base: 20.0,
            level_exponent: 1.4,
            milestone: 3.0,
        }
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            starting_balance: 1000.0,
            guild_base_claims: 16,
            guild_claims_per_member: 0,
            guild_claims_per_level: 0,
            guild_max_members: 0,
            market_max_listings: 0,
            auto_money: AutoMoney::default(),
        }
    }
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            daily_quest_limit: 5,
            weekly_quest_limit: 3,
            quest_rotation: "per_player".into(),
            level_base: 100.0,
            level_exponent: 1.5,
            max_level: 0,
            global_xp_multiplier: 1.0,
            server_xp_multiplier: 1.0,
            xp_rates: XpRates::default(),
            rules: Rules::default(),
        }
    }
}

impl Progression {
    pub fn validate(&self) -> AppResult<()> {
        let bad = |m: &str| Err(AppError::bad_request(m.to_string()));
        if self.daily_quest_limit > 100 || self.weekly_quest_limit > 100 {
            return bad("Quest limits can be at most 100 (use 0 to give out every quest).");
        }
        if !matches!(self.quest_rotation.as_str(), "per_player" | "shared") {
            return bad("Quest rotation must be per_player or shared.");
        }
        if !self.level_base.is_finite() || !(10.0..=100_000.0).contains(&self.level_base) {
            return bad("Level base must be between 10 and 100,000.");
        }
        if !self.level_exponent.is_finite() || !(1.0..=3.0).contains(&self.level_exponent) {
            return bad("Level exponent must be between 1 and 3.");
        }
        if !(0..=LEVEL_CEILING).contains(&self.max_level) || (self.max_level == 1) {
            return bad("Max level must be 0 (no cap) or between 2 and 10,000.");
        }
        for (name, v) in [("Global XP multiplier", self.global_xp_multiplier), ("Server XP multiplier", self.server_xp_multiplier)] {
            if !v.is_finite() || !(0.0..=100.0).contains(&v) {
                return Err(AppError::bad_request(format!("{name} must be between 0 and 100.")));
            }
        }
        let a = &self.rules.auto_money;
        for (name, v, hi) in [
            ("Default money scale", a.scale, 100.0),
            ("Daily quest rate", a.quest_daily, 100.0),
            ("Weekly quest rate", a.quest_weekly, 100.0),
            ("Quest rate", a.quest_other, 100.0),
            ("Achievement rate", a.achievement, 100.0),
            ("Level money base", a.level_base, 100_000.0),
            ("Milestone multiplier", a.milestone, 100.0),
        ] {
            if !v.is_finite() || !(0.0..=hi).contains(&v) {
                return Err(AppError::bad_request(format!("{name} must be between 0 and {hi}.")));
            }
        }
        if !a.level_exponent.is_finite() || !(0.5..=3.0).contains(&a.level_exponent) {
            return bad("Level money exponent must be between 0.5 and 3.");
        }
        let rules = &self.rules;
        if !rules.starting_balance.is_finite() || !(0.0..=1e9).contains(&rules.starting_balance) {
            return bad("Starting balance must be between 0 and 1,000,000,000.");
        }
        for (name, v, max) in [
            ("Guild base claims", rules.guild_base_claims, 100_000),
            ("Claims per member", rules.guild_claims_per_member, 10_000),
            ("Claims per guild level", rules.guild_claims_per_level, 10_000),
            ("Guild member limit", rules.guild_max_members, 10_000),
            ("Market listing limit", rules.market_max_listings, 10_000),
        ] {
            if !(0..=max).contains(&v) {
                return Err(AppError::bad_request(format!("{name} must be between 0 and {max}.")));
            }
        }
        let r = &self.xp_rates;
        for (name, v) in [
            ("Playtime XP", r.playtime_per_hour),
            ("Player kill XP", r.player_kill),
            ("Mob kill XP", r.mob_kill),
            ("Block broken XP", r.block_broken),
            ("Block placed XP", r.block_placed),
            ("Chat message XP", r.message),
        ] {
            if !v.is_finite() || !(0.0..=100_000.0).contains(&v) {
                return Err(AppError::bad_request(format!("{name} must be between 0 and 100,000.")));
            }
        }
        Ok(())
    }

    /// Base XP for one batch of stat deltas, before multipliers.
    pub fn base_xp(&self, playtime_secs: i64, player_kills: i64, mob_kills: i64, broken: i64, placed: i64, messages: i64) -> f64 {
        let r = &self.xp_rates;
        playtime_secs as f64 / 3600.0 * r.playtime_per_hour
            + player_kills as f64 * r.player_kill
            + mob_kills as f64 * r.mob_kill
            + broken as f64 * r.block_broken
            + placed as f64 * r.block_placed
            + messages as f64 * r.message
    }
}

// ---------------------------------------------------------------------------
// Curves belong to the experience database. Every domain operation loads
// the curve from its own pool or existing transaction.
// ---------------------------------------------------------------------------

impl Progression {
    pub fn max_level(&self) -> i64 {
        if self.max_level == 0 {
            LEVEL_CEILING
        } else {
            self.max_level.min(LEVEL_CEILING)
        }
    }
    pub fn xp_for_level(&self, lvl: i64) -> i64 {
        if lvl <= 1 {
            0
        } else {
            (self.level_base * (lvl as f64).powf(self.level_exponent)).round() as i64
        }
    }
    pub fn level_from_xp(&self, xp: i64) -> (i64, i64, i64, f64) {
        let cap = self.max_level();
        let mut lvl = 1;
        if xp > 0 {
            let guess = ((xp as f64 / self.level_base).powf(1.0 / self.level_exponent)).floor() as i64;
            lvl = (guess - 2).clamp(1, cap);
            while lvl > 1 && xp < self.xp_for_level(lvl) {
                lvl -= 1;
            }
            while lvl < cap && xp >= self.xp_for_level(lvl + 1) {
                lvl += 1;
            }
        }
        let progress = (xp - self.xp_for_level(lvl)).max(0);
        if lvl >= cap {
            return (lvl, progress, progress.max(1), 100.0);
        }
        let span = (self.xp_for_level(lvl + 1) - self.xp_for_level(lvl)).max(1);
        (lvl, progress, span, (progress as f64 / span as f64 * 100.0).clamp(0.0, 100.0))
    }
}
// Pure default-curve helpers retained for callers and tests that have no experience.
pub fn xp_for_level(lvl: i64) -> i64 {
    Progression::default().xp_for_level(lvl)
}
pub fn level_from_xp(xp: i64) -> (i64, i64, i64, f64) {
    Progression::default().level_from_xp(xp)
}

pub async fn load(conn: &mut SqliteConnection) -> AppResult<Progression> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM kv WHERE key = 'progression'").fetch_optional(&mut *conn).await?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default())
}

pub async fn load_pool(pool: &sqlx::SqlitePool) -> AppResult<Progression> {
    let mut conn = pool.acquire().await?;
    load(&mut conn).await
}

pub async fn save(pool: &sqlx::SqlitePool, p: &Progression) -> AppResult<()> {
    sqlx::query("INSERT INTO kv (key, value) VALUES ('progression', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(serde_json::to_string(p)?)
        .execute(pool)
        .await?;
    // The same rules as plain rows, so SQL can use them (guild claim limits are worked out in queries).
    for (key, value) in [
        ("rule_claims_per_member", p.rules.guild_claims_per_member),
        ("rule_claims_per_level", p.rules.guild_claims_per_level),
        ("rule_guild_max_members", p.rules.guild_max_members),
    ] {
        sqlx::query("INSERT INTO kv (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
            .bind(key)
            .bind(value.to_string())
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Recompute every stored level from its XP after the curve changed.
pub async fn recalculate_levels(pool: &sqlx::SqlitePool) -> AppResult<usize> {
    let curve = load_pool(pool).await?;
    let mut tx = pool.begin().await?;
    let globals: Vec<(String, i64, i64)> =
        sqlx::query_as("SELECT uuid, global_xp, global_level FROM user_levels").fetch_all(&mut *tx).await?;
    let mut changed = 0;
    for (uuid, xp, level) in globals {
        let new = curve.level_from_xp(xp).0;
        if new != level {
            sqlx::query("UPDATE user_levels SET global_level = ? WHERE uuid = ?").bind(new).bind(&uuid).execute(&mut *tx).await?;
            changed += 1;
        }
    }
    let servers: Vec<(i64, String, i64, i64)> =
        sqlx::query_as("SELECT server_id, uuid, server_xp, server_level FROM server_levels").fetch_all(&mut *tx).await?;
    for (server_id, uuid, xp, level) in servers {
        let new = curve.level_from_xp(xp).0;
        if new != level {
            sqlx::query("UPDATE server_levels SET server_level = ? WHERE server_id = ? AND uuid = ?")
                .bind(new)
                .bind(server_id)
                .bind(&uuid)
                .execute(&mut *tx)
                .await?;
            changed += 1;
        }
    }
    tx.commit().await?;
    Ok(changed)
}

// ---------------------------------------------------------------------------
// Quest assignment
// ---------------------------------------------------------------------------

/// The quests one player is given for a period. `period` is `daily` or
/// `weekly` and `key` the current period key.
///
/// With a limit, the chosen set is stored the first time it is needed so it
/// stays put for the whole period, even if admins add or edit quests; it is
/// only topped up (raised limit, a disabled quest) or truncated (lowered
/// limit). Pinned quests are always chosen first.
pub async fn assigned_quest_ids(
    conn: &mut SqliteConnection,
    cfg: &Progression,
    uuid: &str,
    period: &str,
    key: &str,
) -> AppResult<Vec<String>> {
    let limit = if period == "weekly" { cfg.weekly_quest_limit } else { cfg.daily_quest_limit } as usize;
    // Scope is resolved against the player's current server presence. Global
    // quests remain available everywhere; instance and server overrides only
    // enter the rotation while the player is online in that context.
    let eligible: Vec<(String, bool)> = sqlx::query_as(
        "SELECT q.id, q.pinned FROM quests q
         WHERE q.enabled = 1 AND q.period = ?
           AND (q.instance_id IS NULL OR EXISTS (
             SELECT 1 FROM server_online so JOIN game_servers gs ON gs.id = so.server_id
             WHERE so.uuid = ? AND gs.instance_id = q.instance_id
           ))
           AND (q.server_id IS NULL OR EXISTS (
             SELECT 1 FROM server_online so
             WHERE so.uuid = ? AND so.server_id = q.server_id
           ))
         ORDER BY q.id",
    )
    .bind(period)
    .bind(uuid)
    .bind(uuid)
    .fetch_all(&mut *conn)
    .await?;
    if limit == 0 || eligible.len() <= limit {
        return Ok(eligible.into_iter().map(|(id, _)| id).collect());
    }

    let mut current: Vec<String> = sqlx::query_scalar(
        "SELECT qa.quest_id FROM quest_assignments qa JOIN quests q ON q.id = qa.quest_id
         WHERE qa.user_uuid = ? AND qa.period_key = ? AND q.enabled = 1 AND q.period = ? ORDER BY qa.position",
    )
    .bind(uuid)
    .bind(key)
    .bind(period)
    .fetch_all(&mut *conn)
    .await?;

    if current.len() < limit {
        let seed = if cfg.quest_rotation == "shared" { format!("{period}|{key}") } else { format!("{period}|{key}|{uuid}") };
        let mut ranked: Vec<(bool, String, String)> = eligible
            .into_iter()
            .filter(|(id, _)| !current.contains(id))
            .map(|(id, pinned)| (!pinned, hex::encode(Sha256::digest(format!("{seed}|{id}").as_bytes())), id))
            .collect();
        ranked.sort();
        let mut position: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(position), -1) + 1 FROM quest_assignments WHERE user_uuid = ? AND period_key = ?")
                .bind(uuid)
                .bind(key)
                .fetch_one(&mut *conn)
                .await?;
        for (_, _, id) in ranked.into_iter().take(limit - current.len()) {
            sqlx::query(
                "INSERT OR REPLACE INTO quest_assignments (user_uuid, period, period_key, quest_id, position) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(uuid)
            .bind(period)
            .bind(key)
            .bind(&id)
            .bind(position)
            .execute(&mut *conn)
            .await?;
            position += 1;
            current.push(id);
        }
    }
    current.truncate(limit);
    Ok(current)
}

pub async fn is_assigned(
    conn: &mut SqliteConnection,
    cfg: &Progression,
    uuid: &str,
    period: &str,
    key: &str,
    quest_id: &str,
) -> AppResult<bool> {
    Ok(assigned_quest_ids(conn, cfg, uuid, period, key).await?.iter().any(|id| id == quest_id))
}

// ---------------------------------------------------------------------------
// Manual XP changes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Add,
    Remove,
    SetXp,
    SetLevel,
    Reset,
}

pub struct XpChange {
    pub old_xp: i64,
    pub new_xp: i64,
    pub old_level: i64,
    pub new_level: i64,
}

pub fn target_xp(mode: Mode, current: i64, amount: i64) -> AppResult<i64> {
    target_xp_with(&Progression::default(), mode, current, amount)
}

fn target_xp_with(curve: &Progression, mode: Mode, current: i64, amount: i64) -> AppResult<i64> {
    const MAX_XP: i64 = 2_000_000_000;
    if !(0..=MAX_XP).contains(&amount) {
        return Err(AppError::bad_request("Amount must be between 0 and 2,000,000,000."));
    }
    Ok(match mode {
        Mode::Add => (current + amount).min(MAX_XP),
        Mode::Remove => (current - amount).max(0),
        Mode::SetXp => amount,
        Mode::SetLevel => {
            if amount < 1 || amount > curve.max_level() {
                return Err(AppError::bad_request(format!("Level must be between 1 and {}.", curve.max_level())));
            }
            curve.xp_for_level(amount)
        }
        Mode::Reset => 0,
    })
}

pub async fn adjust_global(conn: &mut SqliteConnection, uuid: &str, mode: Mode, amount: i64, now: &str) -> AppResult<XpChange> {
    let old_xp: i64 =
        sqlx::query_scalar("SELECT global_xp FROM user_levels WHERE uuid = ?").bind(uuid).fetch_optional(&mut *conn).await?.unwrap_or(0);
    let curve = load(conn).await?;
    let new_xp = target_xp_with(&curve, mode, old_xp, amount)?;
    let (old_level, new_level) = (curve.level_from_xp(old_xp).0, curve.level_from_xp(new_xp).0);
    sqlx::query(
        "INSERT INTO user_levels (uuid, global_xp, global_level, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(uuid) DO UPDATE SET global_xp = excluded.global_xp, global_level = excluded.global_level, updated_at = excluded.updated_at",
    )
    .bind(uuid)
    .bind(new_xp)
    .bind(new_level)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    Ok(XpChange { old_xp, new_xp, old_level, new_level })
}

pub async fn adjust_server(
    conn: &mut SqliteConnection,
    server_id: i64,
    uuid: &str,
    mode: Mode,
    amount: i64,
    now: &str,
) -> AppResult<XpChange> {
    let old_xp: i64 = sqlx::query_scalar("SELECT server_xp FROM server_levels WHERE server_id = ? AND uuid = ?")
        .bind(server_id)
        .bind(uuid)
        .fetch_optional(&mut *conn)
        .await?
        .unwrap_or(0);
    let curve = load(conn).await?;
    let new_xp = target_xp_with(&curve, mode, old_xp, amount)?;
    let (old_level, new_level) = (curve.level_from_xp(old_xp).0, curve.level_from_xp(new_xp).0);
    sqlx::query(
        "INSERT INTO server_levels (server_id, uuid, server_xp, server_level, updated_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(server_id, uuid) DO UPDATE SET server_xp = excluded.server_xp, server_level = excluded.server_level, updated_at = excluded.updated_at",
    )
    .bind(server_id)
    .bind(uuid)
    .bind(new_xp)
    .bind(new_level)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    Ok(XpChange { old_xp, new_xp, old_level, new_level })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_original_curve_and_rates() {
        let p = Progression::default();
        assert_eq!(xp_for_level(1), 0);
        assert_eq!(xp_for_level(2), 283);
        assert_eq!(xp_for_level(10), 3162);
        // 10 minutes online, 2 mob kills, 10 blocks broken: 100 + 30 + 2.
        assert_eq!(p.base_xp(600, 0, 2, 10, 0, 0).round() as i64, 132);
        p.validate().unwrap();
    }

    #[test]
    fn levels_follow_xp_both_ways() {
        for xp in [0, 1, 282, 283, 284, 3161, 3162, 1_000_000, 50_000_000] {
            let (lvl, into, span, pct) = level_from_xp(xp);
            assert!(xp_for_level(lvl) <= xp.max(0) && (xp < xp_for_level(lvl + 1) || lvl >= LEVEL_CEILING), "xp {xp} -> level {lvl}");
            assert_eq!(into, xp - xp_for_level(lvl));
            assert!(span >= 1 && (0.0..=100.0).contains(&pct));
        }
    }

    #[test]
    fn modes_compute_targets() {
        assert_eq!(target_xp(Mode::Add, 100, 50).unwrap(), 150);
        assert_eq!(target_xp(Mode::Remove, 100, 500).unwrap(), 0);
        assert_eq!(target_xp(Mode::SetXp, 100, 7).unwrap(), 7);
        assert_eq!(target_xp(Mode::SetLevel, 100, 10).unwrap(), 3162);
        assert_eq!(target_xp(Mode::Reset, 100, 0).unwrap(), 0);
        assert!(target_xp(Mode::Add, 0, -1).is_err());
        assert!(target_xp(Mode::SetLevel, 0, 0).is_err());
    }

    #[test]
    fn rejects_bad_settings() {
        let ok = Progression::default();
        for bad in [
            Progression { daily_quest_limit: 101, ..ok.clone() },
            Progression { quest_rotation: "x".into(), ..ok.clone() },
            Progression { level_exponent: 0.5, ..ok.clone() },
            Progression { level_base: f64::NAN, ..ok.clone() },
            Progression { max_level: 1, ..ok.clone() },
            Progression { global_xp_multiplier: -1.0, ..ok.clone() },
        ] {
            assert!(bad.validate().is_err());
        }
    }
}
