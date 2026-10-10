use super::*;
use rand::Rng;

#[derive(Deserialize)]
pub struct RouletteBody { pub bet: f64, pub selection: String, pub number: Option<u32>, pub operation_id: String }
pub async fn roulette(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<RouletteBody>) -> AppResult<Json<Value>> {
    play(&state, &auth, sid, "roulette", "Roulette", p.bet, false, Some(&p.operation_id), |_, _| {
        let number = rand::thread_rng().gen_range(0..=36);
        let multiplier = casino::roulette_payout(number, &p.selection, p.number).ok_or_else(|| AppError::bad_request("invalid roulette selection"))?;
        Ok(Played { multiplier, detail: json!({"number":number,"red":casino::roulette_red(number),"selection":p.selection,"straight":p.number}) })
    }).await
}

#[derive(sqlx::FromRow)]
struct Ladder { id: i64, bet: f64, steps: i64, survival: f64, house_edge: f64, max_steps: i64, max_payout: f64, status: String }
const COLS: &str = "id,bet,steps,survival,house_edge,max_steps,max_payout,status";
impl Ladder {
    fn multiplier(&self) -> f64 { if self.steps == 0 { 1.0 } else { (1.0-self.house_edge) / self.survival.powi(self.steps as i32) } }
    fn payout(&self) -> f64 { round2((self.bet*self.multiplier()).min(self.max_payout)) }
    fn view(&self) -> Value { json!({"id":self.id,"bet":self.bet,"steps":self.steps,"max_steps":self.max_steps,"survival":self.survival,
        "multiplier":self.multiplier(),"cashout":if self.status=="lost" {0.0} else {self.payout()},"status":self.status}) }
}
pub(super) async fn active_burst(db: &sqlx::SqlitePool, uuid: &str, server: i64) -> AppResult<Option<Value>> {
    let row: Option<Ladder> = sqlx::query_as(&format!("SELECT {COLS} FROM casino_burst WHERE uuid=? AND server_id=? AND status='active'"))
        .bind(uuid).bind(server).fetch_optional(db).await?; Ok(row.map(|row| row.view()))
}
#[derive(Deserialize)]
pub struct BurstStart { pub bet: f64, pub operation_id: String }
pub async fn burst_start(auth: AuthUser, State(state): State<AppState>, Path(sid): Path<i64>, Json(p): Json<BurstStart>) -> AppResult<Json<Value>> {
    let c = ctx_open(&state,sid).await?;
    if !c.cfg.burst.enabled { return Err(AppError::forbidden("Burst is switched off")); }
    let bet = check_bet(p.bet,c.cfg.burst.min_bet,c.cfg.burst.max_bet)?;
    let operation = receipt_id(&auth.uuid,"burst-start",Some(&p.operation_id))?;
    let (mut tx,previous) = begin_operation(&state,sid,&operation).await?;
    if let Some(previous) = previous { return Ok(Json(previous)); }
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM casino_burst WHERE server_id=? AND uuid=? AND status='active')")
        .bind(sid).bind(&auth.uuid).fetch_one(&mut *tx).await?;
    if active { return Err(AppError::conflict("finish your current Burst round first")); }
    if loss_limit_reached(&mut tx,&c.cfg,&auth.uuid).await? { return Err(AppError::forbidden(LOSS_LIMIT_MESSAGE)); }
    let who = Who {economy:c.server.economy_id,server:sid,uuid:&auth.uuid,name:&auth.username};
    let balance = debit(&mut tx,who,bet,"Casino: Burst stake").await?;
    let id: i64 = sqlx::query_scalar("INSERT INTO casino_burst(server_id,uuid,bet,survival,house_edge,max_steps,max_payout,created_at) VALUES(?,?,?,?,?,?,?,?) RETURNING id")
        .bind(sid).bind(&auth.uuid).bind(bet).bind(c.cfg.burst.survival).bind(c.cfg.burst.house_edge).bind(c.cfg.burst.max_steps).bind(c.cfg.max_payout).bind(crate::db::now()).fetch_one(&mut *tx).await?;
    let row = Ladder {id,bet,steps:0,survival:c.cfg.burst.survival,house_edge:c.cfg.burst.house_edge,max_steps:c.cfg.burst.max_steps as i64,max_payout:c.cfg.max_payout,status:"active".into()};
    finish_operation(tx,sid,&operation,json!({"game":row.view(),"balance":balance})).await
}
#[derive(Deserialize)]
pub struct BurstAction { pub id: i64, pub expected_steps: i64, pub operation_id: String }
async fn act(auth: AuthUser,state: AppState,sid:i64,p:BurstAction,advance:bool)->AppResult<Json<Value>> {
    // Settling an existing stake remains possible if the admin closes this game.
    let c = ctx(&state,sid).await?;
    let operation = receipt_id(&auth.uuid,if advance {"burst-next"} else {"burst-cash"},Some(&p.operation_id))?;
    let (mut tx,previous) = begin_operation(&state,sid,&operation).await?;
    if let Some(previous)=previous {return Ok(Json(previous));}
    let mut row: Ladder = sqlx::query_as(&format!("SELECT {COLS} FROM casino_burst WHERE id=? AND uuid=? AND server_id=? AND status='active'"))
        .bind(p.id).bind(&auth.uuid).bind(sid).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::conflict("round is already settled or belongs to another player"))?;
    if row.steps != p.expected_steps { return Err(AppError::conflict("round advanced; reload before acting")); }
    if advance {
        if !c.cfg.enabled || !c.cfg.burst.enabled {return Err(AppError::forbidden("Burst is closed; cash out your stake"));}
        if rand::thread_rng().gen::<f64>() >= row.survival { row.status="lost".into(); }
        else {row.steps+=1; if row.steps >= row.max_steps {row.status="cashed".into();}}
    } else {row.status="cashed".into();}
    sqlx::query("UPDATE casino_burst SET steps=?,status=? WHERE id=?").bind(row.steps).bind(&row.status).bind(row.id).execute(&mut *tx).await?;
    let who=Who {economy:c.server.economy_id,server:sid,uuid:&auth.uuid,name:&auth.username};
    let payout=if row.status=="cashed" {row.payout()} else {0.0};
    let balance=if payout>0.0 {credit(&mut tx,who,payout,"Casino: Burst cashout").await?} else {
        sqlx::query_scalar::<_,f64>("SELECT balance FROM server_economy WHERE server_id=? AND uuid=?").bind(who.economy).bind(who.uuid).fetch_one(&mut *tx).await?
    };
    if row.status!="active" { record(&mut tx,sid,&auth.uuid,&auth.username,"burst",row.bet,payout,&row.view()).await?; }
    finish_operation(tx,sid,&operation,json!({"game":row.view(),"balance":balance,"payout":payout})).await
}
pub async fn burst_advance(auth:AuthUser,State(state):State<AppState>,Path(sid):Path<i64>,Json(p):Json<BurstAction>)->AppResult<Json<Value>> {act(auth,state,sid,p,true).await}
pub async fn burst_cashout(auth:AuthUser,State(state):State<AppState>,Path(sid):Path<i64>,Json(p):Json<BurstAction>)->AppResult<Json<Value>> {act(auth,state,sid,p,false).await}

