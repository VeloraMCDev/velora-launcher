//! Velora Core completion: faction upgrades/outposts/allies, cloud vaults, casino settlement, analytics and permissions.
//! Every account, server and item here is synthetic.
mod common;
use common::{json, Body, Request, ServiceExt, StatusCode, Value};
use velora_panel::{app, auth, bootstrap_admin, build_state_with_keys, config::Config, db, state::AppState};

struct Platform {
    state: AppState,
    router: axum::Router,
    token: String,
    _dir: tempfile::TempDir,
}

impl Platform {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            bind: "127.0.0.1:0".into(),
            data_dir: dir.path().into(),
            web_dir: dir.path().join("web"),
            icons_dir: dir.path().join("icons"),
            admin_username: "admin".into(),
            admin_password: Some("test-admin-pass".into()),
            jwt_secret: Some("test-instance-signing-secret-with-32-bytes".into()),
            curseforge_api_key: None,
            max_upload_mb: 64,
            public_url: None,
            trusted_proxies: vec![],
        };
        let pool = db::connect(dir.path()).await.unwrap();
        let state = build_state_with_keys(cfg, pool, common::test_keys()).await.unwrap();
        bootstrap_admin(&state).await.unwrap();
        let admin = auth::find_user_by_name(&state, "admin").await.unwrap().unwrap();
        let token = state.keys.issue(&admin).unwrap();
        Self { router: app(state.clone()), state, token, _dir: dir }
    }

    async fn call(&self, method: &str, path: &str, instance: Option<&str>, token: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path).header("Authorization", format!("Bearer {token}"));
        if let Some(id) = instance {
            req = req.header("X-SCOPENET-Instance", id);
        }
        let body = if let Some(body) = body {
            req = req.header("Content-Type", "application/json");
            Body::from(body.to_string())
        } else {
            Body::empty()
        };
        let response = self.router.clone().oneshot(req.body(body).unwrap()).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or_else(|_| json!(String::from_utf8_lossy(&body))))
    }

    /// A Fabric 1.20.1 instance activated as Velora SMP, with one game server.
    async fn core(&self) -> Core {
        let (status, body) = self
            .call("POST", "/api/admin/instances", None, &self.token, Some(json!({"name":"Velora SMP","mc_version":"1.20.1","loader":"fabric"})))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let id = body["id"].as_str().unwrap().to_owned();
        sqlx::query("UPDATE instances SET loader_version='0.16.10' WHERE id=?").bind(&id).execute(&self.state.db).await.unwrap();
        let (status, body) = self.call("POST", &format!("/api/admin/instances/{id}/velora-core/activate"), None, &self.token, Some(json!({}))).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, server) =
            self.call("POST", "/api/admin/servers", Some(&id), &self.token, Some(json!({"name":"Synthetic SMP","instance_id":id}))).await;
        assert_eq!(status, StatusCode::OK, "{server}");
        let state = self.state.experiences.state(&self.state, &id).await.unwrap();
        Core { sid: server["server"]["id"].as_i64().unwrap(), server: server["token"].as_str().unwrap().to_owned(), id, state }
    }

    async fn player(&self, name: &str) -> (String, String) {
        let (status, user) =
            self.call("POST", "/api/admin/users", None, &self.token, Some(json!({"username":name,"password":"synthetic-test-pass"}))).await;
        assert_eq!(status, StatusCode::OK, "{user}");
        let (_, login) =
            self.call("POST", "/api/v1/auth/login", None, "", Some(json!({"username":name,"password":"synthetic-test-pass"}))).await;
        let token = login["token"].as_str().unwrap().to_owned();
        let user = auth::find_user_by_name(&self.state, name).await.unwrap().unwrap();
        (user.uuid, token)
    }

    async fn admin_uuid(&self) -> String { auth::find_user_by_name(&self.state, "admin").await.unwrap().unwrap().uuid }
}

struct Core {
    id: String,
    sid: i64,
    server: String,
    state: AppState,
}

impl Core {
    async fn game(&self, p: &Platform, path: &str, body: Value) -> (StatusCode, Value) {
        p.call("POST", &format!("/api/server/v1/{path}"), None, &self.server, Some(body)).await
    }
}

