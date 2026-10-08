use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use velora_panel_instances::{get_instance, instance_files, mutations::*};
async fn connect(options: SqliteConnectOptions) -> SqlitePool {
    sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap()
}
async fn schema(pool: &SqlitePool) {
    // Neutral synthetic metadata schema only; no private migrations or identity tables.
    sqlx::raw_sql(
        "CREATE TABLE instances (
        experience TEXT NOT NULL DEFAULT '{}', id TEXT PRIMARY KEY, name TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '', icon_url TEXT, banner_url TEXT, logo_url TEXT,
        mc_version TEXT NOT NULL DEFAULT 'synthetic', loader TEXT NOT NULL DEFAULT 'custom-loader', loader_version TEXT,
        source_kind TEXT NOT NULL DEFAULT 'custom', source_label TEXT NOT NULL DEFAULT 'Synthetic', source_ref TEXT NOT NULL DEFAULT '{}',
        visibility TEXT NOT NULL DEFAULT 'custom', allowed_groups TEXT NOT NULL DEFAULT '[]',
        memory_min INTEGER NOT NULL DEFAULT 512, memory_max INTEGER NOT NULL DEFAULT 1024,
        jvm_args TEXT NOT NULL DEFAULT '', server TEXT, featured INTEGER NOT NULL DEFAULT 0,
        enabled INTEGER NOT NULL DEFAULT 1, sort INTEGER NOT NULL DEFAULT 0, revision INTEGER NOT NULL DEFAULT 7,
        clean_epoch INTEGER NOT NULL DEFAULT 3, created_at TEXT NOT NULL DEFAULT 'created', updated_at TEXT NOT NULL DEFAULT 'old');
        CREATE TABLE instance_files (instance_id TEXT REFERENCES instances(id) ON DELETE CASCADE, path TEXT, url TEXT, sha1 TEXT, size INTEGER, origin TEXT, note TEXT, UNIQUE(instance_id,path)); CREATE TABLE game_servers(id TEXT PRIMARY KEY, instance_id TEXT REFERENCES instances(id));",
    )
    .execute(pool)
    .await
    .unwrap();
}
async fn memory() -> SqlitePool {
    let pool = connect(SqliteConnectOptions::new().in_memory(true)).await;
    schema(&pool).await;
    pool
}

