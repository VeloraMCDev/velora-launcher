//! SMP faction upkeep. A bill and its bank debit share one transaction.
use crate::{error::{AppError,AppResult},state::AppState,velora_core};
use chrono::{Duration,Utc};

pub async fn settle_upkeep(state:&AppState)->AppResult<String> {
    let Some(policy)=velora_core::load(state).await? else {return Ok("legacy experience: no SMP upkeep".into());};
    if !policy.enabled("factions") || !policy.enabled("economy") {return Ok("upkeep disabled".into());}
    let today=Utc::now().format("%Y-%m-%d").to_string();
    let guilds:Vec<(String,String)>=sqlx::query_as("SELECT id,instance_id FROM guilds WHERE instance_id=? ORDER BY id").bind(state.instance_id.as_deref().unwrap_or_default()).fetch_all(&state.db).await?;
    let mut paid=0;
    for (guild,instance) in guilds {
        let mut tx=state.db.begin().await?;
        let server:Option<i64>=sqlx::query_scalar("SELECT MIN(id) FROM game_servers WHERE instance_id=?").bind(&instance).fetch_one(&mut *tx).await?;
        let Some(server)=server else {continue;};
        let economy:i64=sqlx::query_scalar("SELECT COALESCE((SELECT MIN(b.id) FROM game_servers b WHERE b.economy_group<>'' AND b.economy_group=a.economy_group COLLATE NOCASE),a.id) FROM game_servers a WHERE a.id=?").bind(server).fetch_one(&mut *tx).await?;
        // The first write serializes repeated workers and player deposits against this bill.
        crate::routes::guild_bank::ensure_wallet(&mut tx,economy,&guild).await?;
        let chunks:i64=sqlx::query_scalar("SELECT COUNT(*) FROM guild_claims WHERE guild_id=?").bind(&guild).fetch_one(&mut *tx).await?;
        let members:i64=sqlx::query_scalar("SELECT COUNT(*) FROM guild_members WHERE guild_id=?").bind(&guild).fetch_one(&mut *tx).await?;
        let amount=chunks.checked_mul(policy.upkeep_chunk_cents).and_then(|c|members.checked_mul(policy.upkeep_member_cents).and_then(|m|c.checked_add(m))).ok_or_else(||AppError::bad_request("upkeep overflow"))?;
        sqlx::query("INSERT OR IGNORE INTO faction_upkeep(guild_id,day,server_id,chunks,members,amount_cents,billed_at) VALUES(?,?,?,?,?,?,?)")
            .bind(&guild).bind(&today).bind(server).bind(chunks).bind(members).bind(amount).bind(crate::db::now()).execute(&mut *tx).await?;
        let bills:Vec<(String,i64)>=sqlx::query_as("SELECT day,amount_cents FROM faction_upkeep WHERE guild_id=? AND paid_at IS NULL ORDER BY day")
            .bind(&guild).fetch_all(&mut *tx).await?;
        for(day,cents)in bills {
            let fee=cents as f64/100.0;
            let changed=sqlx::query("UPDATE guild_wallets SET balance=balance-?,updated_at=? WHERE server_id=? AND guild_id=? AND balance>=?")
                .bind(fee).bind(crate::db::now()).bind(economy).bind(&guild).bind(fee).execute(&mut *tx).await?.rows_affected();
            if changed!=1 {break;}
            sqlx::query("UPDATE faction_upkeep SET paid_at=? WHERE guild_id=? AND day=? AND paid_at IS NULL")
                .bind(crate::db::now()).bind(&guild).bind(&day).execute(&mut *tx).await?;
            crate::routes::guild_bank::log(&mut tx,economy,&guild,"server","upkeep",fee,&format!("Daily upkeep {day}")).await?;
            sqlx::query("INSERT INTO economy_transactions(server_id,from_uuid,from_name,to_uuid,to_name,amount,description,created_at) VALUES(?,?,?,'server','Faction upkeep',?,?,?)")
                .bind(server).bind(format!("guild:{guild}")).bind(&guild).bind(fee).bind(format!("Faction upkeep {day}")).bind(crate::db::now()).execute(&mut *tx).await?;
            paid+=1;
        }
        tx.commit().await?;
    }
    Ok(format!("{paid} faction upkeep bills paid"))
}

