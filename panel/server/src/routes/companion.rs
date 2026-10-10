//! Restricted player API for the native companion. Only trusted server tokens
//! may select a UUID; Paper derives it from the connection, never from the mod.
use super::servers::GameServer;
use super::{achievements, casino, contracts, economy, guilds, leveling, notifications, orders, quests, social, worldmap};
use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use axum::{
    extract::{Path, Query},
    Json,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct Request {
    uuid: String,
    request_id: Option<String>,
    operation: String,
    #[serde(default = "empty")]
    args: Value,
}
fn empty() -> Value {
    json!({})
}
fn decode<T: DeserializeOwned>(v: &Value) -> AppResult<T> {
    serde_json::from_value(v.clone()).map_err(|_| AppError::bad_request("Invalid companion arguments"))
}
fn id(v: &Value) -> AppResult<String> {
    v["id"].as_str().filter(|s| !s.is_empty() && s.len() <= 128).map(str::to_owned).ok_or_else(|| AppError::bad_request("Missing id"))
}
fn value<T: serde::Serialize>(v: Json<T>) -> AppResult<Json<Value>> {
    Ok(Json(serde_json::to_value(v.0)?))
}

pub async fn request(GameServer(server): GameServer, State(state): State<AppState>, Json(p): Json<Request>) -> AppResult<Json<Value>> {
    if p.args.to_string().len() > 4096 {
        return Err(AppError::bad_request("Companion request too large"));
    }
    let user = crate::yggdrasil::user_by_uuid(&state, &p.uuid).await?.ok_or_else(|| AppError::unauthorized("Unknown player"))?;
    if user.status != "active" {
        return Err(AppError::forbidden("Account is not active"));
    }
    let mutation = matches!(
        p.operation.as_str(),
        "quest_claim"
            | "friend_request"
            | "notifications_read"
            | "casino_slots"
            | "casino_wheel"
            | "casino_plinko"
            | "casino_daily"
            | "casino_dice"
            | "casino_coinflip"
            | "casino_double"
            | "casino_crash_start"
            | "casino_crash_cashout"
            | "casino_blackjack_start"
            | "casino_blackjack_hit"
            | "casino_blackjack_stand"
            | "casino_blackjack_double"
            | "casino_mines_start"
            | "casino_mines_reveal"
            | "casino_mines_cashout"
            | "casino_roulette"
            | "casino_burst_start"
            | "casino_burst_advance"
            | "casino_burst_cashout"
            | "darknet_buy"
            | "bounty_place"
            | "bet_place"
    );
    let request_id = p.request_id.as_deref().unwrap_or("");
    let fingerprint = format!("{}:{}", p.operation, p.args);
    if mutation {
        if uuid::Uuid::parse_str(request_id).is_err() {
            return Err(AppError::bad_request("A mutation requires a request UUID"));
        }
        let reserved =
            sqlx::query("INSERT OR IGNORE INTO companion_requests(server_id,uuid,request_id,fingerprint,created_at) VALUES(?,?,?,?,?)")
                .bind(server.id)
                .bind(&user.uuid)
                .bind(request_id)
                .bind(&fingerprint)
                .bind(crate::db::now())
                .execute(&state.db)
                .await?
                .rows_affected();
        if reserved == 0 {
            let (saved, response): (String, Option<String>) =
                sqlx::query_as("SELECT fingerprint,response FROM companion_requests WHERE server_id=? AND uuid=? AND request_id=?")
                    .bind(server.id)
                    .bind(&user.uuid)
                    .bind(request_id)
                    .fetch_one(&state.db)
                    .await?;
            if saved != fingerprint {
                return Err(AppError::conflict("Request identity was reused for a different action"));
            }
            return match response {
                Some(raw) => Ok(Json(serde_json::from_str(&raw)?)),
                None => Err(AppError::conflict("This action was already submitted. Check its result before repeating it.")),
            };
        }
    }
    let player_uuid = user.uuid.clone();
    let db = state.db.clone();
    let auth = AuthUser(user);
    let sid = server.id;
    // No arbitrary HTTP paths, admin actions, XP grants, wallet adjustments or
    // caller-selected server/economy scope can pass through this dispatcher.
    let result = dispatch(server, state, auth, &p).await;
    if mutation {
        // A crash between executing and recording leaves a reserved operation,
        // which deliberately cannot execute again. No automatic financial retry.
        if let Ok(Json(ref response)) = result {
            sqlx::query("UPDATE companion_requests SET response=? WHERE server_id=? AND uuid=? AND request_id=?")
                .bind(response.to_string())
                .bind(sid)
                .bind(player_uuid)
                .bind(request_id)
                .execute(&db)
                .await?;
        }
    }
    result
}

async fn dispatch(server: super::servers::ServerRow, state: AppState, auth: AuthUser, p: &Request) -> AppResult<Json<Value>> {
    if let Some(instance) = &state.instance_id {
        let row = crate::store::get_instance(&state, instance).await?;
        let experience: velora_shared::Experience = serde_json::from_str(&row.experience)?;
        let feature = if p.operation.starts_with("casino_") || p.operation.starts_with("bounty_") || p.operation.starts_with("bet_") {
            Some("casino")
        } else if p.operation.starts_with("quest") {
            Some("quests")
        } else if p.operation == "achievements" {
            Some("achievements")
        } else if p.operation == "levels" {
            Some("progression")
        } else if matches!(p.operation.as_str(), "market" | "transactions" | "orders" | "contracts" | "darknet" | "darknet_buy") {
            Some("economy")
        } else if p.operation.starts_with("guild") {
            Some("guilds")
        } else if p.operation == "map" {
            Some("maps")
        } else if p.operation == "casino" || p.operation == "casino_history" {
            Some("casino")
        } else {
            None
        };
        if feature.is_some_and(|f| !experience.enabled(f)) {
            return Err(AppError::forbidden("this feature is disabled for the instance"));
        }
    }
    if let Some(policy)=crate::velora_core::load(&state).await? {
        let module=if p.operation.starts_with("casino"){Some("casino")}else if p.operation.starts_with("darknet"){Some("economy")}else{None};
        if module.is_some_and(|module|!policy.enabled(module)){return Err(AppError::forbidden("this Velora Core module is disabled"));}
    }
    let sid = server.id;
    let mut platform = state.clone();
    platform.db = state.platform_db.clone();
    platform.instance_id = None;
    match p.operation.as_str() {
        "market" => {
            let mut listings = economy::server_market_read(GameServer(server), State(state)).await?.0;
            // Opaque item serialization is for server delivery, never for client previews.
            for listing in &mut listings {
                if let Some(row) = listing.as_object_mut() {
                    row.remove("item_data");
                }
            }
            Ok(Json(json!(listings)))
        }
        "quests" => value(quests::get_my_quests(auth, State(state)).await?),
        "quest_claim" => quests::claim_quest(auth, Path(id(&p.args)?), State(state)).await,
        "achievements" => value(achievements::get_my_achievements(auth, State(state)).await?),
        "levels" => value(leveling::get_my_levels(auth, State(state)).await?),
        "transactions" => value(economy::get_my_transactions(auth, State(state)).await?),
        "friends" => value(social::list_friends(auth, State(platform)).await?),
        "friend_request" => social::send_friend_request(auth, State(platform), Json(decode(&p.args)?)).await,
        "notifications" => notifications::list(auth, State(state), Query(decode(&json!({"limit":40}))?)).await,
        "notifications_read" => notifications::mark_read(auth, State(state), Some(Json(decode(&p.args)?))).await,
        "guild" => {
            value(guilds::get_my_guild(auth, Query(guilds::InstanceQuery { instance_id: Some(server.instance_id) }), State(state)).await?)
        }
        "guilds" => value(guilds::list_guilds(Query(guilds::InstanceQuery { instance_id: Some(server.instance_id) }), State(state)).await?),
        "guild_bank" => {
            let guild = guilds::get_my_guild(
                AuthUser(auth.0.clone()),
                Query(guilds::InstanceQuery { instance_id: Some(server.instance_id.clone()) }),
                State(state.clone()),
            )
            .await?
            .0
            .ok_or_else(|| AppError::forbidden("Guild membership is required"))?;
            guilds::guild_wallet(auth, Path(guild.guild.id), Query(guilds::GuildWalletQuery { server_id: sid }), State(state)).await
        }
        "guild_requests" => {
            super::guild_game::server_guild_manage(
                GameServer(server),
                State(state),
                Json(super::guild_game::ManagePayload {
                    uuid: auth.uuid.clone(),
                    name: auth.username.clone(),
                    action: "requests".into(),
                    target: String::new(),
                    text: String::new(),
                    title: String::new(),
                }),
            )
            .await
        }
        "map" => {
            let mut info = worldmap::viewer_info(AuthUser(auth.0.clone()), State(state.clone()), Path(sid)).await?;
            let mut overlay = worldmap::viewer_overlay(auth, State(state), Path(sid)).await?.0;
            info.0["server_id"] = json!(sid);
            overlay["info"] = info.0;
            Ok(Json(overlay))
        }
        "orders" => orders::orders_board(auth, State(state), Path(sid)).await,
        "contracts" => contracts::contracts_board(auth, State(state), Path(sid)).await,
        "darknet" => super::darknet::player_catalog(auth,State(state),Path(sid)).await,
        "darknet_buy" => super::darknet::player_buy(auth,State(state),Path(sid),Json(decode(&p.args)?)).await,
        "casino" => casino::lobby(auth, State(state), Path(sid)).await,
        "casino_history" => casino::history(auth, State(state), Path(sid)).await,
        "casino_slots" => casino::slots(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_wheel" => casino::wheel(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_plinko" => casino::plinko(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_daily" => casino::daily(auth, State(state), Path(sid)).await,
        "casino_dice" => casino::dice(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_coinflip" => casino::coinflip(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_double" => casino::double(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_crash_start" => casino::crash_start(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_crash_status" => casino::crash_status(auth, State(state), Path(sid)).await,
        "casino_crash_cashout" => casino::crash_cashout(auth, State(state), Path(sid)).await,
        "casino_blackjack_start" => casino::blackjack_start(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_blackjack_hit" => casino::blackjack_hit(auth, State(state), Path(sid)).await,
        "casino_blackjack_stand" => casino::blackjack_stand(auth, State(state), Path(sid)).await,
        "casino_blackjack_double" => casino::blackjack_double(auth, State(state), Path(sid)).await,
        "casino_mines_start" => casino::mines_start(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_mines_reveal" => casino::mines_reveal(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "casino_mines_cashout" => casino::mines_cashout(auth, State(state), Path(sid)).await,
        "casino_roulette" => casino::roulette(auth,State(state),Path(sid),Json(decode(&p.args)?)).await,
        "casino_burst_start" => casino::burst_start(auth,State(state),Path(sid),Json(decode(&p.args)?)).await,
        "casino_burst_advance" => casino::burst_advance(auth,State(state),Path(sid),Json(decode(&p.args)?)).await,
        "casino_burst_cashout" => casino::burst_cashout(auth,State(state),Path(sid),Json(decode(&p.args)?)).await,
        "bounties" => casino::bounties(auth, State(state), Path(sid)).await,
        "betting" => casino::markets(auth, State(state), Path(sid)).await,
        "bounty_place" => casino::place_bounty(auth, State(state), Path(sid), Json(decode(&p.args)?)).await,
        "bet_place" => {
            let market = p.args["market_id"].as_i64().ok_or_else(|| AppError::bad_request("Missing market"))?;
            casino::place_bet(auth, State(state), Path((sid, market)), Json(decode(&p.args)?)).await
        }
        _ => Err(AppError::bad_request("Unsupported companion operation")),
    }
}

// Companion layout management endpoints: HUD layouts designed in the panel, widgets stored as a JSON array.

const MAX_WIDGETS_BYTES: usize = 100 * 1024;

type LayoutRow = (i64, String, String, String, i64, String);
const LAYOUT_COLS: &str = "id, name, description, widgets, is_default, created_at";

fn layout_json((id, name, description, widgets, is_default, created_at): LayoutRow) -> Value {
    json!({
        "id": id,
        "name": name,
        "description": description,
        "widgets": serde_json::from_str::<Value>(&widgets).unwrap_or(json!([])),
        "is_default": is_default != 0,
        "created_at": created_at,
    })
}

fn clean_layout_name(raw: &str) -> AppResult<String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::bad_request("Layout name must be 1-100 characters"));
    }
    Ok(name.to_string())
}

fn clean_widgets(widgets: &Value) -> AppResult<String> {
    if !widgets.is_array() {
        return Err(AppError::bad_request("Widgets must be a list"));
    }
    let text = serde_json::to_string(widgets)?;
    if text.len() > MAX_WIDGETS_BYTES {
        return Err(AppError::bad_request("Layout is too large (max 100KB)"));
    }
    Ok(text)
}

async fn layout_by_id(state: &AppState, id: i64) -> AppResult<Value> {
    let row: Option<LayoutRow> =
        sqlx::query_as(&format!("SELECT {LAYOUT_COLS} FROM companion_layouts WHERE id = ?")).bind(id).fetch_optional(&state.db).await?;
    row.map(layout_json).ok_or_else(|| AppError::not_found("Layout not found"))
}

pub async fn list_layouts(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<Value>> {
    let rows: Vec<LayoutRow> =
        sqlx::query_as(&format!("SELECT {LAYOUT_COLS} FROM companion_layouts ORDER BY is_default DESC, name COLLATE NOCASE"))
            .fetch_all(&state.db)
            .await?;
    Ok(Json(json!({ "layouts": rows.into_iter().map(layout_json).collect::<Vec<_>>() })))
}

#[derive(Deserialize)]
pub struct CreateLayoutBody {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "empty_widgets")]
    widgets: Value,
}

fn empty_widgets() -> Value {
    json!([])
}

pub async fn create_layout(_: AdminUser, State(state): State<AppState>, Json(body): Json<CreateLayoutBody>) -> AppResult<Json<Value>> {
    let name = clean_layout_name(&body.name)?;
    let widgets = clean_widgets(&body.widgets)?;
    // The first layout becomes the default so there is always one to serve.
    let first: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM companion_layouts").fetch_one(&state.db).await?;
    let id = sqlx::query("INSERT INTO companion_layouts (name, description, widgets, is_default, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&name)
        .bind(body.description.trim())
        .bind(&widgets)
        .bind(i64::from(first == 0))
        .bind(crate::db::now())
        .execute(&state.db)
        .await?
        .last_insert_rowid();
    Ok(Json(layout_by_id(&state, id).await?))
}

pub async fn get_layout(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "layout": layout_by_id(&state, id).await? })))
}

