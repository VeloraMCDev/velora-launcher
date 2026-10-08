//! Authority-owned cape records and selection policy. Groups are supplied by the live host adapter.
#[cfg(feature = "sqlite")]
use crate::identity::IdentityRecord;
#[derive(Debug, Clone)]
#[cfg_attr(feature = "sqlite", derive(sqlx::FromRow))]
pub struct CapeRow {
    pub id: i64,
    pub name: String,
    pub hash: String,
    pub visibility: String,
    pub allowed_groups: String,
    pub created_at: String,
}

#[cfg(feature = "sqlite")]
pub async fn find(pool: &sqlx::SqlitePool, id: i64) -> Result<Option<CapeRow>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM capes WHERE id = ?").bind(id).fetch_optional(pool).await
}
#[cfg(feature = "sqlite")]
pub async fn available(pool: &sqlx::SqlitePool, user: &IdentityRecord, groups: &[String]) -> Result<Vec<CapeRow>, sqlx::Error> {
    let all: Vec<CapeRow> = sqlx::query_as("SELECT * FROM capes ORDER BY name COLLATE NOCASE").fetch_all(pool).await?;
    Ok(all
        .into_iter()
        .filter(|c| {
            user.is_admin()
                || c.visibility == "public"
                || (c.visibility == "groups" && {
                    let allowed: Vec<String> = serde_json::from_str(&c.allowed_groups).unwrap_or_default();
                    allowed.iter().any(|g| groups.iter().any(|x| x.eq_ignore_ascii_case(g)))
                })
        })
        .collect())
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn selection_preserves_visibility_group_case_sorting_and_fresh_membership() {
        let pool = crate::identity_store::tests::synthetic_store().await;
        let id = crate::identity_store::insert(
            &pool,
            crate::identity_store::NewAccount {
                username: "ExamplePlayer",
                password_hash: "synthetic-hash",
                email: None,
                role: "member",
                status: "active",
                created_at: "2026-01-01T00:00:00Z",
                uuid: "01234567-89ab-4def-8123-456789abcdef",
            },
        )
        .await
        .unwrap();
        let mut user = crate::identity_store::find_by_id(&pool, id).await.unwrap().unwrap();
        sqlx::query(
            "CREATE TABLE capes (id INTEGER PRIMARY KEY, name TEXT, hash TEXT, visibility TEXT, allowed_groups TEXT, created_at TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        for (id, name, visibility, allowed) in [
            (1, "z Private", "private", "[]"),
            (2, "a Public", "public", "[]"),
            (3, "B Group", "groups", "[\"readers\"]"),
            (4, "c Malformed", "groups", "not-json"),
            (5, "d Unknown", "other", "[]"),
        ] {
            sqlx::query("INSERT INTO capes VALUES (?, ?, 'synthetic-hash', ?, ?, '2026-01-01T00:00:00Z')")
                .bind(id)
                .bind(name)
                .bind(visibility)
                .bind(allowed)
                .execute(&pool)
                .await
                .unwrap();
        }
        let selected = available(&pool, &user, &["Readers".into()]).await.unwrap();
        assert_eq!(selected.iter().map(|cape| cape.id).collect::<Vec<_>>(), [2, 3]);
        assert_eq!(available(&pool, &user, &[]).await.unwrap().iter().map(|cape| cape.id).collect::<Vec<_>>(), [2]);
        user.role = "admin".into();
        assert_eq!(available(&pool, &user, &[]).await.unwrap().len(), 5);
        user.role = "Admin".into();
        assert_eq!(available(&pool, &user, &[]).await.unwrap().len(), 1);
        assert_eq!(find(&pool, 1).await.unwrap().unwrap().visibility, "private");
        assert!(find(&pool, 99).await.unwrap().is_none());
        pool.close().await;
        assert!(find(&pool, 1).await.is_err());
        assert!(available(&pool, &user, &[]).await.is_err());
    }
}
