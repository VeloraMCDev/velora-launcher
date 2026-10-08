//! Original platform mutation SQL. Caller enforces authorization/validation and supplies the pool and clock.
use sqlx::SqlitePool;

#[derive(Debug, Clone, Default)]
pub struct InstanceWrite {
    pub name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub logo_url: Option<String>,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub visibility: String,
    pub allowed_groups: String,
    pub memory_min: i64,
    pub memory_max: i64,
    pub jvm_args: String,
    pub server: Option<String>,
    pub featured: bool,
    pub enabled: bool,
    pub sort: i64,
}

pub async fn create_instance(pool: &SqlitePool, id: &str, input: &InstanceWrite, updated_at: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO instances (id, name, description, icon_url, banner_url, logo_url, mc_version, loader, loader_version, source_kind, source_label,
         visibility, allowed_groups, memory_min, memory_max, jvm_args, server, featured, enabled, sort, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'vanilla', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.icon_url)
    .bind(&input.banner_url)
    .bind(&input.logo_url)
    .bind(&input.mc_version)
    .bind(&input.loader)
    .bind(&input.loader_version)
    .bind(format!("Minecraft {}", input.mc_version))
    .bind(&input.visibility)
    .bind(&input.allowed_groups)
    .bind(input.memory_min)
    .bind(input.memory_max)
    .bind(&input.jvm_args)
    .bind(&input.server)
    .bind(input.featured)
    .bind(input.enabled)
    .bind(input.sort)
    .bind(updated_at)
    .bind(updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_instance(pool: &SqlitePool, id: &str, input: &InstanceWrite, label: &str, updated_at: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE instances SET name = ?, description = ?, icon_url = ?, banner_url = ?, logo_url = ?, mc_version = ?, loader = ?, loader_version = ?,
         source_label = ?, visibility = ?, allowed_groups = ?, memory_min = ?, memory_max = ?, jvm_args = ?, server = ?, featured = ?,
         enabled = ?, sort = ?, revision = revision + 1, updated_at = ? WHERE id = ?",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.icon_url.as_deref().filter(|s| !s.is_empty()))
    .bind(input.banner_url.as_deref().filter(|s| !s.is_empty()))
    .bind(input.logo_url.as_deref().filter(|s| !s.is_empty()))
    .bind(&input.mc_version)
    .bind(&input.loader)
    .bind(&input.loader_version)
    .bind(label)
    .bind(&input.visibility)
    .bind(&input.allowed_groups)
    .bind(input.memory_min)
    .bind(input.memory_max)
    .bind(&input.jvm_args)
    .bind(&input.server)
    .bind(input.featured)
    .bind(input.enabled)
    .bind(input.sort)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_instance(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM game_servers WHERE instance_id=?").bind(id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM instances WHERE id = ?").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn clean_update(pool: &SqlitePool, id: &str, updated_at: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE instances SET clean_epoch = clean_epoch + 1, revision = revision + 1, updated_at = ? WHERE id = ?")
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_icon(pool: &SqlitePool, id: &str, icon: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE instances SET icon_url = ? WHERE id = ?").bind(icon).bind(id).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct UploadedFile<'a> {
    pub path: &'a str,
    pub filename: &'a str,
    pub url: &'a str,
    pub sha1: &'a str,
    pub size: i64,
}

pub async fn record_upload(pool: &SqlitePool, id: &str, file: UploadedFile<'_>) -> Result<(), sqlx::Error> {
    let UploadedFile { path, filename, url, sha1, size } = file;
    sqlx::query("DELETE FROM instance_files WHERE instance_id = ? AND url = '' AND path LIKE ?")
        .bind(id)
        .bind(format!("%/{filename}"))
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO instance_files (instance_id, path, url, sha1, size, origin) VALUES (?, ?, ?, ?, ?, 'upload')
         ON CONFLICT(instance_id, path) DO UPDATE SET url = excluded.url, sha1 = excluded.sha1, size = excluded.size, origin = 'upload', note = NULL",
    )
    .bind(id)
    .bind(path)
    .bind(url)
    .bind(sha1)
    .bind(size)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_file(pool: &SqlitePool, id: &str, path: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM instance_files WHERE instance_id = ? AND path = ?").bind(id).bind(path).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct DistributionFile<'a> {
    pub path: &'a str,
    pub url: &'a str,
    pub sha1: &'a str,
    pub size: u64,
    pub origin: &'a str,
    pub note: Option<&'a str>,
}
#[derive(Debug)]
pub struct PackReplacement<'a> {
    pub mc_version: &'a str,
    pub loader: &'a str,
    pub loader_version: Option<&'a str>,
    pub files: &'a [DistributionFile<'a>],
}

pub async fn replace_pack(
    pool: &SqlitePool,
    instance_id: &str,
    info: &PackReplacement<'_>,
    kind: &str,
    label: &str,
    source_ref: &str,
    clock: impl FnOnce() -> String + Send,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM instance_files WHERE instance_id = ? AND origin != 'upload'").bind(instance_id).execute(&mut *tx).await?;
    for f in info.files {
        sqlx::query(
            "INSERT INTO instance_files (instance_id, path, url, sha1, size, origin, note) VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(instance_id, path) DO UPDATE SET url = excluded.url, sha1 = excluded.sha1, size = excluded.size, origin = excluded.origin, note = excluded.note",
        )
        .bind(instance_id)
        .bind(f.path)
        .bind(f.url)
        .bind(f.sha1)
        .bind(f.size as i64)
        .bind(f.origin)
        .bind(f.note)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "UPDATE instances SET mc_version = ?, loader = ?, loader_version = ?, source_kind = ?, source_label = ?, source_ref = ?,
         revision = revision + 1, updated_at = ? WHERE id = ?",
    )
    .bind(info.mc_version)
    .bind(info.loader)
    .bind(info.loader_version)
    .bind(kind)
    .bind(label)
    .bind(source_ref)
    .bind(clock())
    .bind(instance_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
