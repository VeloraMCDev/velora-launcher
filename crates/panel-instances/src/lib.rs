//! Neutral instance registry and distribution metadata.
//! Callers supply the platform pool, verified authorization, timestamp and reserved storage namespace.
//! Opaque experience JSON is retained; private projections, presets and policy remain host-owned.
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InstanceRow {
    pub experience: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub source_kind: String,
    pub source_label: String,
    pub source_ref: String,
    pub visibility: String,
    pub allowed_groups: String,
    pub memory_min: i64,
    pub memory_max: i64,
    pub jvm_args: String,
    pub server: Option<String>,
    pub featured: bool,
    pub enabled: bool,
    pub sort: i64,
    pub revision: i64,
    pub clean_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FileStats {
    pub count: u32,
    pub size: u64,
    pub missing: u32,
}

pub async fn file_stats(pool: &SqlitePool, instance_id: &str) -> Result<FileStats, sqlx::Error> {
    let (count, size, missing): (i64, Option<i64>, Option<i64>) =
        sqlx::query_as("SELECT COUNT(*), SUM(size), SUM(CASE WHEN url = '' THEN 1 ELSE 0 END) FROM instance_files WHERE instance_id = ?")
            .bind(instance_id)
            .fetch_one(pool)
            .await?;
    Ok(FileStats { count: count as u32, size: size.unwrap_or(0) as u64, missing: missing.unwrap_or(0) as u32 })
}

pub async fn get_instance(pool: &SqlitePool, id: &str) -> Result<Option<InstanceRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM instances WHERE id = ?").bind(id).fetch_optional(pool).await
}

pub async fn list_instances(pool: &SqlitePool) -> Result<Vec<InstanceRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM instances ORDER BY featured DESC, sort ASC, name COLLATE NOCASE ASC").fetch_all(pool).await
}

pub async fn bump_revision(pool: &SqlitePool, id: &str, updated_at: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE instances SET revision = revision + 1, updated_at = ? WHERE id = ?")
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct FileRow {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: i64,
    pub origin: String,
    pub note: Option<String>,
}

pub async fn instance_files(pool: &SqlitePool, id: &str) -> Result<Vec<FileRow>, sqlx::Error> {
    sqlx::query_as("SELECT path, url, sha1, size, origin, note FROM instance_files WHERE instance_id = ? ORDER BY path")
        .bind(id)
        .fetch_all(pool)
        .await
}

/// URL-safe, human-readable id from a name, made unique.
pub async fn unique_slug(pool: &SqlitePool, name: &str, reserved_storage: &Path) -> Result<String, sqlx::Error> {
    let mut base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    base.truncate(40);
    if base.is_empty() {
        base = "instance".into();
    }
    let mut slug = base.clone();
    let mut n = 2;
    while sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM instances WHERE id = ?").bind(&slug).fetch_one(pool).await? > 0
        || reserved_storage.join(&slug).exists()
    {
        slug = format!("{base}-{n}");
        n += 1;
    }
    Ok(slug)
}

pub mod mutations;
