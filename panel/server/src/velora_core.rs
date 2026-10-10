//! Velora SMP policy belongs to the complete host, never the reusable platform.
use crate::{auth::AdminUser, error::{AppError, AppResult}, state::{AppState, RequestState as State}, store};
use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use velora_shared::{Experience, ExperiencePage};

pub const KIND: &str = "velora-smp";
pub const MODULES: &[&str] = &["map", "economy", "vaults", "casino", "analytics", "factions", "permissions_chat"];
pub const MAP_LAYERS: &[&str] = &["players", "claims", "spawn", "warps", "homes", "shops"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub schema: u32,
    pub modules: BTreeMap<String, bool>,
    pub map_layers: BTreeMap<String, bool>,
    pub currency: String,
    pub starting_balance_cents: i64,
    pub virtual_market_fee_bps: u32,
    pub faction_creation_cents: i64,
    pub faction_base_claims: i64,
    pub faction_max_members: i64,
    pub upkeep_chunk_cents: i64,
    pub upkeep_member_cents: i64,
    pub upkeep_grace_days: i64,
    pub rivalry_cooldown_minutes: i64,
    /// Claim groups not connected to the faction's largest territory.
    pub faction_outpost_limit: i64,
    /// Faction upgrade tracks: every tier costs the same and adds a fixed amount.
    pub upgrade_claims_chunks: i64,
    pub upgrade_claims_cents: i64,
    pub upgrade_claims_max: i64,
    pub upgrade_members_slots: i64,
    pub upgrade_members_cents: i64,
    pub upgrade_members_max: i64,
    pub upgrade_outposts_cents: i64,
    pub upgrade_outposts_max: i64,
    /// Each faction vault page is bought once; zero pages disables faction storage.
    pub faction_vault_cents: i64,
    pub faction_vault_max: i64,
    /// Price of each personal vault above the free count.
    pub vault_price_cents: i64,
    /// Active Burst rounds older than this settle at their current cash-out value.
    pub casino_abandon_hours: i64,
    /// Analytics events older than this are deleted by the scheduler.
    pub analytics_retention_days: i64,
    /// Physical market points must stand on land claimed by the owner's faction.
    pub shop_requires_claim: bool,
    pub shop_promotion_cents_per_day: i64,
}

impl Default for Policy {
    fn default() -> Self {
        Self { schema: 1, modules: MODULES.iter().map(|id| ((*id).into(), true)).collect(),
            map_layers: MAP_LAYERS.iter().map(|id| ((*id).into(), true)).collect(), currency: "dollars".into(),
            starting_balance_cents: 100_000, virtual_market_fee_bps: 1_000,
            faction_creation_cents: 75_000, faction_base_claims: 16, faction_max_members: 8,
            upkeep_chunk_cents: 200, upkeep_member_cents: 100, upkeep_grace_days: 3, rivalry_cooldown_minutes: 60,
            faction_outpost_limit: 1, upgrade_claims_chunks: 8, upgrade_claims_cents: 50_000, upgrade_claims_max: 10,
            upgrade_members_slots: 2, upgrade_members_cents: 75_000, upgrade_members_max: 6,
            upgrade_outposts_cents: 100_000, upgrade_outposts_max: 3, faction_vault_cents: 150_000, faction_vault_max: 3,
            vault_price_cents: 250_000, casino_abandon_hours: 24, analytics_retention_days: 90,
            shop_requires_claim: true, shop_promotion_cents_per_day: 5_000 }
    }
}

impl Policy {
    pub fn enabled(&self, module: &str) -> bool { self.modules.get(module).copied().unwrap_or(false) }

    pub fn validate(&self) -> AppResult<()> {
        if self.schema != 1 || self.modules.len() != MODULES.len() || MODULES.iter().any(|id| !self.modules.contains_key(*id))
            || self.map_layers.len() != MAP_LAYERS.len() || MAP_LAYERS.iter().any(|id| !self.map_layers.contains_key(*id)) {
            return Err(AppError::bad_request("Velora Core schema 1 requires exactly the seven registered modules"));
        }
        if self.currency != "dollars" || !(0..=100_000_000_000).contains(&self.starting_balance_cents)
            || !(0..=100_000_000_000).contains(&self.faction_creation_cents) || self.virtual_market_fee_bps > 10_000
            || !(1..=100_000).contains(&self.faction_base_claims) || !(1..=5_000).contains(&self.faction_max_members) {
            return Err(AppError::bad_request("invalid Velora Core economy or faction limits"));
        }
        // Vault storage remains usable without economy; purchasing extra vaults does not.
        if !(0..=100_000_000).contains(&self.upkeep_chunk_cents) || !(0..=100_000_000).contains(&self.upkeep_member_cents)
            || !(1..=30).contains(&self.upkeep_grace_days) || !(1..=43200).contains(&self.rivalry_cooldown_minutes) {
            return Err(AppError::bad_request("invalid upkeep or rivalry policy"));
        }
        let money = 0..=100_000_000_000;
        if !(0..=100).contains(&self.faction_outpost_limit) || !(1..=10_000).contains(&self.upgrade_claims_chunks)
            || !(1..=1_000).contains(&self.upgrade_members_slots) || !(0..=9).contains(&self.faction_vault_max)
            || [self.upgrade_claims_cents, self.upgrade_members_cents, self.upgrade_outposts_cents, self.faction_vault_cents,
                self.vault_price_cents, self.shop_promotion_cents_per_day].iter().any(|cents| !money.contains(cents))
            || [self.upgrade_claims_max, self.upgrade_members_max, self.upgrade_outposts_max].iter().any(|tiers| !(0..=100).contains(tiers))
            || !(1..=720).contains(&self.casino_abandon_hours) || !(7..=3650).contains(&self.analytics_retention_days) {
            return Err(AppError::bad_request("invalid upgrade, vault, casino or analytics policy"));
        }
        if !self.enabled("economy") && (self.enabled("casino") || self.enabled("factions")) {
            return Err(AppError::bad_request("casino and factions require the economy module"));
        }
        Ok(())
    }

