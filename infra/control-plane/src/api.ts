import validateArtifact from './artifact-validator.generated.mjs';
import { artifactFields, type ArtifactInput } from '../../deployment/contracts/artifact-fields.mjs';
import { verifyGitHubJob, authorizeGitHubArtifact } from './github-oidc.mjs';
import { verifyAccessPrincipal } from './access.mjs';
import { verifyGitHubDelivery } from './github-webhook.mjs';
import { requireOperatorPermission, auditStatement, recordGitHubDelivery } from './authorization.mjs';
import {ApiError,json,digest,readJson,uuid} from './http.ts';
import {createAgentApi} from './agent-api.ts';
import {createProbeDeploymentApi,type ProbeEnv} from './probe-deployments.ts';

type ApiEnv = ProbeEnv & {GITHUB_WEBHOOK_SECRET?:string};
type Dependencies = {ci:typeof verifyGitHubJob,operator:typeof verifyAccessPrincipal};
type RegistryPolicy = {
  repository:string,repository_id:string,owner_id:string,ref:string,workflow_path:string,
  workflow_sha:string,subject_format:'immutable'|'legacy',enabled:number,internal_repository_id:string
};
type Service = {id:string,repository_id:string,artifact_type:string,artifact_uri_prefix:string};
const candidateId=/^rc_[a-f0-9]{64}$/;
function pagination(url:URL) {
  const value=url.searchParams.get('limit')??'25';
  if(!/^[1-9][0-9]?$/.test(value) || Number(value)>50)throw new ApiError(400,'INVALID_PAGE_LIMIT');
  return {limit:Number(value),cursor:url.searchParams.get('cursor')??''};
}

