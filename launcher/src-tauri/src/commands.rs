//! Everything the UI can ask the backend to do.

mod panel_client;
use panel_client::{account_api, ok_or_panel_error};

mod telemetry;
pub use telemetry::record_activity;
pub use telemetry::{__cmd__record_activity, __tauri_command_name_record_activity};

mod bootstrap;
pub use bootstrap::{
    __cmd__bootstrap, __cmd__refresh_manifest, __cmd__save_instance_options, __cmd__save_settings, __cmd__select_experience,
    __cmd__set_instance_fov, __cmd__set_panel_url, __cmd__show_window, __tauri_command_name_bootstrap,
    __tauri_command_name_refresh_manifest, __tauri_command_name_save_instance_options, __tauri_command_name_save_settings,
    __tauri_command_name_select_experience, __tauri_command_name_set_instance_fov, __tauri_command_name_set_panel_url,
    __tauri_command_name_show_window,
};
pub use bootstrap::{
    bootstrap, refresh_manifest, save_instance_options, save_settings, select_experience, set_instance_fov, set_panel_url, show_window,
};

mod accounts;
pub use accounts::{
    __cmd__add_offline, __cmd__discord_connection, __cmd__discord_info, __cmd__discord_link_start, __cmd__discord_sign_in_poll,
    __cmd__discord_sign_in_start, __cmd__discord_unlink, __cmd__login_panel, __cmd__register_panel, __cmd__remove_account,
    __cmd__request_password_reset, __cmd__select_account, __cmd__set_username, __tauri_command_name_add_offline,
    __tauri_command_name_discord_connection, __tauri_command_name_discord_info, __tauri_command_name_discord_link_start,
    __tauri_command_name_discord_sign_in_poll, __tauri_command_name_discord_sign_in_start, __tauri_command_name_discord_unlink,
    __tauri_command_name_login_panel, __tauri_command_name_register_panel, __tauri_command_name_remove_account,
    __tauri_command_name_request_password_reset, __tauri_command_name_select_account, __tauri_command_name_set_username,
};
pub use accounts::{
    add_offline, discord_connection, discord_info, discord_link_start, discord_sign_in_poll, discord_sign_in_start, discord_unlink,
    login_panel, register_panel, remove_account, request_password_reset, select_account, set_username,
};

mod profiles;
pub use profiles::{
    __cmd__account_profile, __cmd__apply_skin_profile, __cmd__create_user_post, __cmd__delete_skin, __cmd__delete_skin_profile,
    __cmd__get_skin_profiles, __cmd__get_user_posts, __cmd__get_user_profile, __cmd__like_user_post, __cmd__save_skin_profile,
    __cmd__set_cape, __cmd__set_skin_model, __cmd__update_my_profile, __cmd__upload_skin, __tauri_command_name_account_profile,
    __tauri_command_name_apply_skin_profile, __tauri_command_name_create_user_post, __tauri_command_name_delete_skin,
    __tauri_command_name_delete_skin_profile, __tauri_command_name_get_skin_profiles, __tauri_command_name_get_user_posts,
    __tauri_command_name_get_user_profile, __tauri_command_name_like_user_post, __tauri_command_name_save_skin_profile,
    __tauri_command_name_set_cape, __tauri_command_name_set_skin_model, __tauri_command_name_update_my_profile,
    __tauri_command_name_upload_skin,
};
pub use profiles::{
    account_profile, apply_skin_profile, create_user_post, delete_skin, delete_skin_profile, get_skin_profiles, get_user_posts,
    get_user_profile, like_user_post, save_skin_profile, set_cape, set_skin_model, update_my_profile, upload_skin,
};

mod launch;
pub use launch::{
    __cmd__cancel_launch, __cmd__kill_game, __cmd__launch, __cmd__repair_instance, __tauri_command_name_cancel_launch,
    __tauri_command_name_kill_game, __tauri_command_name_launch, __tauri_command_name_repair_instance,
};
pub use launch::{cancel_launch, kill_game, launch, repair_instance};

