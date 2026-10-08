import {verifyAccessPrincipal} from './access.mjs';
import {requireOperatorPermission} from './authorization.mjs';
import {readAgentRequest,verifyAgentSignature,decodeBase64Url,agentHash} from './agent-protocol.mjs';
import {signProbeJob,verifyProbeJob} from './probe-job.mjs';
import {artifactFields,type ArtifactInput} from '../../deployment/contracts/artifact-fields.mjs';
import validateArtifact from './artifact-validator.generated.mjs';
import {ApiError,json,readJson,uuid,digest} from './http.ts';

export type ProbeEnv=Env & {PROBE_SIGNING_PRIVATE_JWK?:string};
type Deployment={id:string,candidate_id:string,agent_id:string,actor_id:string,job_id:string,status:string,job_json:string|null,payload_sha256:string|null,expires_at:number|null,delivered_at:number|null,receipt_json:string|null};
function exact(value:unknown,keys:string[]):value is Record<string,unknown>{return value!==null && typeof value==='object' && !Array.isArray(value) && Object.keys(value).sort().join() === [...keys].sort().join();}
const agentPattern=/^agt_[a-f0-9]{64}$/;
const candidatePattern=/^rc_[a-f0-9]{64}$/;
const statuses=['SUCCEEDED','FAILED','ROLLED_BACK','BLOCKED','ROLLBACK_FAILED'];
const events=['PREFLIGHT','PULLING_OR_STAGING','HEALTH_CHECK','ROLLING_BACK',...statuses];
export async function probeSigningKey(env:ProbeEnv){
  try{
    const publicBytes=decodeBase64Url(env.PROBE_SIGNING_PUBLIC_KEY,32);
    const jwk:JsonWebKey=JSON.parse(env.PROBE_SIGNING_PRIVATE_JWK??'');
    if(jwk.kty!=='OKP' || jwk.crv!=='Ed25519' || jwk.x!==env.PROBE_SIGNING_PUBLIC_KEY)throw Error('Key mismatch');
    const key=await crypto.subtle.importKey('jwk',jwk,'Ed25519',false,['sign']);
    return {key,keyId:await agentHash(publicBytes)};
  }catch{throw new ApiError(503,'PROBE_SIGNING_UNAVAILABLE');}
}
function enabled(env:ProbeEnv){if(env.ENVIRONMENT!=='development' || env.PROBE_DEPLOYMENTS_ENABLED!=='1')throw new ApiError(403,'PROBE_DEPLOYMENTS_DISABLED');}
export async function prepareProbeJob(env:ProbeEnv,id:string){
  enabled(env);
  const row=await env.HARNESS_DB.prepare('SELECT * FROM probe_deployments WHERE id=?').bind(id).first<Deployment>();
  if(!row)throw Error('Unregistered probe deployment');
  if(row.job_json || row.status!=='RESERVED')return;
  const allowed=await env.HARNESS_DB.prepare(`SELECT t.agent_id FROM probe_targets t JOIN agents a ON a.id=t.agent_id JOIN services s ON s.id=t.service_id
    WHERE t.agent_id=? AND t.enabled=1 AND a.status='ACTIVE' AND s.enabled=1
    AND (SELECT deployment_enabled FROM automation_flags WHERE environment_id='development')=1`).bind(row.agent_id).first();
  if(!allowed)throw Error('Probe target unavailable');
  const candidate=await env.HARNESS_DB.prepare('SELECT metadata_json FROM deployment_candidates WHERE id=? AND service_id=\'deployment-probe\'').bind(row.candidate_id).first<{metadata_json:string}>();
  if(!candidate)throw Error('Probe candidate unavailable');
  const metadata=artifactFields(JSON.parse(candidate.metadata_json) as ArtifactInput,validateArtifact).normalized;
  const now=Math.floor(Date.now()/1000),signer=await probeSigningKey(env);
  const job={schema:1,id:row.job_id,deployment_id:id,agent_id:row.agent_id,environment:'development',action:'DEPLOY_SERVICE',service_id:'deployment-probe',image:metadata.artifact_uri,expected_commit:metadata.git_sha,issued_at:now,expires_at:now+300};
  const envelope=await signProbeJob(job,signer.key,signer.keyId,now);
  await verifyProbeJob(envelope,env.PROBE_SIGNING_PUBLIC_KEY,row.agent_id,now);
  // The native executor hashes the exact base64url payload to bind its ledger.
  const payloadHash=await digest(envelope.payload);
  await env.HARNESS_DB.batch([
    env.HARNESS_DB.prepare(`UPDATE probe_deployments SET job_json=?,payload_sha256=?,expires_at=?,status='JOB_READY',updated_at=?
      WHERE id=? AND status='RESERVED' AND job_json IS NULL
      AND EXISTS(SELECT 1 FROM agents a JOIN probe_targets t ON t.agent_id=a.id WHERE a.id=probe_deployments.agent_id AND a.status='ACTIVE' AND t.enabled=1)
      AND (SELECT deployment_enabled FROM automation_flags WHERE environment_id='development')=1`).bind(JSON.stringify(envelope),payloadHash,now+300,now,id),
    env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO audit_events(event_key,actor_id,action,environment_id,target_id,request_id)
      SELECT ?,'workflow','deployment.job.ready','development',id,id FROM probe_deployments WHERE id=? AND status='JOB_READY'`).bind('probe.ready:'+id,id)
  ]);
}
export async function blockProbeDeployment(env:ProbeEnv,id:string){
  const now=Math.floor(Date.now()/1000);
  await env.HARNESS_DB.batch([
    env.HARNESS_DB.prepare("UPDATE probe_deployments SET status='BLOCKED',updated_at=? WHERE id=? AND status IN ('RESERVED','JOB_READY')").bind(now,id),
    env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO audit_events(event_key,actor_id,action,environment_id,target_id,request_id)
      SELECT ?,'workflow','deployment.blocked','development',id,id FROM probe_deployments WHERE id=? AND status='BLOCKED'`).bind('probe.blocked:'+id,id)
  ]);
}
export function createProbeDeploymentApi(operator:typeof verifyAccessPrincipal=verifyAccessPrincipal){return {
  async fetch(request:Request,env:ProbeEnv,requestId:string):Promise<Response>{
    enabled(env);
    const url=new URL(request.url),now=Math.floor(Date.now()/1000);
    if(url.pathname.startsWith('/api/v1/agents/probe/')){
      if(env.AGENTS_ENABLED!=='1' || url.hostname!==env.CI_API_HOST)throw new ApiError(403,'INGRESS_DENIED');
      let envelope;try{envelope=await readAgentRequest(request,now);}catch{throw new ApiError(403,'AGENT_DENIED');}
      const agent=await env.HARNESS_DB.prepare(`SELECT a.public_key FROM agents a JOIN probe_targets t ON t.agent_id=a.id
        WHERE a.id=? AND a.status='ACTIVE' AND t.enabled=1 AND t.environment_id='development'`).bind(envelope.agent_id).first<{public_key:string}>();
      if(!agent)throw new ApiError(403,'PROBE_TARGET_DENIED');
      try{await verifyAgentSignature(envelope,agent.public_key);}catch{throw new ApiError(403,'AGENT_DENIED');}
      const nonce=await env.HARNESS_DB.batch([
        env.HARNESS_DB.prepare('DELETE FROM agent_nonces WHERE agent_id=? AND expires_at<?').bind(envelope.agent_id,now),
        env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO agent_nonces(agent_id,nonce,request_id,expires_at)
          SELECT id,?,?,? FROM agents WHERE id=? AND status='ACTIVE' AND (SELECT COUNT(*) FROM agent_nonces WHERE agent_id=?)<2000`).bind(envelope.nonce,requestId,now+300,envelope.agent_id,envelope.agent_id)
      ]);
      if(nonce[1].meta.changes!==1)throw new ApiError(403,'AGENT_REPLAY_OR_REVOKED');
      if(url.pathname==='/api/v1/agents/probe/poll'){
        if(!exact(envelope.body,['schema']) || envelope.body.schema!==1)throw new ApiError(400,'INVALID_PROBE_POLL');
        const flag=await env.HARNESS_DB.prepare("SELECT deployment_enabled FROM automation_flags WHERE environment_id='development'").first<{deployment_enabled:number}>();
        if(flag?.deployment_enabled!==1)return json(200,{job:null},requestId);
        await env.HARNESS_DB.prepare(`UPDATE probe_deployments SET delivered_at=?,updated_at=?
          WHERE agent_id=? AND status='JOB_READY' AND expires_at>? AND EXISTS(SELECT 1 FROM agents WHERE id=? AND status='ACTIVE')
          AND EXISTS(SELECT 1 FROM probe_targets WHERE agent_id=? AND enabled=1) AND delivered_at IS NULL
          AND (SELECT deployment_enabled FROM automation_flags WHERE environment_id='development')=1`).bind(now,now,envelope.agent_id,now,envelope.agent_id,envelope.agent_id).run();
        const row=await env.HARNESS_DB.prepare(`SELECT d.job_json FROM probe_deployments d JOIN agents a ON a.id=d.agent_id JOIN probe_targets t ON t.agent_id=a.id
          WHERE d.agent_id=? AND d.status='JOB_READY' AND d.expires_at>? AND d.delivered_at IS NOT NULL AND a.status='ACTIVE' AND t.enabled=1
          AND (SELECT deployment_enabled FROM automation_flags WHERE environment_id='development')=1`).bind(envelope.agent_id,now).first<{job_json:string}>();
        return json(200,{job:row?JSON.parse(row.job_json):null},requestId);
      }
      if(url.pathname!=='/api/v1/agents/probe/result')throw new ApiError(404,'NOT_FOUND');
      const body=envelope.body;
      if(!exact(body,['schema','receipt']) || body.schema!==1 || !exact(body.receipt,['schema','job_id','payload_sha256','status','events']))throw new ApiError(400,'INVALID_PROBE_RECEIPT');
      const receipt=body.receipt;
      if(receipt.schema!==1 || typeof receipt.job_id!=='string' || !uuid.test(receipt.job_id) || typeof receipt.payload_sha256!=='string' || !/^[a-f0-9]{64}$/.test(receipt.payload_sha256)
        || typeof receipt.status!=='string' || !statuses.includes(receipt.status) || !Array.isArray(receipt.events) || receipt.events.length<1 || receipt.events.length>16
        || receipt.events.some(event=>typeof event!=='string' || !events.includes(event)) || receipt.events.at(-1)!==receipt.status)throw new ApiError(400,'INVALID_PROBE_RECEIPT');
      const row=await env.HARNESS_DB.prepare('SELECT * FROM probe_deployments WHERE job_id=? AND agent_id=? AND delivered_at IS NOT NULL').bind(receipt.job_id,envelope.agent_id).first<Deployment>();
      if(!row || row.payload_sha256!==receipt.payload_sha256)throw new ApiError(409,'PROBE_RECEIPT_CONFLICT');
      const canonical=JSON.stringify({schema:1,job_id:receipt.job_id,payload_sha256:receipt.payload_sha256,status:receipt.status,events:receipt.events});
      await env.HARNESS_DB.batch([
        env.HARNESS_DB.prepare(`UPDATE probe_deployments SET status=?,receipt_json=?,updated_at=? WHERE id=? AND receipt_json IS NULL
          AND status IN ('JOB_READY','BLOCKED') AND EXISTS(SELECT 1 FROM agents WHERE id=? AND status='ACTIVE')
          AND EXISTS(SELECT 1 FROM probe_targets WHERE agent_id=? AND enabled=1)`).bind(receipt.status,canonical,now,row.id,envelope.agent_id,envelope.agent_id),
        env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO audit_events(event_key,actor_id,action,environment_id,target_id,request_id)
          SELECT ?,agent_id,'deployment.result','development',id,? FROM probe_deployments WHERE id=? AND receipt_json=?`).bind('probe.result:'+row.id,requestId,row.id,canonical)
      ]);
      const stored=await env.HARNESS_DB.prepare('SELECT receipt_json FROM probe_deployments WHERE id=?').bind(row.id).first<{receipt_json:string|null}>();
      if(stored?.receipt_json!==canonical)throw new ApiError(409,'PROBE_RECEIPT_CONFLICT');
      return json(200,{accepted:true,deployment_id:row.id,job_id:receipt.job_id,status:receipt.status},requestId);
    }
    if(url.hostname!==env.OPERATOR_API_HOST)throw new ApiError(403,'INGRESS_DENIED');
    const environment=url.searchParams.get('environment')??'development';
    if(environment!=='development')throw new ApiError(403,'ENVIRONMENT_NOT_ENABLED');
    let principal;try{principal=await operator(request,env);}catch{throw new ApiError(403,'ACCESS_DENIED');}
    let actor;try{actor=await requireOperatorPermission(env.IDENTITY_DB,{issuer:env.ACCESS_TEAM_DOMAIN,subject:principal.subject},request.method==='GET'?'deployment.read':'deployment.create',environment);}catch{throw new ApiError(403,'PERMISSION_DENIED');}
    const detail=url.pathname.match(/^\/api\/v1\/deployments\/([a-f0-9-]{36})$/);
    if(request.method==='GET'){
      if(detail){
        if(!uuid.test(detail[1]))throw new ApiError(400,'INVALID_DEPLOYMENT_ID');
        const row=await env.HARNESS_DB.prepare('SELECT id,candidate_id,agent_id,job_id,status,created_at,updated_at,delivered_at,receipt_json FROM probe_deployments WHERE id=?').bind(detail[1]).first();
        if(!row)throw new ApiError(404,'NOT_FOUND');return json(200,{deployment:row},requestId);
      }
      if(url.pathname!=='/api/v1/deployments')throw new ApiError(404,'NOT_FOUND');
      const limit=url.searchParams.get('limit')??'25',cursor=url.searchParams.get('cursor')??'';
      if(!/^[1-9][0-9]?$/.test(limit) || Number(limit)>50 || (cursor && !uuid.test(cursor)))throw new ApiError(400,'INVALID_PAGE');
      const rows=await env.HARNESS_DB.prepare('SELECT id,candidate_id,agent_id,job_id,status,created_at,updated_at FROM probe_deployments WHERE id>? ORDER BY id LIMIT ?').bind(cursor,Number(limit)).all<{id:string}>();
      return json(200,{items:rows.results,next_cursor:rows.results.length===Number(limit)?rows.results.at(-1)?.id:null},requestId);
    }
    if(request.method!=='POST' || url.pathname!=='/api/v1/deployments')throw new ApiError(405,'METHOD_NOT_ALLOWED');
    if(url.search || request.headers.get('origin')!==url.origin || request.headers.get('x-velora-action')!=='1')throw new ApiError(403,'ORIGIN_DENIED');
    const id=request.headers.get('idempotency-key')??'';if(!uuid.test(id))throw new ApiError(400,'INVALID_IDEMPOTENCY_KEY');
    const body=await readJson(request,1024);
    if(!exact(body,['schema','candidate_id','agent_id']) || body.schema!==1 || typeof body.candidate_id!=='string' || !candidatePattern.test(body.candidate_id)
      || typeof body.agent_id!=='string' || !agentPattern.test(body.agent_id))throw new ApiError(400,'INVALID_PROBE_DEPLOYMENT');
    const prior=await env.HARNESS_DB.prepare('SELECT * FROM probe_deployments WHERE id=?').bind(id).first<Deployment>();
    if(prior && (prior.candidate_id!==body.candidate_id || prior.agent_id!==body.agent_id || prior.actor_id!==actor.actor_id))throw new ApiError(409,'IDEMPOTENCY_CONFLICT');
    if(!prior){
      const candidate=await env.HARNESS_DB.prepare(`SELECT c.metadata_json FROM deployment_candidates c JOIN services s ON s.id=c.service_id JOIN repositories r ON r.id=c.repository_id
        WHERE c.id=? AND c.service_id='deployment-probe' AND s.repository_id=c.repository_id AND s.enabled=1 AND s.artifact_type='oci' AND r.enabled=1`).bind(body.candidate_id).first<{metadata_json:string}>();
      let metadata:ArtifactInput;
      try{metadata=artifactFields(JSON.parse(candidate?.metadata_json??'') as ArtifactInput,validateArtifact).normalized;
        if(metadata.artifact_type!=='oci' || !/^ghcr\.io\/veloramcdev\/deployment-probe@sha256:[a-f0-9]{64}$/.test(metadata.artifact_uri))throw Error('Invalid probe');
      }catch{throw new ApiError(403,'PROBE_CANDIDATE_DENIED');}
      await probeSigningKey(env);
      await env.HARNESS_DB.batch([
        env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO probe_deployments(id,candidate_id,agent_id,environment_id,service_id,actor_id,job_id,status,created_at,updated_at)
          SELECT ?,?,t.agent_id,'development','deployment-probe',?,?,'RESERVED',?,? FROM probe_targets t JOIN agents a ON a.id=t.agent_id
          WHERE t.agent_id=? AND t.enabled=1 AND a.status='ACTIVE' AND a.heartbeat_at>?
          AND json_extract(a.heartbeat_json,'$.host.os')='linux' AND json_extract(a.heartbeat_json,'$.host.arch')='x86_64' AND (SELECT deployment_enabled FROM automation_flags WHERE environment_id='development')=1
          AND (SELECT COUNT(*) FROM probe_deployments)<1000
          AND (SELECT COUNT(*) FROM probe_deployments WHERE created_at>?)<10`).bind(id,body.candidate_id,actor.actor_id,crypto.randomUUID(),now,now,body.agent_id,now-120,now-86400),
        env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO audit_events(event_key,actor_id,action,environment_id,target_id,request_id)
          SELECT ?,actor_id,'deployment.create','development',id,? FROM probe_deployments WHERE id=?`).bind('probe.create:'+id,requestId,id)
      ]);
    }
    const stored=await env.HARNESS_DB.prepare('SELECT * FROM probe_deployments WHERE id=?').bind(id).first<Deployment>();
    if(!stored)throw new ApiError(409,'PROBE_DEPLOYMENT_LOCKED_OR_DISABLED');
    if(stored.candidate_id!==body.candidate_id || stored.agent_id!==body.agent_id || stored.actor_id!==actor.actor_id)throw new ApiError(409,'IDEMPOTENCY_CONFLICT');
    if(stored.status==='RESERVED' || stored.status==='JOB_READY'){
      try{await (await env.PROBE_DEPLOYMENT.get('probe-'+id)).status();}
      catch{try{await env.PROBE_DEPLOYMENT.create({id:'probe-'+id,params:{id}});}catch{await (await env.PROBE_DEPLOYMENT.get('probe-'+id)).status();}}
    }
    return json(202,{deployment_id:id,workflow_id:'probe-'+id,status:stored.status},requestId);
  }
};}