/// Finish abandoned ladders at their already-earned value. Odds and payout caps are snapshots from the stake.
pub(super) async fn settle_abandoned(state: &AppState) -> AppResult<usize> {
    let Some(policy) = crate::velora_core::load(state).await? else { return Ok(0); };
    let cutoff = (Utc::now() - Duration::hours(policy.casino_abandon_hours as i64))
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let due: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT id,server_id,uuid FROM casino_burst WHERE status='active' AND created_at<=? ORDER BY id LIMIT 100")
        .bind(&cutoff).fetch_all(&state.db).await?;
    let mut count = 0;
    for (id, server, uuid) in due {
        let economy = economy_scope(&state.db, server).await?;
        let operation = format!("burst-abandon:{id}");
        let (mut tx, previous) = begin_operation(state, server, &operation).await?;
        if previous.is_some() { continue; }
        let row: Option<Ladder> = sqlx::query_as(&format!("SELECT {COLS} FROM casino_burst WHERE id=? AND status='active' AND created_at<=?"))
            .bind(id).bind(&cutoff).fetch_optional(&mut *tx).await?;
        let Some(mut row) = row else { continue; };
        let name: String = sqlx::query_scalar("SELECT username FROM server_economy WHERE server_id=? AND uuid=?")
            .bind(economy).bind(&uuid).fetch_one(&mut *tx).await?;
        sqlx::query("UPDATE casino_burst SET status='cashed' WHERE id=? AND status='active'")
            .bind(id).execute(&mut *tx).await?;
        row.status = "cashed".into();
        let payout = row.payout();
        let balance = credit(&mut tx, Who {economy,server,uuid:&uuid,name:&name}, payout, "Casino: abandoned Burst cashout").await?;
        let mut detail = row.view();
        detail["abandoned"] = json!(true);
        record(&mut tx,server,&uuid,&name,"burst",row.bet,payout,&detail).await?;
        let _ = finish_operation(tx,server,&operation,json!({"game":detail,"balance":balance,"payout":payout})).await?;
        count += 1;
    }
    Ok(count)
}
