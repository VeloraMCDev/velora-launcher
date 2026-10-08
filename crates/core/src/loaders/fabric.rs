//! Fabric and Quilt both publish a ready-made launcher profile.

use crate::http;
use crate::meta::{FABRIC_META, QUILT_META};
use crate::paths::Layout;
use crate::version::VersionJson;
use anyhow::{Context, Result};
use scopenet_shared::Loader;

pub async fn profile(client: &reqwest::Client, layout: &Layout, loader: Loader, mc: &str, loader_version: &str) -> Result<VersionJson> {
    let url = match loader {
        Loader::Quilt => format!("{QUILT_META}/versions/loader/{mc}/{loader_version}/profile/json"),
        _ => format!("{FABRIC_META}/versions/loader/{mc}/{loader_version}/profile/json"),
    };
    let cache = layout.cache().join("profiles").join(format!("{}-{mc}-{loader_version}.json", loader.as_str()));
    let v: VersionJson = http::get_json_cached(client, &url, &cache)
        .await
        .with_context(|| format!("fetching {} {loader_version} profile for {mc}", loader.as_str()))?;
    Ok(v)
}
