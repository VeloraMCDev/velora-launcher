import { WorkflowEntrypoint, type WorkflowEvent, type WorkflowStep } from 'cloudflare:workers';
import { verifyBootstrapOperator } from './access.mjs';
import { createControlApi } from './api.ts';
import {serveOperatorAssets,type OperatorAssetsEnv} from './operator-assets.ts';
import {prepareProbeJob,blockProbeDeployment,type ProbeEnv} from './probe-deployments.ts';

const controlApi=createControlApi();

const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
type CheckParams = {id:string};

function response(status:number,body:Record<string,unknown>,requestId:string):Response {
  return Response.json({...body,request_id:requestId},{status,headers:{
    'Cache-Control':'no-store','X-Content-Type-Options':'nosniff','X-Request-Id':requestId
  }});
}

export default {
  async fetch(request:Request,env:OperatorAssetsEnv):Promise<Response> {
    const requestId = crypto.randomUUID();
    const url = new URL(request.url);
    if (url.pathname === '/health' && request.method === 'GET') {
      return response(200,{service:'velora-control-plane',environment:env.ENVIRONMENT,commit_sha:env.BUILD_SHA,phase:env.CONTROL_API_ENABLED==='1'?'candidate-registration':'resource-baseline'},requestId);
    }
    if (!url.pathname.startsWith('/api/v1/bootstrap/')) {
      if(url.pathname.startsWith('/api/v1/'))return controlApi.fetch(request,env);
      return serveOperatorAssets(request,env);
    }
    if (env.ENVIRONMENT !== 'development' || env.BOOTSTRAP_ENABLED !== '1') return response(403,{error:'BOOTSTRAP_DISABLED'},requestId);
    if(env.OPERATOR_API_HOST && url.hostname!==env.OPERATOR_API_HOST)return response(403,{error:'INGRESS_DENIED'},requestId);
    let principal;
    try { principal=await verifyBootstrapOperator(request,env); }
    catch { return response(403,{error:'ACCESS_DENIED'},requestId); }
    if(url.pathname==='/api/v1/bootstrap/principal' && request.method==='GET')return response(200,{issuer:env.ACCESS_TEAM_DOMAIN,subject:principal.subject},requestId);
    try {
      if (url.pathname === '/api/v1/bootstrap/bindings-check' && request.method === 'POST') {
        // Custom header prevents cross-origin simple requests; there is no CORS grant.
        if (request.headers.get('x-velora-bootstrap') !== '1'
            || (request.headers.has('origin') && request.headers.get('origin') !== url.origin)) return response(403,{error:'ORIGIN_DENIED'},requestId);
        const id = request.headers.get('idempotency-key') ?? '';
        if (!uuid.test(id)) return response(400,{error:'INVALID_IDEMPOTENCY_KEY'},requestId);
        await env.HARNESS_DB.prepare("INSERT OR IGNORE INTO bootstrap_checks(id,environment,status) SELECT ?,'development','RESERVED' WHERE (SELECT COUNT(*) FROM bootstrap_checks) < 10").bind(id).run();
        if (!await env.HARNESS_DB.prepare('SELECT id FROM bootstrap_checks WHERE id = ?').bind(id).first()) return response(429,{error:'BOOTSTRAP_LIMIT_REACHED'},requestId);
        const workflowId = 'bindings-' + id;
        try { await env.BINDING_CHECK.get(workflowId).then(instance => instance.status()); }
        catch {
          try { await env.BINDING_CHECK.create({id:workflowId,params:{id}}); }
          catch { await (await env.BINDING_CHECK.get(workflowId)).status(); }
        }
        return response(202,{id,workflow_id:workflowId},requestId);
      }
      const match = url.pathname.match(/^\/api\/v1\/bootstrap\/bindings-check\/([^/]+)$/);
      if (match && request.method === 'GET' && uuid.test(match[1])) {
        const check = await env.HARNESS_DB.prepare('SELECT status FROM bootstrap_checks WHERE id = ?').bind(match[1]).first();
        if (!check) return response(404,{error:'NOT_FOUND'},requestId);
        const status = await (await env.BINDING_CHECK.get('bindings-' + match[1])).status();
        return response(200,{id:match[1],status:status.status,output:status.output ?? null},requestId);
      }
      return response(405,{error:'METHOD_NOT_ALLOWED'},requestId);
    } catch {
      console.error(JSON.stringify({event:'bootstrap_binding_error',request_id:requestId}));
      return response(503,{error:'BINDING_UNAVAILABLE'},requestId);
    }
  }
} satisfies ExportedHandler<Env>;

