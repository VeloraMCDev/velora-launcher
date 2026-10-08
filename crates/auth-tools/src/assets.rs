//! Preserve signing files and every cached texture from a closed, protected backup.
use crate::{check_checkpoint, digest};
use anyhow::{bail, Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::collections::BTreeMap;
use std::fs::{DirBuilder, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

fn regular(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("checkpoint assets must be regular files, without symlinks");
    }
    Ok(())
}
fn textures(source: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let directory = source.join("textures");
    let mut files = BTreeMap::new();
    if !directory.exists() {
        return Ok(files);
    }
    let metadata = std::fs::symlink_metadata(&directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("texture directory must not be a symlink");
    }
    if !directory.canonicalize()?.starts_with(source) {
        bail!("texture directory resolves outside the checkpoint");
    }
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("texture filename must be a hexadecimal hash with .png extension"))?;
        let Some(hash) = name.strip_suffix(".png") else {
            bail!("unexpected file in texture checkpoint");
        };
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("invalid texture filename in checkpoint");
        }
        regular(&entry.path())?;
        if !digest(&entry.path())?.eq_ignore_ascii_case(hash) {
            bail!("texture bytes do not match their content hash");
        }
        files.insert(format!("textures/{name}"), entry.path());
    }
    Ok(files)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn new_directory(path: &Path) -> Result<()> {
    let mut builder = DirBuilder::new();
    builder.recursive(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

/// Copy only verified authority assets to an exclusively created directory.
/// `configured_jwt` carries exact externally configured legacy bytes when no persisted secret was used.
pub async fn copy_new(source: &Path, database: &Path, target: &Path, configured_jwt: Option<&Path>) -> Result<serde_json::Value> {
    let source = source.canonicalize().context("opening closed signing/texture checkpoint directory")?;
    let parent = target.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
    if parent.canonicalize()?.starts_with(&source) {
        bail!("asset destination must be outside the source checkpoint");
    }
    if target.exists() {
        bail!("asset destination must be a new directory");
    }
    check_checkpoint(database)?;
    let database_hash = digest(database)?;
    let options =
        SqliteConnectOptions::new().filename(database).read_only(true).immutable(true).foreign_keys(true).pragma("query_only", "ON");
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
    // Read-only validation rejects mixed stores and never initializes an empty file.
    let database_version = velora_auth_core::schema::validate(&pool).await?;
    let references: Vec<String> =
        sqlx::query_scalar("SELECT skin_hash FROM users WHERE skin_hash IS NOT NULL UNION SELECT hash FROM capes").fetch_all(&pool).await?;
    pool.close().await;
    let mut files = textures(&source)?;
    for hash in references {
        if !files.contains_key(&format!("textures/{hash}.png")) {
            bail!("a referenced skin/cape texture is missing from the checkpoint");
        }
    }
    let signing = source.join("yggdrasil-signing.pem");
    regular(&signing)?;
    let public_key = velora_auth_core::keys::Keys::load(&signing)?.public_der_b64;
    let jwt = configured_jwt.map(Path::to_path_buf).unwrap_or_else(|| source.join("jwt.secret"));
    regular(&jwt)?;
    let secret_size = std::fs::metadata(&jwt)?.len();
    if secret_size == 0 || (configured_jwt.is_none() && secret_size < 32) {
        bail!("checkpoint JWT material is invalid; supply the exact configured legacy secret file if applicable");
    }
    files.insert("yggdrasil-signing.pem".into(), signing);
    files.insert("jwt.secret".into(), jwt);
    let hashes: BTreeMap<String, String> = files.iter().map(|(name, path)| Ok((name.clone(), digest(path)?))).collect::<Result<_>>()?;
    check_checkpoint(database)?;
    if digest(database)? != database_hash {
        bail!("owned database changed during asset preflight");
    }
    // Complete all preflight validation before creating any destination.
    new_directory(target)?;
    new_directory(&target.join("textures"))?;
    for (name, path) in &files {
        let bytes = std::fs::read(path)?;
        let expected = &hashes[name];
        if hex::encode(sha2::Sha256::digest(&bytes)) != *expected {
            bail!("checkpoint asset changed during copy; do not deploy the partial directory");
        }
        let destination = target.join(name);
        write_new(&destination, &bytes)?;
        if digest(&destination)? != *expected || digest(path)? != *expected {
            bail!("asset integrity changed during copy; inspect preserved files");
        }
    }
    if textures(&source)?.keys().ne(files.keys().filter(|name| name.starts_with("textures/"))) {
        bail!("texture inventory changed during copy; inspect preserved files");
    }
    for (name, path) in &files {
        regular(path)?;
        if digest(path)? != hashes[name] || digest(&target.join(name))? != hashes[name] {
            bail!("asset changed before checkpoint completion; inspect preserved files");
        }
    }
    if velora_auth_core::keys::Keys::load(&target.join("yggdrasil-signing.pem"))?.public_der_b64 != public_key {
        bail!("RSA signing identity changed during copy");
    }
    check_checkpoint(database)?;
    if digest(database)? != database_hash {
        bail!("owned database changed during asset copy; inspect preserved files");
    }
    let report = serde_json::json!({"operation":"assets", "schema_version":database_version,
        "authority_database_sha256":database_hash, "jwt_source":if configured_jwt.is_some() {"configured"} else {"persisted"}, "files":hashes});
    write_new(&target.join("assets.manifest.json"), &serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}

use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};
    async fn fixture(root: &Path) -> (PathBuf, PathBuf, Vec<u8>, String) {
        let source = root.join("synthetic-backup");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(source.join("textures")).unwrap();
        let key = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 1024).unwrap();
        std::fs::write(source.join("yggdrasil-signing.pem"), key.to_pkcs8_pem(LineEnding::LF).unwrap().as_bytes()).unwrap();
        let secret = vec![42; 64];
        std::fs::write(source.join("jwt.secret"), &secret).unwrap();
        let bytes = b"synthetic cached texture bytes";
        let hash = hex::encode(sha2::Sha256::digest(bytes));
        std::fs::write(source.join("textures").join(format!("{hash}.png")), bytes).unwrap();
        let database = root.join("synthetic-owned.sqlite");
        let pool = crate::new_store(&database).await.unwrap();
        sqlx::query("INSERT INTO capes(name,hash,created_at) VALUES('SyntheticCape',?,'2026-01-01')")
            .bind(&hash)
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        (source, database, secret, hash)
    }
    #[tokio::test]
    async fn preserves_signing_bytes_identity_referenced_and_cached_textures_without_overwrite() {
        let root = tempfile::tempdir().unwrap();
        let (source, database, secret, _) = fixture(root.path()).await;
        let cached = b"synthetic unreferenced cache bytes";
        let hash = hex::encode(sha2::Sha256::digest(cached));
        std::fs::write(source.join("textures").join(format!("{hash}.png")), cached).unwrap();
        let target = root.path().join("synthetic-assets");
        let before = digest(&database).unwrap();
        let report = copy_new(&source, &database, &target, None).await.unwrap();
        assert_eq!(std::fs::read(target.join("jwt.secret")).unwrap(), secret);
        assert_eq!(
            std::fs::read(target.join("yggdrasil-signing.pem")).unwrap(),
            std::fs::read(source.join("yggdrasil-signing.pem")).unwrap()
        );
        assert_eq!(std::fs::read(target.join("textures").join(format!("{hash}.png"))).unwrap(), cached);
        assert_eq!(digest(&database).unwrap(), before);
        assert_eq!(report["files"].as_object().unwrap().len(), 4);
        assert!(!report.to_string().contains("BEGIN PRIVATE KEY"));
        assert!(copy_new(&source, &database, &target, None).await.is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o700);
            assert_eq!(std::fs::metadata(target.join("jwt.secret")).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(std::fs::metadata(target.join("assets.manifest.json")).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let configured = root.path().join("configured-secret");
        std::fs::write(&configured, b"synthetic-legacy").unwrap();
        let other = root.path().join("configured-assets");
        copy_new(&source, &database, &other, Some(&configured)).await.unwrap();
        assert_eq!(std::fs::read(other.join("jwt.secret")).unwrap(), b"synthetic-legacy");
    }
    #[tokio::test]
    async fn damaged_or_missing_referenced_textures_and_invalid_keys_fail_before_output_creation() {
        let root = tempfile::tempdir().unwrap();
        let (source, database, _, hash) = fixture(root.path()).await;
        let target = root.path().join("rejected");
        let texture = source.join("textures").join(format!("{hash}.png"));
        std::fs::write(&texture, b"changed bytes").unwrap();
        assert!(copy_new(&source, &database, &target, None).await.is_err());
        assert!(!target.exists());
        std::fs::remove_file(&texture).unwrap();
        assert!(copy_new(&source, &database, &target, None).await.is_err());
        assert!(!target.exists());
        std::fs::write(&texture, b"synthetic cached texture bytes").unwrap();
        std::fs::write(source.join("yggdrasil-signing.pem"), b"synthetic-invalid-key").unwrap();
        assert!(copy_new(&source, &database, &target, None).await.is_err());
        assert!(!target.exists());
        #[cfg(unix)]
        {
            std::fs::write(source.join("yggdrasil-signing.pem"), b"synthetic-invalid-key").unwrap();
            let linked = root.path().join("linked-textures");
            std::fs::create_dir(&linked).unwrap();
            let textures = source.join("textures");
            std::fs::remove_file(&texture).unwrap();
            std::fs::remove_dir(&textures).unwrap();
            std::os::unix::fs::symlink(&linked, &textures).unwrap();
            assert!(copy_new(&source, &database, &target, None).await.unwrap_err().to_string().contains("texture directory"));
            assert!(!target.exists());
        }
    }
    #[tokio::test]
    async fn restored_database_and_assets_keep_password_web_game_join_and_certificate_continuity() {
        use velora_auth_core::{admission, certificates, identity_store, password, tokens, ygg_store};
        let root = tempfile::tempdir().unwrap();
        let (source, database, secret, _) = fixture(root.path()).await;
        let options = SqliteConnectOptions::new().filename(&database).foreign_keys(true);
        let original = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
        let hash = password::hash_password("synthetic-original-password").unwrap();
        let uuid = "01234567-89ab-4def-8123-456789abcdef";
        let id = identity_store::insert(
            &original,
            identity_store::NewAccount {
                username: "SyntheticPlayer",
                password_hash: &hash,
                email: None,
                role: "player",
                status: "active",
                created_at: "2026-01-01",
                uuid,
            },
        )
        .await
        .unwrap();
        let web =
            tokens::Keys::new(&secret).issue(tokens::Subject { id, name: "SyntheticPlayer", role: "player", auth_version: 0 }).unwrap();
        let (game, client) = ygg_store::issue_token(&original, id, None).await.unwrap();
        ygg_store::join(&original, "synthetic-server", id, None, "2000-01-01").await.unwrap();
        let signing = velora_auth_core::keys::Keys::load(&source.join("yggdrasil-signing.pem")).unwrap();
        let certificate = certificates::certify(&signing, &certificates::new_player_key().unwrap(), uuid).unwrap();
        certificates::store(&original, id, &certificate).await.unwrap();
        sqlx::query("CREATE TABLE private_gameplay(id INTEGER)").execute(&original).await.unwrap();
        original.close().await;
        let restored_database = root.path().join("restored-authority.sqlite");
        crate::import_new(&database, &restored_database).await.unwrap();
        let restored_assets = root.path().join("restored-assets");
        copy_new(&source, &restored_database, &restored_assets, None).await.unwrap();
        let options = SqliteConnectOptions::new().filename(&restored_database).foreign_keys(true);
        let restored = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
        let account = identity_store::find_by_id(&restored, id).await.unwrap().unwrap();
        assert_eq!(account.uuid, uuid);
        assert!(password::verify_password("synthetic-original-password", &account.password_hash));
        let keys = tokens::Keys::new(&std::fs::read(restored_assets.join("jwt.secret")).unwrap());
        let claims = keys.verify(&web).unwrap();
        admission::session_admission(
            claims.version,
            Some(admission::AccountStatus { status: &account.status, auth_version: account.auth_version }),
        )
        .unwrap();
        assert_eq!(ygg_store::token_user(&restored, &game, Some(&client)).await.unwrap().unwrap().uuid, uuid);
        assert_eq!(ygg_store::joined(&restored, "synthetic-server", "2000-01-01").await.unwrap().unwrap().0, id);
        let restored_certificate = certificates::load(&restored, id).await.unwrap().unwrap();
        assert_eq!(restored_certificate.private_pem, certificate.private_pem);
        assert_eq!(restored_certificate.public_pem, certificate.public_pem);
        assert_eq!(restored_certificate.signature_v1, certificate.signature_v1);
        assert_eq!(restored_certificate.signature_v2, certificate.signature_v2);
        let restored_signing = velora_auth_core::keys::Keys::load(&restored_assets.join("yggdrasil-signing.pem")).unwrap();
        assert_eq!(restored_signing.sign_b64(b"synthetic continuity"), signing.sign_b64(b"synthetic continuity"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_master WHERE name='private_gameplay'")
                .fetch_one(&restored)
                .await
                .unwrap(),
            0
        );
        sqlx::query("UPDATE users SET auth_version=1,status='disabled' WHERE id=?").bind(id).execute(&restored).await.unwrap();
        let disabled = identity_store::find_by_id(&restored, id).await.unwrap().unwrap();
        assert!(admission::session_admission(
            claims.version,
            Some(admission::AccountStatus { status: &disabled.status, auth_version: disabled.auth_version })
        )
        .is_err());
        assert!(ygg_store::token_user(&restored, &game, Some(&client)).await.unwrap().is_none());
    }
}
