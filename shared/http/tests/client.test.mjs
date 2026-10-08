import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHttpClient, ApiError, ApiVersionError, assertApiVersion} from '../client.mjs';
import {createLegacy} from './legacy-fixture.mjs';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';

test('reviewed runtime, declarations and frozen fixture match provenance', () => {
  const provenance = JSON.parse(readFileSync(new URL('../PROVENANCE.json', import.meta.url), 'utf8'));
  for (const file of provenance.outputs) {
    const bytes = readFileSync(new URL('../' + file.path, import.meta.url), 'utf8').replaceAll('\r\n', '\n');
    assert.equal(createHash('sha256').update(bytes).digest('hex'), file.sha256_lf);
  }
});

const scenarios = [
  ['json GET', 'get', undefined, 200, '{"id":7,"extra":null}'],
  ['text GET', 'get', undefined, 200, 'synthetic text'],
  ['empty GET', 'get', undefined, 204, null],
  ['POST default', 'post', undefined, 200, '{}'],
  ['POST null default', 'post', null, 200, '{}'],
  ['PUT JSON', 'put', {name:'synthetic'}, 200, '{}'],
  ['PATCH null', 'patch', null, 200, '{}'],
  ['DELETE', 'del', undefined, 204, null],
  ['JSON denial', 'get', undefined, 403, '{"error":"denied","details":null}'],
  ['plain denial', 'get', undefined, 502, 'unavailable'],
  ['signed-in unauthorized', 'get', undefined, 401, '{"error":"expired"}'],
];
for (const [name, method, body, status, payload] of scenarios) {
  test(`frozen compatibility: ${name}`, async () => {
    const attempts = [];
    for (const legacy of [true,false]) {
      const route = {instanceId:'synthetic-instance'}, session = {token:'synthetic-token'};
      let loggedOut = 0;
      const calls = [];
      const fetch = async (path, options) => {
        calls.push({path, method:options.method, headers:options.headers, body:options.body});
        return new Response(payload, {status});
      };
      const logout = () => { loggedOut++; session.token = null; };
      const client = legacy ? createLegacy(route,session,logout,fetch) : createHttpClient({
        fetch, getToken:()=>session.token, getInstance:()=>route.instanceId, onUnauthorized:logout, timeoutMs:0,
      });
      let result;
      try { result = {value:await client[method]('/api/synthetic?instance=seven',body)}; }
      catch (error) { result = {error:error.message,status:error.status,name:error.name}; }
      attempts.push({calls,result,loggedOut});
    }
    assert.deepEqual(attempts[0],attempts[1]);
  });
}

test('multipart precedence and direct JSON null retain frozen behavior', async () => {
  for (const options of [{body:null},{form:new FormData(),body:{ignored:true}}]) {
    const calls = [];
    const fetch = async (_, init) => { calls.push(init); return new Response('{}'); };
    await createLegacy({}, {}, ()=>{}, fetch).api('/api/synthetic',options);
    await createHttpClient({fetch,timeoutMs:0}).api('/api/synthetic',options);
    assert.deepEqual(calls[0],calls[1]);
    if (options.form) { assert.equal(calls[1].body,options.form); assert.equal(calls[1].headers['Content-Type'],undefined); }
  }
});

test('session and scope are live; unauthorized callback uses current session and preserves network errors', async () => {
  const session = {token:'first'}, route = {instanceId:'one'};
  let unauthorized = 0;
  const calls = [];
  const fetch = async (_,init) => { calls.push(init.headers); session.token=null; return new Response('{}',{status:401}); };
  const client = createHttpClient({fetch,getToken:()=>session.token,getInstance:()=>route.instanceId,onUnauthorized:()=>unauthorized++,timeoutMs:0});
  await assert.rejects(client.get('/api/synthetic'), error=>error instanceof ApiError && error.status===401);
  assert.equal(unauthorized,0);
  session.token='second'; route.instanceId='two';
  await assert.rejects(client.get('/api/synthetic'));
  assert.equal(calls[1].Authorization,'Bearer second'); assert.equal(calls[1]['X-SCOPENET-Instance'],'two');
  const network = new TypeError('synthetic network failure');
  await assert.rejects(createHttpClient({fetch:async()=>{throw network},timeoutMs:0}).get('/api/synthetic'), error=>error===network);
});

test('operator prefix and optional Velora scope preserve request targets without credential-selected origins', async () => {
  const calls = [];
  const client = createHttpClient({baseUrl:'https://example.invalid/velora/',instanceHeader:'X-Velora-Instance',getInstance:()=> 'synthetic',
    fetch:async (url,options)=>{calls.push([url,options]);return new Response('{}')},timeoutMs:0});
  await client.get('/api/example?x=a%2Fb&x=two');
  assert.equal(calls[0][0],'https://example.invalid/velora/api/example?x=a%2Fb&x=two');
  assert.equal(calls[0][1].headers['X-Velora-Instance'],'synthetic');
  for (const path of ['https://other.invalid/api','//other.invalid/api','/\\other.invalid/api','/api\nexample']) await assert.rejects(client.get(path),TypeError);
  assert.equal(calls.length,1);
  assert.throws(()=>createHttpClient({baseUrl:'https://user:synthetic-password@example.invalid'}),error=>!error.message.includes('synthetic-password'));
});

test('timeout and external cancellation preserve abort errors through body consumption', async () => {
  const hanging = async (_,options) => ({ok:true,status:200,text:()=>new Promise((_,reject)=>{
    if (options.signal.aborted) reject(options.signal.reason);
    else options.signal.addEventListener('abort',()=>reject(options.signal.reason),{once:true});
  })});
  await assert.rejects(createHttpClient({fetch:hanging,timeoutMs:15}).get('/api/synthetic'),error=>error.name==='TimeoutError');
  const controller = new AbortController();
  const pending = createHttpClient({fetch:hanging,timeoutMs:0}).api('/api/synthetic',{signal:controller.signal});
  controller.abort();
  await assert.rejects(pending,error=>error.name==='AbortError');
  for (const timeoutMs of [-1,NaN,Infinity]) assert.throws(()=>createHttpClient({timeoutMs}),TypeError);
});

test('explicit manifest negotiation retains fields and rejects unsupported versions without invented headers', async () => {
  const manifest = {api_version:1,unknown:{synthetic:true}};
  const calls = [];
  const client = createHttpClient({fetch:async(url,options)=>{calls.push([url,options]);return new Response(JSON.stringify(manifest))},timeoutMs:0});
  assert.deepEqual(await client.manifest(),manifest);
  assert.equal(calls[0][0],'/api/v1/launcher/manifest'); assert.deepEqual(calls[0][1].headers,{});
  for (const value of [{api_version:2},{api_version:'1'},{}]) assert.throws(()=>assertApiVersion(value),ApiVersionError);
});