// Dependency injection is for local signed-key fixtures, never a network/config switch.
export function createControlApi(dependencies:Dependencies={ci:verifyGitHubJob,operator:verifyAccessPrincipal}) {
  const agents=createAgentApi(dependencies.operator);
  const probes=createProbeDeploymentApi(dependencies.operator);
  return {
    async fetch(request:Request,env:ApiEnv):Promise<Response> {
      const id=crypto.randomUUID(),url=new URL(request.url),path=url.pathname;
      if(env.CONTROL_API_ENABLED!=='1')return json(403,{error:'CONTROL_API_DISABLED'},id);
      try {
        if(path==='/api/v1/deployments' || path.startsWith('/api/v1/deployments/') || path.startsWith('/api/v1/agents/probe/'))return await probes.fetch(request,env,id);
        if(path==='/api/v1/agents' || path.startsWith('/api/v1/agents/'))return agents.fetch(request,env,id);
        if(path==='/api/v1/ci/artifacts/register') {
          if(url.hostname!==env.CI_API_HOST || env.ENVIRONMENT!=='development')throw new ApiError(403,'INGRESS_DENIED');
          if(request.method!=='POST')throw new ApiError(405,'METHOD_NOT_ALLOWED');
          let claims;try{claims=await dependencies.ci(request,env.CI_OIDC_AUDIENCE);}catch{throw new ApiError(403,'CI_IDENTITY_DENIED');}
          const enabled=await env.HARNESS_DB.prepare("SELECT registration_enabled FROM automation_flags WHERE environment_id='development'").first<{registration_enabled:number}>();
          if(enabled?.registration_enabled!==1)throw new ApiError(409,'REGISTRATION_PAUSED');
          let metadata:ArtifactInput & {candidate_id:string};
          try{
            const {normalized,identity}=artifactFields(await readJson(request) as ArtifactInput,validateArtifact);
            const expected='rc_'+await digest(identity);
            if(normalized.candidate_id!==undefined && normalized.candidate_id!==expected)throw Error('Identity mismatch');
            metadata={...normalized,candidate_id:expected};
          }catch(error){if(error instanceof ApiError)throw error;throw new ApiError(400,'INVALID_ARTIFACT');}
          const workflowPath=String(claims.workflow_ref).split('@')[0].slice(String(claims.repository).length+1);
          const policy=await env.HARNESS_DB.prepare(`SELECT r.full_name AS repository,r.github_repository_id AS repository_id,
            r.github_owner_id AS owner_id,p.git_ref AS ref,p.workflow_path,p.workflow_sha,p.subject_format,
            p.enabled,r.id AS internal_repository_id FROM repositories r JOIN ci_identity_policies p ON p.repository_id=r.id
            WHERE r.full_name=? COLLATE NOCASE AND r.enabled=1 AND p.enabled=1 AND p.git_ref=? AND p.workflow_sha=? AND p.workflow_path=?`)
            .bind(metadata.repository,String(claims.ref),String(claims.workflow_sha),workflowPath).first<RegistryPolicy>();
          if(!policy)throw new ApiError(403,'REPOSITORY_NOT_AUTHORIZED');
          let actor;try{actor=authorizeGitHubArtifact(claims,policy,metadata);}catch{throw new ApiError(403,'REGISTRATION_DENIED');}
          const service=await env.HARNESS_DB.prepare('SELECT id,repository_id,artifact_type,artifact_uri_prefix FROM services WHERE id=? AND enabled=1').bind(metadata.service_id).first<Service>();
          if(!service || service.repository_id!==policy.internal_repository_id || service.artifact_type!==metadata.artifact_type
              || !service.artifact_uri_prefix || !metadata.artifact_uri.startsWith(service.artifact_uri_prefix))throw new ApiError(403,'SERVICE_ARTIFACT_DENIED');
          const existing=await env.HARNESS_DB.prepare('SELECT metadata_json FROM deployment_candidates WHERE id=?').bind(metadata.candidate_id).first<{metadata_json:string}>();
          if(existing)return json(200,{candidate:JSON.parse(existing.metadata_json),created:false},id);
          const artifactId='art_'+await digest([service.id,metadata.artifact_type,metadata.artifact_uri,metadata.sha256].join('\0'));
          const canonical=JSON.stringify(metadata);
          const transaction=await env.HARNESS_DB.batch([
            env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO artifacts(id,repository_id,service_id,artifact_type,artifact_uri,sha256)
              SELECT ?,?,?,?,?,? WHERE (SELECT COUNT(*) FROM deployment_candidates)<1000`).bind(artifactId,policy.internal_repository_id,service.id,metadata.artifact_type,metadata.artifact_uri,metadata.sha256),
            env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO deployment_candidates(id,artifact_id,repository_id,service_id,git_sha,git_ref,build_run_id,metadata_json)
              SELECT ?,id,?,?,?,?,?,? FROM artifacts WHERE id=? AND (SELECT COUNT(*) FROM deployment_candidates)<1000`)
              .bind(metadata.candidate_id,policy.internal_repository_id,service.id,metadata.git_sha,metadata.git_ref,metadata.build_run_id,canonical,artifactId),
            env.HARNESS_DB.prepare(`INSERT OR IGNORE INTO audit_events(event_key,actor_id,action,environment_id,target_id,request_id)
              SELECT ?,?,'candidate.register','development',?,? WHERE EXISTS(SELECT 1 FROM deployment_candidates WHERE id=?)`)
              .bind('candidate.register:'+metadata.candidate_id,actor.actor,metadata.candidate_id,id,metadata.candidate_id)
          ]);
          const stored=await env.HARNESS_DB.prepare('SELECT metadata_json FROM deployment_candidates WHERE id=?').bind(metadata.candidate_id).first<{metadata_json:string}>();
          if(!stored)throw new ApiError(429,'REGISTRATION_CAP_REACHED');
          const created=transaction[1].meta.changes===1;
          return json(created?201:200,{candidate:JSON.parse(stored.metadata_json),created},id);
        }
        if(path==='/api/v1/github/webhooks') {
          if(url.hostname!==env.CI_API_HOST || env.WEBHOOKS_ENABLED!=='1')throw new ApiError(403,'WEBHOOK_DISABLED');
          let delivery;try{delivery=await verifyGitHubDelivery(request,env.GITHUB_WEBHOOK_SECRET);}catch{throw new ApiError(403,'WEBHOOK_DENIED');}
          const payload=delivery.payload;
          if(!payload || typeof payload.repository?.id!=='number' || !Number.isSafeInteger(payload.repository.id))throw new ApiError(400,'UNKNOWN_WEBHOOK_REPOSITORY');
          const known=await env.HARNESS_DB.prepare('SELECT id FROM repositories WHERE github_repository_id=? AND enabled=1').bind(String(payload.repository.id)).first();
          if(!known)throw new ApiError(403,'REPOSITORY_NOT_AUTHORIZED');
          let receipt;try{receipt=await recordGitHubDelivery(env.HARNESS_DB,delivery);}catch{throw new ApiError(409,'DELIVERY_CONFLICT');}
          // Events are informational. Candidate creation requires a separate CI identity.
          await env.HARNESS_DB.prepare("UPDATE github_deliveries SET status='IGNORED' WHERE id=? AND status='RECEIVED'").bind(delivery.delivery).run();
          return json(202,{accepted:true,duplicate:!receipt.created,deployment_performed:false},id);
        }
        if(url.hostname!==env.OPERATOR_API_HOST)throw new ApiError(403,'INGRESS_DENIED');
        let principal;try{principal=await dependencies.operator(request,env);}catch{throw new ApiError(403,'ACCESS_DENIED');}
        const environment=url.searchParams.get('environment')??'development';
        const permissions:Record<string,string>={
          '/api/v1/repositories':'repository.read','/api/v1/services':'service.read',
          '/api/v1/environments':'environment.read','/api/v1/candidates':'candidate.read','/api/v1/artifacts':'artifact.read',
          '/api/v1/audit':'audit.read','/api/v1/automation':'automation.read',
          '/api/v1/automation/pause':'automation.registration.manage','/api/v1/automation/resume':'automation.registration.manage'
        };
        const detail=path.match(/^\/api\/v1\/candidates\/(rc_[a-f0-9]{64})$/);
        const permission=detail?'candidate.read':permissions[path];if(!permission)throw new ApiError(404,'NOT_FOUND');
        let actor;try{actor=await requireOperatorPermission(env.IDENTITY_DB,{issuer:env.ACCESS_TEAM_DOMAIN,subject:principal.subject},permission,environment);}catch{throw new ApiError(403,'PERMISSION_DENIED');}
        // Future environments need environment-owned promotion queries first.
        if(environment!=='development' || env.ENVIRONMENT!=='development')throw new ApiError(403,'ENVIRONMENT_NOT_ENABLED');
        if(path.endsWith('/pause') || path.endsWith('/resume')) {
          if(request.method!=='POST')throw new ApiError(405,'METHOD_NOT_ALLOWED');
          if(environment!=='development' || env.ENVIRONMENT!=='development')throw new ApiError(403,'ENVIRONMENT_NOT_ENABLED');
          if(request.headers.get('x-velora-action')!=='1' || request.headers.get('origin')!==url.origin)throw new ApiError(403,'ORIGIN_DENIED');
          const key=request.headers.get('idempotency-key')??'';if(!uuid.test(key))throw new ApiError(400,'INVALID_IDEMPOTENCY_KEY');
          const action=path.endsWith('/resume')?'registration.resume':'registration.pause',eventKey='automation:'+key;
          const prior=await env.HARNESS_DB.prepare('SELECT action,actor_id,environment_id FROM audit_events WHERE event_key=?').bind(eventKey).first<{action:string,actor_id:string,environment_id:string}>();
          if(prior && (prior.action!==action || prior.actor_id!==actor.actor_id || prior.environment_id!==environment))throw new ApiError(409,'IDEMPOTENCY_CONFLICT');
          await env.HARNESS_DB.batch([
            env.HARNESS_DB.prepare(`UPDATE automation_flags SET registration_enabled=?,deployment_enabled=0,updated_at=CURRENT_TIMESTAMP
              WHERE environment_id=? AND NOT EXISTS(SELECT 1 FROM audit_events WHERE event_key=?)`).bind(action==='registration.resume'?1:0,environment,eventKey),
            auditStatement(env.HARNESS_DB,{event_key:eventKey,actor_id:actor.actor_id,action,environment,target_id:environment,request_id:id})
          ]);
          const receipt=await env.HARNESS_DB.prepare('SELECT action,actor_id,environment_id FROM audit_events WHERE event_key=?').bind(eventKey).first<{action:string,actor_id:string,environment_id:string}>();
          if(!receipt || receipt.action!==action || receipt.actor_id!==actor.actor_id || receipt.environment_id!==environment)throw new ApiError(409,'IDEMPOTENCY_CONFLICT');
          return json(200,{registration_enabled:action==='registration.resume',deployment_enabled:false},id);
        }
        if(request.method!=='GET')throw new ApiError(405,'METHOD_NOT_ALLOWED');
        if(detail) {
          const row=await env.HARNESS_DB.prepare('SELECT metadata_json FROM deployment_candidates WHERE id=?').bind(detail[1]).first<{metadata_json:string}>();
          if(!row)throw new ApiError(404,'NOT_FOUND');return json(200,{candidate:JSON.parse(row.metadata_json)},id);
        }
        const {limit,cursor}=pagination(url);
        let rows:Record<string,unknown>[];
        if(path==='/api/v1/candidates') {
          if(cursor && !candidateId.test(cursor))throw new ApiError(400,'INVALID_CURSOR');
          const result=await env.HARNESS_DB.prepare('SELECT id,metadata_json FROM deployment_candidates WHERE id>? ORDER BY id LIMIT ?').bind(cursor,limit).all<{id:string,metadata_json:string}>();
          return json(200,{items:result.results.map(row=>JSON.parse(row.metadata_json)),next_cursor:result.results.length===limit?result.results.at(-1)?.id:null},id);
        }
        if(path==='/api/v1/audit') {
          if(cursor && !/^[1-9][0-9]{0,14}$/.test(cursor))throw new ApiError(400,'INVALID_CURSOR');
          const result=await env.HARNESS_DB.prepare('SELECT id,actor_id,action,environment_id,target_id,request_id,created_at FROM audit_events WHERE environment_id=? AND id>? ORDER BY id LIMIT ?').bind(environment,Number(cursor||0),limit).all();
          return json(200,{items:result.results,next_cursor:result.results.length===limit?String(result.results.at(-1)?.id):null},id);
        }
        if(path==='/api/v1/artifacts') {
          if(cursor && !/^art_[a-f0-9]{64}$/.test(cursor))throw new ApiError(400,'INVALID_CURSOR');
          const result=await env.HARNESS_DB.prepare('SELECT id,repository_id,service_id,artifact_type,artifact_uri,sha256,created_at FROM artifacts WHERE id>? ORDER BY id LIMIT ?').bind(cursor,limit).all();
          return json(200,{items:result.results,next_cursor:result.results.length===limit?result.results.at(-1)?.id:null},id);
        }
        if(path==='/api/v1/repositories' || path==='/api/v1/services') {
          if(cursor && !/^[A-Za-z0-9_.:-]{1,128}$/.test(cursor))throw new ApiError(400,'INVALID_CURSOR');
          const result=path==='/api/v1/repositories'
            ? await env.HARNESS_DB.prepare('SELECT id,full_name,enabled FROM repositories WHERE id>? ORDER BY id LIMIT ?').bind(cursor,limit).all()
            : await env.HARNESS_DB.prepare('SELECT id,repository_id,artifact_type,enabled FROM services WHERE id>? ORDER BY id LIMIT ?').bind(cursor,limit).all();
          return json(200,{items:result.results,next_cursor:result.results.length===limit?result.results.at(-1)?.id:null},id);
        }
        if(cursor)throw new ApiError(400,'CURSOR_NOT_SUPPORTED');
        if(path==='/api/v1/environments')rows=(await env.HARNESS_DB.prepare('SELECT id,display_name FROM environments WHERE id=?').bind(environment).all()).results;
        else rows=(await env.HARNESS_DB.prepare('SELECT environment_id,registration_enabled,deployment_enabled FROM automation_flags WHERE environment_id=?').bind(environment).all()).results;
        return json(200,{items:rows},id);
      } catch(error) {
        if(error instanceof ApiError)return json(error.status,{error:error.code},id);
        console.error(JSON.stringify({event:'control_api_error',request_id:id}));
        return json(503,{error:'CONTROL_API_UNAVAILABLE'},id);
      }
    }
  };
}
