/** Verify original bytes before parsing. The caller must dedupe atomically in D1.
 * @param {Request} request
 * @param {string | undefined} signingSecret
 */
export async function verifyGitHubDelivery(request,signingSecret) {
  if (!signingSecret) throw Error('WEBHOOK_NOT_CONFIGURED');
  const signature=request.headers.get('x-hub-signature-256') ?? '';
  const delivery=request.headers.get('x-github-delivery') ?? '';
  const event=request.headers.get('x-github-event') ?? '';
  if (request.method !== 'POST' || request.headers.get('content-type')?.split(';')[0].trim() !== 'application/json'
      || request.headers.has('content-encoding') || !/^sha256=[a-f0-9]{64}$/.test(signature)
      || !/^[a-f0-9]{8}-[a-f0-9]{4}-[1-8][a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/.test(delivery)
      || !['ping','workflow_run','release'].includes(event)) throw Error('WEBHOOK_DENIED');
  const reader=request.body?.getReader();
  if (!reader) throw Error('WEBHOOK_DENIED');
  const chunks=[];
  let length=0;
  try {
    while(true){const {value,done}=await reader.read();if(done)break;length+=value.byteLength;
      if(length>65536){await reader.cancel();throw Error('WEBHOOK_TOO_LARGE');}chunks.push(value);}
  } finally {reader.releaseLock();}
  const bytes=new Uint8Array(length);
  let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
  const key=await crypto.subtle.importKey('raw',new TextEncoder().encode(signingSecret),{name:'HMAC',hash:'SHA-256'},false,['verify']);
  const supplied=new Uint8Array(Array.from({length:32},(_,n)=>parseInt(signature.slice(7+2*n,9+2*n),16)));
  if (!await crypto.subtle.verify('HMAC',key,supplied,bytes)) throw Error('WEBHOOK_DENIED');
  let payload;try{payload=JSON.parse(new TextDecoder('utf-8',{fatal:true,ignoreBOM:false}).decode(bytes));}catch{throw Error('INVALID_WEBHOOK_JSON');}
  const digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
  return {delivery,event,body_sha256:Array.from(digest,n=>n.toString(16).padStart(2,'0')).join(''),payload};
}
