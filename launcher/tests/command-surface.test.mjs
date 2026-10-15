import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const initialSnapshot = {
  "provenance": {
    "sourcePath": "launcher/src-tauri/src/commands.rs",
    "sourceSha256": "0371ee381c2600f7884c24d58f4020c1ebfd2deb82c5277af41c8360534f5e09",
    "baselineHead": "4f639266a515c3cb05769b055a4772c4dcb89cb8",
    "capturedOn": "2026-10-12",
    "note": "Working-tree snapshot includes previously implemented operation and settings safety changes, not clean HEAD."
  },
  "registrations": [
    "bootstrap",
    "record_activity",
    "refresh_manifest",
    "set_panel_url",
    "save_settings",
    "select_experience",
    "save_instance_options",
    "set_instance_fov",
    "login_panel",
    "discord_sign_in_start",
    "discord_sign_in_poll",
    "request_password_reset",
    "discord_connection",
    "discord_info",
    "map_action",
    "guild_send_invite",
    "my_guild_invites",
    "market_listings",
    "market_mine",
    "market_act",
    "casino_get",
    "casino_post",
    "board_get",
    "board_post",
    "vault_get",
    "faction_upgrades",
    "vault_post",
    "textures_status",
    "item_textures",
    "mc_sprites",
    "notifications_list",
    "notifications_mark",
    "respond_guild_invite",
    "discord_link_start",
    "discord_unlink",
    "register_panel",
    "add_offline",
    "account_profile",
    "upload_skin",
    "set_skin_model",
    "delete_skin",
    "set_cape",
    "get_skin_profiles",
    "save_skin_profile",
    "delete_skin_profile",
    "apply_skin_profile",
    "set_username",
    "select_account",
    "remove_account",
    "launch",
    "repair_instance",
    "cancel_launch",
    "kill_game",
    "instance_local",
    "ping_server",
    "open_folder",
    "open_file",
    "storage_info",
    "delete_instance_data",
    "clear_cache",
    "detect_java",
    "check_update",
    "install_update",
    "show_window",
    "get_player_stats",
    "get_leaderboard",
    "get_public_servers",
    "get_map_info",
    "get_map_overlay",
    "get_my_level",
    "get_my_quests",
    "get_reward_summaries",
    "claim_quest",
    "get_my_achievements",
    "get_my_collections",
    "equip_collection_item",
    "get_guilds",
    "get_my_guild",
    "get_my_guild_memberships",
    "set_primary_guild",
    "get_guild_relations",
    "create_guild_relation",
    "respond_guild_relation",
    "request_guild_join",
    "get_guild_join_requests",
    "respond_guild_join_request",
    "get_guild_roles",
    "create_guild_role",
    "assign_guild_role",
    "kick_guild_member",
    "transfer_guild_leader",
    "delete_guild_role",
    "update_guild",
    "get_guild_claim_flags",
    "set_guild_claim_flags",
    "create_guild",
    "get_guild_claims",
    "rename_guild",
    "disband_guild",
    "claim_guild_chunk",
    "unclaim_guild_chunk",
    "get_guild_posts",
    "create_guild_post",
    "get_guild_members",
    "get_guild_wallet",
    "transfer_guild_wallet",
    "get_friends",
    "send_friend_request",
    "respond_friend_request",
    "remove_friend",
    "search_members",
    "get_economy_balances",
    "get_server_baltop",
    "get_transactions",
    "get_direct_messages",
    "send_direct_message",
    "get_game_invites",
    "send_game_invite",
    "respond_game_invite",
    "get_user_profile",
    "update_my_profile",
    "get_user_posts",
    "create_user_post",
    "like_user_post"
  ],
  "commands": [
    {
      "name": "record_activity",
      "signature": "pub async fn record_activity(state: State<'_, AppState>, kind: String, instance_id: Option<String>) -> Res<()>"
    },
    {
      "name": "bootstrap",
      "signature": "pub fn bootstrap(state: State<'_, AppState>) -> Bootstrap"
    },
    {
      "name": "refresh_manifest",
      "signature": "pub async fn refresh_manifest(state: State<'_, AppState>) -> Res<LauncherManifest>"
    },
    {
      "name": "set_panel_url",
      "signature": "pub async fn set_panel_url(state: State<'_, AppState>, url: String) -> Res<LauncherManifest>"
    },
    {
      "name": "select_experience",
      "signature": "pub fn select_experience(state: State<'_, AppState>, instance_id: Option<String>) -> Res<()>"
    },
    {
      "name": "save_settings",
      "signature": "pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Res<()>"
    },
    {
      "name": "save_instance_options",
      "signature": "pub fn save_instance_options(state: State<'_, AppState>, instance_id: String) -> Res<std::collections::BTreeMap<String, String>>"
    },
    {
      "name": "set_instance_fov",
      "signature": "pub fn set_instance_fov(state: State<'_, AppState>, instance_id: String, fov: f64) -> Res<()>"
    },
    {
      "name": "login_panel",
      "signature": "pub async fn login_panel(state: State<'_, AppState>, username: String, password: String) -> Res<Account>"
    },
    {
      "name": "discord_sign_in_start",
      "signature": "pub async fn discord_sign_in_start(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "discord_sign_in_poll",
      "signature": "pub async fn discord_sign_in_poll(state: State<'_, AppState>, oauth_state: String) -> Res<serde_json::Value>"
    },
    {
      "name": "request_password_reset",
      "signature": "pub async fn request_password_reset(state: State<'_, AppState>, email: String) -> Res<String>"
    },
    {
      "name": "discord_connection",
      "signature": "pub async fn discord_connection(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "discord_info",
      "signature": "pub async fn discord_info(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "discord_link_start",
      "signature": "pub async fn discord_link_start(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "discord_unlink",
      "signature": "pub async fn discord_unlink(state: State<'_, AppState>) -> Res<()>"
    },
    {
      "name": "register_panel",
      "signature": "pub async fn register_panel(state: State<'_, AppState>, username: String, password: String, email: Option<String>) -> Res<RegisterResult>"
    },
    {
      "name": "add_offline",
      "signature": "pub fn add_offline(state: State<'_, AppState>, username: String) -> Res<Account>"
    },
    {
      "name": "select_account",
      "signature": "pub fn select_account(state: State<'_, AppState>, id: String) -> Res<()>"
    },
    {
      "name": "remove_account",
      "signature": "pub fn remove_account(state: State<'_, AppState>, id: String) -> Res<()>"
    },
    {
      "name": "account_profile",
      "signature": "pub async fn account_profile(state: State<'_, AppState>) -> Res<PlayerProfile>"
    },
    {
      "name": "upload_skin",
      "signature": "pub async fn upload_skin(state: State<'_, AppState>, data: String, model: String) -> Res<PlayerProfile>"
    },
    {
      "name": "set_skin_model",
      "signature": "pub async fn set_skin_model(state: State<'_, AppState>, model: String) -> Res<PlayerProfile>"
    },
    {
      "name": "delete_skin",
      "signature": "pub async fn delete_skin(state: State<'_, AppState>) -> Res<PlayerProfile>"
    },
    {
      "name": "set_cape",
      "signature": "pub async fn set_cape(state: State<'_, AppState>, cape_id: Option<i64>) -> Res<PlayerProfile>"
    },
    {
      "name": "get_skin_profiles",
      "signature": "pub fn get_skin_profiles(state: State<'_, AppState>) -> Res<Vec<SkinProfile>>"
    },
    {
      "name": "save_skin_profile",
      "signature": "pub async fn save_skin_profile(state: State<'_, AppState>, mut profile: SkinProfile) -> Res<Vec<SkinProfile>>"
    },
    {
      "name": "delete_skin_profile",
      "signature": "pub fn delete_skin_profile(state: State<'_, AppState>, id: String) -> Res<Vec<SkinProfile>>"
    },
    {
      "name": "apply_skin_profile",
      "signature": "pub async fn apply_skin_profile(state: State<'_, AppState>, id: String) -> Res<PlayerProfile>"
    },
    {
      "name": "set_username",
      "signature": "pub async fn set_username(state: State<'_, AppState>, username: String, password: String) -> Res<accounts::Account>"
    },
    {
      "name": "launch",
      "signature": "pub fn launch(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Res<String>"
    },
    {
      "name": "repair_instance",
      "signature": "pub fn repair_instance(app: AppHandle, state: State<'_, AppState>, instance_id: String) -> Res<String>"
    },
    {
      "name": "cancel_launch",
      "signature": "pub fn cancel_launch(state: State<'_, AppState>, instance_id: String, operation_id: String) -> Res<()>"
    },
    {
      "name": "kill_game",
      "signature": "pub fn kill_game(state: State<'_, AppState>, run_id: String)"
    },
    {
      "name": "instance_local",
      "signature": "pub async fn instance_local(state: State<'_, AppState>, instance_id: String) -> Res<InstanceLocal>"
    },
    {
      "name": "ping_server",
      "signature": "pub async fn ping_server(host: String, port: u16) -> Res<ServerStatus>"
    },
    {
      "name": "open_folder",
      "signature": "pub fn open_folder(app: AppHandle, state: State<'_, AppState>, kind: String, instance_id: Option<String>) -> Res<()>"
    },
    {
      "name": "open_file",
      "signature": "pub fn open_file(app: AppHandle, path: String) -> Res<()>"
    },
    {
      "name": "storage_info",
      "signature": "pub async fn storage_info(state: State<'_, AppState>) -> Res<Storage>"
    },
    {
      "name": "delete_instance_data",
      "signature": "pub async fn delete_instance_data(state: State<'_, AppState>, instance_id: String) -> Res<()>"
    },
    {
      "name": "clear_cache",
      "signature": "pub async fn clear_cache(state: State<'_, AppState>) -> Res<()>"
    },
    {
      "name": "detect_java",
      "signature": "pub async fn detect_java(path: String) -> Option<u32>"
    },
    {
      "name": "check_update",
      "signature": "pub async fn check_update(state: State<'_, AppState>) -> Res<Option<updater::UpdateInfo>>"
    },
    {
      "name": "install_update",
      "signature": "pub async fn install_update(app: AppHandle, state: State<'_, AppState>, url: String) -> Res<()>"
    },
    {
      "name": "show_window",
      "signature": "pub fn show_window(app: AppHandle)"
    },
    {
      "name": "get_player_stats",
      "signature": "pub async fn get_player_stats(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "get_leaderboard",
      "signature": "pub async fn get_leaderboard(state: State<'_, AppState>, server_id: Option<i64>, sort: Option<String>) -> Res<serde_json::Value>"
    },
    {
      "name": "get_public_servers",
      "signature": "pub async fn get_public_servers(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "get_map_info",
      "signature": "pub async fn get_map_info(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value>"
    },
    {
      "name": "get_map_overlay",
      "signature": "pub async fn get_map_overlay(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value>"
    },
    {
      "name": "get_my_level",
      "signature": "pub async fn get_my_level(state: State<'_, AppState>) -> Res<UserLevelInfo>"
    },
    {
      "name": "get_reward_summaries",
      "signature": "pub async fn get_reward_summaries(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "get_my_quests",
      "signature": "pub async fn get_my_quests(state: State<'_, AppState>) -> Res<Vec<UserQuest>>"
    },
    {
      "name": "claim_quest",
      "signature": "pub async fn claim_quest(state: State<'_, AppState>, quest_id: String) -> Res<UserLevelInfo>"
    },
    {
      "name": "get_my_achievements",
      "signature": "pub async fn get_my_achievements(state: State<'_, AppState>) -> Res<Vec<Achievement>>"
    },
    {
      "name": "get_my_collections",
      "signature": "pub async fn get_my_collections(state: State<'_, AppState>) -> Res<Vec<serde_json::Value>>"
    },
    {
      "name": "equip_collection_item",
      "signature": "pub async fn equip_collection_item(state: State<'_, AppState>, unlock_key: String, equipped: bool) -> Res<()>"
    },
    {
      "name": "get_guilds",
      "signature": "pub async fn get_guilds(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Vec<Guild>>"
    },
    {
      "name": "get_my_guild",
      "signature": "pub async fn get_my_guild(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Option<Guild>>"
    },
    {
      "name": "get_my_guild_memberships",
      "signature": "pub async fn get_my_guild_memberships(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Vec<GuildMembership>>"
    },
    {
      "name": "set_primary_guild",
      "signature": "pub async fn set_primary_guild(state: State<'_, AppState>, guild_id: String, server_id: i64) -> Res<()>"
    },
    {
      "name": "get_guild_relations",
      "signature": "pub async fn get_guild_relations(state: State<'_, AppState>, guild_id: String) -> Res<Vec<serde_json::Value>>"
    },
    {
      "name": "create_guild_relation",
      "signature": "pub async fn create_guild_relation(state: State<'_, AppState>, guild_id: String, other_guild_id: String, relation: String, reward_cents: Option<i64>, reward_bps: Option<i64>, duration_hours: Option<i64>) -> Res<()>"
    },
    {
      "name": "respond_guild_relation",
      "signature": "pub async fn respond_guild_relation(state: State<'_, AppState>, guild_id: String, relation_id: i64, accept: bool, expected_revision: Option<String>) -> Res<()>"
    },
    {
      "name": "get_guild_claim_flags",
      "signature": "pub async fn get_guild_claim_flags(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value>"
    },
    {
      "name": "set_guild_claim_flags",
      "signature": "pub async fn set_guild_claim_flags(state: State<'_, AppState>, guild_id: String, flags: serde_json::Value) -> Res<serde_json::Value>"
    },
    {
      "name": "request_guild_join",
      "signature": "pub async fn request_guild_join(state: State<'_, AppState>, guild_id: String, message: String) -> Res<()>"
    },
    {
      "name": "get_guild_join_requests",
      "signature": "pub async fn get_guild_join_requests(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value>"
    },
    {
      "name": "respond_guild_join_request",
      "signature": "pub async fn respond_guild_join_request(state: State<'_, AppState>, guild_id: String, uuid: String, accept: bool) -> Res<()>"
    },
    {
      "name": "get_guild_roles",
      "signature": "pub async fn get_guild_roles(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value>"
    },
    {
      "name": "create_guild_role",
      "signature": "pub async fn create_guild_role(state: State<'_, AppState>, guild_id: String, role: serde_json::Value) -> Res<()>"
    },
    {
      "name": "assign_guild_role",
      "signature": "pub async fn assign_guild_role(state: State<'_, AppState>, guild_id: String, uuid: String, role: String) -> Res<()>"
    },
    {
      "name": "kick_guild_member",
      "signature": "pub async fn kick_guild_member(state: State<'_, AppState>, guild_id: String, uuid: String) -> Res<()>"
    },
    {
      "name": "transfer_guild_leader",
      "signature": "pub async fn transfer_guild_leader(state: State<'_, AppState>, guild_id: String, uuid: String) -> Res<()>"
    },
    {
      "name": "delete_guild_role",
      "signature": "pub async fn delete_guild_role(state: State<'_, AppState>, guild_id: String, role_id: i64) -> Res<()>"
    },
    {
      "name": "update_guild",
      "signature": "pub async fn update_guild( state: State<'_, AppState>, guild_id: String, description: String, motd: String, icon_url: String, banner_url: String, ) -> Res<()>"
    },
    {
      "name": "create_guild",
      "signature": "pub async fn create_guild( state: State<'_, AppState>, instance_id: String, name: String, tag: String, description: String, icon_url: Option<String>, banner_url: Option<String>, ) -> Res<Guild>"
    },
    {
      "name": "get_guild_claims",
      "signature": "pub async fn get_guild_claims( state: State<'_, AppState>, instance_id: String, server_id: i64, dimension: Option<String>, center_x: i32, center_z: i32, guild_id: Option<String>, radius: Option<i32>, ) -> Res<Vec<GuildClaim>>"
    },
    {
      "name": "claim_guild_chunk",
      "signature": "pub async fn claim_guild_chunk( state: State<'_, AppState>, guild_id: String, instance_id: String, server_id: i64, dimension: String, chunk_x: i32, chunk_z: i32, ) -> Res<GuildClaim>"
    },
    {
      "name": "rename_guild",
      "signature": "pub async fn rename_guild(state: State<'_, AppState>, guild_id: String, name: String, tag: Option<String>) -> Res<serde_json::Value>"
    },
    {
      "name": "disband_guild",
      "signature": "pub async fn disband_guild(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value>"
    },
    {
      "name": "unclaim_guild_chunk",
      "signature": "pub async fn unclaim_guild_chunk(state: State<'_, AppState>, claim_id: i64) -> Res<()>"
    },
    {
      "name": "get_guild_posts",
      "signature": "pub async fn get_guild_posts(state: State<'_, AppState>, guild_id: String) -> Res<Vec<GuildPost>>"
    },
    {
      "name": "create_guild_post",
      "signature": "pub async fn create_guild_post(state: State<'_, AppState>, guild_id: String, title: String, content: String) -> Res<GuildPost>"
    },
    {
      "name": "get_guild_members",
      "signature": "pub async fn get_guild_members(state: State<'_, AppState>, guild_id: String) -> Res<Vec<GuildMember>>"
    },
    {
      "name": "get_guild_wallet",
      "signature": "pub async fn get_guild_wallet(state: State<'_, AppState>, guild_id: String, server_id: i64) -> Res<serde_json::Value>"
    },
    {
      "name": "transfer_guild_wallet",
      "signature": "pub async fn transfer_guild_wallet( state: State<'_, AppState>, guild_id: String, server_id: i64, amount: f64, withdraw: bool, ) -> Res<serde_json::Value>"
    },
    {
      "name": "get_friends",
      "signature": "pub async fn get_friends(state: State<'_, AppState>) -> Res<Vec<FriendInfo>>"
    },
    {
      "name": "send_friend_request",
      "signature": "pub async fn send_friend_request(state: State<'_, AppState>, friend_username: Option<String>, username: Option<String>) -> Res<()>"
    },
    {
      "name": "respond_friend_request",
      "signature": "pub async fn respond_friend_request( state: State<'_, AppState>, friend_uuid: Option<String>, target_uuid: Option<String>, accept: bool, ) -> Res<()>"
    },
    {
      "name": "remove_friend",
      "signature": "pub async fn remove_friend(state: State<'_, AppState>, friend_uuid: Option<String>, target_uuid: Option<String>) -> Res<()>"
    },
    {
      "name": "search_members",
      "signature": "pub async fn search_members(state: State<'_, AppState>, query: String) -> Res<Vec<MemberProfile>>"
    },
    {
      "name": "get_economy_balances",
      "signature": "pub async fn get_economy_balances(state: State<'_, AppState>) -> Res<Vec<ServerEconomyBalance>>"
    },
    {
      "name": "get_server_baltop",
      "signature": "pub async fn get_server_baltop(state: State<'_, AppState>, server_id: i64) -> Res<Vec<BaltopEntry>>"
    },
    {
      "name": "get_transactions",
      "signature": "pub async fn get_transactions(state: State<'_, AppState>) -> Res<Vec<EconomyTransaction>>"
    },
    {
      "name": "get_direct_messages",
      "signature": "pub async fn get_direct_messages( state: State<'_, AppState>, friend_uuid: String, before_id: Option<i64>, mark_read: Option<bool>, ) -> Res<Vec<DirectMessage>>"
    },
    {
      "name": "send_direct_message",
      "signature": "pub async fn send_direct_message(state: State<'_, AppState>, friend_uuid: String, content: String) -> Res<DirectMessage>"
    },
    {
      "name": "get_game_invites",
      "signature": "pub async fn get_game_invites(state: State<'_, AppState>) -> Res<Vec<GameInvite>>"
    },
    {
      "name": "send_game_invite",
      "signature": "pub async fn send_game_invite(state: State<'_, AppState>, recipient_uuid: String, instance_id: String, server_id: Option<i64>) -> Res<()>"
    },
    {
      "name": "respond_game_invite",
      "signature": "pub async fn respond_game_invite(state: State<'_, AppState>, invite_id: String, accept: bool) -> Res<()>"
    },
    {
      "name": "get_user_profile",
      "signature": "pub async fn get_user_profile(state: State<'_, AppState>, uuid: String) -> Res<UserProfileView>"
    },
    {
      "name": "update_my_profile",
      "signature": "pub async fn update_my_profile( state: State<'_, AppState>, bio: String, banner_url: Option<String>, custom_badge: Option<String>, featured_achievement_id: Option<String>, accent_color: Option<String>, ) -> Res<UserProfileView>"
    },
    {
      "name": "get_user_posts",
      "signature": "pub async fn get_user_posts(state: State<'_, AppState>, user_uuid: Option<String>) -> Res<Vec<UserPost>>"
    },
    {
      "name": "create_user_post",
      "signature": "pub async fn create_user_post(state: State<'_, AppState>, content: String, image_url: Option<String>) -> Res<UserPost>"
    },
    {
      "name": "like_user_post",
      "signature": "pub async fn like_user_post(state: State<'_, AppState>, post_id: i64) -> Res<bool>"
    },
    {
      "name": "map_action",
      "signature": "pub async fn map_action(state: State<'_, AppState>, server_id: i64, action: String, target_uuid: String, text: Option<String>) -> Res<()>"
    },
    {
      "name": "guild_send_invite",
      "signature": "pub async fn guild_send_invite(state: State<'_, AppState>, guild_id: String, target_uuid: String) -> Res<serde_json::Value>"
    },
    {
      "name": "notifications_list",
      "signature": "pub async fn notifications_list(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "notifications_mark",
      "signature": "pub async fn notifications_mark(state: State<'_, AppState>, ids: Option<Vec<i64>>, clear: Option<bool>) -> Res<()>"
    },
    {
      "name": "market_listings",
      "signature": "pub async fn market_listings(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value>"
    },
    {
      "name": "market_mine",
      "signature": "pub async fn market_mine(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value>"
    },
    {
      "name": "market_act",
      "signature": "pub async fn market_act( state: State<'_, AppState>, server_id: i64, action: String, listing_id: i64, amount: Option<f64>, ) -> Res<serde_json::Value>"
    },
    {
      "name": "casino_get",
      "signature": "pub async fn casino_get(state: State<'_, AppState>, server_id: i64, path: String) -> Res<serde_json::Value>"
    },
    {
      "name": "casino_post",
      "signature": "pub async fn casino_post( state: State<'_, AppState>, server_id: i64, path: String, body: Option<serde_json::Value>, ) -> Res<serde_json::Value>"
    },
    {
      "name": "vault_get",
      "signature": "pub async fn vault_get(state: State<'_, AppState>, server_id: i64) -> Res<serde_json::Value>"
    },
    {
      "name": "faction_upgrades",
      "signature": "pub async fn faction_upgrades(state: State<'_, AppState>, guild_id: String, body: Option<serde_json::Value>) -> Res<serde_json::Value>"
    },
    {
      "name": "vault_post",
      "signature": "pub async fn vault_post(state: State<'_, AppState>, server_id: i64, action: String, body: serde_json::Value) -> Res<serde_json::Value>"
    },
    {
      "name": "board_get",
      "signature": "pub async fn board_get(state: State<'_, AppState>, server_id: i64, path: String) -> Res<serde_json::Value>"
    },
    {
      "name": "board_post",
      "signature": "pub async fn board_post( state: State<'_, AppState>, server_id: i64, path: String, body: Option<serde_json::Value>, ) -> Res<serde_json::Value>"
    },
    {
      "name": "my_guild_invites",
      "signature": "pub async fn my_guild_invites(state: State<'_, AppState>) -> Res<serde_json::Value>"
    },
    {
      "name": "respond_guild_invite",
      "signature": "pub async fn respond_guild_invite(state: State<'_, AppState>, invite_id: i64, accept: bool) -> Res<serde_json::Value>"
    },
    {
      "name": "textures_status",
      "signature": "pub fn textures_status(state: State<'_, AppState>) -> velora_launcher_core::textures::Status"
    },
    {
      "name": "item_textures",
      "signature": "pub fn item_textures(state: State<'_, AppState>, names: Vec<String>) -> std::collections::HashMap<String, String>"
    },
    {
      "name": "mc_sprites",
      "signature": "pub fn mc_sprites(state: State<'_, AppState>, names: Vec<String>) -> std::collections::HashMap<String, String>"
    }
  ]
};