/// Member-facing billing status; ledger rows are never recomputed after billing.
pub async fn upkeep_view(state:&AppState,guild:&str)->AppResult<serde_json::Value> {
    let Some(policy)=velora_core::load(state).await? else {return Ok(serde_json::Value::Null);};
    if !policy.enabled("factions") || !policy.enabled("economy") {return Ok(serde_json::Value::Null);}
    let (chunks,members):(i64,i64)=sqlx::query_as("SELECT (SELECT COUNT(*) FROM guild_claims WHERE guild_id=?),(SELECT COUNT(*) FROM guild_members WHERE guild_id=?)")
        .bind(guild).bind(guild).fetch_one(&state.db).await?;
    let (arrears,oldest):(i64,Option<String>)=sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0),MIN(billed_at) FROM faction_upkeep WHERE guild_id=? AND paid_at IS NULL")
        .bind(guild).fetch_one(&state.db).await?;
    let freezes_at=oldest.as_ref().and_then(|at|chrono::DateTime::parse_from_rfc3339(at).ok()).map(|at|at+Duration::days(policy.upkeep_grace_days));
    Ok(serde_json::json!({"daily_cents":chunks*policy.upkeep_chunk_cents+members*policy.upkeep_member_cents,"arrears_cents":arrears,
        "grace_days":policy.upkeep_grace_days,"freezes_at":freezes_at.map(|at|at.to_rfc3339_opts(chrono::SecondsFormat::Secs,true)),"claims_frozen":freezes_at.is_some_and(|at|at<=Utc::now()),"chunks":chunks,"members":members}))
}

/// A new SMP claim needs paid-up upkeep and must not exceed the faction's outpost allowance.
pub async fn upkeep_allowed(conn:&mut sqlx::SqliteConnection,policy:&velora_core::Policy,guild:&str)->AppResult<()> {
    let cutoff=(Utc::now()-Duration::days(policy.upkeep_grace_days)).to_rfc3339_opts(chrono::SecondsFormat::Secs,true);
    let overdue:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM faction_upkeep WHERE guild_id=? AND paid_at IS NULL AND billed_at<=?)")
        .bind(guild).bind(cutoff).fetch_one(&mut *conn).await?;
    if overdue {return Err(AppError::forbidden("new claims are frozen: deposit funds in the faction bank to pay overdue upkeep"));}
    Ok(())
}

pub async fn claims_allowed(conn:&mut sqlx::SqliteConnection,policy:&velora_core::Policy,guild:&str,server:i64,dimension:&str,x:i32,z:i32)->AppResult<Option<String>> {
    upkeep_allowed(conn,policy,guild).await?;
    crate::routes::outposts::claim_anchor(conn,guild,server,dimension,x,z).await
}

pub async fn outpost_limit(conn:&mut sqlx::SqliteConnection,policy:&velora_core::Policy,guild:&str)->AppResult<i64> {
    let _=(conn,policy,guild);
    Ok(crate::routes::outposts::MAX_OUTPOSTS)
}

/// Separate pieces of land: chunks touching on an edge in the same dimension belong together.
pub fn territories(chunks:&[(String,i32,i32)])->i64 {
    let set:std::collections::HashSet<(&str,i32,i32)>=chunks.iter().map(|(d,x,z)|(d.as_str(),*x,*z)).collect();
    let mut seen=std::collections::HashSet::new();
    let mut groups=0;
    for start in &set {
        if !seen.insert(*start) {continue;}
        groups+=1;
        let mut stack=vec![*start];
        while let Some((d,x,z))=stack.pop() {
            for next in [(d,x+1,z),(d,x-1,z),(d,x,z+1),(d,x,z-1)] {
                if set.contains(&next) && seen.insert(next) {stack.push(next);}
            }
        }
    }
    groups
}

pub async fn tiers(conn:&mut sqlx::SqliteConnection,guild:&str,track:&str)->AppResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT tiers FROM faction_upgrades WHERE guild_id=? AND track=?),0)").bind(guild).bind(track).fetch_one(conn).await?)
}

pub const TRACKS:&[&str]=&["claims","members","outposts","vault"];

