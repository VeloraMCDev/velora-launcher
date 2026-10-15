use super::{account_api, err, ok_or_panel_error, Res};
use crate::panel_client::{ErrorPolicy, PanelClient, PanelRequest};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use velora_shared::{Guild, GuildClaim, GuildMember, GuildPost};

// ---------------------------------------------------------------------------
// Guilds & Land Claims
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_guilds(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Vec<Guild>> {
    let mut req = account_api(&state, reqwest::Method::GET, "/guilds").await?;
    if let Some(id) = instance_id {
        req = req.query(&[("instance_id", id)]);
    }
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load factions")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_my_guild(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Option<Guild>> {
    let mut req = account_api(&state, reqwest::Method::GET, "/guilds/my").await?;
    if let Some(id) = instance_id {
        req = req.query(&[("instance_id", id)]);
    }
    let resp = req.panel_send().await.map_err(err)?;
    let resp = match PanelClient::success(resp, ErrorPolicy::Generic).await {
        Ok(response) => response,
        Err(_) => return Ok(None),
    };
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GuildMembership {
    id: String,
    instance_id: String,
    name: String,
    tag: String,
    role: String,
    server_count: i64,
    primary_server_count: i64,
}

#[tauri::command]
pub async fn get_my_guild_memberships(state: State<'_, AppState>, instance_id: Option<String>) -> Res<Vec<GuildMembership>> {
    let mut req = account_api(&state, reqwest::Method::GET, "/guilds/memberships").await?;
    if let Some(id) = instance_id {
        req = req.query(&[("instance_id", id)]);
    }
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("could not load faction memberships")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn set_primary_guild(state: State<'_, AppState>, guild_id: String, server_id: i64) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/primary"))
        .await?
        .json(&serde_json::json!({"server_id":server_id}))
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not set primary faction")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_guild_relations(state: State<'_, AppState>, guild_id: String) -> Res<Vec<serde_json::Value>> {
    let resp =
        account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/relations")).await?.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("could not load faction relations")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn create_guild_relation(
    state: State<'_, AppState>,
    guild_id: String,
    other_guild_id: String,
    relation: String,
    reward_cents: Option<i64>,
    reward_bps: Option<i64>,
    duration_hours: Option<i64>,
) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/relations"))
        .await?
        .json(&serde_json::json!({"other_guild_id":other_guild_id,"relation":relation,"reward_cents":reward_cents,"reward_bps":reward_bps,"duration_hours":duration_hours}))
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not create faction relation")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn respond_guild_relation(
    state: State<'_, AppState>,
    guild_id: String,
    relation_id: i64,
    accept: bool,
    expected_revision: Option<String>,
) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/relations/{relation_id}/respond"))
        .await?
        .json(&serde_json::json!({"accept":accept,"expected_revision":expected_revision}))
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not respond to faction relation")).await.map_err(err)?;
    Ok(())
}

/// A guild's land rules (what visitors may do, PVP, mob spawning…) with the catalogue the page draws.
#[tauri::command]
pub async fn get_guild_claim_flags(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let resp =
        account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/claim-flags")).await?.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("could not load the land rules")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

/// Change some of a guild's land rules. Only the rules in `flags` change.
#[tauri::command]
pub async fn set_guild_claim_flags(state: State<'_, AppState>, guild_id: String, flags: serde_json::Value) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/claim-flags"))
        .await?
        .json(&serde_json::json!({ "flags": flags }))
        .panel_send()
        .await
        .map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("could not change the land rules")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn request_guild_join(state: State<'_, AppState>, guild_id: String, message: String) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/requests")).await?;
    let resp = req.json(&serde_json::json!({"message": message})).panel_send().await.map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not request to join faction")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_guild_join_requests(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/requests")).await?.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("could not load faction requests")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn respond_guild_join_request(state: State<'_, AppState>, guild_id: String, uuid: String, accept: bool) -> Res<()> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/requests/{uuid}/respond")).await?;
    let resp = req.json(&serde_json::json!({"accept": accept})).panel_send().await.map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not review faction request")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_guild_roles(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let resp = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/roles")).await?.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("could not load faction roles")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn create_guild_role(state: State<'_, AppState>, guild_id: String, role: serde_json::Value) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/roles"))
        .await?
        .json(&role)
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not create faction role")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn assign_guild_role(state: State<'_, AppState>, guild_id: String, uuid: String, role: String) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/members/{uuid}/role"))
        .await?
        .json(&serde_json::json!({"role":role}))
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not assign faction role")).await.map_err(err)?;
    Ok(())
}