    pub fn features(&self) -> Vec<String> {
        [("map", "maps"), ("economy", "economy"), ("casino", "casino"), ("analytics", "progression"),
            ("factions", "guilds")].into_iter().filter(|(module, _)| self.enabled(module)).map(|(_, feature)| feature.into())
            .chain((self.enabled("vaults") || self.enabled("permissions_chat")).then_some("commands".into())).collect()
    }
}

pub async fn filter_map(state: &AppState, mut overlay: Value) -> AppResult<Value> {
    if let Some(policy) = load(state).await? {
        for key in ["players", "claims"] {
            if !policy.map_layers.get(key).copied().unwrap_or(false) { overlay[key] = json!([]); }
        }
        if let Some(pins) = overlay.get_mut("pins").and_then(Value::as_array_mut) {
            pins.retain(|pin| {
                let layer = match pin["kind"].as_str().unwrap_or("") {
                    "spawn" => "spawn", "warp" => "warps", "home" | "guild_home" => "homes", "market" | "shop" => "shops", _ => return false,
                };
                policy.map_layers.get(layer).copied().unwrap_or(false)
            });
        }
    }
    Ok(overlay)
}

pub fn preset() -> Experience {
    let policy = Policy::default();
    Experience { kind: KIND.into(), branding: None, features: policy.features(),
        navigation: vec![ExperiencePage { id: "guilds".into(), label: "Factions".into() }], widgets: vec![],
        modules: BTreeMap::from([("velora_core".into(), serde_json::to_value(policy).expect("serializable policy"))]) }
}

pub fn policy(experience: &Experience) -> AppResult<Option<Policy>> {
    if experience.kind != KIND { return Ok(None); }
    let value = experience.modules.get("velora_core").ok_or_else(|| AppError::bad_request("Velora SMP needs its Velora Core policy"))?;
    let policy: Policy = serde_json::from_value(value.clone()).map_err(|_| AppError::bad_request("invalid Velora Core policy"))?;
    policy.validate()?;
    Ok(Some(policy))
}

pub async fn load(state: &AppState) -> AppResult<Option<Policy>> {
    match state.instance_id.as_deref() {
        Some(id) => policy(&serde_json::from_str(&store::get_instance(state, id).await?.experience)?),
        None => Ok(None),
    }
}

pub async fn for_server(conn: &mut sqlx::SqliteConnection, server: i64) -> AppResult<Option<Policy>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT i.experience FROM instances i JOIN game_servers s ON s.instance_id=i.id WHERE s.id=?")
        .bind(server).fetch_optional(conn).await?;
    raw.map(|raw| serde_json::from_str::<Experience>(&raw).map_err(AppError::from).and_then(|e| policy(&e))).transpose().map(Option::flatten)
}

/// Integer cents for new SMP settlements, without rewriting persisted legacy balances.
pub fn cents(amount: f64) -> AppResult<i64> {
    if !amount.is_finite() || !(0.0..=1_000_000_000.0).contains(&amount) || (amount * 100.0 - (amount * 100.0).round()).abs() > 0.00001 {
        return Err(AppError::bad_request("amounts must be finite dollars with at most two decimal places"));
    }
    Ok((amount * 100.0).round() as i64)
}

pub async fn market_settlement(conn: &mut sqlx::SqliteConnection, server: i64, price: f64) -> AppResult<(f64, f64)> {
    match for_server(conn, server).await? {
        Some(policy) => {
            let gross = cents(price)?;
            // Round to nearest cent, halves upward. Split conserves the gross exactly.
            let fee = (gross * i64::from(policy.virtual_market_fee_bps) + 5_000) / 10_000;
            Ok(((gross - fee) as f64 / 100.0, fee as f64 / 100.0))
        }
        None => Ok((price, 0.0)),
    }
}

