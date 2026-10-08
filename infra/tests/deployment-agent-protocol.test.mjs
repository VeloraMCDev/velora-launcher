import test from 'node:test';
import assert from 'node:assert/strict';
import {generateKeyPairSync,sign,randomUUID} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {agentHash,encodeBase64Url,canonicalAgentRequest,readAgentRequest,verifyAgentSignature} from '../control-plane/src/agent-protocol.mjs';

test('Worker verifies the shared Rust/Node canonical signature vector',async()=>{
  const fixture=JSON.parse(readFileSync(new URL('../deployment/contracts/agent-request.fixture.json',import.meta.url),'utf8'));
  const request=new Request('https://agent.example.com'+fixture.path,{method:'POST',headers:{'content-type':'application/json','velora-agent-id':'agt_'+fixture.key_id,'velora-key-id':fixture.key_id,'velora-timestamp':fixture.timestamp,'velora-nonce':fixture.nonce,'velora-body-sha256':fixture.body_sha256,'velora-signature':fixture.signature},body:fixture.body});
  const envelope=await readAgentRequest(request,Number(fixture.timestamp));
  assert.equal(envelope.canonical,fixture.canonical);await verifyAgentSignature(envelope,fixture.public_key);
});

test('agent signatures bind exact body/path/time/nonce/key and reject tampering, skew and ambiguous encodings',async()=>{
  const {privateKey,publicKey}=generateKeyPairSync('ed25519');
  const raw=publicKey.export({format:'der',type:'spki'}).subarray(-32),keyId=await agentHash(raw),publicValue=encodeBase64Url(raw);
  const timestamp=String(Math.floor(Date.now()/1000)),nonce=randomUUID(),body=JSON.stringify({schema:1,capabilities:['REPORT_HEARTBEAT']});
  const checksum=await agentHash(new TextEncoder().encode(body)),path='/api/v1/agents/heartbeat';
  const canonical=canonicalAgentRequest(path,timestamp,nonce,checksum);
  const signature=encodeBase64Url(sign(null,Buffer.from(canonical),privateKey));
  const headers={'content-type':'application/json','velora-timestamp':timestamp,'velora-nonce':nonce,'velora-body-sha256':checksum,'velora-signature':signature,'velora-key-id':keyId,'velora-agent-id':'agt_'+keyId};
  const request=(changes={},value=body,suffix='')=>new Request('https://agent.example.com'+path+suffix,{method:'POST',headers:{...headers,...changes},body:value});
  const envelope=await readAgentRequest(request());await verifyAgentSignature(envelope,publicValue);
  assert.deepEqual(envelope.body,{schema:1,capabilities:['REPORT_HEARTBEAT']});
  for(const invalid of [request({},body+' '),request({'velora-timestamp':String(Number(timestamp)-121)}),request({'velora-nonce':'invalid'}),request({'velora-signature':signature+'='}),request({'velora-agent-id':'agt_'+'a'.repeat(64)}),request({'content-encoding':'gzip'}),request({},body,'?unexpected=1'),request({},'x'.repeat(4097))])await assert.rejects(readAgentRequest(invalid));
  await assert.rejects(verifyAgentSignature({...envelope,canonical:envelope.canonical.replace('heartbeat','enroll')},publicValue));
  await assert.rejects(verifyAgentSignature({...envelope,canonical:envelope.canonical.replace(nonce,randomUUID())},publicValue));
  const wrong=generateKeyPairSync('ed25519').publicKey.export({format:'der',type:'spki'}).subarray(-32);
  await assert.rejects(verifyAgentSignature(envelope,encodeBase64Url(wrong)));
  assert.throws(()=>canonicalAgentRequest('/api/v1/agents/shell',timestamp,nonce,checksum));
});