/// (price in cents, maximum tiers) for one upgrade track.
pub fn track_terms(policy:&velora_core::Policy,track:&str)->Option<(i64,i64)> {
    match track {
        "claims"=>Some((policy.upgrade_claims_cents,policy.upgrade_claims_max)),
        "members"=>Some((policy.upgrade_members_cents,policy.upgrade_members_max)),
        "outposts"=>Some((policy.upgrade_outposts_cents,policy.upgrade_outposts_max.min(crate::routes::outposts::MAX_OUTPOSTS))),
        "vault"=>Some((policy.faction_vault_cents,policy.faction_vault_max)),
        _=>None,
    }
}

/// Upgrade tiers, prices and the land/vault state they unlock, for the faction screens.
pub async fn upgrades_view(conn:&mut sqlx::SqliteConnection,policy:&velora_core::Policy,guild:&str,economy:i64)->AppResult<serde_json::Value> {
    let mut tracks=serde_json::Map::new();
    for track in TRACKS {
        let (price,max)=track_terms(policy,track).expect("registered track");
        let have=tiers(&mut *conn,guild,track).await?;
        tracks.insert((*track).into(),serde_json::json!({"tiers":have,"max":max,"price_cents":price,"next_price_cents":if have<max {Some(price)} else {None}}));
    }
    let chunks:Vec<(String,i32,i32)>=sqlx::query_as("SELECT dimension,chunk_x,chunk_z FROM guild_claims WHERE guild_id=?").bind(guild).fetch_all(&mut *conn).await?;
    let balance:Option<f64>=sqlx::query_scalar("SELECT balance FROM guild_wallets WHERE server_id=? AND guild_id=?").bind(economy).bind(guild).fetch_optional(&mut *conn).await?;
    let flags:Vec<(String,String,Option<i64>,Option<String>,Option<i32>,Option<i32>,Option<i32>)>=sqlx::query_as("SELECT id,status,server_id,dimension,x,y,z FROM faction_outposts WHERE guild_id=? ORDER BY purchased_at,id").bind(guild).fetch_all(&mut *conn).await?;
    Ok(serde_json::json!({"tracks":tracks,"territories":territories(&chunks),"outposts":flags.iter().filter(|f|f.1=="placed").count(),
        "flags":flags.iter().map(|(id,status,server,dimension,x,y,z)|serde_json::json!({"id":id,"status":status,"server_id":server,"dimension":dimension,"x":x,"y":y,"z":z})).collect::<Vec<_>>(),
        "outpost_limit":outpost_limit(&mut *conn,policy,guild).await?,"claims_per_tier":policy.upgrade_claims_chunks,
        "members_per_tier":policy.upgrade_members_slots,"bank_cents":velora_core::cents(balance.unwrap_or(0.0).max(0.0)).unwrap_or(0)}))
}

