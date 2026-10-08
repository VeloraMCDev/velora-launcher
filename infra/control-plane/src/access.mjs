import { createRemoteJWKSet, jwtVerify } from 'jose';

/** @type {Map<string, ReturnType<typeof createRemoteJWKSet>>} */
const keySets = new Map(); // Public signing keys only; no request identity/state.

/**
 * @param {Request} request
 * @param {{ACCESS_TEAM_DOMAIN:string, ACCESS_AUD:string}} config
 * @param {Parameters<typeof jwtVerify>[1]} [testKeys]
 */
export async function verifyAccessPrincipal(request,config,testKeys) {
  if (!/^https:\/\/[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.cloudflareaccess\.com$/.test(config.ACCESS_TEAM_DOMAIN)
      || !config.ACCESS_AUD) throw Error('ACCESS_NOT_CONFIGURED');
  const token = request.headers.get('cf-access-jwt-assertion');
  if (!token || token.length > 8192) throw Error('ACCESS_DENIED');
  let keys = testKeys;
  if (!keys) {
    let remote = keySets.get(config.ACCESS_TEAM_DOMAIN);
    if (!remote) {
      remote = createRemoteJWKSet(new URL(config.ACCESS_TEAM_DOMAIN + '/cdn-cgi/access/certs'),{timeoutDuration:5000});
      keySets.set(config.ACCESS_TEAM_DOMAIN,remote);
    }
    keys = remote;
  }
  const {payload} = await jwtVerify(token,keys,{
    algorithms:['RS256'],issuer:config.ACCESS_TEAM_DOMAIN,audience:config.ACCESS_AUD,
    requiredClaims:['exp','iat','sub','email','type'],maxTokenAge:'1h',clockTolerance:5
  });
  if (payload.type !== 'app' || typeof payload.email !== 'string'
      || typeof payload.sub !== 'string' || !payload.sub) throw Error('ACCESS_DENIED');
  return {subject:payload.sub,email:payload.email};
}

/**
 * Bootstrap admission is narrower than ordinary operator authentication.
 * @param {Request} request
 * @param {{ACCESS_TEAM_DOMAIN:string, ACCESS_AUD:string, BOOTSTRAP_OPERATOR_EMAIL:string}} config
 * @param {Parameters<typeof jwtVerify>[1]} [testKeys]
 */
export async function verifyBootstrapOperator(request,config,testKeys) {
  if (!config.BOOTSTRAP_OPERATOR_EMAIL) throw Error('ACCESS_NOT_CONFIGURED');
  const principal = await verifyAccessPrincipal(request,config,testKeys);
  if (principal.email.toLowerCase() !== config.BOOTSTRAP_OPERATOR_EMAIL.toLowerCase()) throw Error('ACCESS_DENIED');
  return principal;
}