#[tokio::test]
async fn outposts_are_limited_and_upgrades_are_paid_once_from_the_faction_bank() {
    let p = Platform::new().await;
    let c = p.core().await;
    let (status, faction) =
        p.call("POST", "/api/v1/guilds", Some(&c.id), &p.token, Some(json!({"instance_id":c.id,"name":"Synthetic Outpost","tag":"OUT"}))).await;
    assert_eq!(status, StatusCode::OK, "{faction}");
    let guild = faction["id"].as_str().unwrap().to_owned();
    let claim = |x: i32, z: i32| json!({"server_id":c.sid,"chunk_x":x,"chunk_z":z});
    let path = format!("/api/v1/guilds/{guild}/claims");
    assert_eq!(p.call("POST", &path, Some(&c.id), &p.token, Some(claim(0, 0))).await.0, StatusCode::OK, "home territory");
    assert_eq!(p.call("POST", &path, Some(&c.id), &p.token, Some(claim(5, 5))).await.0, StatusCode::BAD_REQUEST, "remote land needs a purchased, placed flag");
    let (status, refused) = p.call("POST", &path, Some(&c.id), &p.token, Some(claim(10, 10))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a second outpost needs an upgrade: {refused}");
    assert_eq!(p.call("POST", &path, Some(&c.id), &p.token, Some(claim(1, 0))).await.0, StatusCode::OK, "extending land is always fine");

    let upgrades = format!("/api/v1/guilds/{guild}/upgrades");
    let (status, view) = p.call("GET", &upgrades, Some(&c.id), &p.token, None).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!((view["outposts"].as_i64(), view["outpost_limit"].as_i64()), (Some(0), Some(3)));
    let buy = json!({"track":"outposts","expected_tiers":0});
    let (status, _) = p.call("POST", &upgrades, Some(&c.id), &p.token, Some(json!({"track":"outposts","expected_tiers":0,"expected_price_cents":1}))).await;
    assert_eq!(status, StatusCode::CONFLICT, "a changed upgrade price requires review");
    assert_eq!(p.call("POST", &upgrades, Some(&c.id), &p.token, Some(buy.clone())).await.0, StatusCode::BAD_REQUEST, "the bank is empty");
    sqlx::query("INSERT INTO guild_wallets(server_id,guild_id,balance,updated_at) VALUES(?,?,2000,?) ON CONFLICT(server_id,guild_id) DO UPDATE SET balance=2000")
        .bind(c.sid).bind(&guild).bind(db::now()).execute(&c.state.db).await.unwrap();
    let (status, bought) = p.call("POST", &upgrades, Some(&c.id), &p.token, Some(buy.clone())).await;
    assert_eq!(status, StatusCode::OK, "{bought}");
    assert_eq!(p.call("POST", &upgrades, Some(&c.id), &p.token, Some(buy)).await.0, StatusCode::CONFLICT, "a repeated click cannot buy twice");
    let flag = bought["upgrades"]["flags"][0]["id"].as_str().unwrap().to_owned();
    let place = json!({"uuid":p.admin_uuid().await,"id":flag,"dimension":"minecraft:overworld","x":80,"y":64,"z":80});
    assert_eq!(c.game(&p,"guilds/outposts/place",place.clone()).await.0,StatusCode::OK);
    assert_eq!(c.game(&p,"guilds/outposts/place",place).await.0,StatusCode::OK,"lost placement response can replay the same anchor");
    assert_eq!(p.call("POST", &path, Some(&c.id), &p.token, Some(claim(5, 5))).await.0, StatusCode::OK, "a placed flag unlocked its anchor chunk");
    assert_eq!(p.call("POST", &path, Some(&c.id), &p.token, Some(claim(9, 5))).await.0, StatusCode::BAD_REQUEST, "outside the flag area");

    let before: i64 = sqlx::query_scalar("SELECT max_claims FROM guilds WHERE id=?").bind(&guild).fetch_one(&c.state.db).await.unwrap();
    let (status, _) = p.call("POST", &upgrades, Some(&c.id), &p.token, Some(json!({"track":"claims","expected_tiers":0}))).await;
    assert_eq!(status, StatusCode::OK);
    let after: i64 = sqlx::query_scalar("SELECT max_claims FROM guilds WHERE id=?").bind(&guild).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(after - before, 8);
    let bank: f64 = sqlx::query_scalar("SELECT balance FROM guild_wallets WHERE guild_id=?").bind(&guild).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(bank, 500.0, "$1,000 outpost and $500 claim upgrades");
    let logged: f64 = sqlx::query_scalar("SELECT SUM(amount) FROM guild_wallet_transactions WHERE guild_id=? AND kind='upgrade'")
        .bind(&guild).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(logged, 1500.0);

    for (x,z) in [(6,5),(7,5),(8,5),(5,6),(6,6),(7,6),(8,6),(5,7),(6,7),(7,7),(8,7)] {
        let (status,body)=p.call("POST",&path,Some(&c.id),&p.token,Some(claim(x,z))).await;
        assert_eq!(status,StatusCode::OK,"{x},{z}: {body}");
    }
    assert_eq!(p.call("POST",&path,Some(&c.id),&p.token,Some(claim(5,8))).await.0,StatusCode::BAD_REQUEST,"12 chunks per flag, independent of total faction capacity");
    assert_eq!(c.game(&p,"guilds/outposts/place",json!({"uuid":p.admin_uuid().await,"id":flag,"dimension":"minecraft:overworld","x":160,"y":64,"z":160})).await.0,StatusCode::CONFLICT,"a used flag cannot move its anchor");

    let (member, member_token) = p.player("SyntheticRecruit").await;
    sqlx::query("INSERT INTO guild_members(guild_id,uuid,name,role,joined_at) VALUES(?,?,'SyntheticRecruit','member',?)")
        .bind(&guild).bind(&member).bind(db::now()).execute(&c.state.db).await.unwrap();
    let (status, _) = p.call("POST", &upgrades, Some(&c.id), &member_token, Some(json!({"track":"members","expected_tiers":0}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "plain members cannot spend the bank");
    let (status, listed) = c.game(&p, "guilds/upgrades", json!({"uuid":member})).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed["tracks"]["claims"]["tiers"], 1);
    let (status, _) = c.game(&p, "guilds/upgrades", json!({"uuid":p.admin_uuid().await,"track":"claims","expected_tiers":1})).await;
    assert_eq!(status, StatusCode::OK, "leaders buy in game too");
    sqlx::query("UPDATE guild_wallets SET balance=3000 WHERE guild_id=?").bind(&guild).execute(&c.state.db).await.unwrap();
    for tier in [1,2] {
        let (status,body)=p.call("POST",&upgrades,Some(&c.id),&p.token,Some(json!({"track":"outposts","expected_tiers":tier}))).await;
        assert_eq!(status,StatusCode::OK,"{body}");
    }
    assert_eq!(p.call("POST",&upgrades,Some(&c.id),&p.token,Some(json!({"track":"outposts","expected_tiers":3}))).await.0,StatusCode::BAD_REQUEST,"three issued flags exhaust the lifetime cap, even before all are placed");
    let flags:i64=sqlx::query_scalar("SELECT COUNT(*) FROM faction_outposts WHERE guild_id=?").bind(&guild).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(flags,3,"failed purchases leave no extra flag");
}

#[tokio::test]
async fn darknet_unlocks_a_default_vault_once_and_offers_the_next_page() {
    let p=Platform::new().await;
    let c=p.core().await;
    let (player,token)=p.player("SynthDarkVault").await;
    sqlx::query("INSERT INTO server_economy(server_id,uuid,username,balance,updated_at) VALUES(?,?,'SynthDarkVault',6000,?)")
        .bind(c.sid).bind(&player).bind(db::now()).execute(&c.state.db).await.unwrap();
    let path=format!("/api/v1/board/{}/darknet",c.sid);
    let (status,catalog)=p.call("GET",&path,Some(&c.id),&token,None).await;
    assert_eq!(status,StatusCode::OK,"{catalog}");
    assert_eq!(catalog["products"][0]["vault_number"],2);
    let purchase=json!({"product_id":"velora-vault-2","expected_price_cents":250000,"operation_id":uuid::Uuid::new_v4().to_string()});
    let (status,receipt)=p.call("POST",&format!("{path}/buy"),Some(&c.id),&token,Some(purchase.clone())).await;
    assert_eq!(status,StatusCode::OK,"{receipt}");
    assert_eq!(receipt["balance"],3500.0);
    assert_eq!(p.call("POST",&format!("{path}/buy"),Some(&c.id),&token,Some(purchase)).await.1,receipt);
    let (_,catalog)=p.call("GET",&path,Some(&c.id),&token,None).await;
    assert_eq!(catalog["products"][0]["vault_number"],3);
    let (_,vaults)=p.call("GET",&format!("/api/v1/vaults/{}",c.sid),Some(&c.id),&token,None).await;
    assert_eq!(vaults["vaults"].as_array().unwrap().len(),3,"two owned pages and one next unlock");
    assert_eq!(vaults["vaults"][1]["unlocked"],true);
    let queued:i64=sqlx::query_scalar("SELECT COUNT(*) FROM market_mailbox WHERE uuid=?").bind(player).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(queued,0,"an unlock is an entitlement, not a chest item");
}

#[tokio::test]
async fn abandoned_burst_uses_earned_snapshot_and_settles_only_once() {
    let p=Platform::new().await;
    let c=p.core().await;
    let (player,token)=p.player("SynthBurst").await;
    let (status,round)=p.call("POST",&format!("/api/v1/casino/{}/burst/start",c.sid),Some(&c.id),&token,
        Some(json!({"bet":25,"operation_id":uuid::Uuid::new_v4().to_string()}))).await;
    assert_eq!(status,StatusCode::OK,"{round}");
    let id=round["game"]["id"].as_i64().unwrap();
    sqlx::query("UPDATE casino_burst SET steps=2,survival=0.5,house_edge=0.04,created_at='2000-01-01T00:00:00Z' WHERE id=?")
        .bind(id).execute(&c.state.db).await.unwrap();
    velora_panel::routes::casino::settle_due(&c.state).await.unwrap();
    velora_panel::routes::casino::settle_due(&c.state).await.unwrap();
    let balance:f64=sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid=?").bind(&player).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(balance,1071.0,"975 remaining stake balance plus 96 earned payout");
    let status:String=sqlx::query_scalar("SELECT status FROM casino_burst WHERE id=?").bind(id).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(status,"cashed");
    let records:i64=sqlx::query_scalar("SELECT COUNT(*) FROM casino_rounds WHERE uuid=? AND game='burst'").bind(player).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(records,1,"no second payout/history row on a scheduler retry");
}

#[tokio::test]
async fn analytics_retention_expires_raw_events_using_core_policy() {
    let p=Platform::new().await;
    let c=p.core().await;
    for days in [100,70,1] {
        sqlx::query("INSERT INTO server_events(server_id,kind,created_at) VALUES(?,'synthetic',?)")
            .bind(c.sid).bind((chrono::Utc::now()-chrono::Duration::days(days)).to_rfc3339()).execute(&c.state.db).await.unwrap();
    }
    let result=velora_panel::scheduler::run_now(&c.state,"analytics_retention").await.unwrap();
    assert!(result.contains("90-day retention"),"{result}");
    let remaining:i64=sqlx::query_scalar("SELECT COUNT(*) FROM server_events WHERE kind='synthetic'").fetch_one(&c.state.db).await.unwrap();
    assert_eq!(remaining,2,"Core uses 90 days, not legacy 60 days");
}

#[tokio::test]
async fn vault_save_receipt_survives_release_and_rejects_reused_contents() {
    let p=Platform::new().await; let c=p.core().await;
    let (player,_)=p.player("SynthSave").await;
    let (_,open)=c.game(&p,"vault/open",json!({"uuid":player,"owner":"player","number":1})).await;
    let mut save=json!({"key":open["key"],"number":1,"lease":open["lease"],"revision":open["revision"],
        "operation_id":uuid::Uuid::new_v4().to_string(),"release":true,
        "contents":[{"slot":0,"item":"minecraft:diamond","count":2,"name":"Diamond","data":""}]});
    let (status,receipt)=c.game(&p,"vault/save",save.clone()).await;
    assert_eq!(status,StatusCode::OK,"{receipt}");
    assert_eq!(c.game(&p,"vault/save",save.clone()).await.1,receipt,"same write replays even after its lease is released");
    save["contents"][0]["count"]=json!(3);
    assert_eq!(c.game(&p,"vault/save",save).await.0,StatusCode::CONFLICT,"receipt cannot authorize different contents");
    let (_,reopened)=c.game(&p,"vault/open",json!({"uuid":player,"owner":"player","number":1})).await;
    assert_eq!(reopened["revision"],1);
    assert_eq!(reopened["contents"][0]["count"],2);
}

#[tokio::test]
async fn accepted_alliances_publish_mutual_ally_permissions_to_game_servers() {
    let p = Platform::new().await;
    let c = p.core().await;
    let (ally, ally_token) = p.player("SyntheticAlly").await;
    let (_, a) = p.call("POST", "/api/v1/guilds", Some(&c.id), &p.token, Some(json!({"instance_id":c.id,"name":"Synthetic Alpha","tag":"ALP"}))).await;
    let (_, b) = p.call("POST", "/api/v1/guilds", Some(&c.id), &ally_token, Some(json!({"instance_id":c.id,"name":"Synthetic Beta","tag":"BET"}))).await;
    let (a, b) = (a["id"].as_str().unwrap().to_owned(), b["id"].as_str().unwrap().to_owned());
    assert_eq!(
        p.call("POST", &format!("/api/v1/guilds/{a}/claims"), Some(&c.id), &p.token, Some(json!({"server_id":c.sid,"chunk_x":0,"chunk_z":0}))).await.0,
        StatusCode::OK
    );
    let terms = json!({"other_guild_id":b,"relation":"alliance","ally_permissions":{"break":true,"chests":true,"place":false}});
    assert_eq!(p.call("POST", &format!("/api/v1/guilds/{a}/relations"), Some(&c.id), &p.token, Some(terms)).await.0, StatusCode::OK);
    let (_, index) = c.game(&p, "guilds/claim-index", json!({})).await;
    assert!(index["guilds"][0]["allies"].is_null(), "pending terms grant nothing: {index}");
    let revision = index["revision"].clone();

    let (_, relations) = p.call("GET", &format!("/api/v1/guilds/{b}/relations"), Some(&c.id), &ally_token, None).await;
    let (rid, reviewed) = (relations[0]["id"].as_i64().unwrap(), relations[0]["terms_revision"].clone());
    let (status, r) = p
        .call("POST", &format!("/api/v1/guilds/{b}/relations/{rid}/respond"), Some(&c.id), &ally_token, Some(json!({"accept":true,"expected_revision":reviewed})))
        .await;
    assert_eq!(status, StatusCode::OK, "{r}");
    let (_, unchanged) = c.game(&p, "guilds/claim-index", json!({"revision":revision})).await;
    assert_eq!(unchanged["unchanged"], false, "accepting an alliance invalidates cached claim indexes");
    let alpha = unchanged["guilds"].as_array().unwrap().iter().find(|g| g["id"] == a.as_str()).unwrap().clone();
    assert_eq!(alpha["allies"][&b], json!(["break", "chests"]), "only granted permissions are published");
    assert!(unchanged["members"][&b].as_array().unwrap().iter().any(|u| u == ally.as_str()), "allies without land still get a roster");
    let mutual: String = sqlx::query_scalar("SELECT ally_permissions FROM guild_relations WHERE guild_id=? AND other_guild_id=?")
        .bind(&b).bind(&a).fetch_one(&c.state.db).await.unwrap();
    assert!(mutual.contains("break"), "the accepting faction grants the same reviewed terms back");
}

#[tokio::test]
async fn cloud_vaults_lease_per_server_reject_stale_writes_and_store_deliveries_once() {
    let p = Platform::new().await;
    let c = p.core().await;
    let (player, token) = p.player("SyntheticHoarder").await;
    let (_, other) = p.call("POST", "/api/admin/servers", Some(&c.id), &p.token, Some(json!({"name":"Synthetic second","instance_id":c.id}))).await;
    let other = other["token"].as_str().unwrap().to_owned();
    let diamond = |slot: u32, count: u32| json!({"slot":slot,"item":"minecraft:diamond","count":count,"name":"Diamond","data":"{id:\"minecraft:diamond\",Count:1b}"});

    let (status, open) = c.game(&p, "vault/open", json!({"uuid":player,"owner":"player","number":1})).await;
    assert_eq!(status, StatusCode::OK, "{open}");
    assert_eq!((open["revision"].as_i64(), open["rows"].as_i64()), (Some(0), Some(7)));
    let (key, lease) = (open["key"].as_str().unwrap().to_owned(), open["lease"].as_str().unwrap().to_owned());
    let (status, _) = p.call("POST", "/api/server/v1/vault/open", None, &other, Some(json!({"uuid":player,"owner":"player","number":1}))).await;
    assert_eq!(status, StatusCode::CONFLICT, "one server holds an open vault");
    let (_, again) = c.game(&p, "vault/open", json!({"uuid":player,"owner":"player","number":1})).await;
    assert_eq!(again["lease"], lease.as_str(), "the holding server keeps its lease for a second viewer");
    let (status, import) = c.game(&p, "vault/import", json!({"uuid":player,"vaults":[{"number":1,"contents":[diamond(0, 9)]}]})).await;
    assert_eq!(status, StatusCode::OK, "{import}");
    assert_eq!(import["migrated"], false, "import cannot overwrite even an empty leased vault");

    let save = |revision: i64, contents: Value, release: bool| json!({"key":key,"number":1,"lease":lease,"revision":revision,"contents":contents,"release":release});
    assert_eq!(c.game(&p, "vault/save", save(5, json!([diamond(0, 3)]), false)).await.0, StatusCode::CONFLICT, "stale revision");
    let (status, saved) = c.game(&p, "vault/save", save(0, json!([diamond(0, 3)]), false)).await;
    assert_eq!((status, saved["revision"].as_i64()), (StatusCode::OK, Some(1)));
    assert_eq!(c.game(&p, "vault/save", save(1, json!([diamond(0, 3), diamond(0, 1)]), false)).await.0, StatusCode::BAD_REQUEST, "duplicate slots");

    let (_, listed) = p.call("GET", &format!("/api/v1/vaults/{}", c.sid), Some(&c.id), &token, None).await;
    assert_eq!(listed["vaults"][0]["in_game"], true);
    assert!(listed["vaults"][0]["contents"][0]["data"].is_null(), "exact item data never reaches players: {listed}");
    let swap = |revision: i64| json!({"from":{"key":key,"number":1,"slot":0,"revision":revision},"to":{"key":key,"number":1,"slot":8,"revision":revision}});
    assert_eq!(p.call("POST", &format!("/api/v1/vaults/{}/move", c.sid), Some(&c.id), &token, Some(swap(1))).await.0, StatusCode::CONFLICT, "open in game");

    // A delivery and its acknowledgement are one write; a replay cannot add the item again.
    let delivery: i64 = sqlx::query_scalar("INSERT INTO market_mailbox(server_id,uuid,item_id,item_name,amount,note,created_at,to_vault) VALUES(?,?,'minecraft:emerald','Emerald',2,'darknet',?,1) RETURNING id")
        .bind(c.sid).bind(&player).bind(db::now()).fetch_one(&c.state.db).await.unwrap();
    let mut delivered = save(1, json!([diamond(0, 3), {"slot":1,"item":"minecraft:emerald","count":2,"name":"Emerald"}]), true);
    delivered["delivery_ids"] = json!([delivery]);
    let (status, r) = c.game(&p, "vault/save", delivered.clone()).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_mailbox WHERE id=?").bind(delivery).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(left, 0);
    delivered["revision"] = json!(2);
    assert_eq!(c.game(&p, "vault/save", delivered).await.0, StatusCode::CONFLICT, "a replayed delivery is refused");

    let (status, moved) = p.call("POST", &format!("/api/v1/vaults/{}/move", c.sid), Some(&c.id), &token, Some(swap(2))).await;
    assert_eq!(status, StatusCode::OK, "released vaults can be rearranged from the launcher: {moved}");
    let (_, listed) = p.call("GET", &format!("/api/v1/vaults/{}", c.sid), Some(&c.id), &token, None).await;
    let slots: Vec<i64> = listed["vaults"][0]["contents"].as_array().unwrap().iter().map(|s| s["slot"].as_i64().unwrap()).collect();
    assert_eq!(slots, vec![1, 8]);
    let (status, _) = p.call("POST", "/api/server/v1/vault/open", None, &other, Some(json!({"uuid":player,"owner":"player","number":1}))).await;
    assert_eq!(status, StatusCode::OK, "after release any server may open it");
}

#[tokio::test]
async fn vault_imports_happen_once_and_locked_vaults_are_bought_or_permitted() {
    let p = Platform::new().await;
    let c = p.core().await;
    let (player, token) = p.player("SynthCollector").await;
    let stack = json!({"slot":4,"item":"minecraft:iron_ingot","count":9,"name":"Iron Ingot","data":"{id:\"minecraft:iron_ingot\",Count:9b}"});
    let (status, r) = c.game(&p, "vault/import", json!({"uuid":player,"vaults":[{"number":1,"contents":[stack]}]})).await;
    assert_eq!((status, r["imported"].as_bool()), (StatusCode::OK, Some(true)), "{r}");
    let (_, again) = c.game(&p, "vault/import", json!({"uuid":player,"vaults":[{"number":1,"contents":[]}]})).await;
    assert_eq!((again["imported"].as_bool(), again["migrated"].as_bool()), (Some(false), Some(true)), "a second file cannot overwrite");
    let (_, open) = c.game(&p, "vault/open", json!({"uuid":player,"owner":"player","number":1})).await;
    assert_eq!(open["contents"][0]["count"], 9);

    let utilities = json!({"vault":{"enabled":true,"count":3,"rows":7,"free_count":1}});
    let (status, body) = p.call("PUT", "/api/admin/utilities", Some(&c.id), &p.token, Some(utilities)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(c.game(&p, "vault/open", json!({"uuid":player,"owner":"player","number":2})).await.0, StatusCode::FORBIDDEN, "locked");
    let move_locked = json!({"from":{"key":format!("player:{player}"),"number":1,"slot":4,"revision":2},"to":{"key":format!("player:{player}"),"number":2,"slot":0,"revision":0}});
    assert_eq!(p.call("POST", &format!("/api/v1/vaults/{}/move", c.sid), Some(&c.id), &token, Some(move_locked)).await.0, StatusCode::FORBIDDEN, "launcher cannot bypass locked vaults");
    assert_eq!(c.game(&p, "vault/open", json!({"uuid":player,"owner":"player","number":2,"permitted":true})).await.0, StatusCode::OK, "staff permission");
    let buy = |price: i64, op: &str| json!({"number":3,"expected_price_cents":price,"operation_id":op});
    let path = format!("/api/v1/vaults/{}/buy", c.sid);
    assert_eq!(p.call("POST", &path, Some(&c.id), &token, Some(buy(1, &uuid::Uuid::new_v4().to_string()))).await.0, StatusCode::CONFLICT, "price review");
    let op = uuid::Uuid::new_v4().to_string();
    assert_eq!(p.call("POST", &path, Some(&c.id), &token, Some(buy(250_000, &op))).await.0, StatusCode::BAD_REQUEST, "$1,000 cannot pay $2,500");
    sqlx::query("INSERT INTO server_economy(server_id,uuid,username,balance,updated_at) VALUES(?,?,'SynthCollector',3000,?) ON CONFLICT(server_id,uuid) DO UPDATE SET balance=3000")
        .bind(c.sid).bind(&player).bind(db::now()).execute(&c.state.db).await.unwrap();
    let op = uuid::Uuid::new_v4().to_string();
    let (status, bought) = p.call("POST", &path, Some(&c.id), &token, Some(buy(250_000, &op))).await;
    assert_eq!(status, StatusCode::OK, "{bought}");
    let (_, replay) = p.call("POST", &path, Some(&c.id), &token, Some(buy(250_000, &op))).await;
    assert_eq!(replay, bought, "a retry returns the same receipt");
    let balance: f64 = sqlx::query_scalar("SELECT balance FROM server_economy WHERE uuid=?").bind(&player).fetch_one(&c.state.db).await.unwrap();
    assert_eq!(balance, 500.0, "charged once");
    assert_eq!(c.game(&p, "vault/open", json!({"uuid":player,"owner":"player","number":3})).await.0, StatusCode::OK);
    let (_, owned) = c.game(&p, "vault/entitlements", json!({"uuid":player})).await;
    assert_eq!(owned["owned"], json!([3]));
}

#[tokio::test]
async fn faction_vaults_need_a_bought_page_and_block_disbanding_while_full() {
    let p = Platform::new().await;
    let c = p.core().await;
    let admin = p.admin_uuid().await;
    let (_, faction) = p.call("POST", "/api/v1/guilds", Some(&c.id), &p.token, Some(json!({"instance_id":c.id,"name":"Synthetic Storage","tag":"BOX"}))).await;
    let guild = faction["id"].as_str().unwrap().to_owned();
    let open = json!({"uuid":admin,"owner":"faction","number":1});
    assert_eq!(c.game(&p, "vault/open", open.clone()).await.0, StatusCode::FORBIDDEN, "no page bought yet");
    sqlx::query("INSERT INTO guild_wallets(server_id,guild_id,balance,updated_at) VALUES(?,?,1500,?)").bind(c.sid).bind(&guild).bind(db::now()).execute(&c.state.db).await.unwrap();
    let (status, r) = c.game(&p, "guilds/upgrades", json!({"uuid":admin,"track":"vault","expected_tiers":0})).await;
    assert_eq!(status, StatusCode::OK, "{r}");
    let (status, opened) = c.game(&p, "vault/open", open).await;
    assert_eq!(status, StatusCode::OK, "{opened}");
    assert_eq!(opened["key"], format!("faction:{guild}"));
    let stack = json!([{"slot":0,"item":"minecraft:gold_ingot","count":5,"name":"Gold Ingot"}]);
    let (status, _) = c.game(&p, "vault/save", json!({"key":opened["key"],"number":1,"lease":opened["lease"],"revision":0,"contents":stack,"release":true})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, r) = p.call("DELETE", &format!("/api/v1/guilds/{guild}"), Some(&c.id), &p.token, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "stored faction items would be lost: {r}");
    let (stranger, stranger_token) = p.player("SynthStranger").await;
    let (_, listed) = p.call("GET", &format!("/api/v1/vaults/{}", c.sid), Some(&c.id), &stranger_token, None).await;
    assert!(listed["vaults"].as_array().unwrap().iter().all(|v| v["owner"] == "player"), "outsiders do not see faction vaults");
    assert_eq!(c.game(&p, "vault/open", json!({"uuid":stranger,"owner":"faction","number":1})).await.0, StatusCode::FORBIDDEN);
    sqlx::query("INSERT INTO guild_members(guild_id,uuid,name,role,joined_at) VALUES(?,?,'SynthStranger','member',?)")
        .bind(&guild).bind(&stranger).bind(db::now()).execute(&c.state.db).await.unwrap();
    assert_eq!(c.game(&p, "vault/open", json!({"uuid":stranger,"owner":"faction","number":1})).await.0, StatusCode::FORBIDDEN, "ordinary members do not get default storage access");
    let roles_path = format!("/api/v1/guilds/{guild}/roles");
    let (status, _) = p.call("POST", &roles_path, Some(&c.id), &p.token, Some(json!({"name":"Reader"}))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = p.call("POST", &roles_path, Some(&c.id), &p.token, Some(json!({"name":"Keeper","can_vault":true}))).await;
    assert_eq!(status, StatusCode::OK);
    let (_, roles) = p.call("GET", &roles_path, Some(&c.id), &p.token, None).await;
    assert_eq!(roles.as_array().unwrap().iter().find(|role| role["name"] == "Reader").unwrap()["can_vault"], false);
    assert_eq!(roles.as_array().unwrap().iter().find(|role| role["name"] == "Keeper").unwrap()["can_vault"], true);
    sqlx::query("UPDATE guild_members SET role='Keeper' WHERE guild_id=? AND uuid=?").bind(&guild).bind(&stranger).execute(&c.state.db).await.unwrap();
    let (status, allowed) = c.game(&p, "vault/open", json!({"uuid":stranger,"owner":"faction","number":1})).await;
    assert_eq!(status, StatusCode::OK, "explicit storage grant works: {allowed}");
}

#[tokio::test]
async fn prepared_inventory_transfers_survive_lease_expiry_and_resolve_once() {
    let p=Platform::new().await; let c=p.core().await;
    let (player,token)=p.player("CustodyTester").await;
    let (_,open)=c.game(&p,"vault/open",json!({"uuid":player,"owner":"player","number":1})).await;
    let key=open["key"].as_str().unwrap();
    let id=uuid::Uuid::new_v4().to_string();
    let prepare=json!({"id":id,"uuid":player,"key":key,"number":1,"revision":0,"lease":open["lease"],"contents":[{"slot":0,"item":"minecraft:diamond","count":1,"name":"Diamond","data":""}]});
    let (status,receipt)=c.game(&p,"vault/transfers/prepare",prepare.clone()).await;
    assert_eq!(status,StatusCode::OK,"{receipt}");
    assert_eq!(c.game(&p,"vault/transfers/prepare",prepare.clone()).await.1,receipt);
    let mut changed=prepare.clone(); changed["contents"][0]["count"]=json!(2);
    assert_eq!(c.game(&p,"vault/transfers/prepare",changed).await.0,StatusCode::CONFLICT);
    sqlx::query("UPDATE cloud_vaults SET lease_until='2000-01-01T00:00:00Z' WHERE owner=?").bind(key).execute(&c.state.db).await.unwrap();
    let movement=json!({"from":{"key":key,"number":1,"slot":0,"revision":0},"to":{"key":key,"number":1,"slot":8,"revision":0}});
    assert_eq!(p.call("POST",&format!("/api/v1/vaults/{}/move",c.sid),Some(&c.id),&token,Some(movement)).await.0,StatusCode::CONFLICT,"prepared custody outlives ordinary leases");
    assert_eq!(c.game(&p,"vault/open",json!({"uuid":player,"owner":"player","number":1})).await.0,StatusCode::CONFLICT);
    let (_,pending)=c.game(&p,"vault/transfers/pending",json!({"uuid":player})).await;
    assert_eq!(pending["pending"],json!([id]));
    assert_eq!(c.game(&p,"vault/transfers/finish",json!({"id":id,"uuid":uuid::Uuid::new_v4().to_string(),"commit":true})).await.0,StatusCode::NOT_FOUND);
    let finish=json!({"id":id,"uuid":player,"commit":true});
    let (status,committed)=c.game(&p,"vault/transfers/finish",finish.clone()).await;
    assert_eq!(status,StatusCode::OK,"{committed}"); assert_eq!(committed["revision"],1);
    assert_eq!(c.game(&p,"vault/transfers/finish",finish).await.1,committed,"lost commit response replays without adding another revision");
    assert_eq!(c.game(&p,"vault/transfers/finish",json!({"id":id,"uuid":player,"commit":false})).await.0,StatusCode::CONFLICT);
    let (_,opened)=c.game(&p,"vault/open",json!({"uuid":player,"owner":"player","number":1})).await;
    assert_eq!(opened["contents"][0]["count"],1);
    let cancel_id=uuid::Uuid::new_v4().to_string();
    let cancel=json!({"id":cancel_id,"uuid":player,"key":key,"number":1,"revision":1,"lease":opened["lease"],"contents":[]});
    assert_eq!(c.game(&p,"vault/transfers/prepare",cancel).await.0,StatusCode::OK);
    let (_,cancelled)=c.game(&p,"vault/transfers/finish",json!({"id":cancel_id,"uuid":player,"commit":false})).await;
    assert_eq!(cancelled["revision"],1,"a crash before the inventory checkpoint leaves the original vault untouched");
    let (_,opened)=c.game(&p,"vault/open",json!({"uuid":player,"owner":"player","number":1})).await;
    assert_eq!(opened["contents"][0]["count"],1);
}

#[tokio::test]
async fn pedestal_shop_sales_preserve_stock_and_have_no_fee() {
    let p=Platform::new().await;let c=p.core().await;let seller=p.admin_uuid().await;
    let (buyer,_)=p.player("PedestalBuyer").await;
    let (_,faction)=p.call("POST","/api/v1/guilds",Some(&c.id),&p.token,Some(json!({"instance_id":c.id,"name":"Pedestal Guild","tag":"SHOP"}))).await;
    let guild=faction["id"].as_str().unwrap();
    assert_eq!(p.call("POST",&format!("/api/v1/guilds/{guild}/claims"),Some(&c.id),&p.token,Some(json!({"server_id":c.sid,"chunk_x":0,"chunk_z":0}))).await.0,StatusCode::OK);
    let id=uuid::Uuid::new_v4().to_string();let fingerprint="a".repeat(64);
    let (status,created)=c.game(&p,"economy/physical-shops/create",json!({"id":id,"uuid":seller,"dimension":"minecraft:overworld","x":2,"y":64,"z":2,"chest_x":3,"chest_y":64,"chest_z":2,"item_id":"minecraft:diamond","item_name":"Diamond","fingerprint":fingerprint,"display_data":"{id:\"minecraft:diamond\",Count:2b}","quantity":2,"price_cents":10000})).await;
    assert_eq!(status,StatusCode::OK,"{created}");let owner=format!("shop:{id}");
    assert_eq!(c.game(&p,"vault/open",json!({"uuid":buyer,"owner":owner,"number":1})).await.0,StatusCode::FORBIDDEN);
    let (_,open)=c.game(&p,"vault/open",json!({"uuid":seller,"owner":owner,"number":1})).await;assert_eq!(open["rows"],3);
    let stock=json!([{"slot":0,"item":"minecraft:diamond","name":"Diamond","count":4,"data":"{id:\"minecraft:diamond\",Count:4b}","fingerprint":fingerprint},{"slot":1,"item":"minecraft:diamond","name":"Different Diamond","count":2,"data":"","fingerprint":"b".repeat(64)}]);
    assert_eq!(c.game(&p,"vault/save",json!({"key":owner,"number":1,"lease":open["lease"],"revision":0,"contents":stock,"release":false})).await.0,StatusCode::OK);
    let purchase=json!({"uuid":buyer,"username":"PedestalBuyer","id":id,"operation_id":uuid::Uuid::new_v4().to_string(),"expected_price_cents":10000});
    assert_eq!(c.game(&p,"economy/physical-shops/buy",purchase.clone()).await.0,StatusCode::CONFLICT);
    sqlx::query("UPDATE cloud_vaults SET lease_until='2000-01-01T00:00:00Z' WHERE owner=?").bind(&owner).execute(&c.state.db).await.unwrap();
    let (status,receipt)=c.game(&p,"economy/physical-shops/buy",purchase.clone()).await;assert_eq!(status,StatusCode::OK,"{receipt}");assert_eq!(receipt["balance"],900.0);
    assert_eq!(c.game(&p,"economy/physical-shops/buy",purchase).await.1,receipt);
    let balance:f64=sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id=? AND uuid=?").bind(c.sid).bind(&seller).fetch_one(&c.state.db).await.unwrap();assert_eq!(balance,350.0);
    let stock:String=sqlx::query_scalar("SELECT contents FROM cloud_vaults WHERE owner=?").bind(&owner).fetch_one(&c.state.db).await.unwrap();let stock:Value=serde_json::from_str(&stock).unwrap();assert_eq!(stock[0]["count"],2);assert_eq!(stock[1]["count"],2);
    let deliveries:i64=sqlx::query_scalar("SELECT COUNT(*) FROM market_mailbox WHERE server_id=? AND uuid=? AND note='physical_shop'").bind(c.sid).bind(buyer).fetch_one(&c.state.db).await.unwrap();assert_eq!(deliveries,1);
    let promotion=json!({"uuid":seller,"username":"admin","id":id,"days":1,"warp":true,"expected_price_cents":5000,"operation_id":uuid::Uuid::new_v4().to_string()});
    let (status,promoted)=c.game(&p,"economy/physical-shops/promote",promotion.clone()).await;assert_eq!(status,StatusCode::OK,"{promoted}");assert_eq!(promoted["balance"],300.0);
    assert_eq!(c.game(&p,"economy/physical-shops/promote",promotion).await.1,promoted);
    let pins=velora_panel::routes::physical_shops::pins(&c.state,c.sid).await.unwrap();assert_eq!(pins.len(),1);assert_eq!(pins[0]["warp"],true);
}
