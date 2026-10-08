import test from 'node:test';import assert from 'node:assert/strict';
import {createTestHarness} from 'wrangler';
import {serveOperatorAssets} from '../control-plane/src/operator-assets.ts';
test('operator assets require admitted Development role and exact host before touching static assets',{timeout:60000},async()=>{
  const server=createTestHarness({workers:[{configPath:'control-plane/wrangler.jsonc'}]});
  try{
    await server.listen();const worker=server.getWorker();await worker.applyD1Migrations('IDENTITY_DB');
    let fetches=0;
    const env={...await worker.getEnv(),OPERATOR_API_HOST:'operator.example.com',ACCESS_TEAM_DOMAIN:'https://fixture.cloudflareaccess.com',ASSETS:{fetch:async()=>{fetches++;return new Response('<h1>fixture</h1>',{headers:{'content-type':'text/html'}});}}};
    const verify=async()=>({subject:'fixture-subject'}),request=path=>new Request('https://operator.example.com'+path);
    assert.equal((await serveOperatorAssets(request('/'),env,verify)).status,403);assert.equal(fetches,0);
    await env.IDENTITY_DB.batch([
      env.IDENTITY_DB.prepare("INSERT INTO operator_principals VALUES ('fixture',?,'fixture-subject','ACTIVE')").bind(env.ACCESS_TEAM_DOMAIN),
      env.IDENTITY_DB.prepare("INSERT INTO operator_roles VALUES ('viewer','Synthetic viewer')"),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_permissions VALUES ('viewer','environment.read')"),
      env.IDENTITY_DB.prepare("INSERT INTO operator_role_bindings VALUES ('fixture','viewer','development')")]);
    const response=await serveOperatorAssets(request('/'),env,verify);assert.equal(response.status,200);assert.equal(fetches,1);
    assert.equal(response.headers.get('cache-control'),'no-store');assert.match(response.headers.get('content-security-policy'),/frame-ancestors 'none'/);
    assert.equal((await serveOperatorAssets(new Request('https://machine.example.com/'),env,verify)).status,404);
    assert.equal((await serveOperatorAssets(request('/artifact-manifest.json'),env,verify)).status,404);
    assert.equal((await serveOperatorAssets(request('/'),{...env,ENVIRONMENT:'production'},verify)).status,404);
    assert.equal((await serveOperatorAssets(request('/'),env,async()=>{throw Error('Invalid signature');})).status,403);assert.equal(fetches,1);
  }finally{await server.close();}
});