/// Remove a member (leaders and officers; only the leader can remove officers).
#[tauri::command]
pub async fn kick_guild_member(state: State<'_, AppState>, guild_id: String, uuid: String) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::DELETE, &format!("/guilds/{guild_id}/members/{uuid}"))
        .await?
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not remove that member")).await.map_err(err)?;
    Ok(())
}

/// Hand the guild to another member; the old leader becomes an officer.
#[tauri::command]
pub async fn transfer_guild_leader(state: State<'_, AppState>, guild_id: String, uuid: String) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/leader"))
        .await?
        .json(&serde_json::json!({ "uuid": uuid }))
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not hand over leadership")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn delete_guild_role(state: State<'_, AppState>, guild_id: String, role_id: i64) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::DELETE, &format!("/guilds/{guild_id}/roles/{role_id}"))
        .await?
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not delete faction role")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn update_guild(
    state: State<'_, AppState>,
    guild_id: String,
    description: String,
    motd: String,
    icon_url: String,
    banner_url: String,
) -> Res<()> {
    let resp = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}"))
        .await?
        .json(&serde_json::json!({"description":description,"motd":motd,"icon_url":icon_url,"banner_url":banner_url}))
        .panel_send()
        .await
        .map_err(err)?;
    PanelClient::empty(resp, ErrorPolicy::Fallback("could not update faction")).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn create_guild(
    state: State<'_, AppState>,
    instance_id: String,
    name: String,
    tag: String,
    description: String,
    icon_url: Option<String>,
    banner_url: Option<String>,
) -> Res<Guild> {
    let req = account_api(&state, reqwest::Method::POST, "/guilds").await?;
    let resp = req
        .json(&serde_json::json!({
            "instance_id": instance_id,
            "name": name,
            "tag": tag,
            "description": description,
            "icon_url": icon_url,
            "banner_url": banner_url,
        }))
        .panel_send()
        .await
        .map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to create faction")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_guild_claims(
    state: State<'_, AppState>,
    instance_id: String,
    server_id: i64,
    dimension: Option<String>,
    center_x: i32,
    center_z: i32,
    guild_id: Option<String>,
    radius: Option<i32>,
) -> Res<Vec<GuildClaim>> {
    let dim = dimension.unwrap_or_else(|| "minecraft:overworld".into());
    let mut query = vec![
        ("instance_id", instance_id),
        ("server_id", server_id.to_string()),
        ("dimension", dim),
        ("center_x", center_x.to_string()),
        ("center_z", center_z.to_string()),
    ];
    if let Some(guild) = guild_id.filter(|g| !g.is_empty()) {
        query.push(("guild_id", guild));
    }
    if let Some(radius) = radius {
        query.push(("radius", radius.to_string()));
    }
    let resp = account_api(&state, reqwest::Method::GET, "/guilds/claims/grid").await?.query(&query).panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load claims")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn claim_guild_chunk(
    state: State<'_, AppState>,
    guild_id: String,
    instance_id: String,
    server_id: i64,
    dimension: String,
    chunk_x: i32,
    chunk_z: i32,
) -> Res<GuildClaim> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/claim")).await?;
    let resp = req
        .json(&serde_json::json!({
            "instance_id": instance_id,
            "server_id": server_id,
            "dimension": dimension,
            "chunk_x": chunk_x,
            "chunk_z": chunk_z,
        }))
        .panel_send()
        .await
        .map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to claim chunk")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

