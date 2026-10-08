import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {generateKeyPairSync,createHash} from 'node:crypto';
import {signProbeJob,verifyProbeJob,validateProbeJob} from '../control-plane/src/probe-job.mjs';
test('signed probe jobs bind immutable image, target agent, validity and exact fields',async()=>{
  const fixture=JSON.parse(readFileSync(new URL('../deployment/contracts/probe-job.fixture.json',import.meta.url)));
  const {envelope,public_key:publicKey,agent_id:agentId,now}=fixture;
  const job=await verifyProbeJob(envelope,publicKey,agentId,now);
  assert.equal(job.service_id,'deployment-probe');
  for(const modified of [
    {...job,environment:'production'}, {...job,action:'SHELL'}, {...job,service_id:'panel'},
    {...job,image:'ghcr.io/veloramcdev/deployment-probe:latest'}, {...job,command:'anything'},
    {...job,expires_at:now}, {...job,expires_at:now+301}, {...job,issued_at:now+6}
  ])assert.throws(()=>validateProbeJob(modified,agentId,now));
  await assert.rejects(verifyProbeJob(envelope,publicKey,'agt_'+'f'.repeat(64),now));
  await assert.rejects(verifyProbeJob(envelope,publicKey,agentId,now+300));
  const payload=Buffer.from(JSON.stringify({...job,expected_commit:'f'.repeat(40)})).toString('base64url');
  await assert.rejects(verifyProbeJob({...envelope,payload},publicKey,agentId,now));
  const {privateKey,publicKey:other}=generateKeyPairSync('ed25519');
  const trusted=other.export({format:'der',type:'spki'}).subarray(-32);
  const signingKey=await crypto.subtle.importKey('pkcs8',privateKey.export({format:'der',type:'pkcs8'}),'Ed25519',false,['sign']);
  const signed=await signProbeJob(job,signingKey,createHash('sha256').update(trusted).digest('hex'),now);
  assert.deepEqual(await verifyProbeJob(signed,trusted.toString('base64url'),agentId,now),job);
  await assert.rejects(verifyProbeJob(signed,publicKey,agentId,now));
});
