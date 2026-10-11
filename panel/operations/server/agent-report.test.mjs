import test from 'node:test';
import assert from 'node:assert/strict';
import { reportOutcome } from '../agent/monitor.mjs';

test('only the Ops JSON acknowledgement confirms delivery, including after a proxy failure', async () => {
  const privateText = 'synthetic-private-login-page';
  for (const response of [
    new Response(`<html>${privateText}</html>`, {headers:{'content-type':'text/html'}}),
    Response.json({ok:false}), Response.json({}),
    new Response('invalid JSON', {headers:{'content-type':'application/json'}}),
    Response.json({ok:true, oversized:'x'.repeat(5000)}),
  ]) {
    const error = await reportOutcome(response);
    assert.match(error, /did not acknowledge/);
    assert.ok(!error.includes(privateText));
  }
  assert.equal(await reportOutcome(Response.json({ok:true})), null);
});

test('redirects, access failures and rate limits explain the next action without exposing response data', async () => {
  const redirect = await reportOutcome(new Response(null, {status:302, headers:{location:'https://login.example.org/?secret=synthetic'}}));
  assert.match(redirect, /redirected by the proxy\/login layer/);
  assert.ok(!redirect.includes('synthetic'));
  assert.match(await reportOutcome(new Response(null,{status:401})), /enrollment credentials/);
  assert.match(await reportOutcome(new Response(null,{status:429})), /wait for the next interval/);
  assert.match(await reportOutcome(new Response(null,{status:500})), /rejected \(500\)/);
});
