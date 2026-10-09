import {test} from 'node:test';
import assert from 'node:assert/strict';
import {probeRequest} from './probe-plan.mjs';
const candidate={candidate_id:'rc_'+'a'.repeat(64),service_id:'deployment-probe',repository:'veloramcdev/velora-launcher',git_ref:'refs/heads/main',git_sha:'b'.repeat(40),artifact_type:'oci',build_status:'PASSED',test_status:'PASSED',sha256:'c'.repeat(64),artifact_uri:'ghcr.io/veloramcdev/deployment-probe@sha256:'+'c'.repeat(64),oci_digest:'sha256:'+'c'.repeat(64)};
const agent={id:'agt_'+'d'.repeat(64),status:'ACTIVE',heartbeat_at:1000};
test('only a tested immutable monorepo probe and fresh active machine produce a Development request',()=>{
  assert.deepEqual(probeRequest(candidate,agent,1001),{schema:1,candidate_id:candidate.candidate_id,agent_id:agent.id});
  for(const patch of [{service_id:'velora-panel'},{repository:'veloramcdev/infra'},{git_ref:'refs/heads/beta'},{test_status:'FAILED'},{artifact_uri:'ghcr.io/veloramcdev/deployment-probe:latest'},{sha256:'e'.repeat(64)},{candidate_id:'invalid'}])assert.throws(()=>probeRequest({...candidate,...patch},agent,1001),/tested monorepo/);
  for(const patch of [{status:'REVOKED'},{heartbeat_at:881},{heartbeat_at:1100},{id:'invalid'}])assert.throws(()=>probeRequest(candidate,{...agent,...patch},1001),/recent heartbeat/);
});
