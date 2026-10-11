// Linux Docker acceptance using the same installation files delivered by Ops.
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, chmod, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'node:https';
import { randomBytes, randomUUID } from 'node:crypto';
import { installerFiles } from '../server/agent-installer.mjs';
const exec = promisify(execFile), dir = await mkdtemp(join(tmpdir(),'velora-docker-agent-'));
const project = `velora-agent-test-${randomBytes(4).toString('hex')}`;
const compose = (...args) => exec('docker',['compose','-p',project,'-f',join(dir,'compose.yaml'),...args],{timeout:60000});
let server, resolveReport;
const received = new Promise(resolve=>{resolveReport=resolve;});
const token = randomBytes(32).toString('base64url'), id = randomUUID();
try {
  await exec('openssl',['req','-x509','-newkey','rsa:2048','-nodes','-keyout',join(dir,'key.pem'),'-out',join(dir,'ca.pem'),'-days','1','-subj','/CN=localhost','-addext','subjectAltName=DNS:localhost,IP:127.0.0.1']);
  await chmod(join(dir,'ca.pem'),0o644);
  server = createServer({key:await readFile(join(dir,'key.pem')),cert:await readFile(join(dir,'ca.pem'))},async(req,res)=>{
    if(req.url==='/health'){res.end('healthy');return;}
    assert.equal(req.url,`/api/agents/${id}/report`);assert.equal(req.headers.authorization,`Bearer ${token}`);
    let body='';for await(const c of req)body+=c;res.end('{}');resolveReport(JSON.parse(body));
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const origin=`https://127.0.0.1:${server.address().port}`;
  const files=installerFiles({ops_url:origin,id,token,services:[{name:'Calagopus fixture',kind:'http',url:`${origin}/health`}],disks:[]},'linux');
  for(const [name,data]of Object.entries(files))await writeFile(join(dir,name),data,{mode:0o600});
  await exec('sudo',['chown','1000:1000',join(dir,'velora-monitor-config.json')]);
  await writeFile(join(dir,'compose.yaml'),files['compose.yaml']+'      - ./ca.pem:/config/ca.pem:ro\n    environment:\n      NODE_EXTRA_CA_CERTS: /config/ca.pem\n');
  await compose('config','--quiet');await compose('up','-d','--build');
  const report=await Promise.race([received,new Promise((_,reject)=>{const timer=setTimeout(()=>reject(Error('No authenticated Docker report')),30000);timer.unref();})]);
  assert.equal(report.platform,'linux');assert.ok(report.memory.total>0);assert.equal(report.services[0].ok,true);
  const {stdout}=await compose('ps','-q','monitor');
  const {stdout:json}=await exec('docker',['inspect',stdout.trim()]);const container=JSON.parse(json)[0];
  assert.equal(container.Config.User,'1000:1000');assert.equal(container.HostConfig.ReadonlyRootfs,true);assert.equal(container.HostConfig.Privileged,false);
  assert.deepEqual(container.HostConfig.CapDrop,['ALL']);assert.equal(container.HostConfig.NetworkMode,'host');
  assert.ok(container.Mounts.every(m=>!m.RW && !m.Source.endsWith('docker.sock')));
  const started=Date.now();await compose('down','--timeout','10');assert.ok(Date.now()-started<10000,'Agent should stop before Docker force-kills it');
  console.log('Docker bundle: authenticated report, health probe, isolation and graceful shutdown passed');
} finally {
  await compose('down','--timeout','5','--rmi','local').catch(()=>{});
  if(server)await new Promise(resolve=>server.close(resolve));
  // Delete only the explicitly created temporary fixture directory.
  assert.ok(resolve(dir).startsWith(resolve(tmpdir())+sep) && dir.includes('velora-docker-agent-'));
  await rm(dir,{recursive:true,force:true});
}
