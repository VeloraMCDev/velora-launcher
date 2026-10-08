import {verifyAccessPrincipal} from './access.mjs';
import {requireOperatorPermission,auditStatement} from './authorization.mjs';
import {readAgentRequest,verifyAgentSignature,decodeBase64Url,encodeBase64Url} from './agent-protocol.mjs';
import {ApiError,json,digest,readJson,uuid} from './http.ts';

const capabilities='["REPORT_HEARTBEAT"]';
const agentPattern=/^agt_[a-f0-9]{64}$/;
type Agent={id:string,key_id:string,public_key:string,status:string,environment_id:string,enrollment_token_hash:string,capabilities_json:string};
function exact(value:unknown,keys:string[]):value is Record<string,unknown>{return value!==null && typeof value==='object' && !Array.isArray(value) && Object.keys(value).sort().join(',')===keys.sort().join(',');}
function version(value:unknown){return typeof value==='string' && /^[0-9]{1,4}\.[0-9]{1,4}\.[0-9]{1,4}$/.test(value);}
function source(value:unknown){return typeof value==='string' && /^[a-f0-9]{40}$/.test(value);}
function actionKey(request:Request,url:URL){
  if(request.method!=='POST')throw new ApiError(405,'METHOD_NOT_ALLOWED');
  if(request.headers.get('origin')!==url.origin || request.headers.get('x-velora-action')!=='1')throw new ApiError(403,'ORIGIN_DENIED');
  const key=request.headers.get('idempotency-key')??'';if(!uuid.test(key))throw new ApiError(400,'INVALID_IDEMPOTENCY_KEY');return key;
}
export function createAgentApi(operator:typeof verifyAccessPrincipal=verifyAccessPrincipal){return {
  async fetch(request:Request,env:Env,id=crypto.randomUUID()):Promise<Response>{
    const url=new URL(request.url),path=url.pathname,now=Math.floor(Date.now()/1000);
    if(env.AGENTS_ENABLED!=='1' || env.ENVIRONMENT!=='development')return json(403,{error:'AGENT_API_DISABLED'},id);
    try{
      if(path==='/api/v1/agents/enroll' || path==='/api/v1/agents/heartbeat'){
        if(url.hostname!==env.CI_API_HOST)throw new ApiError(403,'INGRESS_DENIED');
        let envelope;try{envelope=await readAgentRequest(request,now);}catch{throw new ApiError(403,'AGENT_DENIED');}
        const body:unknown=envelope.body;
        if(path.endsWith('/enroll')){
          const flags=await env.HARNESS_DB.prepare("SELECT enrollment_enabled FROM automation_flags WHERE environment_id='development'").first<{enrollment_enabled:number}>();
          if(flags?.enrollment_enabled!==1)throw new ApiError(403,'ENROLLMENT_DISABLED');
          if(!exact(body,['schema','token','public_key','agent_version','build_sha']) || body.schema!==1 || !version(body.agent_version) || !source(body.build_sha))throw new ApiError(400,'INVALID_ENROLLMENT');
          let publicKey:string,token:string;try{decodeBase64Url(body.token,32);decodeBase64Url(body.public_key,32);token=String(body.token);publicKey=String(body.public_key);await verifyAgentSignature(envelope,publicKey);}catch{throw new ApiError(403,'AGENT_DENIED');}
          const tokenHash=await digest(token);
          const ready=await env.HARNESS_DB.prepare("SELECT token_hash FROM agent_enrollment_tokens WHERE token_hash=? AND consumed_at IS NULL AND expires_at>? AND environment_id='development'").bind(tokenHash,now).first();
          if(!ready)throw new ApiError(403,'ENROLLMENT_DENIED');
          const transaction=await env.HARNESS_DB.batch([
            env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO agents(id,key_id,public_key,enrollment_token_hash,environment_id,capabilities_json,status,enrolled_at)
              SELECT ?,?,?,token_hash,environment_id,capabilities_json,'ACTIVE',? FROM agent_enrollment_tokens WHERE token_hash=? AND consumed_at IS NULL AND expires_at>? AND (SELECT COUNT(*) FROM agents)<10
              AND (SELECT enrollment_enabled FROM automation_flags WHERE environment_id='development')=1`)
              .bind(envelope.agent_id,envelope.key_id,publicKey,now,tokenHash,now),
            env.HARNESS_DB.prepare(`UPDATE agent_enrollment_tokens SET consumed_at=? WHERE token_hash=? AND consumed_at IS NULL
              AND EXISTS(SELECT 1 FROM agents WHERE id=? AND enrollment_token_hash=?)`).bind(now,tokenHash,envelope.agent_id,tokenHash),
            env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO agent_nonces(agent_id,nonce,request_id,expires_at)
              SELECT id,?,?,? FROM agents WHERE id=? AND enrollment_token_hash=?`).bind(envelope.nonce,id,now+300,envelope.agent_id,tokenHash),
            env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO audit_events(event_key,actor_id,action,environment_id,target_id,request_id)
              SELECT ?,id,'agent.enroll',environment_id,id,? FROM agents WHERE id=? AND enrollment_token_hash=?`).bind('agent.enroll:'+envelope.agent_id,id,envelope.agent_id,tokenHash)
          ]);
          if(transaction[0].meta.changes!==1)throw new ApiError(409,'ENROLLMENT_ALREADY_USED_OR_CAP_REACHED');
          return json(201,{agent_id:envelope.agent_id,key_id:envelope.key_id,environment:'development',capabilities:['REPORT_HEARTBEAT']},id);
        }
        const agent=await env.HARNESS_DB.prepare('SELECT id,key_id,public_key,status,environment_id,capabilities_json FROM agents WHERE id=?').bind(envelope.agent_id).first<Agent>();
        if(!agent || agent.status!=='ACTIVE' || agent.environment_id!=='development' || agent.capabilities_json!==capabilities)throw new ApiError(403,'AGENT_DENIED');
        try{await verifyAgentSignature(envelope,agent.public_key);}catch{throw new ApiError(403,'AGENT_DENIED');}
        if(!exact(body,['schema','agent_version','build_sha','uptime_seconds','capabilities','host']) || body.schema!==1 || !version(body.agent_version) || !source(body.build_sha)
            || typeof body.uptime_seconds!=='number' || !Number.isSafeInteger(body.uptime_seconds) || body.uptime_seconds<0 || JSON.stringify(body.capabilities)!==capabilities
            || !exact(body.host,['os','arch']) || body.host.os!=='linux' || !['x86_64','aarch64'].includes(String(body.host.arch)))throw new ApiError(400,'INVALID_HEARTBEAT');
        // Keep nonce storage bounded. Expiration exceeds the complete skew window.
        const transaction=await env.HARNESS_DB.batch([
          env.HARNESS_DB.prepare('DELETE FROM agent_nonces WHERE agent_id=? AND expires_at<?').bind(agent.id,now),
          env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO agent_nonces(agent_id,nonce,request_id,expires_at) SELECT id,?,?,? FROM agents
            WHERE id=? AND status='ACTIVE' AND (SELECT COUNT(*) FROM agent_nonces WHERE agent_id=?)<2000`).bind(envelope.nonce,id,now+300,agent.id,agent.id),
          env.HARNESS_DB.prepare(`UPDATE agents SET heartbeat_at=?,heartbeat_json=? WHERE id=? AND status='ACTIVE'
            AND EXISTS(SELECT 1 FROM agent_nonces WHERE agent_id=? AND nonce=? AND request_id=?)`).bind(now,JSON.stringify(body),agent.id,agent.id,envelope.nonce,id)
        ]);
        if(transaction[1].meta.changes!==1 || transaction[2].meta.changes!==1)throw new ApiError(403,'AGENT_REPLAY_OR_REVOKED');
        return json(200,{agent_id:agent.id,accepted:true,status:'ONLINE',heartbeat_at:now},id);
      }
      if(url.hostname!==env.OPERATOR_API_HOST)throw new ApiError(403,'INGRESS_DENIED');
      const revoke=path.match(/^\/api\/v1\/agents\/(agt_[a-f0-9]{64})\/revoke$/);
      const permission=path==='/api/v1/agents'?'agent.read':path==='/api/v1/agents/enrollment-tokens'?'agent.enroll':revoke?'agent.revoke':null;
      if(!permission)throw new ApiError(404,'NOT_FOUND');
      let principal;try{principal=await operator(request,env);}catch{throw new ApiError(403,'ACCESS_DENIED');}
      const environment=url.searchParams.get('environment')??'development';
      let actor;try{actor=await requireOperatorPermission(env.IDENTITY_DB,{issuer:env.ACCESS_TEAM_DOMAIN,subject:principal.subject},permission,environment);}catch{throw new ApiError(403,'PERMISSION_DENIED');}
      if(environment!=='development')throw new ApiError(403,'ENVIRONMENT_NOT_ENABLED');
      if(path==='/api/v1/agents'){
        if(request.method!=='GET')throw new ApiError(405,'METHOD_NOT_ALLOWED');
        const limit=url.searchParams.get('limit')??'25',cursor=url.searchParams.get('cursor')??'';
        if(!/^[1-9][0-9]?$/.test(limit) || Number(limit)>50 || (cursor && !agentPattern.test(cursor)))throw new ApiError(400,'INVALID_PAGE');
        const result=await env.HARNESS_DB.prepare('SELECT id,key_id,environment_id,capabilities_json,status,enrolled_at,heartbeat_at,heartbeat_json FROM agents WHERE environment_id=? AND id>? ORDER BY id LIMIT ?').bind(environment,cursor,Number(limit)).all<{id:string,key_id:string,environment_id:string,capabilities_json:string,status:string,enrolled_at:number,heartbeat_at:number|null,heartbeat_json:string|null}>();
        const items=result.results.map(row=>({id:row.id,key_id:row.key_id,environment:row.environment_id,capabilities:JSON.parse(row.capabilities_json),status:row.status==='REVOKED'?'REVOKED':row.heartbeat_at===null?'OFFLINE':now-row.heartbeat_at<=120?'ONLINE':now-row.heartbeat_at<=300?'STALE':'OFFLINE',enrolled_at:row.enrolled_at,heartbeat_at:row.heartbeat_at,heartbeat:row.heartbeat_json?JSON.parse(row.heartbeat_json):null}));
        return json(200,{items,next_cursor:items.length===Number(limit)?items.at(-1)?.id:null},id);
      }
      const key=actionKey(request,url),eventKey='agent.action:'+key;
      if(path==='/api/v1/agents/enrollment-tokens'){
        const body=await readJson(request,1024);if(!exact(body,['schema']) || body.schema!==1)throw new ApiError(400,'INVALID_ENROLLMENT_POLICY');
        const flags=await env.HARNESS_DB.prepare("SELECT enrollment_enabled FROM automation_flags WHERE environment_id='development'").first<{enrollment_enabled:number}>();
        if(flags?.enrollment_enabled!==1)throw new ApiError(403,'ENROLLMENT_DISABLED');
        const token=encodeBase64Url(crypto.getRandomValues(new Uint8Array(32))),hash=await digest(token);
        const transaction=await env.HARNESS_DB.batch([
          env.HARNESS_DB.prepare(`DELETE FROM agent_enrollment_tokens WHERE token_hash IN (SELECT token_hash FROM agent_enrollment_tokens
            WHERE expires_at<? AND token_hash NOT IN (SELECT enrollment_token_hash FROM agents) ORDER BY expires_at LIMIT 100)`).bind(now-86400),
          env.HARNESS_DB.prepare(`INSERT INTO agent_enrollment_tokens(token_hash,environment_id,capabilities_json,expires_at,created_at)
            SELECT ?,'development',?,?,? WHERE NOT EXISTS(SELECT 1 FROM audit_events WHERE event_key=?)
            AND (SELECT COUNT(*) FROM agent_enrollment_tokens WHERE expires_at>? AND consumed_at IS NULL)<20 AND (SELECT COUNT(*) FROM agents)<10
            AND (SELECT enrollment_enabled FROM automation_flags WHERE environment_id='development')=1`).bind(hash,capabilities,now+600,now,eventKey,now),
          env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO audit_events(event_key,actor_id,action,environment_id,target_id,request_id)
            SELECT ?,?,'agent.enrollment-token.create','development','enrollment-token',? WHERE EXISTS(SELECT 1 FROM agent_enrollment_tokens WHERE token_hash=?)`).bind(eventKey,actor.actor_id,id,hash)
        ]);
        if(transaction[1].meta.changes!==1)throw new ApiError(409,'TOKEN_ALREADY_ISSUED_OR_CAP_REACHED');
        return json(201,{token,expires_at:now+600,environment:'development',capabilities:['REPORT_HEARTBEAT']},id);
      }
      const target=revoke![1];
      if(!await env.HARNESS_DB.prepare('SELECT id FROM agents WHERE id=? AND environment_id=?').bind(target,environment).first())throw new ApiError(404,'NOT_FOUND');
      const transaction=await env.HARNESS_DB.batch([
        env.HARNESS_DB.prepare(`UPDATE agents SET status='REVOKED',revoked_at=COALESCE(revoked_at,?) WHERE id=?
          AND NOT EXISTS(SELECT 1 FROM audit_events WHERE event_key=?)`).bind(now,target,eventKey),
        auditStatement(env.HARNESS_DB,{event_key:eventKey,actor_id:actor.actor_id,action:'agent.revoke',environment,target_id:target,request_id:id})
      ]);
      void transaction;
      const receipt=await env.HARNESS_DB.prepare('SELECT actor_id,action,target_id,environment_id FROM audit_events WHERE event_key=?').bind(eventKey).first<{actor_id:string,action:string,target_id:string,environment_id:string}>();
      if(!receipt || receipt.actor_id!==actor.actor_id || receipt.action!=='agent.revoke' || receipt.target_id!==target || receipt.environment_id!==environment)throw new ApiError(409,'IDEMPOTENCY_CONFLICT');
      return json(200,{agent_id:target,status:'REVOKED'},id);
    }catch(error){
      if(error instanceof ApiError)return json(error.status,{error:error.code},id);
      console.error(JSON.stringify({event:'agent_api_error',request_id:id}));return json(503,{error:'AGENT_API_UNAVAILABLE'},id);
    }
  }
};}
