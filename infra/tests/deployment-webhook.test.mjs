import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHmac } from 'node:crypto';
import { verifyGitHubDelivery } from '../control-plane/src/github-webhook.mjs';

test('GitHub webhook verifies original bounded bytes before parsing and exposes stable delivery digest',async()=>{
  const fixtureSecret='synthetic-test-fixture-not-a-real-webhook-secret';
  const body='{ "action": "completed", "repository": {"id":123} }';
  const signature='sha256='+createHmac('sha256',fixtureSecret).update(body).digest('hex');
  const headers={'content-type':'application/json','x-hub-signature-256':signature,'x-github-delivery':'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','x-github-event':'workflow_run'};
  const request=(text=body,extra={})=>new Request('https://control.example.com/api/v1/github/webhooks',{method:'POST',headers:{...headers,...extra},body:text});
  const result=await verifyGitHubDelivery(request(),fixtureSecret);
  assert.equal(result.payload.repository.id,123);
  assert.equal(result.delivery,headers['x-github-delivery']);
  assert.match(result.body_sha256,/^[a-f0-9]{64}$/);
  assert.equal((await verifyGitHubDelivery(request(),fixtureSecret)).body_sha256,result.body_sha256);
  await assert.rejects(verifyGitHubDelivery(request(JSON.stringify(JSON.parse(body))),fixtureSecret));
  await assert.rejects(verifyGitHubDelivery(request(),undefined));
  await assert.rejects(verifyGitHubDelivery(request(),fixtureSecret+'wrong'));
  for(const extra of [{'x-hub-signature-256':'sha256='+'0'.repeat(64)},{'x-github-delivery':'not-a-delivery-id'},{'x-github-event':'push'},{'content-encoding':'gzip'},{'content-type':'text/plain'}])await assert.rejects(verifyGitHubDelivery(request(body,extra),fixtureSecret));
  await assert.rejects(verifyGitHubDelivery(request('x'.repeat(65537)),fixtureSecret),/WEBHOOK_TOO_LARGE/);
  const invalid='{invalid}';const signedInvalid='sha256='+createHmac('sha256',fixtureSecret).update(invalid).digest('hex');
  await assert.rejects(verifyGitHubDelivery(request(invalid,{'x-hub-signature-256':signedInvalid}),fixtureSecret),/INVALID_WEBHOOK_JSON/);
});
