import test from 'node:test';
import assert from 'node:assert/strict';
import {createTestHarness} from 'wrangler';
import {generateKeyPairSync,sign,createHash,randomUUID} from 'node:crypto';
import {canonicalAgentRequest} from '../control-plane/src/agent-protocol.mjs';

test('exported Worker verifies native Ed25519 enrollment and heartbeat, and rejects replay',{timeout:60000},async()=>{
  const server=createTestHarness({workers:[{configPath:'control-plane/wrangler.jsonc',vars:{CONTROL_API_ENABLED:'1',AGENTS_ENABLED:'1',CI_API_HOST:'agent.example.com'}}]});
  try{
    await server.listen();const worker=server.getWorker();await worker.applyD1Migrations('HARNESS_DB');
    const env=await worker.getEnv(),now=Math.floor(Date.now()/1000);
    // Public synthetic fixture. No operator session or real enrollment secret is involved.
    const token=Buffer.alloc(32,9).toString('base64url');
    await env.HARNESS_DB.prepare("UPDATE automation_flags SET enrollment_enabled=1 WHERE environment_id='development'").run();
    await env.HARNESS_DB.prepare('INSERT INTO agent_enrollment_tokens VALUES (?, ?, ?, ?, NULL, ?)')
      .bind(createHash('sha256').update(token).digest('hex'),'development','["REPORT_HEARTBEAT"]',now+600,now).run();
    const {privateKey,publicKey}=generateKeyPairSync('ed25519');
    const raw=publicKey.export({format:'der',type:'spki'}).subarray(-32),keyId=createHash('sha256').update(raw).digest('hex');
    const request=(path,value,nonce=randomUUID())=>{
      const body=JSON.stringify(value),checksum=createHash('sha256').update(body).digest('hex');
      return ['https://agent.example.com'+path,{method:'POST',headers:{'content-type':'application/json',
        'velora-agent-id':'agt_'+keyId,'velora-key-id':keyId,'velora-timestamp':String(now),'velora-nonce':nonce,
        'velora-body-sha256':checksum,'velora-signature':sign(null,Buffer.from(canonicalAgentRequest(path,String(now),nonce,checksum)),privateKey).toString('base64url')},body}];
    };
    const enrolled=await worker.fetch(...request('/api/v1/agents/enroll',{schema:1,token,public_key:raw.toString('base64url'),agent_version:'0.1.0',build_sha:'a'.repeat(40)}));
    assert.equal(enrolled.status,201);assert.equal((await enrolled.json()).agent_id,'agt_'+keyId);
    const heartbeat=request('/api/v1/agents/heartbeat',{schema:1,agent_version:'0.1.0',build_sha:'a'.repeat(40),uptime_seconds:3,capabilities:['REPORT_HEARTBEAT'],host:{os:'linux',arch:'x86_64'}});
    assert.equal((await worker.fetch(...heartbeat)).status,200);
    assert.equal((await worker.fetch(...heartbeat)).status,403);
    assert.ok((await env.HARNESS_DB.prepare('SELECT heartbeat_at FROM agents').first()).heartbeat_at>=now);
  }finally{await server.close();}
});
