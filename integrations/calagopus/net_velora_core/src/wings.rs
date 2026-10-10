//! The few Wings file and state operations the extension needs. Each call goes to the node that hosts the server.
use axum::http::StatusCode;
use shared::{State, models::server::Server};
use wings_api::client::ApiHttpError;

/// Names of the entries directly inside `directory`; empty when the directory does not exist yet.
pub async fn list(state: &State, server: &Server, directory: &str) -> Result<Vec<String>, anyhow::Error> {
    let result = server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .get_servers_server_files_list(
            server.uuid,
            &wings_api::servers_server_files_list::get::Query {
                directory: Some(directory.into()),
                per_page: Some(1000),
                page: Some(1),
                ..Default::default()
            },
        )
        .await;
    match result {
        Ok(listing) => Ok(listing
            .entries
            .iter()
            .filter_map(|entry| serde_json::to_value(entry).ok()?.get("name")?.as_str().map(str::to_owned))
            .collect()),
        Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

/// SHA-256 of a file in the server's filesystem, or `None` if it does not exist.
pub async fn sha256(state: &State, server: &Server, file: &str) -> Result<Option<String>, anyhow::Error> {
    let algorithm: wings_api::Algorithm = serde_json::from_value(serde_json::json!("sha256"))?;
    let result = server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .get_servers_server_files_fingerprints(
            server.uuid,
            &wings_api::servers_server_files_fingerprints::get::Query {
                algorithm: Some(algorithm),
                files: Some(vec![file.into()]),
                ..Default::default()
            },
        )
        .await;
    match result {
        Ok(hashes) => Ok(hashes
            .fingerprints
            .first()
            .and_then(|hash| serde_json::to_value(&hash.1).ok())
            .and_then(|value| value.as_str().map(|s| s.to_ascii_lowercase()))),
        Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

/// Creates `name` inside `root`. An existing directory is fine.
pub async fn create_directory(state: &State, server: &Server, root: &str, name: &str) {
    let result = async {
        server
            .node
            .fetch_cached(&state.database)
            .await?
            .api_client(&state.database)
            .await?
            .post_servers_server_files_create_directory(
                server.uuid,
                &wings_api::servers_server_files_create_directory::post::RequestBody { root: root.into(), name: name.into() },
            )
            .await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if let Err(err) = result {
        tracing::debug!(%err, root, name, "create directory failed (it probably exists)");
    }
}

/// Has the node download `url` into `root` as `name` and waits until it is done.
pub async fn pull(state: &State, server: &Server, root: &str, url: &str, name: &str) -> Result<(), anyhow::Error> {
    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_files_pull(
            server.uuid,
            &wings_api::servers_server_files_pull::post::RequestBody {
                root: root.into(),
                url: url.into(),
                file_name: Some(name.into()),
                use_header: false,
                foreground: true,
            },
        )
        .await?;
    Ok(())
}

/// Writes `contents` to `file`, replacing it.
pub async fn write(state: &State, server: &Server, file: &str, contents: String) -> Result<(), anyhow::Error> {
    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_files_write(
            server.uuid,
            wings_api::client::AsyncRequestReader::new(std::io::Cursor::new(contents.into_bytes())),
            &wings_api::servers_server_files_write::post::Query { file: Some(file.into()), ..Default::default() },
        )
        .await?;
    Ok(())
}

/// Deletes `files` (names) inside `root`.
pub async fn delete(state: &State, server: &Server, root: &str, files: Vec<String>) -> Result<(), anyhow::Error> {
    if files.is_empty() {
        return Ok(());
    }
    server
        .node
        .fetch_cached(&state.database)
        .await?
        .api_client(&state.database)
        .await?
        .post_servers_server_files_delete(
            server.uuid,
            &wings_api::servers_server_files_delete::post::RequestBody {
                root: root.into(),
                files: files.into_iter().map(Into::into).collect(),
            },
        )
        .await?;
    Ok(())
}

/// The server's power state as reported by its node ("offline", "starting", "running", "stopping"), if known.
pub async fn power_state(state: &State, server: &Server) -> Result<Option<String>, anyhow::Error> {
    let usages = server.node.fetch_cached(&state.database).await?.fetch_server_resources(&state.database).await?;
    Ok(usages
        .get(&server.uuid)
        .and_then(|usage| serde_json::to_value(usage).ok())
        .and_then(|value| value.get("state")?.as_str().map(str::to_owned)))
}
