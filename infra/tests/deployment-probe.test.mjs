import test from 'node:test';
import assert from 'node:assert/strict';
import { probeServer } from '../deployment/probe/server.mjs';

test('real probe listener reports baked identity and accepts no infrastructure mutations', async t => {
  const identity = { commit: 'a'.repeat(40), version: '0.1.0', build: '123.1' };
  const server = probeServer(identity);
  t.after(() => { server.closeAllConnections(); server.close(); });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  for (const path of ['/health', '/health/live', '/health/ready']) {
    const response = await fetch(origin + path);
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('cache-control'), 'no-store');
    assert.deepEqual(await response.json(), { status: 'ok', service: 'deployment-probe', ...identity });
  }
  const mutate = await fetch(origin + '/health', { method: 'POST' });
  assert.equal(mutate.status, 405);
  assert.equal((await fetch(origin + '/api/v1/deployments')).status, 404);
  assert.equal((await fetch(origin + '/health?commit=another')).status, 404);
});

test('probe fails startup for absent or malformed release identity', () => {
  for (const identity of [{}, {commit:'main',version:'0.1.0',build:'123'}, {commit:'a'.repeat(40),version:'latest',build:'123'}, {commit:'a'.repeat(40),version:'0.1.0',build:'invalid\nheader'}]) {
    assert.throws(() => probeServer(identity), /Invalid immutable/);
  }
});