mod instances;
pub use instances::{
    __cmd__clear_cache, __cmd__delete_instance_data, __cmd__detect_java, __cmd__instance_local, __cmd__open_file, __cmd__open_folder,
    __cmd__ping_server, __cmd__storage_info, __tauri_command_name_clear_cache, __tauri_command_name_delete_instance_data,
    __tauri_command_name_detect_java, __tauri_command_name_instance_local, __tauri_command_name_open_file,
    __tauri_command_name_open_folder, __tauri_command_name_ping_server, __tauri_command_name_storage_info,
};
pub use instances::{clear_cache, delete_instance_data, detect_java, instance_local, open_file, open_folder, ping_server, storage_info};

mod updates;
pub use updates::{__cmd__check_update, __cmd__install_update, __tauri_command_name_check_update, __tauri_command_name_install_update};
pub use updates::{check_update, install_update};

mod maps;
pub use maps::{
    __cmd__get_leaderboard, __cmd__get_map_info, __cmd__get_map_overlay, __cmd__get_player_stats, __cmd__get_public_servers,
    __cmd__map_action, __tauri_command_name_get_leaderboard, __tauri_command_name_get_map_info, __tauri_command_name_get_map_overlay,
    __tauri_command_name_get_player_stats, __tauri_command_name_get_public_servers, __tauri_command_name_map_action,
};
pub use maps::{get_leaderboard, get_map_info, get_map_overlay, get_player_stats, get_public_servers, map_action};

mod progression;
pub use progression::{
    __cmd__claim_quest, __cmd__equip_collection_item, __cmd__get_my_achievements, __cmd__get_my_collections, __cmd__get_my_level,
    __cmd__get_my_quests, __cmd__get_reward_summaries, __tauri_command_name_claim_quest, __tauri_command_name_equip_collection_item,
    __tauri_command_name_get_my_achievements, __tauri_command_name_get_my_collections, __tauri_command_name_get_my_level,
    __tauri_command_name_get_my_quests, __tauri_command_name_get_reward_summaries,
};
pub use progression::{
    claim_quest, equip_collection_item, get_my_achievements, get_my_collections, get_my_level, get_my_quests, get_reward_summaries,
};

mod guilds;
pub use guilds::{
    __cmd__assign_guild_role, __cmd__claim_guild_chunk, __cmd__create_guild, __cmd__create_guild_post, __cmd__create_guild_relation,
    __cmd__create_guild_role, __cmd__delete_guild_role, __cmd__disband_guild, __cmd__faction_upgrades, __cmd__get_guild_claim_flags,
    __cmd__get_guild_claims, __cmd__get_guild_join_requests, __cmd__get_guild_members, __cmd__get_guild_posts, __cmd__get_guild_relations,
    __cmd__get_guild_roles, __cmd__get_guild_wallet, __cmd__get_guilds, __cmd__get_my_guild, __cmd__get_my_guild_memberships,
    __cmd__guild_send_invite, __cmd__kick_guild_member, __cmd__my_guild_invites, __cmd__rename_guild, __cmd__request_guild_join,
    __cmd__respond_guild_invite, __cmd__respond_guild_join_request, __cmd__respond_guild_relation, __cmd__set_guild_claim_flags,
    __cmd__set_primary_guild, __cmd__transfer_guild_leader, __cmd__transfer_guild_wallet, __cmd__unclaim_guild_chunk, __cmd__update_guild,
    __tauri_command_name_assign_guild_role, __tauri_command_name_claim_guild_chunk, __tauri_command_name_create_guild,
    __tauri_command_name_create_guild_post, __tauri_command_name_create_guild_relation, __tauri_command_name_create_guild_role,
    __tauri_command_name_delete_guild_role, __tauri_command_name_disband_guild, __tauri_command_name_faction_upgrades,
    __tauri_command_name_get_guild_claim_flags, __tauri_command_name_get_guild_claims, __tauri_command_name_get_guild_join_requests,
    __tauri_command_name_get_guild_members, __tauri_command_name_get_guild_posts, __tauri_command_name_get_guild_relations,
    __tauri_command_name_get_guild_roles, __tauri_command_name_get_guild_wallet, __tauri_command_name_get_guilds,
    __tauri_command_name_get_my_guild, __tauri_command_name_get_my_guild_memberships, __tauri_command_name_guild_send_invite,
    __tauri_command_name_kick_guild_member, __tauri_command_name_my_guild_invites, __tauri_command_name_rename_guild,
    __tauri_command_name_request_guild_join, __tauri_command_name_respond_guild_invite, __tauri_command_name_respond_guild_join_request,
    __tauri_command_name_respond_guild_relation, __tauri_command_name_set_guild_claim_flags, __tauri_command_name_set_primary_guild,
    __tauri_command_name_transfer_guild_leader, __tauri_command_name_transfer_guild_wallet, __tauri_command_name_unclaim_guild_chunk,
    __tauri_command_name_update_guild,
};
pub use guilds::{
    assign_guild_role, claim_guild_chunk, create_guild, create_guild_post, create_guild_relation, create_guild_role, delete_guild_role,
    disband_guild, faction_upgrades, get_guild_claim_flags, get_guild_claims, get_guild_join_requests, get_guild_members, get_guild_posts,
    get_guild_relations, get_guild_roles, get_guild_wallet, get_guilds, get_my_guild, get_my_guild_memberships, guild_send_invite,
    kick_guild_member, my_guild_invites, rename_guild, request_guild_join, respond_guild_invite, respond_guild_join_request,
    respond_guild_relation, set_guild_claim_flags, set_primary_guild, transfer_guild_leader, transfer_guild_wallet, unclaim_guild_chunk,
    update_guild,
};

