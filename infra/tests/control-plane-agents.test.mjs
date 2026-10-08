import test from 'node:test';
import assert from 'node:assert/strict';
import {createTestHarness} from 'wrangler';
import {generateKeyPairSync,sign,createHash,randomUUID} from 'node:crypto';
import {generateKeyPair,SignJWT} from 'jose';
import {createAgentApi} from '../control-plane/src/agent-api.ts';
import {verifyAccessPrincipal} from '../control-plane/src/access.mjs';
import {canonicalAgentRequest} from '../control-plane/src/agent-protocol.mjs';

test('actual D1 agent API atomically consumes hash-only tokens, rejects replay/privilege escalation and revokes signatures',{timeout:60000},async()=>{
  const server=createTestHarness({workers:[{configPath:'control-plane/wrangler.jsonc'}]});
  try{
    await server.listen();const worker=server.getWorker();for(const name of ['HARNESS_DB','IDENTITY_DB'])await worker.applyD1Migrations(name);
    const env={...await worker.getEnv(),AGENTS_ENABLED:'1',ENVIRONMENT:'development',CI_API_HOST:'agents.example.com',OPERATOR_API_HOST:'operator.example.com',ACCESS_TEAM_DOMAIN:'https://fixture.cloudflareaccess.com',ACCESS_AUD:'fixture-app'};
    const operatorKey=await generateKeyPair('RS256'),now=Math.floor(Date.now()/1000);
    const operator=await new SignJWT({email:'operator@example.invalid',type:'app'}).setProtectedHeader({alg:'RS256'}).setIssuer(env.ACCESS_TEAM_DOMAIN).setAudience(env.ACCESS_AUD).setSubject('fixture-operator').setIssuedAt(now).setExpirationTime(now+300).sign(operatorKey.privateKey);
    const api=createAgentApi((request,config)=>verifyAccessPrincipal(request,config,operatorKey.publicKey));
    const action=(path,idem=randomUUID(),origin='https://operator.example.com',body={schema:1})=>api.fetch(new Request('https://operator.example.com/api/v1/agents'+path,{method:'POST',headers:{'cf-access-jwt-assertion':operator,'content-type':'application/json','x-velora-action':'1','idempotency-key':idem,origin},body:JSON.stringify(body)}),env);
    const read=(suffix='')=>api.fetch(new Request('https://operator.example.com/api/v1/agents'+suffix,{headers:{'cf-access-jwt-assertion':operator}}),env);
    assert.equal((await action('/enrollment-tokens')).status,403);
    await env.IDENTITY_DB.batch([
      env.IDENTITY_DB.prepare("INSERT INTO operator_principals VALUES ('operator',?,'fixture-operator','ACTIVE')").bind(env.ACCESS_TEAM_DOMAIN),
      env.IDENTITY_DB.prepare("INSERT INTO operator_roles VALUES ('agent-manager','Fixture Development agent manager')"),
      ...['agent.read','agent.enroll','agent.revoke'].map(permission=>env.IDENTITY_DB.prepare("INSERT INTO operator_role_permissions VALUES ('agent-manager',?)").bind(permission)),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_bindings VALUES ('operator','agent-manager','development')")
    ]);
    assert.equal((await action('/enrollment-tokens')).status,403); // Enrollment defaults paused.
    await env.HARNESS_DB.prepare("UPDATE automation_flags SET enrollment_enabled=1 WHERE environment_id='development'").run();
    const issuanceKey=randomUUID(),issued=await action('/enrollment-tokens',issuanceKey);
    assert.equal(issued.status,201);const token=(await issued.json()).token;
    assert.equal((await action('/enrollment-tokens',issuanceKey)).status,409);
    const stored=await env.HARNESS_DB.prepare('SELECT * FROM agent_enrollment_tokens').first();
    assert.equal(stored.token_hash,createHash('sha256').update(token).digest('hex'));
    assert.ok(!JSON.stringify(stored).includes(token));assert.equal(stored.consumed_at,null);
    assert.equal((await action('/enrollment-tokens',randomUUID(),'https://attacker.example.com')).status,403);
    assert.equal((await action('/enrollment-tokens?environment=production')).status,403);
    const {privateKey,publicKey}=generateKeyPairSync('ed25519'),raw=publicKey.export({format:'der',type:'spki'}).subarray(-32);
    const keyId=createHash('sha256').update(raw).digest('hex'),agentId='agt_'+keyId;
    const enrolled={schema:1,token,public_key:raw.toString('base64url'),agent_version:'0.1.0',build_sha:'a'.repeat(40)};
    const heartbeat={schema:1,agent_version:'0.1.0',build_sha:'a'.repeat(40),uptime_seconds:12,capabilities:['REPORT_HEARTBEAT'],host:{os:'linux',arch:'x86_64'}};
    const signed=(path,value,nonce=randomUUID(),timestamp=now,key=privateKey)=>{
      const body=JSON.stringify(value),checksum=createHash('sha256').update(body).digest('hex');
      const signature=sign(null,Buffer.from(canonicalAgentRequest('/api/v1/agents'+path,String(timestamp),nonce,checksum)),key).toString('base64url');
      return api.fetch(new Request('https://agents.example.com/api/v1/agents'+path,{method:'POST',headers:{'content-type':'application/json','velora-agent-id':agentId,'velora-key-id':keyId,'velora-timestamp':String(timestamp),'velora-nonce':nonce,'velora-body-sha256':checksum,'velora-signature':signature},body}),env);
    };
    const expired=(await (await action('/enrollment-tokens')).json()).token;
    await env.HARNESS_DB.prepare('UPDATE agent_enrollment_tokens SET expires_at=? WHERE token_hash=?').bind(now-1,createHash('sha256').update(expired).digest('hex')).run();
    assert.equal((await signed('/enroll',{...enrolled,token:expired})).status,403);
    assert.equal((await signed('/enroll',{...enrolled,public_key:Buffer.alloc(32).toString('base64url')})).status,403);
    const enrollment=await Promise.all(Array.from({length:6},()=>signed('/enroll',enrolled)));
    assert.equal(enrollment.filter(response=>response.status===201).length,1);
    assert.ok(enrollment.filter(response=>response.status!==201).every(response=>[403,409].includes(response.status)));
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS total FROM agents').first()).total,1);
    assert.equal((await env.HARNESS_DB.prepare("SELECT COUNT(*) AS total FROM audit_events WHERE action='agent.enroll'").first()).total,1);
    assert.notEqual((await env.HARNESS_DB.prepare('SELECT consumed_at FROM agent_enrollment_tokens WHERE token_hash=?').bind(stored.token_hash).first()).consumed_at,null);
    assert.equal((await signed('/enroll',enrolled)).status,403);
    const wrongKey=generateKeyPairSync('ed25519').privateKey;
    assert.equal((await signed('/heartbeat',heartbeat,randomUUID(),now,wrongKey)).status,403);
    assert.equal((await signed('/heartbeat',heartbeat,randomUUID(),now-121)).status,403);
    assert.equal((await signed('/heartbeat',{...heartbeat,capabilities:['DEPLOY_COMPOSE']})).status,400);
    const replayNonce=randomUUID(),replays=await Promise.all(Array.from({length:6},()=>signed('/heartbeat',heartbeat,replayNonce)));
    assert.equal(replays.filter(response=>response.status===200).length,1);
    assert.ok(replays.filter(response=>response.status!==200).every(response=>response.status===403));
    assert.equal((await signed('/heartbeat',heartbeat,replayNonce)).status,403);
    const listing=await read();assert.equal(listing.status,200);const items=(await listing.json()).items;
    assert.equal(items.length,1);assert.equal(items[0].status,'ONLINE');assert.deepEqual(items[0].capabilities,['REPORT_HEARTBEAT']);assert.ok(!JSON.stringify(items).includes(token));
    assert.equal((await read('?environment=production')).status,403);
    const revokeKey=randomUUID();assert.equal((await action('/'+agentId+'/revoke',revokeKey,'https://attacker.example.com')).status,403);
    assert.equal((await action('/'+agentId+'/revoke',revokeKey)).status,200);
    assert.equal((await action('/'+agentId+'/revoke',revokeKey)).status,200);
    assert.equal((await signed('/heartbeat',heartbeat)).status,403);
    assert.equal((await env.HARNESS_DB.prepare("SELECT COUNT(*) AS total FROM audit_events WHERE action='agent.revoke'").first()).total,1);
    assert.equal((await read()).status,200);assert.equal((await (await read()).json()).items[0].status,'REVOKED');
    assert.equal((await api.fetch(new Request('https://agents.example.com/api/v1/agents/heartbeat'),{...env,AGENTS_ENABLED:'0'})).status,403);
  }finally{await server.close();}
});
