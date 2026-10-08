// Ed25519 request protocol v1. No generic command, shell or Docker API exists.
const noncePattern=/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
/** @param {Uint8Array<ArrayBuffer>} bytes */
export function encodeBase64Url(bytes){return btoa(String.fromCharCode(...bytes)).replaceAll('+','-').replaceAll('/','_').replaceAll('=','');}
/** @param {unknown} value @param {number} length */
export function decodeBase64Url(value,length){
  if(typeof value!=='string' || !/^[A-Za-z0-9_-]+$/.test(value) || value.length!==Math.ceil(length*8/6))throw Error('AGENT_DENIED');
  const bytes=Uint8Array.from(atob(value.replaceAll('-','+').replaceAll('_','/')),character=>character.charCodeAt(0));
  if(bytes.length!==length || encodeBase64Url(bytes)!==value)throw Error('AGENT_DENIED');return bytes;
}
/** @param {Uint8Array<ArrayBuffer>} bytes */
export async function agentHash(bytes){return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),byte=>byte.toString(16).padStart(2,'0')).join('');}
/** @param {string} path @param {string} timestamp @param {string} nonce @param {string} bodySha */
export function canonicalAgentRequest(path,timestamp,nonce,bodySha){
  if(!['/api/v1/agents/enroll','/api/v1/agents/heartbeat','/api/v1/agents/probe/poll','/api/v1/agents/probe/result'].includes(path) || !/^[1-9][0-9]{9,10}$/.test(timestamp) || !noncePattern.test(nonce) || !/^[a-f0-9]{64}$/.test(bodySha))throw Error('AGENT_DENIED');
  return ['POST',path,'',timestamp,nonce,bodySha].join('\n');
}
/** @param {Request} request @param {number} [now] */
export async function readAgentRequest(request,now=Math.floor(Date.now()/1000)){
  const url=new URL(request.url);
  if(request.method!=='POST' || url.search || request.headers.has('content-encoding') || request.headers.get('content-type')?.split(';')[0].trim()!=='application/json')throw Error('AGENT_DENIED');
  const timestamp=request.headers.get('velora-timestamp')??'',nonce=request.headers.get('velora-nonce')??'',bodySha=request.headers.get('velora-body-sha256')??'';
  const canonical=canonicalAgentRequest(url.pathname,timestamp,nonce,bodySha);
  if(Math.abs(now-Number(timestamp))>120)throw Error('AGENT_DENIED');
  const signature=decodeBase64Url(request.headers.get('velora-signature'),64);
  const keyId=request.headers.get('velora-key-id')??'',agentId=request.headers.get('velora-agent-id')??'';
  if(!/^[a-f0-9]{64}$/.test(keyId) || agentId!=='agt_'+keyId)throw Error('AGENT_DENIED');
  const reader=request.body?.getReader();if(!reader)throw Error('AGENT_DENIED');
  const chunks=[];let length=0;
  try{while(true){const {value,done}=await reader.read();if(done)break;length+=value.byteLength;if(length>4096){await reader.cancel();throw Error('AGENT_DENIED');}chunks.push(value);}}
  finally{reader.releaseLock();}
  const bytes=new Uint8Array(length);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
  if(await agentHash(bytes)!==bodySha)throw Error('AGENT_DENIED');
  let body;try{body=JSON.parse(new TextDecoder('utf-8',{fatal:true,ignoreBOM:false}).decode(bytes));}catch{throw Error('AGENT_DENIED');}
  return {agent_id:agentId,key_id:keyId,timestamp:Number(timestamp),nonce,body,canonical,signature};
}
/** @param {Awaited<ReturnType<typeof readAgentRequest>>} envelope @param {string} publicKey */
export async function verifyAgentSignature(envelope,publicKey){
  const bytes=decodeBase64Url(publicKey,32);
  if(await agentHash(bytes)!==envelope.key_id)throw Error('AGENT_DENIED');
  const key=await crypto.subtle.importKey('raw',bytes,{name:'Ed25519'},false,['verify']);
  if(!await crypto.subtle.verify('Ed25519',key,envelope.signature,new TextEncoder().encode(envelope.canonical)))throw Error('AGENT_DENIED');
}
