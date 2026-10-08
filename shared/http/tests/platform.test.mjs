import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {once} from 'node:events';
import {createPlatformClient} from '../platform.mjs';
import {ApiError, ApiVersionError} from '../client.mjs';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';

test('reviewed output hashes and actual source route selection are complete', () => {
  const provenance = JSON.parse(readFileSync(new URL('../PLATFORM_PROVENANCE.json', import.meta.url)));
  for (const file of provenance.outputs) {
    const bytes = readFileSync(new URL('../' + file.path, import.meta.url), 'utf8').replaceAll('\r\n', '\n');
    assert.equal(createHash('sha256').update(bytes).digest('hex'), file.sha256_lf);
  }
  const catalog = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url)));
  const expected = routes.map(([,path,method]) => [method,path.replace('synthetic%20instance', '{id}')]);
  expected.push(['POST', '/api/v1/account/skin']);
  assert.deepEqual(expected.sort(), catalog.routes.map(([method,path]) => [method,catalog.prefix+path]).sort());
});

const routes = [
  [c => c.launcher.manifest(), '/api/v1/launcher/manifest', 'GET', undefined],
  [c => c.launcher.instanceManifest('synthetic instance'), '/api/v1/launcher/instances/synthetic%20instance', 'GET', undefined],
  [c => c.launcher.event({instance_id: 'synthetic', kind: 'launch'}), '/api/v1/launcher/events', 'POST', {instance_id: 'synthetic', kind: 'launch'}],
  [c => c.launcher.latestUpdate(), '/api/v1/launcher/update', 'GET', undefined],
  [c => c.auth.login({username: 'synthetic', password: 'example'}), '/api/v1/auth/login', 'POST', {username: 'synthetic', password: 'example'}],
  [c => c.auth.register({username: 'synthetic', password: 'example', email: null}), '/api/v1/auth/register', 'POST', {username: 'synthetic', password: 'example', email: null}],
  [c => c.auth.me(), '/api/v1/auth/me', 'GET', undefined],
  [c => c.auth.forgotPassword('test@example.invalid'), '/api/v1/auth/forgot-password', 'POST', {email: 'test@example.invalid'}],
  [c => c.auth.resetPassword('synthetic', 'example'), '/api/v1/auth/reset-password', 'POST', {token: 'synthetic', password: 'example'}],
  [c => c.account.profile(), '/api/v1/account/profile', 'GET', undefined],
  [c => c.account.setUsername('synthetic', 'example'), '/api/v1/account/username', 'PUT', {username: 'synthetic', password: 'example'}],
  [c => c.account.deleteSkin(), '/api/v1/account/skin', 'DELETE', undefined],
  [c => c.account.setSkinModel('slim'), '/api/v1/account/skin/model', 'PUT', {model: 'slim'}],
  [c => c.account.setCape(null), '/api/v1/account/cape', 'PUT', {cape_id: null}],
];

test('endpoint methods, JSON and live identity/scope reach a real HTTP listener', async t => {
  const received = [];
  const server = createServer(async (req, res) => {
    const chunks = [];
    for await (const bytes of req) chunks.push(bytes);
    received.push({url: req.url, method: req.method, headers: req.headers, body: Buffer.concat(chunks).toString()});
    res.setHeader('Content-Type', 'application/json');
    res.end(JSON.stringify({api_version: 1, extra: {opaque: ['preserved']}}));
  }).listen(0, '127.0.0.1');
  t.after(() => {server.closeAllConnections(); server.close();});
  await once(server, 'listening');
  let token = 'first', scope = 'first';
  const client = createPlatformClient({baseUrl: `http://127.0.0.1:${server.address().port}/prefix`, getToken: () => token, getInstance: () => scope});
  for (const [invoke, path, method, body] of routes) {
    token += 'x'; scope += 'x';
    const response = await invoke(client);
    assert.deepEqual(response.extra, {opaque: ['preserved']});
    const call = received.at(-1);
    assert.equal(call.url, '/prefix' + path);
    assert.equal(call.method, method);
    assert.equal(call.headers.authorization, 'Bearer ' + token);
    assert.equal(call.headers['x-scopenet-instance'], scope);
    assert.deepEqual(call.body ? JSON.parse(call.body) : undefined, body);
  }
});

test('skin upload retains browser boundaries, original bytes and supplied model', async () => {
  let called = false;
  const client = createPlatformClient({fetch: async (url, init) => {
    called = true;
    assert.equal(url, '/api/v1/account/skin');
    assert.equal(init.method, 'POST');
    assert.equal(new Headers(init.headers).has('Content-Type'), false);
    assert.ok(init.body instanceof FormData);
    assert.equal(init.body.get('model'), 'slim');
    assert.deepEqual(new Uint8Array(await init.body.get('file').arrayBuffer()), new Uint8Array([0, 255, 10]));
    return new Response('{"skin_model":"slim","unknown":7}');
  }});
  assert.deepEqual(await client.account.uploadSkin(new Blob([new Uint8Array([0, 255, 10])]), 'slim'), {skin_model:'slim', unknown:7});
  assert.ok(called);
});

test('path values cannot traverse or introduce query/hash routing', async () => {
  const urls = [];
  const client = createPlatformClient({fetch: async url => {urls.push(url); return new Response('{}');}});
  for (const id of ['.', '..', '', null, 7]) assert.throws(() => client.launcher.instanceManifest(id), TypeError);
  for (const id of ['../other?token=x#fragment', 'a\\b', '%2e%2e', '汉字']) {
    await client.launcher.instanceManifest(id);
    assert.equal(urls.at(-1), '/api/v1/launcher/instances/' + encodeURIComponent(id));
  }
});

test('manifest incompatibility, null update and authority failures remain distinct', async () => {
  await assert.rejects(createPlatformClient({fetch: async () => new Response('{"api_version":2}')}).launcher.manifest(), ApiVersionError);
  assert.equal(await createPlatformClient({fetch: async () => new Response('null')}).launcher.latestUpdate(), null);
  let loggedOut = 0;
  const client = createPlatformClient({getToken: () => 'synthetic', onUnauthorized: () => loggedOut++, fetch: async () => new Response('{"error":"expired"}', {status:401})});
  await assert.rejects(client.account.setCape(7), error => error instanceof ApiError && error.status === 401 && error.message === 'expired');
  assert.equal(loggedOut, 1);
});

test('JSON and multipart endpoints forward external cancellation', async () => {
  const client = createPlatformClient({fetch: async (_url, init) => new Promise((_resolve, reject) => {
    if (init.signal.aborted) reject(init.signal.reason);
    else init.signal.addEventListener('abort', () => reject(init.signal.reason), {once:true});
  })});
  const cancel = new AbortController();
  const reason = new Error('synthetic cancellation');
  const pending = client.auth.me({signal: cancel.signal});
  cancel.abort(reason);
  await assert.rejects(pending, error => error === reason);
  const already = new AbortController(); already.abort(reason);
  await assert.rejects(client.account.uploadSkin(new Blob(), 'classic', {signal:already.signal}), error => error === reason);
});