const hostRoot = new URL('../src-tauri/src/', import.meta.url);
const read = relative => readFileSync(new URL(relative, hostRoot), 'utf8');
const sourceFiles = () => [
  ['commands.rs', read('commands.rs')],
  ...readdirSync(new URL('commands/', hostRoot)).filter(name => name.endsWith('.rs')).sort()
    .map(name => ['commands/' + name, read('commands/' + name)]),
];

function definitions(source) {
  const found = [];
  const declarationPattern = /^(?:(?:pub(?:\(super\))? )?(?:async )?fn|(?:pub )?struct|type|mod) (\w+)/gm;
  for (const match of source.matchAll(declarationPattern)) {
    if (match[0].startsWith('mod') && source.slice(match.index).split('\n')[0].trimEnd().endsWith(';')) continue;
    let start = match.index;
    while (start > 0) {
      const previousEnd = start - 1;
      const previousStart = source.lastIndexOf('\n', previousEnd - 1) + 1;
      const previous = source.slice(previousStart, previousEnd).trim();
      if (previous && !previous.startsWith('//') && !previous.startsWith('#[')) break;
      start = previousStart;
    }
    let end;
    if (match[0].startsWith('type')) end = source.indexOf('\n', match.index) + 1;
    else {
      const closing = /^}\r?$/gm;
      closing.lastIndex = match.index;
      const boundary = closing.exec(source);
      assert.ok(boundary, 'missing closing delimiter for ' + match[1]);
      end = boundary.index + boundary[0].length;
    }
    const text = source.slice(start, end);
    const firstDeclaration = text.search(/^(?:#\[|(?:(?:pub(?:\(super\))? )?(?:async )?fn|(?:pub )?struct|type|mod) )/m);
    found.push({ name: match[1], text: text.slice(firstDeclaration) });
  }
  return found;
}

function normalizedSignature(signature) {
  return signature.replace(/\s+/g, ' ').replace(/\(\s+/g, '(').replace(/,?\s+\)/g, ')').trim();
}

function commandSurface(files) {
  return files.flatMap(([path, source]) => definitions(source).filter(item => item.text.includes('#[tauri::command]')).map(item => ({
    name: item.name,
    signature: normalizedSignature(item.text.match(/pub (?:async )?fn [\s\S]*?\{/)[0].slice(0, -1)),
  }))).sort((left, right) => left.name.localeCompare(right.name, 'en'));
}

const expectedSurface = initialSnapshot.commands.map(command => ({ ...command, signature: normalizedSignature(command.signature) }))
  .toSorted((left, right) => left.name.localeCompare(right.name, 'en'));

test('all IPC names, sync/async signatures, argument names/types and return types match pre-extraction snapshot', () => {
  assert.deepEqual(commandSurface(sourceFiles()), expectedSurface);
});

test('registration order and every existing commands:: path remain unchanged', () => {
  const registered = [...read('lib.rs').matchAll(/commands::(\w+)/g)].map(match => match[1]);
  assert.deepEqual(registered, initialSnapshot.registrations);
  assert.deepEqual([...registered].sort(), initialSnapshot.commands.map(command => command.name).sort());
  const root = read('commands.rs');
  const exports = new Map();
  for (const match of root.matchAll(/pub use (\w+)::(?:\{([\s\S]*?)\}|(\w+));/g)) {
    const names = (match[2] ?? match[3]).split(',').map(name => name.trim()).filter(Boolean);
    exports.set(match[1], [...(exports.get(match[1]) ?? []), ...names]);
  }
  for (const [module, names] of exports) {
    const actual = commandSurface([[module, read('commands/' + module + '.rs')]]);
    for (const command of actual) {
      for (const exported of [command.name, '__cmd__' + command.name, '__tauri_command_name_' + command.name]) {
        assert.equal(names.filter(name => name === exported).length, 1, 'missing/duplicate reexport: ' + exported);
      }
    }
  }
  const exportedNames = [...exports.values()].flat();
  for (const command of initialSnapshot.commands) assert.equal(exportedNames.filter(name => name === command.name).length, 1);
});

test('surface detector rejects a renamed command, changed payload type, return type and async mode', () => {
  const files = sourceFiles();
  const targetIndex = files.findIndex(([path]) => path === 'commands/telemetry.rs');
  assert.notEqual(targetIndex, -1);
  const formatted = files.map(([path, source], index) => [path, index === targetIndex
    ? source.replace('record_activity(', 'record_activity(\n    ').replace(') -> Res<()>', ',\n) -> Res<()>')
    : source]);
  assert.deepEqual(commandSurface(formatted), expectedSurface);
  for (const [before, after] of [
    ['fn record_activity', 'fn renamed_activity'],
    ['kind: String', 'kind: Option<String>'],
    ['-> Res<()>', '-> Res<String>'],
    ['pub async fn', 'pub fn'],
  ]) {
    const changed = files.map(([path, source], index) => [path, index === targetIndex ? source.replace(before, after) : source]);
    assert.notDeepEqual(commandSurface(changed), expectedSurface);
  }
});

test('snapshot records dirty-tree source provenance rather than claiming clean HEAD equivalence', () => {
  assert.match(initialSnapshot.provenance.sourceSha256, /^[a-f0-9]{64}$/);
  assert.match(initialSnapshot.provenance.baselineHead, /^[a-f0-9]{40}$/);
  assert.match(initialSnapshot.provenance.note, /Working-tree snapshot/);
  assert.equal(initialSnapshot.provenance.sourcePath, 'launcher/src-tauri/src/commands.rs');
});
