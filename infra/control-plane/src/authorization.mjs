/** Access authenticates; explicit active principal + scoped role grants authorize.
 * @param {D1Database} database
 * @param {{issuer:string,subject:string}} principal
 * @param {string} permission
 * @param {string} environment
 */
export async function requireOperatorPermission(database,principal,permission,environment) {
  if (!['development','beta','production'].includes(environment) || !principal.subject
      || !principal.issuer || !/^[a-z][a-z0-9.]{0,100}$/.test(permission)) throw Error('PERMISSION_DENIED');
  const actor = await database.prepare(`SELECT p.id FROM operator_principals p
    JOIN operator_role_bindings b ON b.principal_id=p.id
    JOIN operator_role_permissions rp ON rp.role_id=b.role_id
    WHERE p.access_issuer=? AND p.access_subject=? AND p.status='ACTIVE'
      AND b.environment_id=? AND rp.permission=? LIMIT 1`)
    .bind(principal.issuer,principal.subject,environment,permission).first();
  if (!actor) throw Error('PERMISSION_DENIED');
  return {actor_id:String(actor.id),environment,permission};
}

/** Audit participates in the caller's D1 batch; retries must not duplicate events.
 * @param {D1Database} database
 * @param {{event_key:string,actor_id:string,action:string,environment:string,target_id:string,request_id:string}} event
 */
export function auditStatement(database,event) {
  return database.prepare(`INSERT OR IGNORE INTO audit_events
    (event_key,actor_id,action,environment_id,target_id,request_id) VALUES (?,?,?,?,?,?)`)
    .bind(event.event_key,event.actor_id,event.action,event.environment,event.target_id,event.request_id);
}

/** Signature verification must precede this bounded durable replay check.
 * @param {D1Database} database
 * @param {{delivery:string,event:string,body_sha256:string}} verified
 */
export async function recordGitHubDelivery(database,verified) {
  const result=await database.prepare(`INSERT OR IGNORE INTO github_deliveries
    (id,event_name,body_sha256,status) VALUES (?,?,?,'RECEIVED')`)
    .bind(verified.delivery,verified.event,verified.body_sha256).run();
  const existing=await database.prepare('SELECT event_name,body_sha256 FROM github_deliveries WHERE id=?').bind(verified.delivery).first();
  if (existing && (existing.event_name!==verified.event || existing.body_sha256!==verified.body_sha256)) throw Error('DELIVERY_CONFLICT');
  return {created:result.meta.changes===1};
}