/// Buy the next tier from the faction bank. The bank debit, ledger rows and new tier commit together;
/// `expected_tiers` makes a repeated click fail instead of buying twice.
pub async fn buy_upgrade(tx:&mut sqlx::Transaction<'_,sqlx::Sqlite>,policy:&velora_core::Policy,guild:&str,server:i64,economy:i64,actor:&str,track:&str,expected_tiers:i64)->AppResult<i64> {
    if !policy.enabled("factions")||!policy.enabled("economy") {return Err(AppError::forbidden("factions or economy is disabled"));}
    let (price,max)=track_terms(policy,track).ok_or_else(||AppError::bad_request("unknown upgrade"))?;
    let have=tiers(&mut **tx,guild,track).await?;
    if have!=expected_tiers {return Err(AppError::conflict("this upgrade changed; reload before buying"));}
    if have>=max {return Err(AppError::bad_request("this upgrade is already at its maximum"));}
    if track=="outposts" {
        if !policy.enabled("vaults") {return Err(AppError::forbidden("enable vaults before purchasing an outpost flag"));}
        crate::routes::outposts::issue(tx,guild,server,actor).await?;
    }
    let fee=price as f64/100.0;
    if price>0 {
        crate::routes::guild_bank::adjust_wallet(&mut **tx,economy,guild,-fee).await
            .map_err(|_|AppError::bad_request("the faction bank cannot pay for this upgrade"))?;
        crate::routes::guild_bank::log(&mut **tx,economy,guild,actor,"upgrade",fee,&format!("Upgrade: {track} tier {}",have+1)).await?;
        sqlx::query("INSERT INTO economy_transactions(server_id,from_uuid,from_name,to_uuid,to_name,amount,description,created_at) VALUES(?,?,?,'server','Faction upgrades',?,?,?)")
            .bind(server).bind(format!("guild:{guild}")).bind(guild).bind(fee).bind(format!("Faction upgrade: {track}")).bind(crate::db::now()).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO faction_upgrades(guild_id,track,tiers,updated_at) VALUES(?,?,1,?) ON CONFLICT(guild_id,track) DO UPDATE SET tiers=tiers+1,updated_at=excluded.updated_at")
        .bind(guild).bind(track).bind(crate::db::now()).execute(&mut **tx).await?;
    // Claim capacity uses the existing limit column so every claim path honours it.
    if track=="claims" {
        sqlx::query("UPDATE guilds SET max_claims=max_claims+? WHERE id=?").bind(policy.upgrade_claims_chunks).bind(guild).execute(&mut **tx).await?;
    }
    Ok(have+1)
}

pub async fn reward_rival_kill(tx:&mut sqlx::Transaction<'_,sqlx::Sqlite>,server:&crate::routes::servers::ServerRow,killer:&str,name:&str,victim:&str)->AppResult<()> {
    if killer==victim {return Ok(());}
    let Some(policy)=velora_core::for_server(&mut **tx,server.id).await? else {return Ok(());};
    if !policy.enabled("factions")||!policy.enabled("economy") {return Ok(());}
    let now=crate::db::now();
    let relation:Option<(i64,String,i64,i64)>=sqlx::query_as("SELECT r.id,r.guild_id,r.reward_cents,r.reward_bps FROM guild_relations r JOIN guild_members k ON k.guild_id=r.guild_id JOIN guild_members v ON v.guild_id=r.other_guild_id WHERE k.uuid=? AND v.uuid=? AND r.instance_id=? AND r.relation='rival' AND r.status='accepted' AND (r.expires_at IS NULL OR r.expires_at>?) ORDER BY r.id LIMIT 1")
        .bind(killer).bind(victim).bind(&server.instance_id).bind(&now).fetch_optional(&mut **tx).await?;
    let Some((relation,guild,fixed,bps))=relation else {return Ok(());};
    let cutoff=(Utc::now()-Duration::minutes(policy.rivalry_cooldown_minutes)).to_rfc3339_opts(chrono::SecondsFormat::Secs,true);
    let repeated:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM faction_rival_kills WHERE guild_id=? AND victim_uuid=? AND created_at>?)")
        .bind(&guild).bind(victim).bind(cutoff).fetch_one(&mut **tx).await?;
    if repeated {return Ok(());}
    let balance:Option<f64>=sqlx::query_scalar("SELECT balance FROM server_economy WHERE server_id=? AND uuid=?").bind(server.economy_id).bind(victim).fetch_optional(&mut **tx).await?;
    let Some(balance)=balance else {return Ok(());};
    let available=velora_core::cents(balance)?;
    let amount=(if bps>0 {available*bps/10000} else {fixed}).min(available);
    if amount<=0 {return Ok(());}
    crate::routes::economy::ensure_balance(tx,server.economy_id,killer,name).await?;
    sqlx::query("UPDATE server_economy SET balance=balance-?,updated_at=? WHERE server_id=? AND uuid=?").bind(amount as f64/100.0).bind(&now).bind(server.economy_id).bind(victim).execute(&mut **tx).await?;
    sqlx::query("UPDATE server_economy SET balance=balance+?,updated_at=? WHERE server_id=? AND uuid=?").bind(amount as f64/100.0).bind(&now).bind(server.economy_id).bind(killer).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO faction_rival_kills(server_id,relation_id,guild_id,killer_uuid,victim_uuid,amount_cents,created_at) VALUES(?,?,?,?,?,?,?)")
        .bind(server.id).bind(relation).bind(&guild).bind(killer).bind(victim).bind(amount).bind(&now).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO economy_transactions(server_id,from_uuid,from_name,to_uuid,to_name,amount,description,created_at) VALUES(?,?,'Rival player',?,?,?,'Rivalry kill reward',?)")
        .bind(server.id).bind(victim).bind(killer).bind(name).bind(amount as f64/100.0).bind(&now).execute(&mut **tx).await?;
    Ok(())
}
