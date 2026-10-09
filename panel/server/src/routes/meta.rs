//! Version pickers and modpack search, proxied so the browser never needs
//! API keys.

use crate::state::RequestState as State;
use crate::auth::AdminUser;
use crate::error::AppResult;
use crate::packs;
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::Json;
use velora_platform_utils::meta::{self, LoaderVersion};
use velora_shared::Loader;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct McVersion {
    id: String,
    kind: String,
    released: Option<String>,
}

pub async fn minecraft(_: AdminUser, State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let m = meta::mojang_manifest(&state.http).await?;
    let versions: Vec<McVersion> = m.versions.into_iter().map(|v| McVersion { id: v.id, kind: v.kind, released: v.release_time }).collect();
    Ok(Json(serde_json::json!({ "latest": m.latest, "versions": versions })))
}

#[derive(Deserialize)]
pub struct LoaderQuery {
    mc: String,
}

pub async fn loaders(
    _: AdminUser,
    State(state): State<AppState>,
    Path(loader): Path<String>,
    Query(q): Query<LoaderQuery>,
) -> AppResult<Json<Vec<LoaderVersion>>> {
    let loader = Loader::parse(&loader).unwrap_or_default();
    Ok(Json(meta::loader_versions(&state.http, loader, &q.mc).await?))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
}

pub async fn modrinth_search(
    _: AdminUser,
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = state
        .http
        .get("https://api.modrinth.com/v2/search")
        .query(&[("query", q.q.as_str()), ("facets", r#"[["project_type:modpack"]]"#), ("limit", "24"), ("index", "relevance")])
        .send()
        .await
        .map_err(anyhow::Error::from)?
        .error_for_status()
        .map_err(anyhow::Error::from)?;
    Ok(Json(resp.json().await.map_err(anyhow::Error::from)?))
}

pub async fn modrinth_versions(
    _: AdminUser,
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> AppResult<Json<Vec<packs::MrVersion>>> {
    let url = format!("https://api.modrinth.com/v2/project/{}/version", urlencode(&project));
    Ok(Json(velora_platform_utils::http::get_json(&state.http, &url).await?))
}

pub async fn curseforge_search(
    _: AdminUser,
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let path = format!("/mods/search?gameId=432&classId=4471&pageSize=24&sortField=2&sortOrder=desc&searchFilter={}", urlencode(&q.q));
    Ok(Json(packs::cf_raw_get(&state, &path).await?))
}

pub async fn curseforge_files(_: AdminUser, State(state): State<AppState>, Path(mod_id): Path<i64>) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(packs::cf_raw_get(&state, &format!("/mods/{mod_id}/files?pageSize=40")).await?))
}

fn urlencode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}
