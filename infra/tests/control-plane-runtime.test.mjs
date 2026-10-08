import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createTestHarness } from 'wrangler';
import { setTimeout } from 'node:timers/promises';
import { randomUUID } from 'node:crypto';

test('actual local Workers runtime applies both migrations and executes bounded binding Workflow with cleanup', {timeout:60000}, async () => {
  const server = createTestHarness({workers:[{configPath:'control-plane/wrangler.jsonc',vars:{BUILD_SHA:'a'.repeat(40)}}]});
  try {
    await server.listen();
    const worker = server.getWorker();
    for (const binding of ['HARNESS_DB','IDENTITY_DB']) {
      await worker.applyD1Migrations(binding);
      await worker.applyD1Migrations(binding); // Already-applied migrations are safe.
    }
    const response = await server.fetch('/health');
    assert.equal(response.status,200);
    assert.equal((await response.json()).commit_sha,'a'.repeat(40));
    const denied = await server.fetch('/api/v1/bootstrap/bindings-check',{method:'POST'});
    assert.equal(denied.status,403);
    assert.equal((await denied.json()).error,'BOOTSTRAP_DISABLED');
    const env = await worker.getEnv();
    const id = randomUUID();
    await env.HARNESS_DB.prepare("INSERT INTO bootstrap_checks(id,environment,status) VALUES (?,'development','RESERVED')").bind(id).run();
    const instance = await env.BINDING_CHECK.create({id:'bindings-' + id,params:{id}});
    let state;
    for (let attempt=0;attempt<60;attempt++) {
      state = await instance.status();
      if (['complete','errored','terminated'].includes(state.status)) break;
      await setTimeout(250);
    }
    assert.equal(state.status,'complete',JSON.stringify(state));
    assert.deepEqual(state.output,{harness:'PASS',identity:'PASS',r2:'PASS',workflow:'PASS',cleanup:'PASS',deployment_performed:false});
    assert.equal(await env.OPERATIONS.head('bootstrap/' + id + '.json'),null);
    assert.equal(await env.IDENTITY_DB.prepare('SELECT id FROM bootstrap_checks WHERE id = ?').bind(id).first(),null);
    assert.equal((await env.HARNESS_DB.prepare('SELECT status FROM bootstrap_checks WHERE id = ?').bind(id).first()).status,'SUCCEEDED');
    assert.equal((await env.OPERATIONS.list()).objects.length,0);
    await worker.applyD1Migrations('HARNESS_DB');
    assert.equal((await env.HARNESS_DB.prepare('SELECT status FROM bootstrap_checks WHERE id = ?').bind(id).first()).status,'SUCCEEDED');
    await env.IDENTITY_DB.prepare("UPDATE schema_metadata SET purpose='wrong-database' WHERE id = 1").run();
    const failedId = randomUUID();
    await env.HARNESS_DB.prepare("INSERT INTO bootstrap_checks(id,environment,status) VALUES (?,'development','RESERVED')").bind(failedId).run();
    const failed = await env.BINDING_CHECK.create({id:'bindings-' + failedId,params:{id:failedId}});
    for (let attempt=0;attempt<60;attempt++) {
      state = await failed.status();
      if (['complete','errored','terminated'].includes(state.status)) break;
      await setTimeout(250);
    }
    assert.equal(state.status,'errored');
    assert.equal((await env.HARNESS_DB.prepare('SELECT status FROM bootstrap_checks WHERE id = ?').bind(failedId).first()).status,'FAILED');
    assert.equal((await env.OPERATIONS.list()).objects.length,0);
  } finally { await server.close(); }
});
