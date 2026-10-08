import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createTestHarness } from 'wrangler';
import { auditStatement, requireOperatorPermission, recordGitHubDelivery } from '../control-plane/src/authorization.mjs';

test('expand-only registration models preserve baseline records and deny unregistered or cross-environment authority', {timeout:30000}, async()=>{
  const server=createTestHarness({workers:[{configPath:'control-plane/wrangler.jsonc'}]});
  try{
    await server.listen();const env=await server.getWorker().getEnv();
    const migrate=async(db,path)=>{for(const sql of (await readFile(path,'utf8')).replace(/^--.*$/gm,'').split(';').filter(s=>s.trim()))await db.prepare(sql).run();};
    await migrate(env.HARNESS_DB,'control-plane/migrations/harness/0001_bootstrap.sql');
    await migrate(env.IDENTITY_DB,'control-plane/migrations/identity/0001_bootstrap.sql');
    const proof='aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
    await env.HARNESS_DB.prepare("INSERT INTO bootstrap_checks(id,environment,status) VALUES (?,'development','SUCCEEDED')").bind(proof).run();
    await migrate(env.HARNESS_DB,'control-plane/migrations/harness/0002_registration_models.sql');
    await migrate(env.IDENTITY_DB,'control-plane/migrations/identity/0002_operator_authorization.sql');
    assert.equal((await env.HARNESS_DB.prepare('SELECT status FROM bootstrap_checks WHERE id=?').bind(proof).first()).status,'SUCCEEDED');
    assert.equal((await env.HARNESS_DB.prepare('SELECT schema_version FROM schema_metadata').first()).schema_version,2);
    assert.equal((await env.IDENTITY_DB.prepare('SELECT schema_version FROM schema_metadata').first()).schema_version,2);
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS total FROM repositories').first()).total,0);
    assert.equal((await env.IDENTITY_DB.prepare('SELECT COUNT(*) AS total FROM operator_principals').first()).total,0);
    assert.equal((await env.HARNESS_DB.prepare('SELECT SUM(registration_enabled)+SUM(deployment_enabled) AS enabled FROM automation_flags').first()).enabled,0);
    const principal={issuer:'https://fixture.cloudflareaccess.com',subject:'synthetic-subject'};
    await assert.rejects(requireOperatorPermission(env.IDENTITY_DB,principal,'candidate.read','development'));
    await env.IDENTITY_DB.batch([
      env.IDENTITY_DB.prepare("INSERT INTO operator_principals VALUES ('operator',?,?,'ACTIVE')").bind(principal.issuer,principal.subject),
      env.IDENTITY_DB.prepare("INSERT INTO operator_roles VALUES ('reader','Development reader')"),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_permissions VALUES ('reader','candidate.read')"),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_bindings VALUES ('operator','reader','development')")
    ]);
    assert.equal((await requireOperatorPermission(env.IDENTITY_DB,principal,'candidate.read','development')).actor_id,'operator');
    await assert.rejects(requireOperatorPermission(env.IDENTITY_DB,principal,'candidate.read','production'));
    await assert.rejects(requireOperatorPermission(env.IDENTITY_DB,principal,'deployment.approve.production','development'));
    await assert.rejects(requireOperatorPermission(env.IDENTITY_DB,{...principal,issuer:'https://other.cloudflareaccess.com'},'candidate.read','development'));
    await env.IDENTITY_DB.prepare("UPDATE operator_principals SET status='DISABLED' WHERE id='operator'").run();
    await assert.rejects(requireOperatorPermission(env.IDENTITY_DB,principal,'candidate.read','development'));
    const event={event_key:'synthetic-check',actor_id:'operator',action:'candidate.read',environment:'development',target_id:'fixture',request_id:proof};
    await env.HARNESS_DB.batch([auditStatement(env.HARNESS_DB,event),auditStatement(env.HARNESS_DB,event)]);
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS total FROM audit_events').first()).total,1);
    const delivery={delivery:proof,event:'workflow_run',body_sha256:'a'.repeat(64)};
    const attempts=await Promise.all(Array.from({length:6},()=>recordGitHubDelivery(env.HARNESS_DB,delivery)));
    assert.equal(attempts.filter(x=>x.created).length,1);
    assert.equal((await recordGitHubDelivery(env.HARNESS_DB,{...delivery,delivery:'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb'})).created,false);
    await assert.rejects(recordGitHubDelivery(env.HARNESS_DB,{...delivery,body_sha256:'b'.repeat(64)}),/DELIVERY_CONFLICT/);
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS total FROM github_deliveries').first()).total,1);
    await assert.rejects(env.HARNESS_DB.prepare("INSERT INTO services VALUES ('orphan','unknown','oci','ghcr.io/example/',1)").run());
  }finally{await server.close();}
});