pub async fn charge_faction(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, instance: &str, uuid: &str, name: &str) -> AppResult<Option<Policy>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT experience FROM instances WHERE id=?").bind(instance).fetch_optional(&mut **tx).await?;
    let Some(raw) = raw else { return Ok(None); }; // Legacy integrations can have an unassigned instance label.
    let Some(policy) = policy(&serde_json::from_str(&raw)?)? else { return Ok(None); };
    if !policy.enabled("factions") || !policy.enabled("economy") { return Err(AppError::forbidden("factions or economy is disabled")); }
    let server: Option<i64> = sqlx::query_scalar("SELECT MIN(id) FROM game_servers WHERE instance_id=?").bind(instance).fetch_one(&mut **tx).await?;
    let server = server.ok_or_else(|| AppError::bad_request("assign a game server to Velora SMP before creating factions"))?;
    let economy: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT MIN(b.id) FROM game_servers b WHERE b.economy_group<>'' AND b.economy_group=a.economy_group COLLATE NOCASE),a.id) FROM game_servers a WHERE a.id=?")
        .bind(server).fetch_one(&mut **tx).await?;
    crate::routes::economy::ensure_balance(tx, economy, uuid, name).await?;
    let already: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM guild_members m JOIN guilds g ON g.id=m.guild_id WHERE m.uuid=? AND g.instance_id=?)")
        .bind(uuid).bind(instance).fetch_one(&mut **tx).await?;
    if already { return Err(AppError::bad_request("leave your existing faction before creating another")); }
    let cost = policy.faction_creation_cents as f64 / 100.0;
    let changed = sqlx::query("UPDATE server_economy SET balance=balance-?,updated_at=? WHERE server_id=? AND uuid=? AND balance>=?")
        .bind(cost).bind(crate::db::now()).bind(economy).bind(uuid).bind(cost).execute(&mut **tx).await?.rows_affected();
    if changed != 1 { return Err(AppError::bad_request("insufficient funds to create a faction")); }
    sqlx::query("INSERT INTO economy_transactions(server_id,from_uuid,from_name,to_uuid,to_name,amount,description,created_at) VALUES(?,?,?,'server','Faction registry',?,'Faction creation',?)")
        .bind(server).bind(uuid).bind(name).bind(cost).bind(crate::db::now()).execute(&mut **tx).await?;
    Ok(Some(policy))
}

pub fn module_for_path(path: &str) -> Option<&'static str> {
    if matches!(path,"/api/server/v1/vault/transfers/finish"|"/api/server/v1/vault/transfers/pending") {return None;}
    let p = path.strip_prefix("/api/admin/").or_else(|| path.strip_prefix("/api/v1/")).or_else(|| path.strip_prefix("/api/server/v1/"))?;
    if p.starts_with("economy/market/vault") || p.starts_with("vault") { return Some("vaults"); }
    if p.starts_with("utilities") { return None; } // Mixed utilities are filtered individually in sync.
    if p.starts_with("chat") || p.starts_with("luckperms") { return Some("permissions_chat"); }
    match crate::experience::feature_for_path(path)? {
        "maps" => Some("map"), "economy" => Some("economy"), "casino" => Some("casino"),
        "guilds" => Some("factions"), "progression" => Some("analytics"), _ => None,
    }
}

/// Explicitly adopt an existing 1.20.1 Fabric installation and hide other experiences.
/// Retain every ID, file, database and server credential. No deployment occurs.
pub async fn activate(_: AdminUser, State(state): State<AppState>, Path(id): Path<String>) -> AppResult<Json<Value>> {
    let instance = store::get_instance(&state, &id).await?;
    if instance.mc_version != "1.20.1" || instance.loader != "fabric" {
        return Err(AppError::bad_request("Velora SMP requires a Fabric 1.20.1 installation; configure installation first"));
    }
    let mut experience = preset();
    experience.branding = serde_json::from_str::<Experience>(&instance.experience)?.branding;
    let mut tx = state.platform_db.begin().await?;
    let now = crate::db::now();
    sqlx::query("UPDATE instances SET enabled=0, revision=revision+1, updated_at=? WHERE id<>? AND enabled<>0")
        .bind(&now).bind(&id).execute(&mut *tx).await?;
    sqlx::query("UPDATE instances SET enabled=1, experience=?, revision=revision+1, updated_at=? WHERE id=?")
        .bind(serde_json::to_string(&experience)?).bind(&now).bind(&id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"ok":true,"experience":experience})))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn dependencies_and_unknown_modules_are_rejected() {
        let mut p = Policy::default();
        p.modules.insert("economy".into(), false);
        assert!(p.validate().is_err());
        p.modules.insert("casino".into(), false); p.modules.insert("factions".into(), false);
        assert!(p.validate().is_ok());
        p.modules.insert("typo".into(), true); assert!(p.validate().is_err());
    }
    #[test] fn new_smp_excludes_legacy_systems() {
        let e = preset();
        assert!(e.enabled("guilds"));
        for id in ["quests", "achievements", "collections", "content", "events", "companion"] { assert!(!e.enabled(id)); }
        assert_eq!(policy(&e).unwrap().unwrap().starting_balance_cents, 100_000);
        assert_eq!(module_for_path("/api/server/v1/economy/market/vault/ack"), Some("vaults"));
    }
}
