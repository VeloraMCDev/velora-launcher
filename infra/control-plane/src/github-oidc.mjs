import { createRemoteJWKSet, jwtVerify } from 'jose';

const issuer = 'https://token.actions.githubusercontent.com';
const keys = createRemoteJWKSet(new URL(issuer + '/.well-known/jwks'), {timeoutDuration:5000});
const fullSha = /^[a-f0-9]{40}$/;
const numericId = /^[1-9][0-9]{0,19}$/;

/**
 * Verify a GitHub job identity. This does not by itself authorize registration.
 * @param {Request} request
 * @param {string} audience
 * @param {Parameters<typeof jwtVerify>[1]} [testKeys]
 */
export async function verifyGitHubJob(request,audience,testKeys) {
  if (!audience) throw Error('OIDC_NOT_CONFIGURED');
  const header = request.headers.get('authorization') ?? '';
  if (!header.startsWith('Bearer ') || header.length > 16384) throw Error('OIDC_DENIED');
  const {payload} = await jwtVerify(header.slice(7),testKeys ?? keys,{
    algorithms:['RS256'],issuer,audience,typ:'JWT',clockTolerance:5,maxTokenAge:'10m',
    requiredClaims:['sub','exp','iat','nbf','jti','repository','repository_id',
      'repository_owner','repository_owner_id','sha','ref','run_id','run_attempt',
      'workflow_ref','workflow_sha','event_name','runner_environment']
  });
  if (typeof payload.exp !== 'number' || typeof payload.iat !== 'number'
      || payload.exp <= payload.iat || payload.exp - payload.iat > 600) throw Error('OIDC_DENIED');
  for (const field of ['repository_id','repository_owner_id','run_id','run_attempt']) {
    if (typeof payload[field] !== 'string' || !numericId.test(payload[field])) throw Error('OIDC_DENIED');
  }
  for (const field of ['sha','workflow_sha']) {
    if (typeof payload[field] !== 'string' || !fullSha.test(payload[field])) throw Error('OIDC_DENIED');
  }
  if (typeof payload.repository !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}\/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$/.test(payload.repository)
      || typeof payload.repository_owner !== 'string' || typeof payload.ref !== 'string'
      || typeof payload.workflow_ref !== 'string' || typeof payload.sub !== 'string'
      || typeof payload.jti !== 'string' || !payload.jti || payload.jti.length > 256) throw Error('OIDC_DENIED');
  // No untrusted PR, reusable workflow or persistent runner trust is implicit.
  if (!['push','workflow_dispatch'].includes(String(payload.event_name))
      || payload.runner_environment !== 'github-hosted'
      || payload.environment !== undefined) throw Error('OIDC_DENIED');
  // GitHub may include job-workflow claims for the workflow itself. Accept only
  // an exact self-reference; foreign or independently pinned reusable code needs
  // a separate reviewed policy and cannot inherit this workflow's authority.
  if ((payload.job_workflow_ref !== undefined || payload.job_workflow_sha !== undefined)
      && (payload.job_workflow_ref !== payload.workflow_ref
          || payload.job_workflow_sha !== payload.workflow_sha)) throw Error('OIDC_DENIED');
  return payload;
}

/**
 * Exact, operator-registered policy; never an organization-wide name-only grant.
 * @param {import('jose').JWTPayload} claims
 * @param {{repository:string,repository_id:string,owner_id:string,ref:string,
 * workflow_path:string,workflow_sha:string,subject_format:'immutable'|'legacy',enabled:number}} policy
 * @param {{repository:string,git_sha:string,git_ref:string,build_run_id:string}} artifact
 */
export function authorizeGitHubArtifact(claims,policy,artifact) {
  const [owner,repo] = policy.repository.split('/');
  const expectedSubject = policy.subject_format === 'immutable'
    ? `repo:${owner}@${policy.owner_id}/${repo}@${policy.repository_id}:ref:${policy.ref}`
    : `repo:${policy.repository}:ref:${policy.ref}`;
  if (policy.enabled !== 1 || !['immutable','legacy'].includes(policy.subject_format)
      || !fullSha.test(policy.workflow_sha) || !policy.workflow_path.startsWith('.github/workflows/')
      || String(claims.repository).toLowerCase() !== policy.repository.toLowerCase()
      || String(claims.repository_owner).toLowerCase() !== owner.toLowerCase()
      || claims.repository_id !== policy.repository_id || claims.repository_owner_id !== policy.owner_id
      || claims.ref !== policy.ref || claims.sub !== expectedSubject
      || claims.workflow_ref !== `${policy.repository}/${policy.workflow_path}@${policy.ref}`
      || claims.workflow_sha !== policy.workflow_sha
      || artifact.repository.toLowerCase() !== policy.repository.toLowerCase()
      || artifact.git_sha !== claims.sha || artifact.git_ref !== claims.ref
      || artifact.build_run_id !== claims.run_id) throw Error('REGISTRATION_DENIED');
  return {actor:`github:${policy.repository_id}`,run_id:claims.run_id};
}
