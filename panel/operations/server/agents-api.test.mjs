import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
process.env.OPS_DATA_DIR=mkdtempSync(join(tmpdir(),'velora-agent-test-'));
const { server }=await import('./index.mjs');
const { _test, COOKIE }=await import('./auth.mjs');
test('HTTP enrollment requires admin session and CSRF; report tokens cannot administer Ops',async()=>{
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const base=`http://127.0.0.1:${server.address().port}`;
  const id='synthetic-session',csrf='synthetic-csrf';
  _test.sessions.set(id,{username:'Synthetic admin',csrf,token:'synthetic-panel',checked:Date.now(),expires:Date.now()+60000});
  const admin={cookie:`${COOKIE}=${id}`,'x-csrf-token':csrf,'content-type':'application/json'};
  const request=(path,options={})=>fetch(base+path,options);
  try {
    for(const path of ['/api/agents','/api/agents/download','/api/artifacts','/api/artifacts/9/download']) assert.equal((await request(path)).status,401);
    assert.equal((await request('/api/agents',{method:'POST',headers:{cookie:admin.cookie},body:'{}'})).status,403);
    const response=await request('/api/agents',{method:'POST',headers:admin,body:JSON.stringify({name:'Synthetic host'})});assert.equal(response.status,201);
    const enrolled=await response.json(), bearer={authorization:`Bearer ${enrolled.token}`,'content-type':'application/json'};
    assert.equal((await request('/api/agents',{headers:bearer})).status,401);
    assert.equal((await request('/api/actions/redeploy',{method:'POST',headers:bearer})).status,401);
    const path=`/api/agents/${enrolled.id}/report`;
    assert.equal((await request(path,{method:'POST',body:'{}'})).status,401);
    const report={platform:'linux',version:'1',uptime:1,cpu:null,memory:{total:2,used:1},services:[],disks:[]};
    assert.equal((await request(path,{method:'POST',headers:bearer,body:JSON.stringify(report)})).status,200);
    const list=await(await request('/api/agents',{headers:admin})).json();assert.equal(list[0].online,true);assert.ok(!JSON.stringify(list).includes(enrolled.token));
    const download=await request('/api/agents/download',{headers:admin});assert.equal(download.status,200);assert.match(await download.text(),/Monitoring report/);
    assert.equal((await request(`/api/agents/${enrolled.id}`,{method:'DELETE',headers:admin})).status,200);
    assert.equal((await request(path,{method:'POST',headers:bearer,body:JSON.stringify(report)})).status,401);
  } finally { _test.sessions.delete(id);await new Promise(resolve=>server.close(resolve)); }
});
test.after(()=>rmSync(process.env.OPS_DATA_DIR,{recursive:true,force:true}));
