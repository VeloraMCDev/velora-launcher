//! Removing an account removes everything the panel knows about its player.
//!
//! Rows that belong to the player are deleted. Rows that belong to *other*
//! people but mention the player (the other side of an economy transfer, who
//! performed a guild-bank deposit) are kept with the player anonymised.

use crate::auth::UserRow;
use crate::error::AppResult;
use crate::state::AppState;
use serde::Serialize;
use sqlx::SqliteConnection;

/// Stand-in UUID for deleted players in other people's records.
pub const DELETED_UUID: &str = "00000000-0000-0000-0000-000000000000";
pub const DELETED_NAME: &str = "Deleted player";

#[derive(Debug, Default, Serialize)]
pub struct PurgeReport {
    /// Rows deleted, by area.
    pub removed: std::collections::BTreeMap<&'static str, u64>,
    /// Guilds dissolved because the player was their only member.
    pub guilds_dissolved: u64,
    /// Guilds whose leadership was handed to someone else.
    pub guilds_transferred: u64,
    pub skin_file_removed: bool,
}

impl PurgeReport {
    fn add(&mut self, area: &'static str, n: u64) {
        if n > 0 {
            *self.removed.entry(area).or_default() += n;
        }
    }
    pub fn total(&self) -> u64 {
        self.removed.values().sum()
    }
}

async fn run(conn: &mut SqliteConnection, sql: &str, binds: &[&str]) -> AppResult<u64> {
    // Shared identity/social rows are purged once through the platform, never
    // through an experience's read-only identity views.
    if crate::experience::PLATFORM_TABLES
        .iter()
        .any(|table| sql.contains(&format!("DELETE FROM {table} ")) || sql.contains(&format!("UPDATE {table} ")))
    {
        let scoped: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pragma_database_list WHERE name='platform')").fetch_one(&mut *conn).await?;
        if scoped {
            return Ok(0);
        }
    }
    let mut q = sqlx::query(sql);
    for b in binds {
        q = q.bind(*b);
    }
    Ok(q.execute(&mut *conn).await?.rows_affected())
}

