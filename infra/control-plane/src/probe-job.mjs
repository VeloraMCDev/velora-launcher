import {decodeBase64Url,encodeBase64Url} from './agent-protocol.mjs';
const uuid=/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
const fields=['schema','id','deployment_id','agent_id','environment','action','service_id','image','expected_commit','issued_at','expires_at'];
/** @typedef {{schema:number,id:string,deployment_id:string,agent_id:string,environment:string,action:string,service_id:string,image:string,expected_commit:string,issued_at:number,expires_at:number}} ProbeJob */
/** @typedef {{schema:number,key_id:string,payload:string,signature:string}} ProbeEnvelope */
/** @param {ProbeJob} job @param {string} agentId @param {number} [now] */
export function validateProbeJob(job,agentId,now=Math.floor(Date.now()/1000)) {
  if(!job || typeof job!=='object' || Array.isArray(job) || Object.keys(job).sort().join()!==[...fields].sort().join()
    || job.schema!==1 || !uuid.test(job.id) || !uuid.test(job.deployment_id)
    || job.agent_id!==agentId || !/^agt_[a-f0-9]{64}$/.test(agentId)
    || job.environment!=='development' || job.action!=='DEPLOY_SERVICE' || job.service_id!=='deployment-probe'
    || !/^ghcr\.io\/veloramcdev\/deployment-probe@sha256:[a-f0-9]{64}$/.test(job.image)
    || !/^[a-f0-9]{40}$/.test(job.expected_commit) || !Number.isSafeInteger(job.issued_at) || !Number.isSafeInteger(job.expires_at)
    || job.issued_at<1 || job.issued_at>now+5 || job.expires_at<=now || job.expires_at<=job.issued_at || job.expires_at-job.issued_at>300) {
    throw Error('PROBE_JOB_DENIED');
  }
  return job;
}
/** @param {ProbeJob} job @param {CryptoKey} privateKey @param {string} keyId @param {number} [now] */
export async function signProbeJob(job,privateKey,keyId,now=Math.floor(Date.now()/1000)) {
  validateProbeJob(job,job.agent_id,now);
  if(!/^[a-f0-9]{64}$/.test(keyId))throw Error('PROBE_KEY_DENIED');
  const encoded=new TextEncoder().encode(JSON.stringify(job));
  const bytes=new Uint8Array(encoded.length);bytes.set(encoded);
  const message=new TextEncoder().encode('velora-probe-job-v1\n'+new TextDecoder().decode(bytes));
  const signature=await crypto.subtle.sign('Ed25519',privateKey,message);
  return {schema:1,key_id:keyId,payload:encodeBase64Url(bytes),signature:encodeBase64Url(Uint8Array.from(new Uint8Array(signature)))};
}
/** @param {ProbeEnvelope} envelope @param {string} publicKey @param {string} agentId @param {number} [now] */
export async function verifyProbeJob(envelope,publicKey,agentId,now=Math.floor(Date.now()/1000)) {
  if(!envelope || typeof envelope!=='object' || Object.keys(envelope).sort().join()!==['schema','key_id','payload','signature'].sort().join()
    || envelope.schema!==1 || typeof envelope.payload!=='string' || envelope.payload.length>4096 || !/^[A-Za-z0-9_-]+$/.test(envelope.payload))throw Error('PROBE_JOB_DENIED');
  const raw=decodeBase64Url(publicKey,32);
  const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',raw)),b=>b.toString(16).padStart(2,'0')).join('');
  if(envelope.key_id!==hash)throw Error('PROBE_KEY_DENIED');
  const bytes=Uint8Array.from(atob(envelope.payload.replaceAll('-','+').replaceAll('_','/')),c=>c.charCodeAt(0));
  if(encodeBase64Url(bytes)!==envelope.payload)throw Error('PROBE_JOB_DENIED');
  const text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:false}).decode(bytes);
  const key=await crypto.subtle.importKey('raw',raw,'Ed25519',false,['verify']);
  if(!await crypto.subtle.verify('Ed25519',key,decodeBase64Url(envelope.signature,64),new TextEncoder().encode('velora-probe-job-v1\n'+text)))throw Error('PROBE_JOB_DENIED');
  return validateProbeJob(JSON.parse(text),agentId,now);
}
