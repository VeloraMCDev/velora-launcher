import test from 'node:test';import assert from 'node:assert/strict';
import {ControlClient,SessionUnavailable} from './client.mjs';
test('browser transport uses its global receiver rather than the client instance',async()=>{
  const client=new ControlClient(function(){assert.equal(this,globalThis);return Promise.resolve(Response.json({items:[]}));});
  assert.deepEqual(await client.request('/api/v1/repositories'),{items:[]});
});
test('permission loss invalidates concurrent reads and clears session through the host callback',async()=>{
  let release,cleared=0;
  const client=new ControlClient(path=>path.endsWith('agents')?Promise.resolve(new Response('{}',{status:403})):new Promise(resolve=>release=resolve),()=>cleared++);
  const pending=client.request('/api/v1/repositories');const denied=client.request('/api/v1/agents');
  await assert.rejects(denied,SessionUnavailable);release(Response.json({items:[{full_name:'synthetic/repository'}]}));
  await assert.rejects(pending,SessionUnavailable);assert.equal(cleared,1);
});
test('operator mutations stay same-origin, use fresh action IDs and never follow sign-in redirects',async()=>{
  const requests=[];const client=new ControlClient(async(path,init)=>{requests.push({path,init});return Response.json({accepted:true});});
  await client.request('/api/v1/agents/enrollment-tokens',{schema:1});await client.request('/api/v1/agents/enrollment-tokens',{schema:1});
  assert.notEqual(requests[0].init.headers['idempotency-key'],requests[1].init.headers['idempotency-key']);
  for(const {init} of requests){assert.equal(init.redirect,'error');assert.equal(init.credentials,'same-origin');assert.equal(init.cache,'no-store');assert.equal(init.headers['x-velora-action'],'1');}
  await assert.rejects(client.request('https://attacker.example.com/api/v1/agents'),/Invalid API path/);
});
