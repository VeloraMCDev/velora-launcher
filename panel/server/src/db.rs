use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

/// Schema migrations, applied in order. Never edit a released entry —
/// append a new one instead.
/// Schema version a fully migrated store reports through `PRAGMA user_version`.
pub fn schema_version() -> i64 {
    MIGRATIONS.len() as i64
}

const MIGRATIONS: &[&str] = &[
    // 1: initial schema
    r#"
    CREATE TABLE users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL UNIQUE COLLATE NOCASE,
        password_hash TEXT NOT NULL,
        email TEXT,
        role TEXT NOT NULL DEFAULT 'player',
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL,
        last_login TEXT
    );
    CREATE TABLE groups (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE COLLATE NOCASE,
        color TEXT NOT NULL DEFAULT '#7c5cff'
    );
    CREATE TABLE user_groups (
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
        PRIMARY KEY (user_id, group_id)
    );
    CREATE TABLE instances (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        icon_url TEXT,
        banner_url TEXT,
        mc_version TEXT NOT NULL,
        loader TEXT NOT NULL DEFAULT 'vanilla',
        loader_version TEXT,
        source_kind TEXT NOT NULL DEFAULT 'vanilla',
        source_label TEXT NOT NULL DEFAULT '',
        source_ref TEXT NOT NULL DEFAULT '{}',
        visibility TEXT NOT NULL DEFAULT 'public',
        allowed_groups TEXT NOT NULL DEFAULT '[]',
        memory_min INTEGER NOT NULL DEFAULT 1024,
        memory_max INTEGER NOT NULL DEFAULT 4096,
        jvm_args TEXT NOT NULL DEFAULT '',
        server TEXT,
        featured INTEGER NOT NULL DEFAULT 0,
        enabled INTEGER NOT NULL DEFAULT 1,
        sort INTEGER NOT NULL DEFAULT 0,
        revision INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE TABLE instance_files (
        instance_id TEXT NOT NULL REFERENCES instances(id) ON DELETE CASCADE,
        path TEXT NOT NULL,
        url TEXT NOT NULL,
        sha1 TEXT NOT NULL,
        size INTEGER NOT NULL,
        origin TEXT NOT NULL,
        note TEXT,
        PRIMARY KEY (instance_id, path)
    );
    CREATE TABLE kv (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        instance_id TEXT,
        username TEXT,
        kind TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX events_created ON events(created_at);
    "#,
    // 2: Yggdrasil auth server (UUIDs, skins, capes, sessions) and game
    //    server integration (plugin/mod tracking)
    r#"
    ALTER TABLE users ADD COLUMN uuid TEXT NOT NULL DEFAULT '';
    ALTER TABLE users ADD COLUMN skin_hash TEXT;
    ALTER TABLE users ADD COLUMN skin_model TEXT NOT NULL DEFAULT 'classic';
    ALTER TABLE users ADD COLUMN cape_id INTEGER;
    ALTER TABLE users ADD COLUMN status_reason TEXT;

    CREATE TABLE capes (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        hash TEXT NOT NULL,
        visibility TEXT NOT NULL DEFAULT 'public',
        allowed_groups TEXT NOT NULL DEFAULT '[]',
        created_at TEXT NOT NULL
    );
    CREATE TABLE ygg_tokens (
        access_token TEXT PRIMARY KEY,
        client_token TEXT NOT NULL,
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL
    );
    CREATE INDEX ygg_tokens_user ON ygg_tokens(user_id);
    CREATE TABLE ygg_sessions (
        server_id TEXT PRIMARY KEY,
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        ip TEXT,
        created_at TEXT NOT NULL
    );
    CREATE TABLE player_keys (
        user_id INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
        private_pem TEXT NOT NULL,
        public_pem TEXT NOT NULL,
        signature_v1 TEXT NOT NULL,
        signature_v2 TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        refreshed_after TEXT NOT NULL
    );
    CREATE TABLE launcher_sessions (
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        ip TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX launcher_sessions_user ON launcher_sessions(user_id, created_at);

    CREATE TABLE game_servers (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        token_hash TEXT NOT NULL UNIQUE,
        token_hint TEXT NOT NULL,
        access TEXT NOT NULL DEFAULT 'all',
        allowed_groups TEXT NOT NULL DEFAULT '[]',
        require_launcher INTEGER NOT NULL DEFAULT 0,
        software TEXT,
        mc_version TEXT,
        plugin_version TEXT,
        online_mode INTEGER,
        max_players INTEGER NOT NULL DEFAULT 0,
        online_count INTEGER NOT NULL DEFAULT 0,
        tps REAL,
        last_seen TEXT,
        created_at TEXT NOT NULL
    );
    CREATE TABLE server_online (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        joined_at TEXT NOT NULL,
        PRIMARY KEY (server_id, uuid)
    );
    CREATE TABLE player_stats (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        playtime_secs INTEGER NOT NULL DEFAULT 0,
        joins INTEGER NOT NULL DEFAULT 0,
        deaths INTEGER NOT NULL DEFAULT 0,
        player_kills INTEGER NOT NULL DEFAULT 0,
        mob_kills INTEGER NOT NULL DEFAULT 0,
        blocks_broken INTEGER NOT NULL DEFAULT 0,
        blocks_placed INTEGER NOT NULL DEFAULT 0,
        messages INTEGER NOT NULL DEFAULT 0,
        first_seen TEXT NOT NULL,
        last_seen TEXT NOT NULL,
        PRIMARY KEY (server_id, uuid)
    );
    CREATE INDEX player_stats_uuid ON player_stats(uuid);
    CREATE TABLE server_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT,
        name TEXT,
        kind TEXT NOT NULL,
        detail TEXT,
        created_at TEXT NOT NULL
    );
    CREATE INDEX server_events_server ON server_events(server_id, id);
    CREATE INDEX server_events_uuid ON server_events(uuid, id);
    "#,
    // 3: idempotent server stat batches (retained until the server is deleted).
    r#"
    CREATE TABLE server_sync_receipts (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        batch_id TEXT NOT NULL,
        created_at TEXT NOT NULL,
        PRIMARY KEY (server_id, batch_id)
    );
    "#,
    // 4: permanent player identity, reserved historical names and audit data.
    r#"
    ALTER TABLE users ADD COLUMN auth_version INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE events ADD COLUMN uuid TEXT;
    ALTER TABLE events ADD COLUMN detail TEXT;
    ALTER TABLE events ADD COLUMN source TEXT NOT NULL DEFAULT 'launcher';
    CREATE INDEX events_uuid ON events(uuid, id);
    CREATE TABLE reserved_usernames (
        name TEXT PRIMARY KEY COLLATE NOCASE,
        uuid TEXT NOT NULL
    );
    INSERT INTO reserved_usernames SELECT username, uuid FROM users WHERE uuid <> '';
    CREATE TRIGGER immutable_player_uuid BEFORE UPDATE OF uuid ON users
    WHEN OLD.uuid <> '' AND NEW.uuid <> OLD.uuid
    BEGIN SELECT RAISE(ABORT, 'player UUID is immutable'); END;
    CREATE TRIGGER reserve_name_insert BEFORE INSERT ON users
    WHEN EXISTS(SELECT 1 FROM reserved_usernames WHERE name = NEW.username AND uuid <> NEW.uuid)
    BEGIN SELECT RAISE(ABORT, 'username reserved'); END;
    CREATE TRIGGER reserve_name_update BEFORE UPDATE OF username ON users
    WHEN EXISTS(SELECT 1 FROM reserved_usernames WHERE name = NEW.username AND uuid <> NEW.uuid)
    BEGIN SELECT RAISE(ABORT, 'username reserved'); END;
    CREATE TRIGGER remember_name_insert AFTER INSERT ON users
    WHEN NEW.uuid <> ''
    BEGIN INSERT OR IGNORE INTO reserved_usernames VALUES (NEW.username, NEW.uuid); END;
    CREATE TRIGGER remember_name_update AFTER UPDATE OF username, uuid ON users
    WHEN NEW.uuid <> ''
    BEGIN INSERT OR IGNORE INTO reserved_usernames VALUES (NEW.username, NEW.uuid); END;
    "#,
    // 5: instance logo url and flexible launcher sessions.
    r#"
    ALTER TABLE instances ADD COLUMN logo_url TEXT;
    CREATE TABLE launcher_sessions_new (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
        ip TEXT NOT NULL,
        username TEXT,
        created_at TEXT NOT NULL
    );
    INSERT INTO launcher_sessions_new (user_id, ip, created_at)
        SELECT user_id, ip, created_at FROM launcher_sessions;
    DROP TABLE launcher_sessions;
    ALTER TABLE launcher_sessions_new RENAME TO launcher_sessions;
    CREATE INDEX launcher_sessions_user ON launcher_sessions(user_id, created_at);
    CREATE INDEX launcher_sessions_ip ON launcher_sessions(ip, created_at);
    "#,
    // 6: Leveling, Quests, Achievements, Guilds, Land Claims, Friends, Profiles & Social Posts
    r#"
    CREATE TABLE user_levels (
        uuid TEXT PRIMARY KEY,
        global_xp INTEGER NOT NULL DEFAULT 0,
        global_level INTEGER NOT NULL DEFAULT 1,
        title TEXT,
        badges TEXT NOT NULL DEFAULT '[]',
        updated_at TEXT NOT NULL
    );

    CREATE TABLE server_levels (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        server_xp INTEGER NOT NULL DEFAULT 0,
        server_level INTEGER NOT NULL DEFAULT 1,
        rank_name TEXT,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (server_id, uuid)
    );

    CREATE TABLE level_rewards (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        level_type TEXT NOT NULL,
        server_id INTEGER REFERENCES game_servers(id) ON DELETE CASCADE,
        level_req INTEGER NOT NULL,
        reward_type TEXT NOT NULL,
        reward_name TEXT NOT NULL,
        reward_data TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL
    );

    CREATE TABLE quests (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        description TEXT NOT NULL,
        period TEXT NOT NULL,
        category TEXT NOT NULL,
        target_stat TEXT NOT NULL,
        target_count INTEGER NOT NULL,
        xp_reward INTEGER NOT NULL,
        icon TEXT NOT NULL DEFAULT 'star',
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL
    );

    CREATE TABLE user_quests (
        user_uuid TEXT NOT NULL,
        quest_id TEXT NOT NULL REFERENCES quests(id) ON DELETE CASCADE,
        period_key TEXT NOT NULL,
        progress INTEGER NOT NULL DEFAULT 0,
        completed INTEGER NOT NULL DEFAULT 0,
        claimed INTEGER NOT NULL DEFAULT 0,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (user_uuid, quest_id, period_key)
    );
    CREATE INDEX user_quests_uuid ON user_quests(user_uuid, period_key);

    CREATE TABLE achievements (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        description TEXT NOT NULL,
        category TEXT NOT NULL DEFAULT 'general',
        icon_frame TEXT NOT NULL DEFAULT 'task',
        icon_item TEXT NOT NULL DEFAULT 'diamond',
        icon_bg TEXT NOT NULL DEFAULT 'stone',
        icon_border TEXT NOT NULL DEFAULT 'gold',
        requirement_type TEXT NOT NULL DEFAULT 'stat',
        requirement_key TEXT NOT NULL,
        requirement_value INTEGER NOT NULL DEFAULT 1,
        xp_reward INTEGER NOT NULL DEFAULT 100,
        secret INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );

    CREATE TABLE user_achievements (
        user_uuid TEXT NOT NULL,
        achievement_id TEXT NOT NULL REFERENCES achievements(id) ON DELETE CASCADE,
        unlocked_at TEXT NOT NULL,
        PRIMARY KEY (user_uuid, achievement_id)
    );
    CREATE INDEX user_achievements_uuid ON user_achievements(user_uuid);

    CREATE TABLE guilds (
        id TEXT PRIMARY KEY,
        instance_id TEXT NOT NULL,
        name TEXT NOT NULL UNIQUE COLLATE NOCASE,
        tag TEXT NOT NULL COLLATE NOCASE,
        description TEXT NOT NULL DEFAULT '',
        motd TEXT NOT NULL DEFAULT '',
        leader_uuid TEXT NOT NULL,
        icon_url TEXT,
        banner_url TEXT,
        level INTEGER NOT NULL DEFAULT 1,
        xp INTEGER NOT NULL DEFAULT 0,
        max_claims INTEGER NOT NULL DEFAULT 16,
        created_at TEXT NOT NULL
    );
    CREATE INDEX guilds_instance ON guilds(instance_id);

    CREATE TABLE guild_members (
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        role TEXT NOT NULL DEFAULT 'member',
        joined_at TEXT NOT NULL,
        PRIMARY KEY (guild_id, uuid)
    );
    CREATE UNIQUE INDEX guild_member_unique ON guild_members(uuid);

    CREATE TABLE guild_roles (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        priority INTEGER NOT NULL DEFAULT 0,
        can_invite INTEGER NOT NULL DEFAULT 0,
        can_kick INTEGER NOT NULL DEFAULT 0,
        can_claim INTEGER NOT NULL DEFAULT 0,
        can_post INTEGER NOT NULL DEFAULT 1,
        can_manage INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE guild_posts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        author_uuid TEXT NOT NULL,
        author_name TEXT NOT NULL,
        title TEXT NOT NULL,
        content TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX guild_posts_guild ON guild_posts(guild_id, id DESC);

    CREATE TABLE guild_claims (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        server_id INTEGER REFERENCES game_servers(id) ON DELETE CASCADE,
        dimension TEXT NOT NULL DEFAULT 'minecraft:overworld',
        chunk_x INTEGER NOT NULL,
        chunk_z INTEGER NOT NULL,
        claimed_by_uuid TEXT NOT NULL,
        claimed_at TEXT NOT NULL,
        UNIQUE (server_id, dimension, chunk_x, chunk_z)
    );
    CREATE INDEX guild_claims_guild ON guild_claims(guild_id);
    CREATE INDEX guild_claims_loc ON guild_claims(server_id, dimension, chunk_x, chunk_z);

    CREATE TABLE friendships (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_uuid TEXT NOT NULL,
        friend_uuid TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        action_uuid TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        UNIQUE (user_uuid, friend_uuid)
    );
    CREATE INDEX friendships_user ON friendships(user_uuid);
    CREATE INDEX friendships_friend ON friendships(friend_uuid);

    CREATE TABLE direct_messages (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sender_uuid TEXT NOT NULL,
        sender_name TEXT NOT NULL,
        recipient_uuid TEXT NOT NULL,
        content TEXT NOT NULL,
        is_read INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );
    CREATE INDEX dm_conversation ON direct_messages(sender_uuid, recipient_uuid, id);
    CREATE INDEX dm_recipient ON direct_messages(recipient_uuid, is_read);

    CREATE TABLE game_invites (
        id TEXT PRIMARY KEY,
        sender_uuid TEXT NOT NULL,
        sender_name TEXT NOT NULL,
        recipient_uuid TEXT NOT NULL,
        instance_id TEXT NOT NULL,
        server_id INTEGER REFERENCES game_servers(id) ON DELETE CASCADE,
        status TEXT NOT NULL DEFAULT 'pending',
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL
    );
    CREATE INDEX game_invites_recipient ON game_invites(recipient_uuid, status);

    CREATE TABLE user_profiles (
        uuid TEXT PRIMARY KEY,
        bio TEXT NOT NULL DEFAULT '',
        banner_url TEXT,
        custom_badge TEXT,
        featured_achievement_id TEXT,
        social_links TEXT NOT NULL DEFAULT '{}',
        updated_at TEXT NOT NULL
    );

    CREATE TABLE user_posts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_uuid TEXT NOT NULL,
        author_name TEXT NOT NULL,
        content TEXT NOT NULL,
        image_url TEXT,
        likes_count INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );
    CREATE INDEX user_posts_user ON user_posts(user_uuid, id DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS server_economy (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        username TEXT NOT NULL,
        balance REAL NOT NULL DEFAULT 1000.0,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (server_id, uuid)
    );
    CREATE INDEX IF NOT EXISTS economy_user ON server_economy(uuid);
    CREATE INDEX IF NOT EXISTS economy_bal ON server_economy(server_id, balance DESC);

    CREATE TABLE IF NOT EXISTS economy_transactions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        from_uuid TEXT NOT NULL,
        from_name TEXT NOT NULL,
        to_uuid TEXT NOT NULL,
        to_name TEXT NOT NULL,
        amount REAL NOT NULL,
        description TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS econ_tx_user ON economy_transactions(server_id, from_uuid, to_uuid);

    CREATE TABLE IF NOT EXISTS server_market (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        seller_uuid TEXT NOT NULL,
        seller_name TEXT NOT NULL,
        item_id TEXT NOT NULL,
        item_name TEXT NOT NULL,
        amount INTEGER NOT NULL DEFAULT 1,
        price REAL NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS market_srv ON server_market(server_id, id DESC);
    "#,
    r#"
    ALTER TABLE game_servers ADD COLUMN instance_id TEXT NOT NULL DEFAULT '';
    DROP INDEX guild_member_unique;
    CREATE INDEX guild_member_uuid ON guild_members(uuid);
    CREATE TRIGGER guild_members_instance_unique BEFORE INSERT ON guild_members
    WHEN EXISTS (SELECT 1 FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id
      WHERE gm.uuid = NEW.uuid AND gm.guild_id <> NEW.guild_id
      AND g.instance_id = (SELECT instance_id FROM guilds WHERE id = NEW.guild_id))
    BEGIN SELECT RAISE(ABORT, 'Already in a guild for this instance'); END;
    CREATE TRIGGER achievement_xp AFTER INSERT ON user_achievements BEGIN
      INSERT INTO user_levels(uuid, global_xp, updated_at)
      VALUES(NEW.user_uuid, (SELECT MAX(0, xp_reward) FROM achievements WHERE id = NEW.achievement_id), NEW.unlocked_at)
      ON CONFLICT(uuid) DO UPDATE SET global_xp = global_xp + excluded.global_xp, updated_at = excluded.updated_at;
    END;
    INSERT INTO user_levels(uuid, global_xp, updated_at)
      SELECT ua.user_uuid, SUM(MAX(0, a.xp_reward)), MAX(ua.unlocked_at) FROM user_achievements ua JOIN achievements a ON a.id = ua.achievement_id GROUP BY ua.user_uuid
      ON CONFLICT(uuid) DO UPDATE SET global_xp = global_xp + excluded.global_xp;
    CREATE TABLE granted_rewards (uuid TEXT NOT NULL, reward_id INTEGER NOT NULL REFERENCES level_rewards(id) ON DELETE CASCADE,
      granted_at TEXT NOT NULL, PRIMARY KEY (uuid, reward_id));
    ALTER TABLE server_market ADD COLUMN item_data TEXT;
    CREATE TABLE economy_operations (server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
      operation_id TEXT NOT NULL, response TEXT NOT NULL, PRIMARY KEY(server_id, operation_id));
    "#,
    r#"
    CREATE TABLE user_post_likes (
        post_id INTEGER NOT NULL REFERENCES user_posts(id) ON DELETE CASCADE,
        user_uuid TEXT NOT NULL,
        PRIMARY KEY (post_id, user_uuid)
    );
    "#,
    r#"
    ALTER TABLE game_servers ADD COLUMN map_url TEXT NOT NULL DEFAULT '';
    "#,
    r#"
    CREATE TABLE account_connections (
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        provider TEXT NOT NULL,
        provider_id TEXT NOT NULL,
        display_name TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL,
        PRIMARY KEY (provider, provider_id),
        UNIQUE (user_id, provider)
    );
    CREATE TABLE oauth_attempts (
        state TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        result TEXT,
        consumed_at TEXT
    );
    CREATE TABLE password_resets (
        token_hash TEXT PRIMARY KEY,
        user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        expires_at TEXT NOT NULL,
        used_at TEXT
    );
    CREATE INDEX password_resets_user ON password_resets(user_id);
    "#,
    r#"
    CREATE TABLE guild_wallets (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        balance REAL NOT NULL DEFAULT 0 CHECK(balance >= 0),
        updated_at TEXT NOT NULL,
        PRIMARY KEY(server_id,guild_id)
    );
    CREATE TABLE guild_wallet_transactions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        actor_uuid TEXT NOT NULL,
        kind TEXT NOT NULL,
        amount REAL NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX guild_wallet_transactions_recent ON guild_wallet_transactions(guild_id,server_id,id DESC);
    "#,
    r#"
    ALTER TABLE game_servers ADD COLUMN live_map_enabled INTEGER NOT NULL DEFAULT 0;
    "#,
    r#"
    ALTER TABLE quests ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
    CREATE TABLE quest_assignments (
        user_uuid TEXT NOT NULL,
        period TEXT NOT NULL,
        period_key TEXT NOT NULL,
        quest_id TEXT NOT NULL REFERENCES quests(id) ON DELETE CASCADE,
        position INTEGER NOT NULL,
        PRIMARY KEY (user_uuid, period_key, quest_id)
    );
    CREATE INDEX quest_assignments_user ON quest_assignments(user_uuid, period_key, position);
    "#,
    // Guild bank: listings sold on behalf of a guild, and a note on each wallet movement.
    r#"
    ALTER TABLE server_market ADD COLUMN seller_guild_id TEXT;
    ALTER TABLE guild_wallet_transactions ADD COLUMN note TEXT NOT NULL DEFAULT '';
    "#,
    // Plugin integrations (LuckPerms ranks, WorldGuard, Spark, CoreProtect…) and
    // events queued for game servers (level-ups, guild joins, achievements).
    r#"
    ALTER TABLE groups ADD COLUMN luckperms_group TEXT NOT NULL DEFAULT '';
    CREATE TABLE server_integrations (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        version TEXT NOT NULL DEFAULT '',
        data TEXT NOT NULL DEFAULT '{}',
        updated_at TEXT NOT NULL,
        PRIMARY KEY (server_id, name)
    );
    CREATE TABLE player_ranks (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        primary_group TEXT NOT NULL DEFAULT '',
        display TEXT NOT NULL DEFAULT '',
        prefix TEXT NOT NULL DEFAULT '',
        suffix TEXT NOT NULL DEFAULT '',
        groups TEXT NOT NULL DEFAULT '[]',
        permissions TEXT NOT NULL DEFAULT '[]',
        weight INTEGER NOT NULL DEFAULT 0,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (server_id, uuid)
    );
    CREATE INDEX player_ranks_uuid ON player_ranks(uuid);
    CREATE TABLE server_notifications (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        uuid TEXT NOT NULL,
        payload TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL
    );
    CREATE INDEX server_notifications_server ON server_notifications(server_id, id);
    "#,
    // Every code path that changes a level, unlocks an achievement or moves a
    // member queues the matching event for the servers the player is on.
    r#"
    CREATE TRIGGER notify_global_level AFTER UPDATE OF global_level ON user_levels WHEN NEW.global_level > OLD.global_level BEGIN
        INSERT INTO server_notifications (server_id, kind, uuid, payload, created_at)
        SELECT so.server_id, 'level_up', NEW.uuid,
               json_object('scope', 'global', 'level', NEW.global_level, 'previous', OLD.global_level, 'xp', NEW.global_xp),
               strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        FROM server_online so WHERE so.uuid = NEW.uuid;
    END;
    CREATE TRIGGER notify_server_level AFTER UPDATE OF server_level ON server_levels WHEN NEW.server_level > OLD.server_level BEGIN
        INSERT INTO server_notifications (server_id, kind, uuid, payload, created_at)
        SELECT NEW.server_id, 'level_up', NEW.uuid,
               json_object('scope', 'server', 'level', NEW.server_level, 'previous', OLD.server_level, 'xp', NEW.server_xp),
               strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        WHERE EXISTS (SELECT 1 FROM server_online so WHERE so.server_id = NEW.server_id AND so.uuid = NEW.uuid);
    END;
    CREATE TRIGGER notify_achievement AFTER INSERT ON user_achievements BEGIN
        INSERT INTO server_notifications (server_id, kind, uuid, payload, created_at)
        SELECT so.server_id, 'achievement', NEW.user_uuid,
               json_object('id', NEW.achievement_id,
                           'title', (SELECT title FROM achievements WHERE id = NEW.achievement_id),
                           'xp', (SELECT xp_reward FROM achievements WHERE id = NEW.achievement_id)),
               strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        FROM server_online so WHERE so.uuid = NEW.user_uuid;
    END;
    CREATE TRIGGER notify_guild_join AFTER INSERT ON guild_members BEGIN
        INSERT INTO server_notifications (server_id, kind, uuid, payload, created_at)
        SELECT so.server_id, 'guild_join', NEW.uuid,
               json_object('guild_id', NEW.guild_id, 'guild', (SELECT name FROM guilds WHERE id = NEW.guild_id),
                           'tag', (SELECT tag FROM guilds WHERE id = NEW.guild_id), 'role', NEW.role),
               strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        FROM server_online so WHERE so.uuid = NEW.uuid;
    END;
    CREATE TRIGGER notify_guild_leave AFTER DELETE ON guild_members BEGIN
        INSERT INTO server_notifications (server_id, kind, uuid, payload, created_at)
        SELECT so.server_id, 'guild_leave', OLD.uuid,
               json_object('guild_id', OLD.guild_id, 'guild', (SELECT name FROM guilds WHERE id = OLD.guild_id),
                           'tag', (SELECT tag FROM guilds WHERE id = OLD.guild_id)),
               strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        FROM server_online so WHERE so.uuid = OLD.uuid;
    END;
    "#,
    // Bumped by any change to claims, guilds or membership so game servers can
    // poll a single number instead of asking about every block.
    r#"
    INSERT OR IGNORE INTO kv (key, value) VALUES ('guild_rev', '1');
    CREATE TRIGGER guild_claims_rev_i AFTER INSERT ON guild_claims BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guild_claims_rev_d AFTER DELETE ON guild_claims BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guild_claims_rev_u AFTER UPDATE ON guild_claims BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guild_members_rev_i AFTER INSERT ON guild_members BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guild_members_rev_d AFTER DELETE ON guild_members BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guild_members_rev_u AFTER UPDATE ON guild_members BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guilds_rev_i AFTER INSERT ON guilds BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guilds_rev_d AFTER DELETE ON guilds BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guilds_rev_u AFTER UPDATE OF name, tag, leader_uuid ON guilds BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    "#,
    // Discord role <-> panel group mapping.
    r#"
    ALTER TABLE groups ADD COLUMN discord_role TEXT NOT NULL DEFAULT '';
    "#,
    // Profile cosmetics.
    r#"
    ALTER TABLE user_profiles ADD COLUMN accent_color TEXT;
    "#,
    // Servers can share one economy (balances and guild banks) by sharing a group name.
    r#"
    ALTER TABLE game_servers ADD COLUMN economy_group TEXT NOT NULL DEFAULT '';
    "#,
    // BlueMap: a map address per server, in-game actions triggered from the map, and guild invitations.
    r#"
    ALTER TABLE game_servers ADD COLUMN map_address TEXT NOT NULL DEFAULT '';
    CREATE TABLE server_actions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        from_uuid TEXT NOT NULL DEFAULT '',
        from_name TEXT NOT NULL DEFAULT '',
        to_uuid TEXT NOT NULL DEFAULT '',
        text TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL,
        taken_at TEXT
    );
    CREATE INDEX server_actions_pending ON server_actions(server_id, taken_at);
    CREATE INDEX server_actions_sender ON server_actions(from_uuid, created_at);
    CREATE TABLE guild_invites (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        inviter_uuid TEXT NOT NULL,
        inviter_name TEXT NOT NULL,
        target_uuid TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        created_at TEXT NOT NULL
    );
    CREATE INDEX guild_invites_target ON guild_invites(target_uuid, status);
    "#,
    // Player initiated guild applications.
    r#"
    CREATE TABLE guild_join_requests (
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        message TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL,
        PRIMARY KEY (guild_id, uuid)
    );
    CREATE INDEX guild_join_requests_player ON guild_join_requests(uuid);
    "#,
    // Rewards beyond XP and titles: what quests, achievements and rank milestones hand out (money, items, permissions,
    // claim chunks, commands…), the queue of earned bundles, and the deliveries waiting for a game server.
    r#"
    CREATE TABLE reward_bundles (
        source_type TEXT NOT NULL,
        source_id TEXT NOT NULL,
        actions TEXT NOT NULL DEFAULT '[]',
        PRIMARY KEY (source_type, source_id)
    );
    CREATE TABLE reward_queue (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        uuid TEXT NOT NULL,
        source_type TEXT NOT NULL,
        source_id TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE TABLE reward_deliveries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        uuid TEXT NOT NULL,
        server_id INTEGER REFERENCES game_servers(id) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        payload TEXT NOT NULL DEFAULT '{}',
        source TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL,
        sent_at TEXT,
        delivered_at TEXT,
        attempts INTEGER NOT NULL DEFAULT 0,
        error TEXT
    );
    CREATE INDEX reward_deliveries_pending ON reward_deliveries(delivered_at, server_id);
    CREATE TABLE player_bonuses (
        uuid TEXT PRIMARY KEY,
        claim_chunks INTEGER NOT NULL DEFAULT 0
    );
    CREATE TRIGGER reward_on_achievement AFTER INSERT ON user_achievements
    WHEN EXISTS (SELECT 1 FROM reward_bundles WHERE source_type = 'achievement' AND source_id = NEW.achievement_id) BEGIN
      INSERT INTO reward_queue (uuid, source_type, source_id, created_at)
      VALUES (NEW.user_uuid, 'achievement', NEW.achievement_id, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
    END;
    "#,
    // Emails sent from the panel: players can opt out of non-essential mail, and every send is logged.
    r#"
    ALTER TABLE users ADD COLUMN email_optout INTEGER NOT NULL DEFAULT 0;
    CREATE TABLE email_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        subject TEXT NOT NULL,
        audience TEXT NOT NULL,
        total INTEGER NOT NULL DEFAULT 0,
        sent INTEGER NOT NULL DEFAULT 0,
        failed INTEGER NOT NULL DEFAULT 0,
        last_error TEXT,
        started_at TEXT NOT NULL,
        finished_at TEXT
    );
    "#,
    // The notification bell shown in the launcher and the admin panel.
    r#"
    CREATE TABLE user_notifications (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        uuid TEXT NOT NULL,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        body TEXT NOT NULL DEFAULT '',
        link TEXT,
        created_at TEXT NOT NULL,
        read_at TEXT
    );
    CREATE INDEX user_notifications_uuid ON user_notifications(uuid, id);
    "#,
    // Scheduled tasks: settings, status and a short run history for the admin "Scheduled tasks" page.
    r#"
    CREATE TABLE scheduled_tasks (
        id TEXT PRIMARY KEY,
        enabled INTEGER NOT NULL DEFAULT 1,
        interval_secs INTEGER NOT NULL,
        settings TEXT NOT NULL DEFAULT '{}',
        last_run_at INTEGER,
        last_ok INTEGER,
        last_message TEXT,
        last_duration_ms INTEGER,
        next_run_at INTEGER,
        runs INTEGER NOT NULL DEFAULT 0,
        failures INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE scheduled_task_runs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        task_id TEXT NOT NULL,
        started_at INTEGER NOT NULL,
        duration_ms INTEGER NOT NULL,
        ok INTEGER NOT NULL,
        message TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX scheduled_task_runs_task ON scheduled_task_runs(task_id, id);
    "#,
    // Admin claims: named, described regions owned by the server rather than a guild. Guild claims can't overlap them.
    r#"
    CREATE TABLE admin_claims (
        id TEXT PRIMARY KEY,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        name TEXT NOT NULL COLLATE NOCASE,
        description TEXT NOT NULL DEFAULT '',
        color TEXT NOT NULL DEFAULT '#f59e0b',
        created_at TEXT NOT NULL,
        UNIQUE (server_id, name)
    );
    CREATE TABLE admin_claim_chunks (
        claim_id TEXT NOT NULL REFERENCES admin_claims(id) ON DELETE CASCADE,
        server_id INTEGER NOT NULL,
        dimension TEXT NOT NULL DEFAULT 'minecraft:overworld',
        chunk_x INTEGER NOT NULL,
        chunk_z INTEGER NOT NULL,
        PRIMARY KEY (server_id, dimension, chunk_x, chunk_z)
    );
    CREATE INDEX admin_claim_chunks_claim ON admin_claim_chunks(claim_id);
    CREATE TRIGGER admin_claims_rev_i AFTER INSERT ON admin_claims BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER admin_claims_rev_u AFTER UPDATE ON admin_claims BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER admin_claims_rev_d AFTER DELETE ON admin_claims BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER admin_chunks_rev_i AFTER INSERT ON admin_claim_chunks BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER admin_chunks_rev_d AFTER DELETE ON admin_claim_chunks BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    CREATE TRIGGER guild_claim_not_admin BEFORE INSERT ON guild_claims
    WHEN EXISTS (SELECT 1 FROM admin_claim_chunks a WHERE a.server_id = NEW.server_id AND a.dimension = NEW.dimension AND a.chunk_x = NEW.chunk_x AND a.chunk_z = NEW.chunk_z)
    BEGIN SELECT RAISE(ABORT, 'chunk belongs to an admin claim'); END;
    "#,
    // Admin-designed items, handed out in game with /customitem and by kits.
    r#"
    CREATE TABLE custom_items (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        spec TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    "#,
    // Market auctions: bids are held from the bidder's balance, and winnings/returns wait in a mailbox.
    r#"
    ALTER TABLE server_market ADD COLUMN kind TEXT NOT NULL DEFAULT 'buy_now';
    ALTER TABLE server_market ADD COLUMN ends_at TEXT;
    ALTER TABLE server_market ADD COLUMN current_bid REAL;
    ALTER TABLE server_market ADD COLUMN bidder_uuid TEXT;
    ALTER TABLE server_market ADD COLUMN bidder_name TEXT;
    ALTER TABLE server_market ADD COLUMN bid_count INTEGER NOT NULL DEFAULT 0;
    CREATE INDEX server_market_auction ON server_market(kind, ends_at);
    CREATE TABLE market_mailbox (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        item_id TEXT NOT NULL,
        item_name TEXT NOT NULL,
        amount INTEGER NOT NULL,
        item_data TEXT,
        note TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL
    );
    CREATE INDEX market_mailbox_owner ON market_mailbox(server_id, uuid);
    "#,
    // Launcher purchases and auction results are delivered into the player's vault by the game server.
    r#"
    ALTER TABLE market_mailbox ADD COLUMN to_vault INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE market_mailbox ADD COLUMN leased_until TEXT;
    "#,
    // Custom content beyond items: blocks, chests, decorations, NPCs, vehicles, crops and mobs. Item-like kinds stay in
    // custom_items so kits, rewards and /customitem keep working.
    r#"
    CREATE TABLE custom_content (
        id TEXT PRIMARY KEY,
        kind TEXT NOT NULL,
        title TEXT NOT NULL,
        source TEXT NOT NULL DEFAULT 'manual',
        spec TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX custom_content_kind ON custom_content(kind);
    "#,
    // 26: Casino: rounds, Mines games, daily free spins, bounties and player bets.
    r#"
    CREATE TABLE casino_rounds (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        game TEXT NOT NULL,
        bet REAL NOT NULL,
        payout REAL NOT NULL,
        detail TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL
    );
    CREATE INDEX casino_rounds_player ON casino_rounds(uuid, id);
    CREATE INDEX casino_rounds_time ON casino_rounds(created_at);
    CREATE TABLE casino_mines (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        bet REAL NOT NULL,
        size INTEGER NOT NULL,
        mines INTEGER NOT NULL,
        layout TEXT NOT NULL,
        revealed TEXT NOT NULL DEFAULT '[]',
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL
    );
    CREATE INDEX casino_mines_player ON casino_mines(uuid, status);
    CREATE TABLE casino_free_spins (
        uuid TEXT NOT NULL,
        day TEXT NOT NULL,
        spins INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (uuid, day)
    );
    CREATE TABLE casino_bounties (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        target_uuid TEXT NOT NULL,
        target_name TEXT NOT NULL,
        placer_uuid TEXT NOT NULL,
        placer_name TEXT NOT NULL,
        paid REAL NOT NULL,
        reward REAL NOT NULL,
        anonymous INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'active',
        claimer_uuid TEXT,
        claimer_name TEXT,
        created_at TEXT NOT NULL,
        expires_at TEXT,
        resolved_at TEXT
    );
    CREATE INDEX casino_bounties_target ON casino_bounties(server_id, target_uuid, status);
    CREATE INDEX casino_bounties_placer ON casino_bounties(placer_uuid, status);
    CREATE TABLE casino_markets (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        creator_uuid TEXT NOT NULL,
        creator_name TEXT NOT NULL,
        subject_uuid TEXT NOT NULL,
        subject_name TEXT NOT NULL,
        metric TEXT NOT NULL,
        threshold INTEGER NOT NULL,
        baseline INTEGER NOT NULL,
        final_value INTEGER,
        created_at TEXT NOT NULL,
        locks_at TEXT NOT NULL,
        ends_at TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'open',
        outcome TEXT,
        resolved_at TEXT
    );
    CREATE INDEX casino_markets_open ON casino_markets(server_id, status, ends_at);
    CREATE TABLE casino_bets (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        market_id INTEGER NOT NULL REFERENCES casino_markets(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        side TEXT NOT NULL,
        stake REAL NOT NULL,
        payout REAL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX casino_bets_market ON casino_bets(market_id);
    CREATE INDEX casino_bets_player ON casino_bets(uuid, id);
    "#,
    // 27: Companion mutation receipts prevent replaying a casino bet or claim.
    r#"
    CREATE TABLE companion_requests (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        request_id TEXT NOT NULL,
        fingerprint TEXT NOT NULL,
        response TEXT,
        created_at TEXT NOT NULL,
        PRIMARY KEY(server_id, uuid, request_id)
    );
    "#,
    // 28: Casino: live Crash and Blackjack rounds, and the Double or Nothing offers made after a win.
    r#"
    CREATE TABLE casino_crash (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        bet REAL NOT NULL,
        crash_point REAL NOT NULL,
        auto_cashout REAL,
        started_ms INTEGER NOT NULL,
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL
    );
    CREATE INDEX casino_crash_player ON casino_crash(uuid, status);
    CREATE TABLE casino_blackjack (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        uuid TEXT NOT NULL,
        name TEXT NOT NULL,
        bet REAL NOT NULL,
        player TEXT NOT NULL,
        dealer TEXT NOT NULL,
        doubled INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL
    );
    CREATE INDEX casino_blackjack_player ON casino_blackjack(uuid, status);
    CREATE TABLE casino_double (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        uuid TEXT NOT NULL,
        stake REAL NOT NULL,
        streak INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'open',
        expires_at TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX casino_double_player ON casino_double(uuid, status);
    "#,
    // 29: Buy orders (escrowed requests that other players fill with items) and Contracts (randomly generated tasks).
    r#"
    CREATE TABLE buy_orders (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        buyer_uuid TEXT NOT NULL,
        buyer_name TEXT NOT NULL,
        item_id TEXT NOT NULL,
        item_name TEXT NOT NULL,
        amount INTEGER NOT NULL,
        total REAL NOT NULL,
        status TEXT NOT NULL DEFAULT 'open',
        claimer_uuid TEXT,
        claimer_name TEXT,
        claim_until TEXT,
        filler_uuid TEXT,
        filler_name TEXT,
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        resolved_at TEXT
    );
    CREATE INDEX buy_orders_board ON buy_orders(server_id, status);
    CREATE INDEX buy_orders_buyer ON buy_orders(buyer_uuid, status);
    CREATE TABLE contracts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL,
        uuid TEXT NOT NULL,
        kind TEXT NOT NULL,
        target TEXT NOT NULL,
        title TEXT NOT NULL,
        required INTEGER NOT NULL,
        progress INTEGER NOT NULL DEFAULT 0,
        reward REAL NOT NULL,
        bonus INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        resolved_at TEXT
    );
    CREATE INDEX contracts_player ON contracts(uuid, server_id, status);
    "#,
    // 30: Admin claim flags (what is allowed inside each region) and a heartbeat on Crash rounds so a player who leaves is cashed out.
    r#"
    ALTER TABLE admin_claims ADD COLUMN flags TEXT NOT NULL DEFAULT '{}';
    ALTER TABLE casino_crash ADD COLUMN last_poll_ms INTEGER;
    "#,
    // 31: "Clean update": bumping this makes every launcher wipe an instance's old files before it downloads them all again.
    r#"
    ALTER TABLE instances ADD COLUMN clean_epoch INTEGER NOT NULL DEFAULT 0;
    "#,
    // 32: Scoped progression, multi-membership guilds, relations, collections and events.
    r#"
    ALTER TABLE quests ADD COLUMN instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE;
    ALTER TABLE quests ADD COLUMN server_id INTEGER REFERENCES game_servers(id) ON DELETE CASCADE;
    ALTER TABLE quests ADD COLUMN reward_data TEXT NOT NULL DEFAULT '{}';
    ALTER TABLE quests ADD COLUMN chain_id TEXT;
    ALTER TABLE quests ADD COLUMN chain_step INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE quests ADD COLUMN difficulty TEXT NOT NULL DEFAULT 'standard';
    CREATE INDEX quests_scope ON quests(instance_id, server_id, enabled, period);

    DROP INDEX IF EXISTS guild_member_unique;
    CREATE TABLE guild_primary_memberships (
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        updated_at TEXT NOT NULL,
        PRIMARY KEY (server_id, uuid)
    );
    CREATE INDEX guild_primary_by_guild ON guild_primary_memberships(guild_id);
    INSERT OR IGNORE INTO guild_primary_memberships(server_id, uuid, guild_id, updated_at)
      SELECT server_id, uuid, guild_id, strftime('%Y-%m-%dT%H:%M:%SZ', 'now') FROM (
        SELECT s.id AS server_id, gm.uuid, gm.guild_id,
               ROW_NUMBER() OVER (PARTITION BY s.id, gm.uuid ORDER BY (g.leader_uuid = gm.uuid) DESC, g.created_at, g.id) AS rn
        FROM guild_members gm JOIN guilds g ON g.id = gm.guild_id
        JOIN game_servers s ON s.instance_id = g.instance_id
      ) WHERE rn = 1;
    CREATE TABLE guild_relations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        instance_id TEXT NOT NULL,
        guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        other_guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
        relation TEXT NOT NULL CHECK (relation IN ('alliance', 'rival')),
        status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'declined', 'ended')),
        actor_uuid TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        UNIQUE(instance_id, guild_id, other_guild_id)
    );
    CREATE INDEX guild_relations_lookup ON guild_relations(instance_id, guild_id, status);

    CREATE TABLE player_unlocks (
        uuid TEXT NOT NULL, unlock_key TEXT NOT NULL, unlock_type TEXT NOT NULL,
        source_type TEXT NOT NULL, source_id TEXT NOT NULL, unlocked_at TEXT NOT NULL,
        equipped INTEGER NOT NULL DEFAULT 0, metadata TEXT NOT NULL DEFAULT '{}',
        PRIMARY KEY(uuid, unlock_key)
    );
    CREATE INDEX player_unlocks_type ON player_unlocks(uuid, unlock_type, equipped);
    ALTER TABLE user_profiles ADD COLUMN profile_theme TEXT NOT NULL DEFAULT 'default';
    ALTER TABLE user_profiles ADD COLUMN showcase TEXT NOT NULL DEFAULT '{}';

    CREATE TABLE community_events (
        id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
        instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE,
        server_id INTEGER REFERENCES game_servers(id) ON DELETE CASCADE,
        status TEXT NOT NULL DEFAULT 'draft', starts_at TEXT NOT NULL, ends_at TEXT NOT NULL,
        objective_data TEXT NOT NULL DEFAULT '[]', reward_data TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL, updated_at TEXT NOT NULL
    );
    CREATE INDEX community_events_schedule ON community_events(status, starts_at, ends_at);
    CREATE TABLE community_event_participants (
        event_id TEXT NOT NULL REFERENCES community_events(id) ON DELETE CASCADE,
        uuid TEXT NOT NULL, guild_id TEXT REFERENCES guilds(id) ON DELETE SET NULL,
        joined_at TEXT NOT NULL, contribution INTEGER NOT NULL DEFAULT 0,
        placement INTEGER, participation_rewarded INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY(event_id, uuid)
    );
    CREATE INDEX community_event_participants_rank ON community_event_participants(event_id, contribution DESC);
    "#,
    r#"
    CREATE TABLE cosmetic_templates (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        key TEXT NOT NULL UNIQUE,
        type TEXT NOT NULL,
        label TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        metadata TEXT NOT NULL DEFAULT '{}',
        auto_grant INTEGER NOT NULL DEFAULT 0,
        grant_on_level INTEGER,
        grant_on_achievement TEXT,
        created_at TEXT NOT NULL
    );
    CREATE INDEX cosmetic_templates_type ON cosmetic_templates(type);
    "#,
    // Companion HUD layouts designed in the panel; the widgets are a JSON array.
    r#"
    CREATE TABLE companion_layouts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        widgets TEXT NOT NULL DEFAULT '[]',
        is_default INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL
    );
    "#,
    // Cosmetic templates marked auto-grant unlock themselves when a player reaches the level or earns the achievement.
    r#"
    CREATE TRIGGER cosmetic_auto_level_u AFTER UPDATE OF global_level ON user_levels WHEN NEW.global_level > OLD.global_level BEGIN
      INSERT OR IGNORE INTO player_unlocks(uuid, unlock_key, unlock_type, source_type, source_id, unlocked_at, equipped, metadata)
      SELECT NEW.uuid, key, type, 'level', CAST(NEW.global_level AS TEXT), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 0, metadata
      FROM cosmetic_templates WHERE auto_grant = 1 AND grant_on_level IS NOT NULL AND grant_on_level <= NEW.global_level;
    END;
    CREATE TRIGGER cosmetic_auto_level_i AFTER INSERT ON user_levels WHEN NEW.global_level > 1 BEGIN
      INSERT OR IGNORE INTO player_unlocks(uuid, unlock_key, unlock_type, source_type, source_id, unlocked_at, equipped, metadata)
      SELECT NEW.uuid, key, type, 'level', CAST(NEW.global_level AS TEXT), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 0, metadata
      FROM cosmetic_templates WHERE auto_grant = 1 AND grant_on_level IS NOT NULL AND grant_on_level <= NEW.global_level;
    END;
    CREATE TRIGGER cosmetic_auto_achievement AFTER INSERT ON user_achievements BEGIN
      INSERT OR IGNORE INTO player_unlocks(uuid, unlock_key, unlock_type, source_type, source_id, unlocked_at, equipped, metadata)
      SELECT NEW.user_uuid, key, type, 'achievement', NEW.achievement_id, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 0, metadata
      FROM cosmetic_templates WHERE auto_grant = 1 AND grant_on_achievement = NEW.achievement_id;
    END;
    "#,
    // LuckPerms manager: instructions queued for game servers (group edits, player moves) and the level milestones that grant groups.
    r#"
    CREATE TABLE luckperms_commands (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        server_id INTEGER NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
        op TEXT NOT NULL,
        summary TEXT NOT NULL DEFAULT '',
        actor TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL,
        taken_at TEXT,
        done_at TEXT,
        ok INTEGER,
        error TEXT
    );
    CREATE INDEX luckperms_commands_pending ON luckperms_commands(server_id, done_at, id);
    CREATE TABLE luckperms_level_links (
        level INTEGER PRIMARY KEY CHECK (level >= 1),
        group_name TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    -- What each guild lets happen on its claims (see routes/guild_flags.rs); game servers re-read the claim index when it changes.
    ALTER TABLE guilds ADD COLUMN claim_flags TEXT NOT NULL DEFAULT '{}';
    CREATE TRIGGER guilds_flags_rev_u AFTER UPDATE OF claim_flags ON guilds BEGIN UPDATE kv SET value = CAST(value AS INTEGER) + 1 WHERE key = 'guild_rev'; END;
    "#,
    // Quest chains are real records, so an empty chain can exist (and show up) before any quest is added to it.
    r#"
    CREATE TABLE quest_chains (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        created_at TEXT NOT NULL
    );
    "#,
    // Instance presentation is platform metadata; gameplay lives in an instance store.
    r#"ALTER TABLE instances ADD COLUMN experience TEXT NOT NULL DEFAULT '{}';"#,
    r#"CREATE TABLE experience_group_links (
        group_id INTEGER PRIMARY KEY,
        luckperms_group TEXT NOT NULL DEFAULT '',
        discord_role TEXT NOT NULL DEFAULT ''
    );"#,
];

pub async fn connect(data_dir: &Path) -> Result<SqlitePool> {
    std::fs::create_dir_all(data_dir)?;
    let url = format!("sqlite://{}", data_dir.join("panel.db").display());
    let opts = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(8).connect_with(opts).await?;
    migrate(&pool).await?;
    Ok(pool)
}

pub async fn connect_memory() -> Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await?;
    migrate(&pool).await?;
    Ok(pool)
}

pub(crate) async fn migrate(pool: &SqlitePool) -> Result<()> {
    migrate_mode(pool, false).await
}

pub(crate) async fn migrate_mode(pool: &SqlitePool, experience: bool) -> Result<()> {
    let current: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(pool).await?;
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = i as i64 + 1;
        if version <= current {
            continue;
        }
        let mut tx = pool.begin().await?;
        let mut schema = sql.to_string();
        if experience {
            // SQLite cannot enforce foreign keys across attached databases. Identity
            // references are validated by platform auth; experience-local FKs remain.
            for table in crate::experience::PLATFORM_TABLES {
                for field in ["id", "uuid", "name"] {
                    schema = schema
                        .replace(&format!("REFERENCES {table}({field}) ON DELETE CASCADE"), "")
                        .replace(&format!("REFERENCES {table}({field}) ON DELETE SET NULL"), "")
                        .replace(&format!("REFERENCES {table}({field})"), "");
                }
            }
        }
        sqlx::Executor::execute(&mut *tx, schema.as_str()).await?;
        sqlx::query(&format!("PRAGMA user_version = {version}")).execute(&mut *tx).await?;
        tx.commit().await?;
        tracing::info!("applied database migration {version}");
    }
    backfill_uuids(pool).await?;
    crate::seed::seed_quests_and_achievements(pool).await?;
    // Every quest, achievement and milestone without a money reward gets the default one for its difficulty.
    if let Err(e) = crate::rewards::apply_defaults_to(pool).await {
        tracing::warn!("default money rewards failed: {:?}", e.message);
    }
    if current < 9 {
        crate::seed::upgrade_seeded_quest_targets(pool).await?;
    }
    Ok(())
}

/// Accounts created before the auth server existed get the offline-mode UUID
/// for their name — the one offline servers already knew them by.
async fn backfill_uuids(pool: &SqlitePool) -> Result<()> {
    let missing: Vec<(i64, String)> = sqlx::query_as("SELECT id, username FROM users WHERE uuid = ''").fetch_all(pool).await?;
    for (id, name) in missing {
        sqlx::query("UPDATE users SET uuid = ? WHERE id = ?").bind(velora_shared::offline_uuid(&name)).bind(id).execute(pool).await?;
    }
    if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name = 'users_uuid'").fetch_one(pool).await? == 0 {
        sqlx::query("CREATE UNIQUE INDEX users_uuid ON users(uuid)").execute(pool).await?;
    }
    Ok(())
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn identity_migration_preserves_existing_player_data_keys() {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::raw_sql(MIGRATIONS[0]).execute(&pool).await.unwrap();
        sqlx::raw_sql(MIGRATIONS[1]).execute(&pool).await.unwrap();
        let existing = velora_shared::offline_uuid("LegacyPlayer");
        sqlx::query("INSERT INTO users(username,password_hash,created_at,uuid) VALUES('LegacyPlayer','test',?,?)")
            .bind(now())
            .bind(&existing)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql("PRAGMA user_version=2").execute(&pool).await.unwrap();
        migrate(&pool).await.unwrap();
        let uuid: String = sqlx::query_scalar("SELECT uuid FROM users WHERE username='LegacyPlayer'").fetch_one(&pool).await.unwrap();
        assert_eq!(uuid, existing);
        sqlx::query("UPDATE users SET username='RenamedPlayer' WHERE uuid=?").bind(&existing).execute(&pool).await.unwrap();
        let uuid: String = sqlx::query_scalar("SELECT uuid FROM users WHERE username='RenamedPlayer'").fetch_one(&pool).await.unwrap();
        assert_eq!(uuid, existing);
        let owner: String =
            sqlx::query_scalar("SELECT uuid FROM reserved_usernames WHERE name='legacyplayer'").fetch_one(&pool).await.unwrap();
        assert_eq!(owner, existing);
    }
}
