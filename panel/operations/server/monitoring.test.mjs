import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:net';
import { createAgentRegistry, validateReport } from './agents.mjs';
import { artifactCatalog, artifactDownload, trustedBuild, createCatalogCache } from './artifacts.mjs';
import { managedContainers, STACK_PROJECT, evaluate } from './monitor.mjs';
import { config, tcpProbe, probe } from '../agent/monitor.mjs';

const report = () => ({platform:'linux',version:'1.0.0',uptime:40,cpu:20,memory:{total:100,used:50},disks:[],services:[{name:'SMP',kind:'minecraft',ok:true,ms:8,players:{online:2,max:20,sample:[{name:'private'}]},url:'private'}]});
const run = (extra={}) => ({id:12,name:'Plugins',conclusion:'success',status:'completed',head_branch:'main',event:'workflow_dispatch',repository:{full_name:'VeloraMCDev/velora-launcher'},head_repository:{full_name:'VeloraMCDev/velora-launcher'},head_sha:'abc',html_url:'https://github.com/example/run',...extra});
const artifact = (extra={}) => ({id:9,name:'plugin-jars',expired:false,expires_at:'2099-01-01',created_at:'2026-01-01',size_in_bytes:40,workflow_run:{id:12,head_branch:'main',head_sha:'abc',repository_id:1,head_repository_id:1},...extra});

