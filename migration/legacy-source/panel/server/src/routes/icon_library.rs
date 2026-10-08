//! Icon library endpoints: one public SVG per glyph, plus pack list and search for the admin picker.
use crate::state::RequestState as State;
use crate::{auth::AdminUser, error::{AppError, AppResult}, icons, state::AppState};
use axum::{
    extract::{Path, Query},
    http::header,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct SvgQuery {
    color: Option<String>,
}

/// `GET /api/v1/icons/<pack>/<name>.svg?color=ffd700` — public, because the launcher draws these as images.
pub async fn svg(State(state): State<AppState>, Path((pack, file)): Path<(String, String)>, Query(q): Query<SvgQuery>) -> AppResult<impl IntoResponse> {
    let name = file.strip_suffix(".svg").ok_or_else(|| AppError::not_found("Icons are served as .svg"))?.to_string();
    let lib = icons::library(&state.cfg.icons_dir);
    let color = q.color.map(|c| c.trim_start_matches('#').to_string());
    let svg = tokio::task::spawn_blocking(move || lib.svg(&pack, &name, color.as_deref()))
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("No such icon"))?;
    Ok(([(header::CONTENT_TYPE, "image/svg+xml"), (header::CACHE_CONTROL, "public, max-age=86400, immutable")], svg))
}

pub async fn packs(_: AdminUser, State(state): State<AppState>) -> Json<Value> {
    let lib = icons::library(&state.cfg.icons_dir);
    Json(json!({"packs": lib.packs, "total": lib.packs.iter().map(|p| p.total).sum::<usize>()}))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
    pack: Option<String>,
    #[serde(default)]
    offset: usize,
    limit: Option<usize>,
}

pub async fn search(_: AdminUser, State(state): State<AppState>, Query(q): Query<SearchQuery>) -> AppResult<Json<Value>> {
    let lib = icons::library(&state.cfg.icons_dir);
    let limit = q.limit.unwrap_or(120).clamp(1, 300);
    let (total, hits) = tokio::task::spawn_blocking(move || lib.search(&q.q, q.pack.as_deref().filter(|p| !p.is_empty()), q.offset, limit))
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    Ok(Json(json!({"total": total, "icons": hits.into_iter().map(|(pack, name)| json!({"pack": pack, "name": name})).collect::<Vec<_>>()})))
}
