//! Velora launcher (Tauri shell). The heavy lifting lives in
//! `velora-launcher-core`; this crate wires it to the UI.

mod accounts;
mod commands;
mod game;
mod secrets;
mod settings;
mod state;
mod telemetry;
mod updater;

use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    tauri::Builder::default()
        // A second launch just focuses the existing window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                w.show().ok();
                w.unminimize().ok();
                w.set_focus().ok();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            app.manage(state::AppState::new(data_dir));
            // The window starts hidden and the UI reveals it once painted (no
            // white flash). If the UI never does, show it anyway.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(4));
                if let Some(w) = handle.get_webview_window("main") {
                    if !w.is_visible().unwrap_or(true) {
                        tracing::warn!("UI didn't signal ready; showing window");
                        w.show().ok();
                    }
                }
            });
            // Returning players: make sure the textures exist (and are rebuilt if the folder was lost or the game version changed).
            game::refresh_textures(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::record_activity,
            commands::refresh_manifest,
            commands::set_panel_url,
            commands::save_settings,
            commands::select_experience,
            commands::save_instance_options,
            commands::set_instance_fov,
            commands::login_panel,
            commands::discord_sign_in_start,
            commands::discord_sign_in_poll,
            commands::request_password_reset,
            commands::discord_connection,
            commands::discord_info,
            commands::map_action,
            commands::guild_send_invite,
            commands::my_guild_invites,
            commands::market_listings,
            commands::market_mine,
            commands::market_act,
            commands::casino_get,
            commands::casino_post,
            commands::board_get,
            commands::board_post,
            commands::vault_get,
            commands::faction_upgrades,
            commands::vault_post,
            commands::textures_status,
            commands::item_textures,
            commands::mc_sprites,
            commands::notifications_list,
            commands::notifications_mark,
            commands::respond_guild_invite,
            commands::discord_link_start,
            commands::discord_unlink,
            commands::register_panel,
            commands::add_offline,
            commands::account_profile,
            commands::upload_skin,
            commands::set_skin_model,
            commands::delete_skin,
            commands::set_cape,
            commands::get_skin_profiles,
            commands::save_skin_profile,
            commands::delete_skin_profile,
            commands::apply_skin_profile,
            commands::set_username,
            commands::select_account,
            commands::remove_account,
            commands::launch,
            commands::repair_instance,
            commands::cancel_launch,
            commands::kill_game,
            commands::instance_local,
            commands::ping_server,
            commands::open_folder,
            commands::open_file,
            commands::storage_info,
            commands::delete_instance_data,
            commands::clear_cache,
            commands::detect_java,
            commands::check_update,
            commands::install_update,
            commands::show_window,
            commands::get_player_stats,
            commands::get_leaderboard,
            commands::get_public_servers,
            commands::get_map_info,
            commands::get_map_overlay,
            commands::get_my_level,
            commands::get_my_quests,
            commands::get_reward_summaries,
            commands::claim_quest,
            commands::get_my_achievements,
            commands::get_my_collections,
            commands::equip_collection_item,
            commands::get_guilds,
            commands::get_my_guild,
            commands::get_my_guild_memberships,
            commands::set_primary_guild,
            commands::get_guild_relations,
            commands::create_guild_relation,
            commands::respond_guild_relation,
            commands::request_guild_join,
            commands::get_guild_join_requests,
            commands::respond_guild_join_request,
            commands::get_guild_roles,
            commands::create_guild_role,
            commands::assign_guild_role,
            commands::kick_guild_member,
            commands::transfer_guild_leader,
            commands::delete_guild_role,
            commands::update_guild,
            commands::get_guild_claim_flags,
            commands::set_guild_claim_flags,
            commands::create_guild,
            commands::get_guild_claims,
            commands::rename_guild,
            commands::disband_guild,
            commands::claim_guild_chunk,
            commands::unclaim_guild_chunk,
            commands::get_guild_posts,
            commands::create_guild_post,
            commands::get_guild_members,
            commands::get_guild_wallet,
            commands::transfer_guild_wallet,
            commands::get_friends,
            commands::send_friend_request,
            commands::respond_friend_request,
            commands::remove_friend,
            commands::search_members,
            commands::get_economy_balances,
            commands::get_server_baltop,
            commands::get_transactions,
            commands::get_direct_messages,
            commands::send_direct_message,
            commands::get_game_invites,
            commands::send_game_invite,
            commands::respond_game_invite,
            commands::get_user_profile,
            commands::update_my_profile,
            commands::get_user_posts,
            commands::create_user_post,
            commands::like_user_post,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the launcher");
}
