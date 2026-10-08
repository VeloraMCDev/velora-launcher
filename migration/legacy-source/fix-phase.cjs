const fs=require('fs');function edit(p,f){fs.writeFileSync(p,f(fs.readFileSync(p,'utf8').replace(/\r\n/g,'\n')))}
edit('panel/server/src/routes/economy.rs',s=>{
 s=s.replace('if payload.amount <= 0.0 {','if !payload.amount.is_finite() || payload.amount <= 0.0 {');
 s=s.replace('pub struct MarketListPayload {','pub struct MarketListPayload {\n    pub operation_id: String,\n    pub item_data: Option<String>,');
 const a=s.indexOf('pub async fn server_market_list('),b=s.indexOf('#[derive(Deserialize)]\npub struct MarketBuyPayload',a);
 let seg=s.slice(a,b).replace('    let now =', `    if !payload.price.is_finite() || payload.price <= 0.0 || payload.amount <= 0 || payload.amount > 64 { return Err(AppError::bad_request("Invalid listing")); }
    let (mut tx, previous) = begin_operation(&state, server.id, &payload.operation_id).await?;
    if let Some(previous) = previous { return Ok(Json(previous)); }
    let now =`).replace('amount, price, created_at)','amount, price, created_at, item_data)').replace('VALUES (?, ?, ?, ?, ?, ?, ?, ?)','VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)').replace('.bind(&now)\n    .fetch_one(&state.db)', '.bind(&now)\n    .bind(&payload.item_data)\n    .fetch_one(&mut *tx)').replace('Ok(Json(serde_json::json!({ "id": id, "ok": true })))','finish_operation(tx, server.id, &payload.operation_id, serde_json::json!({ "id": id, "ok": true })).await');
 s=s.slice(0,a)+seg+s.slice(b);
 s=s.replace('pub struct MarketBuyPayload {','pub struct MarketBuyPayload {\n    pub operation_id: String,');
 const c=s.indexOf('pub async fn server_market_buy(');
 seg=s.slice(c).replace('    let listing:', `    let (mut tx, previous) = begin_operation(&state, server.id, &payload.operation_id).await?;
    if let Some(previous) = previous { return Ok(Json(previous)); }
    let listing:`).replace('String, String, String, String, i32, f64)>','String, String, String, String, i32, f64, Option<String>)>').replace('SELECT seller_uuid, seller_name, item_id, item_name, amount, price','DELETE FROM server_market\n         WHERE id = ? AND server_id = ?\n         RETURNING seller_uuid, seller_name, item_id, item_name, amount, price, item_data').replace('         FROM server_market\n         WHERE id = ? AND server_id = ?', '').replace('.fetch_optional(&state.db)', '.fetch_optional(&mut *tx)').replace('item_name, amount, price))','item_name, amount, price, item_data))').replace('    let mut tx = state.db.begin().await?;', '    ensure_balance(&mut tx, server.id, &payload.buyer_uuid, &payload.buyer_name).await?;');
 seg=seg.replace('    tx.commit().await?;\n\n    Ok(Json(serde_json::json!({','    finish_operation(tx, server.id, &payload.operation_id, serde_json::json!({').replace('        "item_id": item_id,','        "item_id": item_id,\n        "item_data": item_data,').replace('    })))\n}', '    })).await\n}');
 return s.slice(0,c)+seg;
});
edit('panel/server/src/routes/mod.rs',s=>s.replace('        .route("/economy/sync-balance",', '        .route("/economy/adjust", post(economy::server_adjust_balance))\n        .route("/economy/market", post(economy::server_market_read))\n        .route("/economy/sync-balance",'));