mod social;
pub use social::{
    __cmd__get_direct_messages, __cmd__get_friends, __cmd__get_game_invites, __cmd__notifications_list, __cmd__notifications_mark,
    __cmd__remove_friend, __cmd__respond_friend_request, __cmd__respond_game_invite, __cmd__search_members, __cmd__send_direct_message,
    __cmd__send_friend_request, __cmd__send_game_invite, __tauri_command_name_get_direct_messages, __tauri_command_name_get_friends,
    __tauri_command_name_get_game_invites, __tauri_command_name_notifications_list, __tauri_command_name_notifications_mark,
    __tauri_command_name_remove_friend, __tauri_command_name_respond_friend_request, __tauri_command_name_respond_game_invite,
    __tauri_command_name_search_members, __tauri_command_name_send_direct_message, __tauri_command_name_send_friend_request,
    __tauri_command_name_send_game_invite,
};
pub use social::{
    get_direct_messages, get_friends, get_game_invites, notifications_list, notifications_mark, remove_friend, respond_friend_request,
    respond_game_invite, search_members, send_direct_message, send_friend_request, send_game_invite,
};

mod economy;
pub use economy::{
    __cmd__get_economy_balances, __cmd__get_server_baltop, __cmd__get_transactions, __tauri_command_name_get_economy_balances,
    __tauri_command_name_get_server_baltop, __tauri_command_name_get_transactions,
};
pub use economy::{get_economy_balances, get_server_baltop, get_transactions};

mod market;
pub use market::{
    __cmd__market_act, __cmd__market_listings, __cmd__market_mine, __tauri_command_name_market_act, __tauri_command_name_market_listings,
    __tauri_command_name_market_mine,
};
pub use market::{market_act, market_listings, market_mine};

mod casino;
pub use casino::{
    __cmd__board_get, __cmd__board_post, __cmd__casino_get, __cmd__casino_post, __cmd__vault_get, __cmd__vault_post,
    __tauri_command_name_board_get, __tauri_command_name_board_post, __tauri_command_name_casino_get, __tauri_command_name_casino_post,
    __tauri_command_name_vault_get, __tauri_command_name_vault_post,
};
pub use casino::{board_get, board_post, casino_get, casino_post, vault_get, vault_post};

mod textures;
pub use textures::{
    __cmd__item_textures, __cmd__mc_sprites, __cmd__textures_status, __tauri_command_name_item_textures, __tauri_command_name_mc_sprites,
    __tauri_command_name_textures_status,
};
pub use textures::{item_textures, mc_sprites, textures_status};

/// Commands return `Err(String)`; the UI shows it as-is.
type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// anyhow's alternate format keeps the context chain readable.
fn aerr(e: anyhow::Error) -> String {
    format!("{e:#}")
}
