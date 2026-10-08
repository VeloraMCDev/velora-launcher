import test from 'node:test';
import assert from 'node:assert/strict';
import {createTestHarness} from 'wrangler';
import {generateKeyPairSync,sign,createHash,randomUUID} from 'node:crypto';
import {generateKeyPair,exportJWK,SignJWT} from 'jose';
import {setTimeout} from 'node:timers/promises';
import {createControlApi} from '../control-plane/src/api.ts';
import {verifyAccessPrincipal} from '../control-plane/src/access.mjs';
import {canonicalAgentRequest} from '../control-plane/src/agent-protocol.mjs';
import {verifyProbeJob} from '../control-plane/src/probe-job.mjs';
import {blockProbeDeployment,prepareProbeJob} from '../control-plane/src/probe-deployments.ts';
import {artifactMetadata} from '../scripts/artifact-metadata.mjs';

test('actual D1 and Workflow deliver only approved immutable probe jobs, deduplicate results and retain uncertain locks',{timeout:120000},async()=>{
  const signer=await generateKeyPair('EdDSA',{extractable:true});
  const jwk=await exportJWK(signer.privateKey),publicKey=jwk.x;
  const server=createTestHarness({workers:[{configPath:'control-plane/wrangler.jsonc',vars:{PROBE_DEPLOYMENTS_ENABLED:'1',PROBE_SIGNING_PUBLIC_KEY:publicKey,PROBE_SIGNING_PRIVATE_JWK:JSON.stringify(jwk)}}]});
  try{
    await server.listen();const worker=server.getWorker();for(const binding of ['HARNESS_DB','IDENTITY_DB'])await worker.applyD1Migrations(binding);
    const env={...await worker.getEnv(),CONTROL_API_ENABLED:'1',AGENTS_ENABLED:'1',CI_API_HOST:'agents.example.com',OPERATOR_API_HOST:'operator.example.com',ACCESS_TEAM_DOMAIN:'https://fixture.cloudflareaccess.com',ACCESS_AUD:'fixture'};
    const operatorKeys=await generateKeyPair('RS256'),now=Math.floor(Date.now()/1000);
    const operator=await new SignJWT({type:'app',email:'operator@example.invalid'}).setProtectedHeader({alg:'RS256'}).setIssuer(env.ACCESS_TEAM_DOMAIN).setAudience(env.ACCESS_AUD).setSubject('operator').setIssuedAt(now).setExpirationTime(now+600).sign(operatorKeys.privateKey);
    const api=createControlApi({operator:(request,config)=>verifyAccessPrincipal(request,config,operatorKeys.publicKey),ci:async()=>{throw Error('No CI in this fixture');}});
    const keys=generateKeyPairSync('ed25519'),raw=keys.publicKey.export({format:'der',type:'spki'}).subarray(-32);
    const keyId=createHash('sha256').update(raw).digest('hex'),agentId='agt_'+keyId;
    const metadata=artifactMetadata({schema:1,repository:'Example/infra',service_id:'deployment-probe',git_sha:'a'.repeat(40),git_ref:'refs/heads/main',build_run_id:'123',artifact_type:'oci',artifact_uri:'ghcr.io/veloramcdev/deployment-probe@sha256:'+'b'.repeat(64),sha256:'b'.repeat(64),oci_digest:'sha256:'+'b'.repeat(64),version:'0.1.0',build_status:'PASSED',test_status:'PASSED'});
    await env.HARNESS_DB.batch([
      env.HARNESS_DB.prepare("INSERT INTO repositories VALUES ('infra','Example/infra','123','456',1)"),
      env.HARNESS_DB.prepare("INSERT INTO services VALUES ('deployment-probe','infra','oci','ghcr.io/veloramcdev/deployment-probe@sha256:',1)"),
      env.HARNESS_DB.prepare("INSERT INTO artifacts(id,repository_id,service_id,artifact_type,artifact_uri,sha256) VALUES ('artifact','infra','deployment-probe','oci',?,?)").bind(metadata.artifact_uri,metadata.sha256),
      env.HARNESS_DB.prepare("INSERT INTO deployment_candidates(id,artifact_id,repository_id,service_id,git_sha,git_ref,build_run_id,metadata_json) VALUES (?,'artifact','infra','deployment-probe',?,?,?,?)").bind(metadata.candidate_id,metadata.git_sha,metadata.git_ref,metadata.build_run_id,JSON.stringify(metadata)),
      env.HARNESS_DB.prepare("INSERT INTO agent_enrollment_tokens(token_hash,environment_id,capabilities_json,expires_at,created_at) VALUES (?,'development','[\"REPORT_HEARTBEAT\"]',?,?)").bind('c'.repeat(64),now+600,now),
      env.HARNESS_DB.prepare("INSERT INTO agents(id,key_id,public_key,enrollment_token_hash,environment_id,capabilities_json,status,enrolled_at,heartbeat_at,heartbeat_json) VALUES (?,?,?,?,'development','[\"REPORT_HEARTBEAT\"]','ACTIVE',?,?,?)").bind(agentId,keyId,raw.toString('base64url'),'c'.repeat(64),now,now,JSON.stringify({host:{os:'linux',arch:'x86_64'}})),
      env.HARNESS_DB.prepare("INSERT INTO probe_targets VALUES (?,'development','deployment-probe',0)").bind(agentId)
    ]);
    await env.IDENTITY_DB.batch([
      env.IDENTITY_DB.prepare("INSERT INTO operator_principals VALUES ('operator',?,'operator','ACTIVE')").bind(env.ACCESS_TEAM_DOMAIN),
      env.IDENTITY_DB.prepare("INSERT INTO operator_roles VALUES ('deployer','Fixture deployer')"),
      ...['deployment.read','deployment.create'].map(permission=>env.IDENTITY_DB.prepare("INSERT INTO operator_role_permissions VALUES ('deployer',?)").bind(permission)),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_bindings VALUES ('operator','deployer','development')")
    ]);
    const input={schema:1,candidate_id:metadata.candidate_id,agent_id:agentId};
    const deploy=(id=randomUUID(),body=input,origin='https://operator.example.com')=>api.fetch(new Request('https://operator.example.com/api/v1/deployments',{method:'POST',headers:{'cf-access-jwt-assertion':operator,'content-type':'application/json',origin,'x-velora-action':'1','idempotency-key':id},body:JSON.stringify(body)}),env);
    const signed=(path,body={schema:1},nonce=randomUUID(),privateKey=keys.privateKey)=>{
      const text=JSON.stringify(body),checksum=createHash('sha256').update(text).digest('hex'),timestamp=String(Math.floor(Date.now()/1000));
      const signature=sign(null,Buffer.from(canonicalAgentRequest(path,timestamp,nonce,checksum)),privateKey).toString('base64url');
      return api.fetch(new Request('https://agents.example.com'+path,{method:'POST',headers:{'content-type':'application/json','velora-agent-id':agentId,'velora-key-id':keyId,'velora-timestamp':timestamp,'velora-nonce':nonce,'velora-body-sha256':checksum,'velora-signature':signature},body:text}),env);
    };
    const poll=()=>signed('/api/v1/agents/probe/poll');
    assert.equal((await deploy()).status,409); // Disabled automation and unapproved target.
    await env.HARNESS_DB.prepare("UPDATE automation_flags SET deployment_enabled=1 WHERE environment_id='development'").run();
    assert.equal((await deploy()).status,409);assert.equal((await poll()).status,403);
    await env.HARNESS_DB.prepare('UPDATE probe_targets SET enabled=1').run();
    assert.equal((await deploy(randomUUID(),input,'https://attacker.example.com')).status,403);
    assert.equal((await deploy(randomUUID(),{...input,command:'shell'})).status,400);
    assert.equal((await signed('/api/v1/agents/probe/poll',{schema:1},randomUUID(),generateKeyPairSync('ed25519').privateKey)).status,403);
    const id=randomUUID(),responses=await Promise.all(Array.from({length:5},()=>deploy(id)));
    assert.ok(responses.every(response=>response.status===202));
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS count FROM probe_deployments').first()).count,1);
    assert.equal((await deploy()).status,409); // Service lock.
    assert.equal((await deploy(id,{...input,candidate_id:'rc_'+'d'.repeat(64)})).status,409);
    let job;
    for(let attempt=0;attempt<80;attempt++){
      job=(await (await poll()).json()).job;if(job)break;await setTimeout(250);
    }
    assert.ok(job,'Workflow prepared a signed job');
    const decoded=await verifyProbeJob(job,publicKey,agentId);
    assert.equal(decoded.deployment_id,id);assert.equal(decoded.image,metadata.artifact_uri);assert.equal(decoded.expected_commit,metadata.git_sha);
    assert.deepEqual((await (await poll()).json()).job,job); // Retry returns the same signature/payload.
    const nonce=randomUUID();assert.equal((await signed('/api/v1/agents/probe/poll',{schema:1},nonce)).status,200);assert.equal((await signed('/api/v1/agents/probe/poll',{schema:1},nonce)).status,403);
    const receipt={schema:1,job_id:decoded.id,payload_sha256:createHash('sha256').update(job.payload).digest('hex'),status:'SUCCEEDED',events:['PREFLIGHT','PULLING_OR_STAGING','HEALTH_CHECK','SUCCEEDED']};
    const report=value=>signed('/api/v1/agents/probe/result',{schema:1,receipt:value});
    assert.equal((await report({...receipt,payload_sha256:'e'.repeat(64)})).status,409);
    assert.equal((await report({...receipt,events:['arbitrary-output']})).status,400);
    const results=await Promise.all(Array.from({length:5},()=>report(receipt)));assert.ok(results.every(response=>response.status===200));
    assert.equal((await report({...receipt,status:'FAILED',events:['FAILED']})).status,409);
    assert.equal((await env.HARNESS_DB.prepare("SELECT COUNT(*) AS count FROM audit_events WHERE action='deployment.result'").first()).count,1);
    const instance=await env.PROBE_DEPLOYMENT.get('probe-'+id);let state;
    for(let attempt=0;attempt<180;attempt++){state=await instance.status();if(state.status==='complete')break;await setTimeout(250);}
    assert.equal(state.status,'complete',JSON.stringify(state));assert.equal(state.output.status,'SUCCEEDED');
    const detail=await api.fetch(new Request('https://operator.example.com/api/v1/deployments/'+id,{headers:{'cf-access-jwt-assertion':operator}}),env);
    assert.equal(detail.status,200);assert.equal((await detail.json()).deployment.status,'SUCCEEDED');
    assert.equal((await api.fetch(new Request('https://operator.example.com/api/v1/deployments?environment=production',{headers:{'cf-access-jwt-assertion':operator}}),env)).status,403);
    // A delivered job with a lost result must not free its lock. A late authenticated
    // terminal result can reconcile it, without reissuing or restoring data.
    const blockedId=randomUUID(),blockedJob=randomUUID();
    await env.HARNESS_DB.prepare("INSERT INTO probe_deployments(id,candidate_id,agent_id,environment_id,service_id,actor_id,job_id,status,created_at,updated_at) VALUES (?,?,?,'development','deployment-probe','operator',?,'RESERVED',?,?)").bind(blockedId,metadata.candidate_id,agentId,blockedJob,now,now).run();
    await prepareProbeJob(env,blockedId);await poll();await blockProbeDeployment(env,blockedId);
    assert.equal((await deploy()).status,409);assert.equal((await (await poll()).json()).job,null);
    const row=await env.HARNESS_DB.prepare('SELECT payload_sha256 FROM probe_deployments WHERE id=?').bind(blockedId).first();
    const rollback={schema:1,job_id:blockedJob,payload_sha256:row.payload_sha256,status:'ROLLED_BACK',events:['PREFLIGHT','ROLLING_BACK','ROLLED_BACK']};
    await env.HARNESS_DB.prepare("UPDATE agents SET status='REVOKED'").run();assert.equal((await report(rollback)).status,403);
    await env.HARNESS_DB.prepare("UPDATE agents SET status='ACTIVE'").run();assert.equal((await report(rollback)).status,200);
    assert.equal((await env.HARNESS_DB.prepare('SELECT status FROM probe_deployments WHERE id=?').bind(blockedId).first()).status,'ROLLED_BACK');
    // Migration reapplication preserves existing heartbeat-only scope.
    await worker.applyD1Migrations('HARNESS_DB');assert.equal((await env.HARNESS_DB.prepare('SELECT capabilities_json FROM agents').first()).capabilities_json,'["REPORT_HEARTBEAT"]');
    for(let index=0;index<8;index++)await env.HARNESS_DB.prepare("INSERT INTO probe_deployments(id,candidate_id,agent_id,environment_id,service_id,actor_id,job_id,status,created_at,updated_at) VALUES (?,?,?,'development','deployment-probe','operator',?,'FAILED',?,?)").bind(randomUUID(),metadata.candidate_id,agentId,randomUUID(),now,now).run();
    assert.equal((await deploy()).status,409); // Ten new deployments per rolling day.
    assert.equal((await deploy(id)).status,202); // A retry does not consume budget.
    await env.HARNESS_DB.prepare("UPDATE probe_deployments SET created_at=? WHERE status='FAILED'").bind(now-86401).run();
    await env.HARNESS_DB.prepare('UPDATE agents SET heartbeat_at=?').bind(now-121).run();assert.equal((await deploy()).status,409);
    assert.equal((await api.fetch(new Request('https://operator.example.com/api/v1/deployments',{headers:{'cf-access-jwt-assertion':operator}}),{...env,PROBE_DEPLOYMENTS_ENABLED:'0'})).status,403);
  }finally{await server.close();}
});