export class BindingCheckWorkflow extends WorkflowEntrypoint<Env,CheckParams> {
  async run(event:Readonly<WorkflowEvent<CheckParams>>,step:WorkflowStep) {
    if (this.env.ENVIRONMENT !== 'development' || !uuid.test(event.payload.id)
        || event.instanceId !== 'bindings-' + event.payload.id) throw Error('Invalid binding check');
    const id = event.payload.id;
    return step.do('verify-development-bindings',{
      retries:{limit:2,delay:'1 second',backoff:'exponential'},timeout:'30 seconds'
    },async () => {
      const key = 'bootstrap/' + id + '.json';
      if (!await this.env.HARNESS_DB.prepare('SELECT id FROM bootstrap_checks WHERE id = ?').bind(id).first()) throw Error('Unregistered binding check');
      try {
        const harness = await this.env.HARNESS_DB.prepare('SELECT purpose FROM schema_metadata WHERE id = 1').first<{purpose:string}>();
        const identity = await this.env.IDENTITY_DB.prepare('SELECT purpose FROM schema_metadata WHERE id = 1').first<{purpose:string}>();
        if (harness?.purpose !== 'velora-harness' || identity?.purpose !== 'velora-identity') throw Error('Wrong database');
        await this.env.IDENTITY_DB.prepare("INSERT OR IGNORE INTO bootstrap_checks VALUES (?, 'binding-check')").bind(id).run();
        if (!await this.env.IDENTITY_DB.prepare('SELECT id FROM bootstrap_checks WHERE id = ?').bind(id).first()) throw Error('Identity write failed');
        await this.env.OPERATIONS.put(key,JSON.stringify({schema_version:1,id}),{customMetadata:{check_id:id}});
        const object = await this.env.OPERATIONS.head(key);
        if (object?.customMetadata?.check_id !== id || object.size > 256) throw Error('Object verification failed');
      } catch (error) {
        await this.env.HARNESS_DB.prepare("UPDATE bootstrap_checks SET status='FAILED',completed_at=CURRENT_TIMESTAMP WHERE id = ?").bind(id).run();
        throw error;
      } finally {
        await this.env.OPERATIONS.delete(key);
        await this.env.IDENTITY_DB.prepare('DELETE FROM bootstrap_checks WHERE id = ?').bind(id).run();
      }
      await this.env.HARNESS_DB.prepare("UPDATE bootstrap_checks SET status='SUCCEEDED',completed_at=CURRENT_TIMESTAMP WHERE id = ?").bind(id).run();
      return {harness:'PASS',identity:'PASS',r2:'PASS',workflow:'PASS',cleanup:'PASS',deployment_performed:false};
    });
  }
}

export class ProbeDeploymentWorkflow extends WorkflowEntrypoint<ProbeEnv,CheckParams>{
  async run(event:Readonly<WorkflowEvent<CheckParams>>,step:WorkflowStep){
    if(!uuid.test(event.payload.id) || event.instanceId!=='probe-'+event.payload.id)throw Error('Invalid probe deployment');
    const id=event.payload.id;
    const options={retries:{limit:2,delay:'1 second',backoff:'exponential'},timeout:'30 seconds'} as const;
    try{
      await step.do('prepare-signed-job',options,async()=>{await prepareProbeJob(this.env,id);return {id};});
      for(let attempt=0;attempt<15;attempt++){
        const row=await step.do('read-result-'+attempt,options,async()=>this.env.HARNESS_DB.prepare('SELECT status,receipt_json FROM probe_deployments WHERE id=?').bind(id).first<{status:string,receipt_json:string|null}>());
        if(row?.receipt_json)return {deployment_id:id,status:row.status};
        await step.sleep('wait-result-'+attempt,'30 seconds');
      }
    }catch{
      // No automatic second job, image retry or data restore after uncertainty.
    }
    await step.do('retain-uncertain-deployment-lock',options,async()=>{await blockProbeDeployment(this.env,id);return {id};});
    const row=await step.do('final-status',options,async()=>this.env.HARNESS_DB.prepare('SELECT status FROM probe_deployments WHERE id=?').bind(id).first<{status:string}>());
    return {deployment_id:id,status:row?.status??'BLOCKED'};
  }
}