#[derive(Deserialize)]
pub struct UpdateLayoutBody {
    name: Option<String>,
    description: Option<String>,
    widgets: Option<Value>,
}

pub async fn update_layout(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateLayoutBody>,
) -> AppResult<Json<Value>> {
    let current = layout_by_id(&state, id).await?;
    let name = match &body.name {
        Some(n) => clean_layout_name(n)?,
        None => current["name"].as_str().unwrap_or_default().to_string(),
    };
    let description =
        body.description.as_deref().map(str::trim).unwrap_or_else(|| current["description"].as_str().unwrap_or_default()).to_string();
    let widgets = match &body.widgets {
        Some(w) => clean_widgets(w)?,
        None => current["widgets"].to_string(),
    };
    sqlx::query("UPDATE companion_layouts SET name = ?, description = ?, widgets = ? WHERE id = ?")
        .bind(name)
        .bind(description)
        .bind(widgets)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(layout_by_id(&state, id).await?))
}

pub async fn delete_layout(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let was_default = layout_by_id(&state, id).await?["is_default"].as_bool().unwrap_or(false);
    sqlx::query("DELETE FROM companion_layouts WHERE id = ?").bind(id).execute(&state.db).await?;
    if was_default {
        // Hand the default to the oldest remaining layout.
        sqlx::query("UPDATE companion_layouts SET is_default = 1 WHERE id = (SELECT MIN(id) FROM companion_layouts)")
            .execute(&state.db)
            .await?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn set_default_layout(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    layout_by_id(&state, id).await?;
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE companion_layouts SET is_default = (id = ?)").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({ "ok": true })))
}
