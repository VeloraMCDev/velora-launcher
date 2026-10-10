//! Velora Map endpoints: ingest from game servers, and what launchers and the admin panel load.

use super::servers::{get_server, GameServer};
use crate::auth::{AdminUser, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::state::RequestState as State;
use crate::worldmap::{dim_slug, parse_tiles, LivePlayer, MAX_ZOOM, TILE_SIZE, TOKEN_TTL};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

const PLAYER_INTERVAL_MS: u64 = 2000;

fn require_enabled(enabled: bool) -> AppResult<()> {
    if enabled {
        Ok(())
    } else {
        Err(AppError::forbidden("The map is turned off for this server. Switch it on under Servers in the admin panel."))
    }
}

// ---------------------------------------------------------------------------
// Game server -> panel
// ---------------------------------------------------------------------------

pub async fn game_config(GameServer(server): GameServer, State(state): State<AppState>) -> Json<Value> {
    // What the panel actually holds, so a game server whose cache says "done" can tell when the panel lost or never got tiles.
    let have: serde_json::Map<String, Value> = state.worldmap.summary(server.id)["dimensions"]
        .as_array()
        .map(|dims| dims.iter().filter_map(|d| Some((d["slug"].as_str()?.to_string(), d["tiles"].clone()))).collect())
        .unwrap_or_default();
    Json(json!({
        "have": have,
        "enabled": server.map_enabled,
        "epoch": state.worldmap.epoch(server.id),
        "tile_size": TILE_SIZE,
        "max_zoom": MAX_ZOOM,
        "player_interval_ms": PLAYER_INTERVAL_MS,
    }))
}

#[derive(Deserialize)]
pub struct DimQuery {
    dim: String,
}

pub async fn game_tiles(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Query(q): Query<DimQuery>,
    body: Bytes,
) -> AppResult<Json<Value>> {
    require_enabled(server.map_enabled)?;
    let slug = dim_slug(&q.dim).ok_or_else(|| AppError::bad_request("unknown dimension"))?;
    let tiles = parse_tiles(&body).map_err(AppError::bad_request)?;
    let map = state.worldmap.clone();
    let id = server.id;
    let stored = tokio::task::spawn_blocking(move || {
        let stored = map.store_tiles(id, &slug, &tiles)?;
        // Keep every zoomed-out level in step with what just arrived, whatever the game server did or did not send.
        let fresh: Vec<(i32, i32)> = tiles.iter().filter(|t| t.zoom == 0).map(|t| (t.x, t.y)).collect();
        if !fresh.is_empty() {
            map.build_pyramid(id, &slug, &fresh);
        }
        Ok::<_, std::io::Error>(stored)
    })
    .await
    .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok(Json(json!({ "stored": stored })))
}

#[derive(Deserialize)]
pub struct PlayersBody {
    players: Vec<LivePlayer>,
}

pub async fn game_players(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(b): Json<PlayersBody>,
) -> AppResult<Json<Value>> {
    require_enabled(server.map_enabled)?;
    state.worldmap.set_players(server.id, b.players);
    Ok(Json(json!({ "ok": true })))
}

/// Claims and pins, already drawn into shapes by the game server. Stored as is, size-limited.
pub async fn game_overlay(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(overlay): Json<Value>,
) -> AppResult<Json<Value>> {
    require_enabled(server.map_enabled)?;
    let ok = overlay.is_object() && overlay.get("claims").is_none_or(|c| c.is_array()) && overlay.get("pins").is_none_or(|p| p.is_array());
    if !ok || overlay.to_string().len() > 4 * 1024 * 1024 {
        return Err(AppError::bad_request("overlay must be an object with claims and pins lists (up to 4 MiB)"));
    }
    state.worldmap.set_overlay(server.id, overlay);
    Ok(Json(json!({ "ok": true })))
}

pub async fn game_diagnostics(
    GameServer(server): GameServer,
    State(state): State<AppState>,
    Json(value): Json<Value>,
) -> AppResult<Json<Value>> {
    require_enabled(server.map_enabled)?;
    if !value.is_object() || value.to_string().len() > 4096 {
        return Err(AppError::bad_request("invalid map diagnostics"));
    }
    state.worldmap.set_diagnostics(server.id, value);
    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------------------
// Launcher and admin viewers
// ---------------------------------------------------------------------------

async fn info(state: &AppState, id: i64, enabled: bool) -> Value {
    let summary = state.worldmap.summary(id);
    let dims = summary["dimensions"].clone();
    let has = dims.as_array().is_some_and(|d| !d.is_empty());
    let message = if !enabled {
        "The map is off for this server."
    } else if !has {
        "Waiting for the game server to draw its world. The first tiles appear within a minute or two of the server starting."
    } else {
        ""
    }
    .to_string();
    // Say what the game server itself reports, so an empty map explains itself instead of just waiting.
    let message = match state.worldmap.diagnostics(id) {
        Value::Object(d) if !has && enabled => {
            let stage = d.get("stage").and_then(Value::as_str).unwrap_or("");
            let error = d.get("error").and_then(Value::as_str).unwrap_or("");
            let error: String = error.chars().take(300).collect();
            let detail = [stage, error.as_str()].iter().filter(|t| !t.is_empty()).copied().collect::<Vec<_>>().join(". ");
            if detail.is_empty() {
                message
            } else {
                format!("{message} The game server reports: {detail}")
            }
        }
        _ => message,
    };
    json!({
        "enabled": enabled,
        "ready": enabled && has,
        "message": message,
        "tile_size": TILE_SIZE,
        "max_zoom": MAX_ZOOM,
        "tile_base": state.instance_id.as_ref().map(|instance| format!("/api/experience-map/{instance}/{id}")).unwrap_or_else(|| format!("/api/map/{id}")),
        "token": if enabled && has { json!(state.worldmap.issue_token(id)) } else { Value::Null },
        "token_ttl_secs": TOKEN_TTL.as_secs(),
        "dimensions": dims,
        "players": state.worldmap.live_players(id).len(),
    })
}

/// Lets a launcher decide whether to offer a Live Map button, and gives it what it needs to draw the map.
pub async fn viewer_info(_: AuthUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let server = get_server(&state, id).await?;
    Ok(Json(info(&state, id, server.map_enabled).await))
}

/// Players now, plus the claims and pins the game server last sent.
pub async fn viewer_overlay(_: AuthUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let server = get_server(&state, id).await?;
    require_enabled(server.map_enabled)?;
    let overlay = state.worldmap.overlay(id);
    let mut pins=overlay.get("pins").and_then(Value::as_array).cloned().unwrap_or_default();
    pins.extend(super::physical_shops::pins(&state,id).await?);
    Ok(Json(crate::velora_core::filter_map(&state, json!({
        "players": state.worldmap.live_players(id),
        "claims": overlay.get("claims").cloned().unwrap_or(json!([])),
        "pins": pins,
        "updated": state.worldmap.stats(id).last_overlay_at,
    })).await?))
}

pub async fn admin_status(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    let server = get_server(&state, id).await?;
    let mut v = info(&state, id, server.map_enabled).await;
    v["stats"] = serde_json::to_value(state.worldmap.stats(id))?;
    v["epoch"] = json!(state.worldmap.epoch(id));
    v["roster"] = serde_json::to_value(state.worldmap.live_players(id))?;
    v["diagnostics"] = state.worldmap.diagnostics(id);
    Ok(Json(v))
}

pub async fn admin_reset(_: AdminUser, State(state): State<AppState>, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    get_server(&state, id).await?;
    let map = state.worldmap.clone();
    tokio::task::spawn_blocking(move || map.reset(id))
        .await
        .map_err(|e| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct TokenQuery {
    #[serde(default)]
    t: String,
}

/// `GET /api/map/{server}/{dimension}/{zoom}/{x}/{y}.png?t=<token>`. The token comes from [`viewer_info`], so only signed-in
/// players can load tiles, yet plain `<img>` tags work.
pub async fn tile(
    State(state): State<AppState>,
    Path((id, dim, z, x, file)): Path<(i64, String, u8, i32, String)>,
    Query(q): Query<TokenQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    let not_found = || (StatusCode::NOT_FOUND, "no tile").into_response();
    if !state.worldmap.token_ok(id, &q.t) {
        return (StatusCode::UNAUTHORIZED, "map link expired").into_response();
    }
    let (Some(slug), Some(y)) = (dim_slug(&dim), file.strip_suffix(".png").and_then(|s| s.parse::<i32>().ok())) else { return not_found() };
    if z > MAX_ZOOM {
        return not_found();
    }
    let map = state.worldmap.clone();
    let found = tokio::task::spawn_blocking(move || map.read_tile(id, &slug, z, x, y)).await.ok().flatten();
    let Some((png, modified)) = found else { return not_found() };
    let etag = format!("\"{modified}-{}\"", png.len());
    if headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response();
    }
    let mut resp = (StatusCode::OK, png).into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=30"));
    if let Ok(v) = HeaderValue::from_str(&etag) {
        h.insert(header::ETAG, v);
    }
    resp
}

// ---------------------------------------------------------------------------
// Public map (landing page)
// ---------------------------------------------------------------------------

/// The landing page's map section decides what the public may see. Without an enabled map section, nothing is public.
async fn landing_map(state: &AppState, requested: i64) -> AppResult<(i64, Value)> {
    let cfg = crate::routes::landing::get_config(state).await?;
    let block = cfg.blocks.iter().find(|b| b.block_type == "map" && b.enabled).ok_or_else(|| AppError::not_found("no public map"))?;
    let wanted = block.options.get("server_id").and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0);
    let id: i64 = if wanted > 0 {
        wanted
    } else {
        sqlx::query_scalar("SELECT COALESCE(MIN(id), 0) FROM game_servers WHERE live_map_enabled = 1").fetch_one(&state.db).await?
    };
    if id == 0 || (requested != 0 && requested != id) {
        return Err(AppError::not_found("no public map"));
    }
    let server = get_server(state, id).await?;
    if !server.map_enabled {
        return Err(AppError::not_found("no public map"));
    }
    let flag = |key: &str, default: bool| {
        block.options.get(key).and_then(|v| v.as_bool().or_else(|| v.as_str().map(|s| s == "true"))).unwrap_or(default)
    };
    Ok((id, json!({ "claims": flag("show_claims", true), "pins": flag("show_pins", true), "players": flag("show_players", false) })))
}

/// `GET /api/v1/public/map/{server}`: tile key and dimensions for the landing page map (0 = the server the page chose).
pub async fn public_info(State(state): State<AppState>, Path(requested): Path<i64>) -> AppResult<Json<Value>> {
    let (id, layers) = landing_map(&state, requested).await?;
    let mut v = info(&state, id, true).await;
    v["server_id"] = json!(id);
    v["layers"] = layers;
    Ok(Json(v))
}

pub async fn public_overlay(State(state): State<AppState>, Path(requested): Path<i64>) -> AppResult<Json<Value>> {
    let (id, layers) = landing_map(&state, requested).await?;
    let overlay = state.worldmap.overlay(id);
    let on = |k: &str| layers[k].as_bool().unwrap_or(false);
    Ok(Json(crate::velora_core::filter_map(&state, json!({
        "players": if on("players") { json!(state.worldmap.live_players(id)) } else { json!([]) },
        "claims": if on("claims") { overlay.get("claims").cloned().unwrap_or(json!([])) } else { json!([]) },
        "pins": if on("pins") { overlay.get("pins").cloned().unwrap_or(json!([])) } else { json!([]) },
        "updated": state.worldmap.stats(id).last_overlay_at,
    })).await?))
}

pub async fn experience_tile(
    State(state): State<AppState>,
    Path((_instance, id, dim, z, x, file)): Path<(String, i64, String, u8, i32, String)>,
    Query(q): Query<TokenQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    tile(State(state), Path((id, dim, z, x, file)), Query(q), headers).await
}