/// The guild leader renames their guild (and optionally changes its tag).
#[tauri::command]
pub async fn rename_guild(state: State<'_, AppState>, guild_id: String, name: String, tag: Option<String>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::PUT, &format!("/guilds/{guild_id}/name")).await?;
    let resp = req.json(&serde_json::json!({ "name": name, "tag": tag })).panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to rename the faction")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

/// The guild leader disbands their guild. The reply says how much money from the treasury was paid back.
#[tauri::command]
pub async fn disband_guild(state: State<'_, AppState>, guild_id: String) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::DELETE, &format!("/guilds/{guild_id}")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to disband the faction")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn unclaim_guild_chunk(state: State<'_, AppState>, claim_id: i64) -> Res<()> {
    let req = account_api(&state, reqwest::Method::DELETE, &format!("/guilds/claims/{claim_id}")).await?;
    PanelClient::empty(req.panel_send().await.map_err(err)?, ErrorPolicy::Generic).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
pub async fn get_guild_posts(state: State<'_, AppState>, guild_id: String) -> Res<Vec<GuildPost>> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/posts")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load faction posts")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn create_guild_post(state: State<'_, AppState>, guild_id: String, title: String, content: String) -> Res<GuildPost> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/posts")).await?;
    let resp = req
        .json(&serde_json::json!({
            "title": title,
            "content": content,
        }))
        .panel_send()
        .await
        .map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to post announcement")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_guild_members(state: State<'_, AppState>, guild_id: String) -> Res<Vec<GuildMember>> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/members")).await?;
    let resp = req.panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("unable to load members")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn get_guild_wallet(state: State<'_, AppState>, guild_id: String, server_id: i64) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, &format!("/guilds/{guild_id}/wallet?server_id={server_id}")).await?;
    PanelClient::json(req.panel_send().await.map_err(err)?, ErrorPolicy::Generic).await.map_err(err)
}

#[tauri::command]
pub async fn transfer_guild_wallet(
    state: State<'_, AppState>,
    guild_id: String,
    server_id: i64,
    amount: f64,
    withdraw: bool,
) -> Res<serde_json::Value> {
    let action = if withdraw { "withdraw" } else { "deposit" };
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/wallet/{action}")).await?;
    let resp = req.json(&serde_json::json!({"server_id":server_id,"amount":amount})).panel_send().await.map_err(err)?;
    let resp = PanelClient::success(resp, ErrorPolicy::Fallback("Faction wallet transfer failed")).await.map_err(err)?;
    PanelClient::json(resp, ErrorPolicy::Generic).await.map_err(err)
}

/// Invite a player to a guild you lead or officiate.
#[tauri::command]
pub async fn guild_send_invite(state: State<'_, AppState>, guild_id: String, target_uuid: String) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::POST, &format!("/guilds/{guild_id}/invites")).await?;
    let resp = req.json(&serde_json::json!({ "uuid": target_uuid })).panel_send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to send the invitation").await
}

#[tauri::command]
pub async fn faction_upgrades(state: State<'_, AppState>, guild_id: String, body: Option<serde_json::Value>) -> Res<serde_json::Value> {
    let method = if body.is_some() { reqwest::Method::POST } else { reqwest::Method::GET };
    uuid::Uuid::parse_str(&guild_id).map_err(|_| "invalid faction ID".to_string())?;
    let path = format!("/guilds/{guild_id}/upgrades");
    let mut req = account_api(&state, method, &path).await?;
    if let Some(body) = body {
        req = req.json(&body);
    }
    ok_or_panel_error(req.panel_send().await.map_err(err)?, "unable to access faction upgrades").await
}

#[tauri::command]
pub async fn my_guild_invites(state: State<'_, AppState>) -> Res<serde_json::Value> {
    let req = account_api(&state, reqwest::Method::GET, "/guilds/invites").await?;
    let resp = req.panel_send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to load invitations").await
}

#[tauri::command]
pub async fn respond_guild_invite(state: State<'_, AppState>, invite_id: i64, accept: bool) -> Res<serde_json::Value> {
    let path = format!("/guilds/invites/{invite_id}/{}", if accept { "accept" } else { "decline" });
    let req = account_api(&state, reqwest::Method::POST, &path).await?;
    let resp = req.panel_send().await.map_err(err)?;
    ok_or_panel_error(resp, "unable to answer the invitation").await
}
