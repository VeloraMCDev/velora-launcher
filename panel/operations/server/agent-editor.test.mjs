import test from 'node:test';
import assert from 'node:assert/strict';
import { loadHostConfiguration, serviceTargets, updateHostConfiguration } from '../web/src/lib/agent-editor.ts';
import { config } from '../agent/monitor.mjs';
const fixture=()=>({id:'a'.repeat(36),token:'synthetic'.repeat(5),ops_url:'https://ops.example.org',interval_seconds:45,
  ops_headers:{'P-Access-Token':'synthetic-access'},disks:[{name:'Data',path:'/data'}],
  services:[{name:'HTTP',kind:'http',url:'http://127.0.0.1:8080/health',headers:{'X-Service-Key':'synthetic-probe'}},
    {name:'SMP',kind:'minecraft',host:'127.0.0.1',port:25565,virtual_host:'smp.example.org'}]});
test('editing probes preserves host enrollment, SSO/probe headers, disks and interval and remains agent-compatible',()=>{
  const input=fixture(),c=loadHostConfiguration(JSON.stringify(input),input.id),targets=serviceTargets(c.services);
  targets[0].url='http://127.0.0.1:8081/health';targets[1].port=25566;
  targets.push({name:'Extra',kind:'tcp',host:'127.0.0.1',port:9000,url:'',virtual_host:''});
  const next=updateHostConfiguration(c,targets);config(next);
  for(const field of ['id','token','ops_url','ops_headers','disks','interval_seconds'])assert.deepEqual(next[field],input[field]);
  assert.deepEqual(next.services[0].headers,input.services[0].headers);assert.equal(next.services[1].virtual_host,'smp.example.org');
  assert.equal(input.services[1].port,25565);
  targets.splice(0,1);assert.equal(updateHostConfiguration(c,targets).services.length,2);
});
test('wrong-host imports and invalid probe edits are rejected before a configuration is downloaded',()=>{
  const c=fixture();assert.throws(()=>loadHostConfiguration(JSON.stringify(c),'another-host'),/agent ID/);
  for(const change of [t=>t[1].port=0,t=>t[1].name='HTTP',t=>t[0].url='file:///secret',t=>t[0].url='https://user:pass@example.org']){
    const targets=serviceTargets(c.services);change(targets);assert.throws(()=>updateHostConfiguration(c,targets));
  }
});
