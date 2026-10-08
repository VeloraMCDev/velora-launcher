import { test } from 'node:test';
import assert from 'node:assert/strict';
import { generateKeyPair, SignJWT } from 'jose';
import { verifyGitHubJob, authorizeGitHubArtifact } from '../control-plane/src/github-oidc.mjs';

test('GitHub OIDC requires genuine signature and exact immutable repository, workflow, source and run policy', async () => {
  const {privateKey,publicKey} = await generateKeyPair('RS256');
  const now=Math.floor(Date.now()/1000), sha='a'.repeat(40);
  const policy={repository:'Example/infra',repository_id:'123',owner_id:'456',ref:'refs/heads/main',workflow_path:'.github/workflows/release.yml',workflow_sha:sha,subject_format:'immutable',enabled:1};
  const claims={repository:policy.repository,repository_id:'123',repository_owner:'Example',repository_owner_id:'456',sha,ref:policy.ref,run_id:'789',run_attempt:'1',workflow_ref:'Example/infra/.github/workflows/release.yml@refs/heads/main',workflow_sha:sha,event_name:'push',runner_environment:'github-hosted'};
  const artifact={repository:policy.repository,git_sha:sha,git_ref:policy.ref,build_run_id:'789'};
  const sign=async (overrides={},subject='repo:Example@456/infra@123:ref:refs/heads/main') => new SignJWT({...claims,...overrides}).setProtectedHeader({alg:'RS256',typ:'JWT'}).setIssuer('https://token.actions.githubusercontent.com').setAudience('velora-control-plane').setSubject(subject).setJti('synthetic-job-token').setIssuedAt(now).setNotBefore(now-1).setExpirationTime(now+300).sign(privateKey);
  const request=token=>new Request('https://control.example.com/api/v1/ci/artifacts/register',{headers:{authorization:'Bearer '+token}});
  const verified=await verifyGitHubJob(request(await sign()),'velora-control-plane',publicKey);
  assert.deepEqual(authorizeGitHubArtifact(verified,policy,artifact),{actor:'github:123',run_id:'789'});
  const self=await verifyGitHubJob(request(await sign({job_workflow_ref:claims.workflow_ref,job_workflow_sha:sha})),'velora-control-plane',publicKey);
  authorizeGitHubArtifact(self,policy,artifact);
  for(const patch of [{job_workflow_ref:claims.workflow_ref},{job_workflow_ref:claims.workflow_ref,job_workflow_sha:'b'.repeat(40)},{job_workflow_sha:sha}])await assert.rejects(verifyGitHubJob(request(await sign(patch)),'velora-control-plane',publicKey));
  for(const patch of [{repository_id:'124'},{repository_owner_id:'457'},{ref:'refs/heads/other'},{sha:'b'.repeat(40)},{run_id:'790'},{workflow_sha:'b'.repeat(40)},{workflow_ref:'Example/infra/.github/workflows/untrusted.yml@refs/heads/main'}]) {
    const job=await verifyGitHubJob(request(await sign(patch)),'velora-control-plane',publicKey);
    assert.throws(()=>authorizeGitHubArtifact(job,policy,artifact));
  }
  for(const patch of [{event_name:'pull_request'},{event_name:'pull_request_target'},{runner_environment:'self-hosted'},{environment:'production'},{job_workflow_ref:'Example/other/.github/workflows/release.yml@refs/heads/main'},{run_id:789}]) await assert.rejects(verifyGitHubJob(request(await sign(patch)),'velora-control-plane',publicKey));
  assert.throws(()=>authorizeGitHubArtifact(verified,{...policy,enabled:0},artifact));
  const wrongSubject=await verifyGitHubJob(request(await sign({},'repo:Example/infra:ref:refs/heads/main')),'velora-control-plane',publicKey);
  assert.throws(()=>authorizeGitHubArtifact(wrongSubject,policy,artifact));
  authorizeGitHubArtifact(wrongSubject,{...policy,subject_format:'legacy'},artifact); // IDs remain mandatory.
  const wrongKeys=await generateKeyPair('RS256');
  await assert.rejects(verifyGitHubJob(request(await sign()),'velora-control-plane',wrongKeys.publicKey));
  await assert.rejects(verifyGitHubJob(request(await sign()),'other-audience',publicKey));
  await assert.rejects(verifyGitHubJob(request(await sign()),'',publicKey));
  await assert.rejects(verifyGitHubJob(new Request('https://control.example.com/'),'velora-control-plane',publicKey));
});
