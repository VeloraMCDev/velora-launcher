const fs = require('fs');
function edit(p, fn) { const s=fs.readFileSync(p,'utf8').replace(/\r\n/g,'\n'); fs.writeFileSync(p,fn(s)); }
function rep(s,a,b) { if(!s.includes(a)) throw new Error('Missing: '+a.slice(0,100)); return s.replace(a,b); }
edit('launcher/src-tauri/src/telemetry.rs',s=>rep(rep(s,'let accounts = state.accounts.read().unwrap();','let token = accounts::panel_token(state)?;\n    let accounts = state.accounts.read().unwrap();'),'    let token = accounts::panel_token(state);','').replace('        token,','        token: Some(token),'));
edit('panel/server/src/routes/servers.rs',s=>{
 const start=s.indexOf('fn is_private_or_local'); const end=s.indexOf('async fn verify_launcher_ip',start);
 return s.slice(0,start)+`pub fn ips_match(sess_str: &str, req_str: &str) -> bool {
    match (sess_str.trim().parse::<std::net::IpAddr>(), req_str.trim().parse::<std::net::IpAddr>()) {
        (Ok(a), Ok(b)) => canonical_ip(a) == canonical_ip(b),
        _ => false,
    }
}

`+s.slice(end).replace('assert!(ips_match("127.0.0.1", "::1"))','assert!(!ips_match("127.0.0.1", "::1"))').replace('assert!(ips_match("::1", "127.0.0.1"))','assert!(!ips_match("::1", "127.0.0.1"))').replace('assert!(ips_match("172.18.0.1", "192.168.1.50"))','assert!(!ips_match("172.18.0.1", "192.168.1.50"))').replace('assert!(ips_match("10.0.0.1", "10.0.0.2"))','assert!(!ips_match("10.0.0.1", "10.0.0.2"))').replace('assert!(ips_match("203.0.113.5", "203.0.113.9"))','assert!(!ips_match("203.0.113.5", "203.0.113.9"))');
});
edit('launcher/src/pages/Settings.svelte',s=>rep(s,"{@const isCurrent = profile.skin_model === sp.skin_model && (sp.cape_id === (profile.cape?.id ?? null))}","{@const isCurrent = !!sp.skin_data && sp.skin_data === profile.skin_url && profile.skin_model === sp.skin_model && sp.cape_id === (profile.cape?.id ?? null)}"));
edit('launcher/src/pages/Social.svelte',s=>s.replaceAll('pending_received','pending_incoming').replaceAll('pending_sent','pending_outgoing').replaceAll('.is_online','.online').replaceAll('toUuid: activeChatFriend.uuid','friendUuid: activeChatFriend.uuid').replaceAll('toUuid: inviteTargetFriend.uuid','recipientUuid: inviteTargetFriend.uuid').replaceAll('.from_uuid','.sender_uuid').replaceAll('.from_username','.sender_name').replaceAll('.to_uuid','.recipient_uuid').replaceAll('.server_address','.instance_name'));
edit('launcher/src/lib/types.ts',s=>{
 s=s.replace("'pending_sent' | 'pending_received'", "'pending_outgoing' | 'pending_incoming'").replace('  is_online: boolean;','  online: boolean;');
 s=s.replaceAll('  from_uuid: string;','  sender_uuid: string;').replaceAll('  to_uuid: string;','  recipient_uuid: string;').replace('  from_username: string;','  sender_name: string;').replace('  server_address: string;','  instance_name: string;');
 s=rep(s,'export interface GuildMember {\n  guild_id: string;\n  user_uuid: string;\n  username: string;', 'export interface GuildMember {\n  uuid: string;\n  name: string;').replace('  is_online?: boolean;','  online?: boolean;');
 s=s.replace('export interface GuildPost {\n  id: string;', 'export interface GuildPost {\n  id: number;').replace('export interface UserPost {\n  id: string;', 'export interface UserPost {\n  id: number;');
 s=rep(s,'export interface GuildClaim {\n  id: number;\n  guild_id: string;\n  instance_id: string;', 'export interface GuildClaim {\n  id: number;\n  guild_id: string;\n  server_id: number;');
 s=rep(s,'  global_level: UserLevelInfo;', '  global_level: number;\n  global_xp: number;\n  title: string | null;\n  level_info: UserLevelInfo;');
 return s;
});
edit('launcher/src/pages/Guilds.svelte',s=>s.replaceAll('m.user_uuid','m.uuid').replaceAll('m.username','m.name').replaceAll('m.is_online','m.online').replace(' / {myGuild.max_members} Members',' Members'));
edit('launcher/src/components/PlayerProfileModal.svelte',s=>s.replaceAll('profile.global_level.', 'profile.level_info.').replaceAll('profile.global_level?.', 'profile.level_info?.').replace("bio: editBio.trim() || null", "bio: editBio.trim()").replace("      const liked = await invoke<boolean>('like_user_post', { postId: post.id });\n      post.liked_by_me = liked;\n      post.likes_count = liked ? post.likes_count + 1 : Math.max(0, post.likes_count - 1);", "      await invoke('like_user_post', { postId: post.id });\n      post.likes_count += 1;"));
edit('launcher/src-tauri/src/commands.rs',s=>{
 s=rep(s,'pub async fn send_game_invite(\n', 'pub async fn send_game_invite(\n');
 const a=s.indexOf('pub async fn send_game_invite('), b=s.indexOf('#[tauri::command]',a);
 let seg=s.slice(a,b).replace(') -> Res<GameInvite>',') -> Res<()>').replace('    resp.json().await.map_err(err)','    Ok(())'); s=s.slice(0,a)+seg+s.slice(b);
 s=s.replace('json!({ "accept": accept })','json!({ "action": if accept { "accept" } else { "decline" } })');
 const c=s.indexOf('pub async fn update_my_profile('), d=s.indexOf('#[tauri::command]',c);
 seg=s.slice(c,d).replace('    resp.json().await.map_err(err)', '    account_api(&state, reqwest::Method::GET, "/auth/me").await?\n        .send().await.map_err(err)?\n        .json::<serde_json::Value>().await.map_err(err)\n        .and_then(|v| v["uuid"].as_str().map(str::to_owned).ok_or("missing account UUID".into()))\n        .map(|uuid| (state, uuid))?;');
 // Fetch the updated public profile using the selected account UUID.
 seg=s.slice(c,d).replace('    resp.json().await.map_err(err)', '    let uuid = state.accounts.read().unwrap().active().ok_or("no active account")?.uuid.clone();\n    get_user_profile(state, uuid).await'); s=s.slice(0,c)+seg+s.slice(d);
 s=s.replace('reqwest::Method::POST, "/posts"','reqwest::Method::POST, "/profiles/me/posts"');
 return s;
});
edit('panel/server/src/routes/social.rs',s=>rep(s,'    let rows: Vec<(i64, String, String, String, String, bool, String)>', '    let mut rows: Vec<(i64, String, String, String, String, bool, String)>').replace('ORDER BY id ASC LIMIT 100','ORDER BY id DESC LIMIT 100').replace('    // Mark unread messages sent to me as read','    rows.reverse();\n\n    // Mark unread messages sent to me as read'));
edit('panel/web/src/pages/AchievementEditor.svelte',s=>rep(s,"        stat_type: a.stat_type", "        requirement_type: a.requirement_type || 'stat',\n        stat_type: a.stat_type"));
edit('panel/web/src/lib/types.ts',s=>rep(s,'export interface Achievement {','export interface Achievement {\n  requirement_type?: string;'));
edit('panel/server/src/routes/achievements.rs',s=>{
 const a=s.indexOf('pub async fn admin_update_achievement');
 return s.slice(0,a)+s.slice(a).replace('let req_type = payload.requirement_type.unwrap_or_else(|| "stat".into());','let req_type = payload.requirement_type;').replace('requirement_type = ?,','requirement_type = COALESCE(?, requirement_type),');
});
