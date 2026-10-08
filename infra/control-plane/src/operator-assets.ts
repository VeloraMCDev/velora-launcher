import {verifyAccessPrincipal} from './access.mjs';
import {requireOperatorPermission} from './authorization.mjs';
import {json} from './http.ts';

export type OperatorAssetsEnv=Env & {ASSETS?:Fetcher};
export async function serveOperatorAssets(request:Request,env:OperatorAssetsEnv,verify=verifyAccessPrincipal):Promise<Response>{
  const id=crypto.randomUUID(),url=new URL(request.url);
  if(!env.ASSETS || !env.OPERATOR_API_HOST || url.hostname!==env.OPERATOR_API_HOST
      || env.ENVIRONMENT!=='development' || !['GET','HEAD'].includes(request.method)
      || !['/','/index.html','/styles.css','/app.mjs','/client.mjs'].includes(url.pathname))return json(404,{error:'NOT_FOUND'},id);
  try{
    const principal=await verify(request,env);
    await requireOperatorPermission(env.IDENTITY_DB,{issuer:env.ACCESS_TEAM_DOMAIN,subject:principal.subject},'environment.read','development');
  }catch{return json(403,{error:'ACCESS_DENIED'},id);}
  const asset=await env.ASSETS.fetch(request),headers=new Headers(asset.headers);
  headers.set('Cache-Control','no-store');headers.set('X-Content-Type-Options','nosniff');
  headers.set('Content-Security-Policy',"default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'");
  headers.set('Referrer-Policy','no-referrer');headers.set('X-Frame-Options','DENY');
  return new Response(asset.body,{status:asset.status,headers});
}
