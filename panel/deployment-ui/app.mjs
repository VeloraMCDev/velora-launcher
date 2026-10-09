import {ControlClient,SessionUnavailable} from './client.mjs';
import {probeRequest} from './probe-plan.mjs';
const el=id=>document.getElementById(id),collections=['repositories','candidates','agents','audit'];
let codeTimer,refreshing=false,sessionLost=false;const cursors={};
const choices={candidates:[],agents:[]};let pendingProbe;
function updateProbeChoices(){
  if(pendingProbe)return;
  for(const [name,id] of [['candidates','probe-candidate'],['agents','probe-agent']]){
    const select=el(id),previous=select.value;select.replaceChildren();
    const placeholder=document.createElement('option');placeholder.value='';placeholder.textContent=name==='candidates'?'Select a tested probe image':'Select a connected machine';select.append(placeholder);
    for(const item of choices[name]){
      if(name==='candidates' && (item.service_id!=='deployment-probe' || item.repository!=='veloramcdev/velora-launcher'))continue;
      if(name==='agents' && (item.status!=='ONLINE' || item.environment!=='development'))continue;
      const option=document.createElement('option');option.value=item.candidate_id??item.id;option.textContent=name==='candidates'?item.git_sha+' · '+item.sha256:item.id;select.append(option);
    }
    select.value=previous;select.disabled=sessionLost;
  }
  el('start-probe').disabled=sessionLost || !el('probe-candidate').value || !el('probe-agent').value;
}
function clearCode(){clearTimeout(codeTimer);el('enrollment-code').value='';el('code-expiry').textContent='';el('code-box').hidden=true;}
function clearSession(){sessionLost=true;clearCode();pendingProbe=undefined;choices.candidates=[];choices.agents=[];updateProbeChoices();for(const name of [...collections,'deployments']){el(name).replaceChildren();el('next-'+name).hidden=true;cursors[name]=null;}for(const id of ['repository-count','candidate-count','agent-count','deployment-count'])el(id).textContent='—';el('enroll').disabled=true;}
const client=new ControlClient(fetch,clearSession);
function notice(message){el('notice').textContent=message;}
function row(title,detail,badge){const node=document.createElement('div');node.className='row';const heading=document.createElement('strong');heading.textContent=title;node.append(heading);if(badge){const state=document.createElement('p');state.className='badge';state.textContent=badge;node.append(state);}const text=document.createElement('p');text.textContent=detail;node.append(text);return node;}
const deploymentStates={RESERVED:'Reserved',JOB_READY:'Waiting for result',SUCCEEDED:'Succeeded',FAILED:'Failed',ROLLED_BACK:'Rolled back',BLOCKED:'Needs reconciliation',ROLLBACK_FAILED:'Rollback failed'};
const executionEvents={PREFLIGHT:'Checked host policy',PULLING_OR_STAGING:'Pulled and staged the exact image',HEALTH_CHECK:'Checked the expected source revision',ROLLING_BACK:'Restoring the previous image',SUCCEEDED:'Deployment succeeded',FAILED:'Deployment failed and was stopped',ROLLED_BACK:'Previous image restored',BLOCKED:'Execution needs reconciliation',ROLLBACK_FAILED:'Rollback failed; deployment lock retained'};
async function showTimeline(item,node,button){
  button.disabled=true;
  try{
    const result=await client.request('/api/v1/deployments/'+item.id+'?environment=development');
    if(sessionLost)return;
    const deployment=result.deployment;
    if(!deployment || deployment.id!==item.id || !deploymentStates[deployment.status])throw Error('Unexpected deployment record.');
    const lines=['Deployment reserved'];
    if(deployment.delivered_at)lines.push('Signed job delivered '+new Date(deployment.delivered_at*1000).toLocaleString());
    if(deployment.receipt_json){
      if(typeof deployment.receipt_json!=='string' || deployment.receipt_json.length>4096)throw Error('Unexpected execution receipt.');
      const receipt=JSON.parse(deployment.receipt_json);
      if(receipt.job_id!==deployment.job_id || receipt.status!==deployment.status || !Array.isArray(receipt.events) || receipt.events.length>16
        || receipt.events.some(event=>!Object.hasOwn(executionEvents,event)))throw Error('Unexpected execution receipt.');
      lines.push(...receipt.events.map(event=>executionEvents[event]));
    }else if(deployment.status==='BLOCKED')lines.push('No confirmed execution result. Deployment lock retained.');
    else lines.push('Awaiting a confirmed execution result.');
    const timeline=document.createElement('ol');
    for(const text of lines){const entry=document.createElement('li');entry.textContent=text;timeline.append(entry);}
    node.querySelector('ol')?.remove();node.append(timeline);node.querySelector('.badge').textContent=deploymentStates[deployment.status];button.textContent='Refresh timeline';
  }catch(error){notice(error.message);}finally{if(!sessionLost)button.disabled=false;}
}
async function load(name,cursor=''){
  const result=await client.request('/api/v1/'+name+'?environment=development&limit=20'+(cursor?'&cursor='+encodeURIComponent(cursor):''));
  if(sessionLost)return;
  if(!Array.isArray(result.items))throw Error('Unexpected response. Refresh to try again.');
  if(Object.hasOwn(choices,name)){if(!cursor)choices[name]=[];const seen=new Map(choices[name].map(item=>[item.candidate_id??item.id,item]));for(const item of result.items)seen.set(item.candidate_id??item.id,item);choices[name]=[...seen.values()];updateProbeChoices();}
  const nodes=result.items.map(item=>{
    if(name==='deployments'){
      if(typeof item.id!=='string' || !/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(item.id) || !Object.hasOwn(deploymentStates,item.status))throw Error('Unexpected deployment record.');
      const node=row('Deployment '+item.id,'Candidate '+item.candidate_id+' · Machine '+item.agent_id,deploymentStates[item.status]);
      const button=document.createElement('button');button.textContent='View timeline';button.onclick=()=>showTimeline(item,node,button);node.append(button);return node;
    }
    if(name==='repositories')return row(item.full_name,'Repository connected',item.enabled?'Registration enabled':'Registration paused');
    if(name==='candidates')return row(item.service_id??'Release candidate','Source '+(item.git_sha??'Unknown')+' · '+(item.artifact_type??'Artifact'),item.candidate_id);
    if(name==='audit')return row(item.action,item.created_at+' · '+item.target_id,item.environment_id);
    const node=row(item.id,'Last report: '+(item.heartbeat_at?new Date(item.heartbeat_at*1000).toLocaleString():'Awaiting first report'),item.status);
    if(item.status!=='REVOKED'){const button=document.createElement('button');button.className='danger';button.textContent='Revoke connection';button.onclick=async()=>{
      if(!confirm('Revoke this machine’s connection? It will stop reporting to Velora.'))return;
      button.disabled=true;try{await client.request('/api/v1/agents/'+item.id+'/revoke',{schema:1});await load('agents');notice('Machine connection revoked.');}catch(error){notice(error.message);if(!sessionLost)button.disabled=false;}
    };node.append(button);}return node;
  });
  el(name).replaceChildren(...(nodes.length?nodes:[row('No records yet','This section updates when records are available.')]));
  cursors[name]=result.next_cursor;el('next-'+name).hidden=!result.next_cursor;
  const counter={repositories:'repository-count',candidates:'candidate-count',agents:'agent-count',deployments:'deployment-count'}[name];if(counter)el(counter).textContent=result.items.length+(result.next_cursor?'+':'');
}
async function refresh(){if(refreshing)return;refreshing=true;sessionLost=false;clearCode();el('refresh').disabled=true;el('enroll').disabled=true;notice('Refreshing Development…');
  try{await Promise.all(collections.map(name=>load(name)));if(!sessionLost){el('enroll').disabled=false;notice('Development view updated.');}}
  catch(error){if(error instanceof SessionUnavailable)clearSession();notice(error.message);}finally{refreshing=false;el('refresh').disabled=false;}
}
el('refresh').onclick=refresh;el('dismiss-code').onclick=clearCode;
for(const id of ['probe-candidate','probe-agent'])el(id).onchange=updateProbeChoices;
el('probe-form').onsubmit=async event=>{
  event.preventDefault();if(sessionLost)return;
  const button=el('start-probe');button.disabled=true;
  try{
    if(!pendingProbe){const body=probeRequest(choices.candidates.find(item=>item.candidate_id===el('probe-candidate').value),choices.agents.find(item=>item.id===el('probe-agent').value));pendingProbe={body,idempotencyKey:crypto.randomUUID()};}
    el('probe-candidate').disabled=true;el('probe-agent').disabled=true;
    const result=await client.request('/api/v1/deployments',pendingProbe.body,{idempotencyKey:pendingProbe.idempotencyKey});
    if(sessionLost)return;
    if(!/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(result.deployment_id??'') || !Object.hasOwn(deploymentStates,result.status))throw Error('Unconfirmed deployment response. Retry this action to reconcile it.');
    pendingProbe=undefined;button.textContent='Start Development probe';updateProbeChoices();notice('Development probe reserved. Load its timeline for the confirmed execution result.');await load('deployments');
  }catch(error){notice(error.message);if(!sessionLost){button.disabled=false;if(pendingProbe)button.textContent='Retry the same probe request';}}
};
for(const name of [...collections,'deployments'])el('next-'+name).onclick=()=>load(name,cursors[name]).catch(error=>notice(error.message));
el('refresh-deployments').onclick=async()=>{if(sessionLost){notice('Refresh your protected session before loading deployments.');return;}el('refresh-deployments').disabled=true;try{await load('deployments');if(!sessionLost)notice('Deployment history updated.');}catch(error){notice(error.message);}finally{el('refresh-deployments').disabled=false;}};
el('enroll').onclick=async()=>{el('enroll').disabled=true;clearCode();try{
  const result=await client.request('/api/v1/agents/enrollment-tokens',{schema:1});
  if(sessionLost)return;
  if(typeof result.token!=='string'||!/^[A-Za-z0-9_-]{43}$/.test(result.token)||!Number.isSafeInteger(result.expires_at))throw Error('Invalid enrollment response.');
  const remaining=result.expires_at*1000-Date.now();if(remaining<=0||remaining>600000)throw Error('Enrollment code expired.');
  el('enrollment-code').value=result.token;el('code-expiry').textContent='Expires '+new Date(result.expires_at*1000).toLocaleTimeString()+'. Used once.';el('code-box').hidden=false;codeTimer=setTimeout(clearCode,remaining);notice('Enrollment code ready. Paste it only into your Hermes setup terminal.');
}catch(error){notice(error.message);}finally{if(!sessionLost)el('enroll').disabled=false;}};
window.addEventListener('pagehide',clearSession);document.addEventListener('visibilitychange',()=>{if(document.hidden)clearCode();});
refresh();