/// Everything keyed by a player UUID (progress, stats, social, guild membership, economy). Shared by account removal and the
/// scheduled clean-up of players whose account was removed before this existed. The caller owns the transaction.
pub async fn purge_player_data(c: &mut SqliteConnection, uuid: &str, name: &str, report: &mut PurgeReport) -> AppResult<()> {
    // ---- progression ----
    report.add("levels", run(c, "DELETE FROM user_levels WHERE uuid = ?", &[uuid]).await?);
    report.add("levels", run(c, "DELETE FROM server_levels WHERE uuid = ?", &[uuid]).await?);
    report.add("rewards", run(c, "DELETE FROM granted_rewards WHERE uuid = ?", &[uuid]).await?);
    report.add("quests", run(c, "DELETE FROM user_quests WHERE user_uuid = ?", &[uuid]).await?);
    report.add("quests", run(c, "DELETE FROM quest_assignments WHERE user_uuid = ?", &[uuid]).await?);
    report.add("achievements", run(c, "DELETE FROM user_achievements WHERE user_uuid = ?", &[uuid]).await?);
    report.add("rewards", run(c, "DELETE FROM reward_queue WHERE uuid = ?", &[uuid]).await?);

    // ---- gameplay records ----
    report.add("stats", run(c, "DELETE FROM player_stats WHERE uuid = ?", &[uuid]).await?);
    report.add("stats", run(c, "DELETE FROM server_online WHERE uuid = ?", &[uuid]).await?);
    report.add("ranks", run(c, "DELETE FROM player_ranks WHERE uuid = ?", &[uuid]).await?);
    report.add("notifications", run(c, "DELETE FROM server_notifications WHERE uuid = ?", &[uuid]).await?);
    report.add("activity", run(c, "DELETE FROM server_events WHERE uuid = ?", &[uuid]).await?);
    report.add(
        "activity",
        run(c, "DELETE FROM events WHERE uuid = ? OR (uuid IS NULL AND username = ? COLLATE NOCASE)", &[uuid, name]).await?,
    );

    // ---- social ----
    report.add("friends", run(c, "DELETE FROM friendships WHERE user_uuid = ? OR friend_uuid = ?", &[uuid, uuid]).await?);
    report.add("messages", run(c, "DELETE FROM direct_messages WHERE sender_uuid = ? OR recipient_uuid = ?", &[uuid, uuid]).await?);
    report.add("invites", run(c, "DELETE FROM game_invites WHERE sender_uuid = ? OR recipient_uuid = ?", &[uuid, uuid]).await?);
    report.add("invites", run(c, "DELETE FROM guild_invites WHERE inviter_uuid = ? OR target_uuid = ?", &[uuid, uuid]).await?);
    report.add("actions", run(c, "DELETE FROM server_actions WHERE from_uuid = ? OR to_uuid = ?", &[uuid, uuid]).await?);
    report.add("profile", run(c, "DELETE FROM user_profiles WHERE uuid = ?", &[uuid]).await?);
    // Likes they gave come off other people's post counters.
    run(
        c,
        "UPDATE user_posts SET likes_count = MAX(0, likes_count - 1) WHERE id IN (SELECT post_id FROM user_post_likes WHERE user_uuid = ?)",
        &[uuid],
    )
    .await?;
    report.add("posts", run(c, "DELETE FROM user_post_likes WHERE user_uuid = ?", &[uuid]).await?);
    report.add("posts", run(c, "DELETE FROM user_posts WHERE user_uuid = ?", &[uuid]).await?);

    // ---- guilds ----
    let led: Vec<String> = sqlx::query_scalar("SELECT id FROM guilds WHERE leader_uuid = ?").bind(uuid).fetch_all(&mut *c).await?;
    for guild in led {
        // Next in line: an officer, then whoever has been a member longest.
        let heir: Option<(String, String)> = sqlx::query_as(
            "SELECT uuid, name FROM guild_members WHERE guild_id = ? AND uuid <> ?
             ORDER BY (role = 'officer') DESC, joined_at ASC LIMIT 1",
        )
        .bind(&guild)
        .bind(uuid)
        .fetch_optional(&mut *c)
        .await?;
        match heir {
            Some((heir_uuid, _)) => {
                run(c, "UPDATE guilds SET leader_uuid = ? WHERE id = ?", &[&heir_uuid, &guild]).await?;
                run(c, "UPDATE guild_members SET role = 'leader' WHERE guild_id = ? AND uuid = ?", &[&guild, &heir_uuid]).await?;
                report.guilds_transferred += 1;
            }
            None => {
                // Claims, wallet, posts and roles go with the guild.
                report.add("guild_data", run(c, "DELETE FROM guilds WHERE id = ?", &[&guild]).await?);
                report.guilds_dissolved += 1;
            }
        }
    }
    report.add("guild_membership", run(c, "DELETE FROM guild_members WHERE uuid = ?", &[uuid]).await?);
    report.add("guild_membership", run(c, "DELETE FROM guild_primary_memberships WHERE uuid = ?", &[uuid]).await?);
    report.add("guild_posts", run(c, "DELETE FROM guild_posts WHERE author_uuid = ?", &[uuid]).await?);
    report.add("events", run(c, "DELETE FROM community_event_participants WHERE uuid = ?", &[uuid]).await?);
    report.add("collections", run(c, "DELETE FROM player_unlocks WHERE uuid = ?", &[uuid]).await?);
    // Territory they claimed stays with the guild.
    run(c, "UPDATE guild_claims SET claimed_by_uuid = ? WHERE claimed_by_uuid = ?", &[DELETED_UUID, uuid]).await?;
    run(c, "UPDATE guild_wallet_transactions SET actor_uuid = ? WHERE actor_uuid = ?", &[DELETED_UUID, uuid]).await?;

    // ---- economy ----
    report.add("economy", run(c, "DELETE FROM server_economy WHERE uuid = ?", &[uuid]).await?);
    // Items a guild had listed stay for the guild; only the lister is anonymised.
    run(
        c,
        "UPDATE server_market SET seller_uuid = ?, seller_name = ? WHERE seller_uuid = ? AND seller_guild_id IS NOT NULL",
        &[DELETED_UUID, DELETED_NAME, uuid],
    )
    .await?;
    report.add("economy", run(c, "DELETE FROM server_market WHERE seller_uuid = ?", &[uuid]).await?);
    report.add("economy", run(c, "DELETE FROM market_mailbox WHERE uuid = ?", &[uuid]).await?);
    // Auctions they led go back to the start; the money they held is gone with them.
    run(
        c,
        "UPDATE server_market SET current_bid = NULL, bidder_uuid = NULL, bidder_name = NULL, bid_count = 0 WHERE bidder_uuid = ?",
        &[uuid],
    )
    .await?;
    run(c, "DELETE FROM user_notifications WHERE uuid = ?", &[uuid]).await?;
    run(c, "UPDATE economy_transactions SET from_uuid = ?, from_name = ? WHERE from_uuid = ?", &[DELETED_UUID, DELETED_NAME, uuid]).await?;
    run(c, "UPDATE economy_transactions SET to_uuid = ?, to_name = ? WHERE to_uuid = ?", &[DELETED_UUID, DELETED_NAME, uuid]).await?;
    Ok(())
}

