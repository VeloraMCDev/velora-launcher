import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createTestHarness } from 'wrangler';
import { generateKeyPair, SignJWT } from 'jose';
import { randomUUID, createHmac } from 'node:crypto';
import { createControlApi } from '../control-plane/src/api.ts';
import { verifyGitHubJob } from '../control-plane/src/github-oidc.mjs';
import { verifyAccessPrincipal } from '../control-plane/src/access.mjs';
import { artifactMetadata } from '../scripts/artifact-metadata.mjs';

test('registration API validates signed job policy, persists exact candidates atomically, enforces scoped reads and pause', {timeout:60000},async()=>{
  const server=createTestHarness({workers:[{configPath:'control-plane/wrangler.jsonc'}]});
  try{
    await server.listen();const worker=server.getWorker();
    for(const binding of ['HARNESS_DB','IDENTITY_DB'])await worker.applyD1Migrations(binding);
    const bindings=await worker.getEnv();
    const env={...bindings,CONTROL_API_ENABLED:'1',ENVIRONMENT:'development',CI_API_HOST:'ci.example.com',OPERATOR_API_HOST:'operator.example.com',CI_OIDC_AUDIENCE:'velora-control-plane',ACCESS_TEAM_DOMAIN:'https://fixture.cloudflareaccess.com',ACCESS_AUD:'fixture-app'};
    const ciKeys=await generateKeyPair('RS256'),operatorKeys=await generateKeyPair('RS256');
    const api=createControlApi({ci:(request,audience)=>verifyGitHubJob(request,audience,ciKeys.publicKey),operator:(request,config)=>verifyAccessPrincipal(request,config,operatorKeys.publicKey)});
    const now=Math.floor(Date.now()/1000),sha='a'.repeat(40);
    const job=await new SignJWT({repository:'Example/infra',repository_id:'123',repository_owner:'Example',repository_owner_id:'456',sha,ref:'refs/heads/main',run_id:'789',run_attempt:'1',workflow_ref:'Example/infra/.github/workflows/release.yml@refs/heads/main',workflow_sha:sha,event_name:'push',runner_environment:'github-hosted'})
      .setProtectedHeader({alg:'RS256',typ:'JWT'}).setIssuer('https://token.actions.githubusercontent.com').setAudience(env.CI_OIDC_AUDIENCE).setSubject('repo:Example@456/infra@123:ref:refs/heads/main').setJti('synthetic-ci-job').setIssuedAt(now).setNotBefore(now-1).setExpirationTime(now+300).sign(ciKeys.privateKey);
    const operator=await new SignJWT({email:'operator@example.invalid',type:'app'}).setProtectedHeader({alg:'RS256'}).setIssuer(env.ACCESS_TEAM_DOMAIN).setAudience(env.ACCESS_AUD).setSubject('synthetic-operator').setIssuedAt(now).setExpirationTime(now+300).sign(operatorKeys.privateKey);
    const metadata={schema:1,repository:'Example/infra',service_id:'probe',git_sha:sha,git_ref:'refs/heads/main',build_run_id:'789',artifact_type:'oci',artifact_uri:'ghcr.io/example/probe@sha256:'+'b'.repeat(64),sha256:'b'.repeat(64),oci_digest:'sha256:'+'b'.repeat(64),version:'0.1.0',build_status:'PASSED',test_status:'PASSED'};
    const register=(body=metadata,token=job,host=env.CI_API_HOST)=>api.fetch(new Request('https://'+host+'/api/v1/ci/artifacts/register',{method:'POST',headers:{authorization:'Bearer '+token,'content-type':'application/json'},body:JSON.stringify(body)}),env);
    const read=path=>api.fetch(new Request('https://'+env.OPERATOR_API_HOST+path,{headers:{'cf-access-jwt-assertion':operator}}),env);
    assert.equal((await register()).status,409); // Initially paused.
    await env.HARNESS_DB.prepare("UPDATE automation_flags SET registration_enabled=1 WHERE environment_id='development'").run();
    assert.equal((await register()).status,403); // No repository trust seeded.
    await env.HARNESS_DB.batch([
      env.HARNESS_DB.prepare("INSERT INTO repositories VALUES ('repo','Example/infra','123','456',1)"),
      env.HARNESS_DB.prepare("INSERT INTO services VALUES ('probe','repo','oci','ghcr.io/example/probe@sha256:',1)"),
      env.HARNESS_DB.prepare("INSERT INTO ci_identity_policies VALUES ('policy','repo','refs/heads/main','.github/workflows/release.yml',?,'immutable',1)").bind(sha)
    ]);
    assert.equal((await register(metadata,'invalid')).status,403);
    assert.equal((await register(metadata,job,env.OPERATOR_API_HOST)).status,403);
    assert.equal((await register({...metadata,build_run_id:'790'})).status,403);
    assert.equal((await register({...metadata,artifact_uri:'ghcr.io/attacker/probe@sha256:'+'b'.repeat(64)})).status,403);
    assert.equal((await register({...metadata,sha256:'c'.repeat(64)})).status,400);
    assert.equal((await register({...metadata,unexpected:true})).status,400);
    assert.equal((await register({...metadata,version:'x'.repeat(17000)})).status,413);
    const attempts=await Promise.all(Array.from({length:6},()=>register()));
    assert.equal(attempts.filter(r=>r.status===201).length,1);
    assert.equal(attempts.filter(r=>r.status===200).length,5);
    const candidate=(await attempts[0].json()).candidate;
    assert.deepEqual(candidate,artifactMetadata(metadata));
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS total FROM deployment_candidates').first()).total,1);
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS total FROM artifacts').first()).total,1);
    assert.equal((await env.HARNESS_DB.prepare("SELECT COUNT(*) AS total FROM audit_events WHERE action='candidate.register'").first()).total,1);
    assert.equal((await read('/api/v1/candidates')).status,403); // Access is not RBAC.
    await env.IDENTITY_DB.batch([
      env.IDENTITY_DB.prepare("INSERT INTO operator_principals VALUES ('operator',?,'synthetic-operator','ACTIVE')").bind(env.ACCESS_TEAM_DOMAIN),
      env.IDENTITY_DB.prepare("INSERT INTO operator_roles VALUES ('dev-reader','Fixture role')"),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_permissions VALUES ('dev-reader','candidate.read')"),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_permissions VALUES ('dev-reader','automation.registration.manage')"),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_bindings VALUES ('operator','dev-reader','development')")
    ]);
    const detail=await read('/api/v1/candidates/'+candidate.candidate_id);
    assert.equal(detail.status,200);assert.deepEqual((await detail.json()).candidate,candidate);
    assert.equal((await read('/api/v1/candidates?environment=production')).status,403);
    assert.equal((await read('/api/v1/candidates?limit=51')).status,400);
    assert.equal((await read('/api/v1/candidates?cursor=invalid')).status,400);
    assert.equal((await read('/api/v1/candidates/'+('rc_'+'c'.repeat(64)))).status,404);
    assert.equal((await read('/api/v1/deployments')).status,403); // Probe execution API defaults disabled.
    const pauseKey=randomUUID();
    const action=(path,key=pauseKey,origin='https://'+env.OPERATOR_API_HOST)=>api.fetch(new Request('https://'+env.OPERATOR_API_HOST+'/api/v1/automation/'+path,{method:'POST',headers:{'cf-access-jwt-assertion':operator,'X-Velora-Action':'1',Origin:origin,'Idempotency-Key':key}}),env);
    assert.equal((await action('pause',pauseKey,'https://attacker.example.com')).status,403);
    assert.equal((await action('pause')).status,200);
    assert.equal((await action('pause')).status,200);
    assert.equal((await action('resume')).status,409);
    assert.equal((await env.HARNESS_DB.prepare("SELECT COUNT(*) AS total FROM audit_events WHERE action='registration.pause'").first()).total,1);
    assert.equal((await register()).status,409);
    assert.equal((await action('resume',randomUUID())).status,200);
    assert.equal((await register()).status,200);
    assert.equal((await env.HARNESS_DB.prepare("SELECT deployment_enabled FROM automation_flags WHERE environment_id='development'").first()).deployment_enabled,0);
    const raceKey=randomUUID();
    const race=await Promise.all([action('pause',raceKey),action('resume',raceKey)]);
    assert.deepEqual(race.map(response=>response.status).sort(),[200,409]);
    const receipt=await env.HARNESS_DB.prepare('SELECT action FROM audit_events WHERE event_key=?').bind('automation:'+raceKey).first();
    const flags=await env.HARNESS_DB.prepare("SELECT registration_enabled,deployment_enabled FROM automation_flags WHERE environment_id='development'").first();
    assert.equal(flags.registration_enabled,receipt.action==='registration.resume'?1:0);
    assert.equal(flags.deployment_enabled,0);
    const secret='synthetic-webhook-fixture';
    const deliveryId=randomUUID();
    const webhook=(payload={repository:{id:123},action:'completed'},delivery=deliveryId,signatureSecret=secret)=>{
      const body=JSON.stringify(payload);
      return api.fetch(new Request('https://'+env.CI_API_HOST+'/api/v1/github/webhooks',{method:'POST',headers:{'content-type':'application/json','x-github-event':'workflow_run','x-github-delivery':delivery,'x-hub-signature-256':'sha256='+createHmac('sha256',signatureSecret).update(body).digest('hex')},body}),env);
    };
    assert.equal((await webhook()).status,403); // Disabled until configured explicitly.
    env.WEBHOOKS_ENABLED='1';env.GITHUB_WEBHOOK_SECRET=secret;
    assert.equal((await webhook(undefined,randomUUID(),'wrong-secret')).status,403);
    assert.equal((await webhook({repository:{id:999}},randomUUID())).status,403);
    const deliveries=await Promise.all(Array.from({length:6},()=>webhook()));
    assert.ok(deliveries.every(response=>response.status===202));
    const responses=await Promise.all(deliveries.map(response=>response.json()));
    assert.equal(responses.filter(response=>!response.duplicate).length,1);
    assert.ok(responses.every(response=>response.deployment_performed===false));
    assert.equal((await webhook({repository:{id:123},action:'changed'})).status,409);
    assert.equal((await env.HARNESS_DB.prepare('SELECT COUNT(*) AS total FROM deployment_candidates').first()).total,1);
    await env.IDENTITY_DB.prepare("UPDATE operator_principals SET status='DISABLED' WHERE id='operator'").run();
    assert.equal((await read('/api/v1/candidates')).status,403);
    assert.equal((await api.fetch(new Request('https://ci.example.com/api/v1/ci/artifacts/register'),{...env,CONTROL_API_ENABLED:'0'})).status,403);
    assert.equal((await server.fetch('/api/v1/candidates')).status,403); // Actual exported Worker defaults deny.
  }finally{await server.close();}
});
