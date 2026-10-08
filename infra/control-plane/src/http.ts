export class ApiError extends Error {
  status:number;code:string;
  constructor(status:number,code:string){super(code);this.status=status;this.code=code;}
}
export const uuid=/^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
export function json(status:number,body:Record<string,unknown>,id:string) {
  return Response.json({...body,request_id:id},{status,headers:{'Cache-Control':'no-store','X-Content-Type-Options':'nosniff','X-Request-Id':id}});
}
export async function digest(text:string) {
  return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text))),n=>n.toString(16).padStart(2,'0')).join('');
}
export async function readJson(request:Request,maxBytes=16384):Promise<unknown> {
  if(request.headers.get('content-type')?.split(';')[0].trim()!=='application/json' || request.headers.has('content-encoding'))throw new ApiError(415,'JSON_REQUIRED');
  const reader=request.body?.getReader();if(!reader)throw new ApiError(400,'BODY_REQUIRED');
  const chunks:Uint8Array[]=[];let length=0;
  try{while(true){const {value,done}=await reader.read();if(done)break;length+=value.byteLength;if(length>maxBytes){await reader.cancel();throw new ApiError(413,'BODY_TOO_LARGE');}chunks.push(value);}}
  finally{reader.releaseLock();}
  const bytes=new Uint8Array(length);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
  try{return JSON.parse(new TextDecoder('utf-8',{fatal:true,ignoreBOM:false}).decode(bytes));}catch{throw new ApiError(400,'INVALID_JSON');}
}