/// Clean every experience before deleting the shared account. Each store commits
/// independently; a failed cleanup leaves identity intact and can be retried.
pub async fn purge_user(state: &AppState, user: &UserRow) -> AppResult<PurgeReport> {
    let uuid = user.uuid.as_str();
    let name = user.username.as_str();
    let mut report = PurgeReport::default();
    if state.instance_id.is_none() && state.cfg.data_dir.join("panel.db").is_file() {
        for instance in crate::store::list_instances(state).await? {
            let scoped = state.experiences.state(state, &instance.id).await?;
            let mut tx = scoped.db.begin().await?;
            purge_player_data(&mut tx, uuid, name, &mut report).await?;
            tx.commit().await?;
            scoped.worldmap.forget_player(uuid);
        }
    }
    let mut tx = state.db.begin().await?;
    let c = &mut *tx;

    purge_player_data(c, uuid, name, &mut report).await?;

    // ---- identity ----
    // `reserved_usernames` is kept on purpose: a deleted player's name stays
    // taken so nobody can impersonate them. It holds only the name and UUID.
    // Explicit, in case a foreign key is ever relaxed.
    let id = user.id.to_string();
    for (area, sql) in [
        ("sessions", "DELETE FROM ygg_tokens WHERE user_id = ?"),
        ("sessions", "DELETE FROM ygg_sessions WHERE user_id = ?"),
        ("sessions", "DELETE FROM player_keys WHERE user_id = ?"),
        ("sessions", "DELETE FROM launcher_sessions WHERE user_id = ?"),
        ("connections", "DELETE FROM account_connections WHERE user_id = ?"),
        ("connections", "DELETE FROM oauth_attempts WHERE user_id = ?"),
        ("connections", "DELETE FROM password_resets WHERE user_id = ?"),
        ("groups", "DELETE FROM user_groups WHERE user_id = ?"),
    ] {
        report.add(area, run(c, sql, &[&id]).await?);
    }
    report.add("account", run(c, "DELETE FROM users WHERE id = ?", &[&id]).await?);

    // A skin nobody else uses is deleted from disk.
    let skin = user.skin_hash.clone();
    let skin_shared: bool = match &skin {
        Some(h) => sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE skin_hash = ?)").bind(h).fetch_one(&mut *c).await?,
        None => true,
    };
    tx.commit().await?;

    if let (Some(hash), false) = (skin, skin_shared) {
        if let Some(path) = crate::textures::path(&state.cfg.textures_dir(), &hash) {
            report.skin_file_removed = tokio::fs::remove_file(path).await.is_ok();
        }
    }
    // Live map: stop showing them.
    state.worldmap.forget_player(uuid);
    Ok(report)
}