test('agent credentials are persisted hashed, scoped to reports, revocable and survive restart',()=>{
  let saved=[]; const registry=()=>createAgentRegistry({read:()=>structuredClone(saved),write:v=>{saved=structuredClone(v);}});
  const a=registry(), enrolled=a.enroll('Game host');
  assert.ok(!JSON.stringify(saved).includes(enrolled.token));
  assert.ok(!JSON.stringify(a.list()).includes('token_hash'));
  assert.throws(()=>a.authenticate(enrolled.id,'wrong'),{status:401});
  const now=Date.now();a.report(enrolled.id,enrolled.token,report(),now);
  assert.equal(a.list(now)[0].online,true);assert.equal(a.list(now+90001)[0].online,false);
  assert.deepEqual(a.list()[0].report.services[0].players,{online:2,max:20});
  assert.ok(!JSON.stringify(a.list()).includes('private'));
  assert.throws(()=>a.report(enrolled.id,enrolled.token,report(),now+1),{status:429});
  const b=registry();b.authenticate(enrolled.id,enrolled.token);b.revoke(enrolled.id);
  assert.throws(()=>b.authenticate(enrolled.id,enrolled.token),{status:401});assert.deepEqual(saved,[]);
});
test('bad reports and failed persistence never change accepted state',()=>{
  for(const services of [[null],[{...report().services[0],players:null}],[...report().services,...report().services]]) assert.throws(()=>validateReport({...report(),services}),{status:400});
  let fail=false;const a=createAgentRegistry({read:()=>[],write:()=>{if(fail)throw Error('disk full');}}),e=a.enroll('Host');fail=true;
  assert.throws(()=>a.report(e.id,e.token,report()),/disk full/);assert.equal(a.list()[0].report,null);
  assert.throws(()=>a.revoke(e.id),/disk full/);a.authenticate(e.id,e.token);
});
test('catalog lists all release assets and trusted main output, with expiry and partial source failures',async()=>{
  const get=async p=>p.includes('releases?') ? [{tag_name:'core-v1',published_at:'2026-01-01',assets:[{id:1,state:'uploaded',name:'fabric.jar',size:10},{id:2,state:'uploaded',name:'paper.jar',size:10}]}] : p.includes('artifacts?') ? {total_count:101,artifacts:[artifact({expired:true})]} : run();
  const c=await artifactCatalog('',1,get);assert.equal(c.items.length,3);assert.equal(c.has_more,true);assert.equal(c.items.find(i=>i.source==='build').expired,true);
  const rejected=await artifactCatalog('',1,async p=>p.includes('runs/')?run({event:'pull_request'}):get(p));assert.equal(rejected.items.length,2);
  const partial=await artifactCatalog('',1,async p=>{if(p.includes('releases?'))throw Error('unavailable');return get(p);});assert.equal(partial.items.length,1);assert.equal(partial.errors.length,1);
  assert.equal(trustedBuild(run({conclusion:'failure'})),false);assert.equal(trustedBuild(run({head_repository:{full_name:'other/fork'}})),false);
});
test('downloads recheck trust and expiration; GitHub token goes only to the API request',async()=>{
  const get=async p=>p.includes('artifacts/')?artifact():run();
  const credential='fixture'.repeat(6);
  const url=await artifactDownload(9,credential,get,async(url,options)=>{assert.match(url,/^https:\/\/api.github.com\//);assert.equal(options.redirect,'manual');assert.equal(options.headers.authorization,`Bearer ${credential}`);return new Response(null,{status:302,headers:{location:'https://storage.example/archive.zip'}});});
  assert.equal(url,'https://storage.example/archive.zip');
  await assert.rejects(artifactDownload(9,'',get),{status:503});
  await assert.rejects(artifactDownload(9,credential,async()=>artifact({expired:true})),{status:410});
  await assert.rejects(artifactDownload(9,credential,async p=>p.includes('artifacts/')?artifact():run({head_sha:'changed'})),{status:403});
  await assert.rejects(artifactDownload(9,credential,get,async()=>new Response(null,{status:302,headers:{location:'http://storage.example/archive'}})),{status:502});
});
test('catalog cache shares concurrent requests, expires and invalidates on credential changes',async()=>{
  let calls=0;const catalog=createCatalogCache(async()=>({call:++calls}));
  const first=await Promise.all([catalog('a',1,0),catalog('a',1,0)]);assert.equal(calls,1);assert.deepEqual(first[0],first[1]);
  await catalog('a',1,300001);assert.equal(calls,2);await catalog('b',1,300002);assert.equal(calls,3);
});
test('only managed containers are surfaced and agents participate in health alerts',()=>{
  const owned={Labels:{'com.docker.compose.project':STACK_PROJECT}};
  assert.deepEqual(managedContainers([owned,{Labels:{}},{Labels:{...owned.Labels,'org.velora.operations.helper':'true'}}]),[owned]);
  const snapshot={agents:[{id:'a',name:'Host',received_at:'2026-01-01',online:false}],containers:[],endpoints:[],backups:{last:null},host:{disk:null}};
  assert.ok(evaluate(snapshot,{alerts:{backup_max_age_hours:24}}).some(a=>a.key==='agent:a'));
  snapshot.agents[0]={...snapshot.agents[0],online:true,report:{services:[{name:'Calagopus',kind:'http',ok:false}]}};
  assert.ok(evaluate(snapshot,{alerts:{backup_max_age_hours:24}}).some(a=>a.key==='agent:a:Calagopus'));
});
test('agent configuration enforces HTTPS and prevents overriding its report credential',()=>{
  const c={ops_url:'https://ops.example.org',id:'a'.repeat(36),token:'fixture'.repeat(6)};
  assert.equal(config(c).interval_seconds,30);
  assert.throws(()=>config({...c,ops_url:'http://ops.example.org'}));
  assert.throws(()=>config({...c,ops_headers:{Authorization:'override'}}));
  assert.throws(()=>config({...c,services:[{name:'bad\n',kind:'tcp',host:'localhost',port:1}]}));
});
test('Minecraft probe handles fragmented status packets and strips player identities',async()=>{
  const json=Buffer.from(JSON.stringify({players:{online:3,max:20,sample:[{id:'private',name:'private'}]}}));
  const vi=n=>{const a=[];do{let b=n&127;n>>>=7;if(n)b|=128;a.push(b);}while(n);return Buffer.from(a);};
  const body=Buffer.concat([Buffer.from([0]),vi(json.length),json]), packet=Buffer.concat([vi(body.length),body]);
  const server=createServer(socket=>{socket.once('data',()=>{socket.write(packet.subarray(0,2));setTimeout(()=>socket.end(packet.subarray(2)),20);});});
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  try { const port=server.address().port;assert.deepEqual(await tcpProbe({kind:'minecraft',host:'127.0.0.1',port}),{ok:true,players:{online:3,max:20}}); }
  finally {await new Promise(resolve=>server.close(resolve));}
  const result=await probe({name:'Closed service',kind:'tcp',host:'127.0.0.1',port:server.address()?.port??1});assert.equal(result.ok,false);
});
