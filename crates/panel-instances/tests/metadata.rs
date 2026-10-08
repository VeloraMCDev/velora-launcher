use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use velora_panel_instances::{bump_revision, file_stats, get_instance, instance_files, list_instances, unique_slug};

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
        CREATE TABLE instance_files (instance_id TEXT, path TEXT, url TEXT, sha1 TEXT, size INTEGER, origin TEXT, note TEXT);",
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

#[tokio::test]
async fn reads_keep_unknown_opaque_values_nulls_disabled_rows_and_original_order() {
    let pool = memory().await;
    for (id, name, featured, sort) in
        [("later", "Beta", false, 0), ("first", "alpha", false, 0), ("sorted", "Zulu", false, -1), ("featured", "Zulu", true, 20)]
    {
        sqlx::query("INSERT INTO instances(id,name,featured,sort) VALUES(?,?,?,?)")
            .bind(id)
            .bind(name)
            .bind(featured)
            .bind(sort)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE instances SET experience='opaque invalid JSON',source_ref='opaque-source',allowed_groups='opaque-groups',enabled=0 WHERE id='first'")
        .execute(&pool).await.unwrap();
    let rows = list_instances(&pool).await.unwrap();
    assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["featured", "sorted", "first", "later"]);
    let row = get_instance(&pool, "first").await.unwrap().unwrap();
    assert_eq!(row.experience, "opaque invalid JSON");
    assert_eq!(row.source_ref, "opaque-source");
    assert_eq!(row.allowed_groups, "opaque-groups");
    assert_eq!(row.loader, "custom-loader");
    assert!(!row.enabled);
    assert!(row.icon_url.is_none() && row.server.is_none() && row.loader_version.is_none());
    assert!(get_instance(&pool, "missing").await.unwrap().is_none());
    assert!(get_instance(&pool, "' OR 1=1 --").await.unwrap().is_none());
}

#[tokio::test]
async fn file_queries_preserve_instance_isolation_empty_aggregates_url_rules_order_and_nulls() {
    let pool = memory().await;
    let empty = file_stats(&pool, "empty").await.unwrap();
    assert_eq!((empty.count, empty.size, empty.missing), (0, 0, 0));
    for (id, path, url, size) in [
        ("one", "mods/z.jar", "", 5),
        ("one", "mods/a.jar", "https://example.invalid/a", 9),
        ("one", "mods/b.jar", " ", 7),
        ("two", "other", "", 1000),
    ] {
        sqlx::query(
            "INSERT INTO instance_files(instance_id,path,url,sha1,size,origin,note) VALUES(?,?,?,'synthetic-hash',?,'custom-origin',NULL)",
        )
        .bind(id)
        .bind(path)
        .bind(url)
        .bind(size)
        .execute(&pool)
        .await
        .unwrap();
    }
    let stats = file_stats(&pool, "one").await.unwrap();
    assert_eq!((stats.count, stats.size, stats.missing), (3, 21, 1));
    let rows = instance_files(&pool, "one").await.unwrap();
    assert_eq!(rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(), ["mods/a.jar", "mods/b.jar", "mods/z.jar"]);
    assert_eq!(
        serde_json::to_value(&rows[0]).unwrap(),
        serde_json::json!({"path":"mods/a.jar","url":"https://example.invalid/a","sha1":"synthetic-hash","size":9,"origin":"custom-origin","note":null})
    );
    assert_eq!(rows[1].url, " ");
    assert!(instance_files(&pool, "' OR 1=1 --").await.unwrap().is_empty());
}

#[tokio::test]
async fn revision_updates_use_supplied_time_and_pool_without_touching_other_metadata() {
    let a = memory().await;
    let b = memory().await;
    for pool in [&a, &b] {
        sqlx::query("INSERT INTO instances(id,name) VALUES('same','Synthetic')").execute(pool).await.unwrap();
    }
    bump_revision(&a, "same", "supplied-clock").await.unwrap();
    let row = get_instance(&a, "same").await.unwrap().unwrap();
    assert_eq!(row.revision, 8);
    assert_eq!(row.updated_at, "supplied-clock");
    assert_eq!(row.clean_epoch, 3);
    assert_eq!(row.created_at, "created");
    assert_eq!(get_instance(&b, "same").await.unwrap().unwrap().revision, 7);
    // Preserve the legacy missing-row UPDATE behavior; the host owns existence checks.
    bump_revision(&a, "missing", "supplied-clock").await.unwrap();
}

#[tokio::test]
async fn slug_reservations_keep_database_and_retired_storage_collisions_and_legacy_normalization() {
    let pool = memory().await;
    let reserved = tempfile::tempdir().unwrap();
    sqlx::query("INSERT INTO instances(id,name) VALUES('example-world','Synthetic')").execute(&pool).await.unwrap();
    std::fs::create_dir(reserved.path().join("example-world-2")).unwrap();
    std::fs::write(reserved.path().join("example-world-3"), b"synthetic reservation").unwrap();
    assert_eq!(unique_slug(&pool, "Example World!", reserved.path()).await.unwrap(), "example-world-4");
    assert_eq!(unique_slug(&pool, "世界", reserved.path()).await.unwrap(), "instance");
    assert_eq!(unique_slug(&pool, "../../Custom_Name", reserved.path()).await.unwrap(), "custom-name");
    assert_eq!(unique_slug(&pool, &"a".repeat(70), reserved.path()).await.unwrap(), "a".repeat(40));
    assert!(reserved.path().join("example-world-2").is_dir());
}

#[tokio::test]
async fn metadata_survives_real_database_restart_without_reinitializing_values() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("synthetic.db");
    let options = SqliteConnectOptions::new().filename(&path).create_if_missing(true);
    let pool = connect(options.clone()).await;
    schema(&pool).await;
    sqlx::query("INSERT INTO instances(id,name,experience,server,clean_epoch) VALUES('retained','Original','{\"kind\":\"custom\",\"modules\":{\"synthetic\":{\"value\":17}}}','{\"address\":\"synthetic.invalid\"}',19)")
        .execute(&pool).await.unwrap();
    bump_revision(&pool, "retained", "restart-marker").await.unwrap();
    pool.close().await;
    let restarted = connect(options).await;
    let row = get_instance(&restarted, "retained").await.unwrap().unwrap();
    assert_eq!((row.revision, row.clean_epoch), (8, 19));
    assert_eq!(row.updated_at, "restart-marker");
    assert!(row.experience.contains("17"));
    assert_eq!(row.server.as_deref(), Some("{\"address\":\"synthetic.invalid\"}"));
    restarted.close().await;
}

#[tokio::test]
async fn closed_and_incompatible_pools_propagate_failures_instead_of_returning_empty_success() {
    let pool = memory().await;
    let reserved = tempfile::tempdir().unwrap();
    pool.close().await;
    assert!(get_instance(&pool, "missing").await.is_err());
    assert!(list_instances(&pool).await.is_err());
    assert!(file_stats(&pool, "missing").await.is_err());
    assert!(instance_files(&pool, "missing").await.is_err());
    assert!(bump_revision(&pool, "missing", "time").await.is_err());
    assert!(unique_slug(&pool, "name", reserved.path()).await.is_err());
    let wrong = connect(SqliteConnectOptions::new().in_memory(true)).await;
    assert!(get_instance(&wrong, "missing").await.is_err());
    assert!(file_stats(&wrong, "missing").await.is_err());
}