fn input() -> InstanceWrite {
    InstanceWrite {
        name: "Synthetic".into(),
        description: "description".into(),
        mc_version: "test-version".into(),
        loader: "custom".into(),
        visibility: "groups".into(),
        allowed_groups: "[\"Synthetic\"]".into(),
        memory_min: 512,
        memory_max: 2048,
        enabled: true,
        icon_url: Some(String::new()),
        ..Default::default()
    }
}
#[tokio::test]
async fn registry_mutations_preserve_opaque_fields_and_create_update_asymmetry() {
    let pool = memory().await;
    let mut data = input();
    create_instance(&pool, "one", &data, "created-time").await.unwrap();
    create_instance(&pool, "other", &data, "other-time").await.unwrap();
    sqlx::query("UPDATE instances SET experience='opaque-private',source_kind='modrinth',source_ref='opaque-ref' WHERE id='one'")
        .execute(&pool)
        .await
        .unwrap();
    let row = get_instance(&pool, "one").await.unwrap().unwrap();
    assert_eq!(row.icon_url.as_deref(), Some(""));
    assert_eq!(row.source_label, "Minecraft test-version");
    data.name = " Changed ".into();
    data.server = Some("opaque-server".into());
    data.enabled = false;
    data.loader_version = Some("pinned".into());
    update_instance(&pool, "one", &data, "Original modpack label", "update-time").await.unwrap();
    let row = get_instance(&pool, "one").await.unwrap().unwrap();
    assert_eq!(row.name, " Changed ");
    assert_eq!(row.icon_url, None);
    assert_eq!(row.server.as_deref(), Some("opaque-server"));
    assert_eq!(row.experience, "opaque-private");
    assert_eq!(row.source_kind, "modrinth");
    assert_eq!(row.source_ref, "opaque-ref");
    assert_eq!(row.source_label, "Original modpack label");
    assert_eq!(row.revision, 8);
    assert_eq!(row.clean_epoch, 3);
    assert_eq!(row.created_at, "created-time");
    assert_eq!(row.updated_at, "update-time");
    assert!(!row.enabled);
    clean_update(&pool, "one", "clean-time").await.unwrap();
    set_icon(&pool, "one", "/supplied-icon").await.unwrap();
    let row = get_instance(&pool, "one").await.unwrap().unwrap();
    assert_eq!((row.revision, row.clean_epoch), (9, 4));
    assert_eq!(row.updated_at, "clean-time");
    assert_eq!(row.icon_url.as_deref(), Some("/supplied-icon"));
    assert_eq!(get_instance(&pool, "other").await.unwrap().unwrap().revision, 7);
    assert!(create_instance(&pool, "one", &data, "duplicate").await.is_err());
    assert_eq!(get_instance(&pool, "one").await.unwrap().unwrap().updated_at, "clean-time");
}
#[tokio::test]
async fn uploaded_records_retain_legacy_matching_conflicts_and_isolation() {
    let pool = memory().await;
    for id in ["one", "other"] {
        create_instance(&pool, id, &input(), "t").await.unwrap();
        sqlx::query("INSERT INTO instance_files VALUES(?, 'mods/a.jar','','',1,'curseforge','missing')")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    record_upload(&pool, "one", UploadedFile { path: "mods/a.jar", filename: "a.jar", url: "/files/one/mods/a.jar", sha1: "sha", size: 8 })
        .await
        .unwrap();
    let files = instance_files(&pool, "one").await.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].origin, "upload");
    assert_eq!(files[0].note, None);
    assert_eq!(files[0].size, 8);
    assert_eq!(instance_files(&pool, "other").await.unwrap()[0].url, "");
    record_upload(&pool, "one", UploadedFile { path: "mods/a.jar", filename: "a.jar", url: "new-url", sha1: "new-sha", size: 9 })
        .await
        .unwrap();
    assert_eq!(instance_files(&pool, "one").await.unwrap()[0].sha1, "new-sha");
    delete_file(&pool, "one", "mods/a.jar").await.unwrap();
    assert!(instance_files(&pool, "one").await.unwrap().is_empty());
    assert_eq!(instance_files(&pool, "other").await.unwrap().len(), 1);
}
#[tokio::test]
async fn pack_replacement_keeps_uploads_and_rolls_back_every_sql_write_on_failure() {
    let pool = memory().await;
    create_instance(&pool, "one", &input(), "original").await.unwrap();
    record_upload(&pool, "one", UploadedFile { path: "manual", filename: "manual", url: "/files/manual", sha1: "x", size: 1 })
        .await
        .unwrap();
    sqlx::query("INSERT INTO instance_files VALUES('one','old','old-url','old-sha',2,'override',NULL)").execute(&pool).await.unwrap();
    let files = [DistributionFile { path: "bad", url: "url", sha1: "sha", size: 4, origin: "override", note: None }];
    let info = PackReplacement { mc_version: "new", loader: "fabric", loader_version: Some("1"), files: &files };
    sqlx::raw_sql("CREATE TRIGGER refuse_file BEFORE INSERT ON instance_files WHEN NEW.path='bad' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").execute(&pool).await.unwrap();
    assert!(replace_pack(&pool, "one", &info, "modrinth", "label", "opaque-ref", || "new-time".into()).await.is_err());
    assert_eq!(instance_files(&pool, "one").await.unwrap().iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["manual", "old"]);
    assert_eq!(get_instance(&pool, "one").await.unwrap().unwrap().updated_at, "original");
    sqlx::query("DROP TRIGGER refuse_file").execute(&pool).await.unwrap();
    replace_pack(&pool, "one", &info, "modrinth", "label", "opaque-ref", || "new-time".into()).await.unwrap();
    let row = get_instance(&pool, "one").await.unwrap().unwrap();
    assert_eq!((row.revision, row.clean_epoch), (8, 3));
    assert_eq!(row.source_ref, "opaque-ref");
    assert_eq!(row.mc_version, "new");
    assert_eq!(instance_files(&pool, "one").await.unwrap().iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["bad", "manual"]);
}
#[tokio::test]
async fn deletion_cleans_only_platform_links_and_is_atomic() {
    let pool = memory().await;
    for id in ["one", "other"] {
        create_instance(&pool, id, &input(), "t").await.unwrap();
        sqlx::query("INSERT INTO game_servers VALUES(?,?)").bind(id).bind(id).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        "CREATE TRIGGER refuse_delete BEFORE DELETE ON instances WHEN OLD.id='one' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(delete_instance(&pool, "one").await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM game_servers WHERE instance_id='one'").fetch_one(&pool).await.unwrap(),
        1
    );
    sqlx::query("DROP TRIGGER refuse_delete").execute(&pool).await.unwrap();
    delete_instance(&pool, "one").await.unwrap();
    assert!(get_instance(&pool, "one").await.unwrap().is_none());
    assert!(get_instance(&pool, "other").await.unwrap().is_some());
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM game_servers").fetch_one(&pool).await.unwrap(), 1);
}
#[tokio::test]
async fn mutation_outages_never_report_success() {
    let pool = memory().await;
    pool.close().await;
    assert!(create_instance(&pool, "one", &input(), "t").await.is_err());
    assert!(clean_update(&pool, "one", "t").await.is_err());
    assert!(delete_instance(&pool, "one").await.is_err());
}
