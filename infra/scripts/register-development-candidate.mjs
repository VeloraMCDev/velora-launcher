// Run only in the separately gated, successful-build-dependent GitHub job.
// Short-lived OIDC credentials stay in memory and are never printed or persisted.
import {readFileSync,statSync} from 'node:fs';
import {artifactMetadata} from './artifact-metadata.mjs';
const required=name=>{const value=process.env[name];if(!value)throw Error('Missing registration configuration');return value;};
const registrationErrors=['CONTROL_API_DISABLED','INGRESS_DENIED','CI_IDENTITY_DENIED','REGISTRATION_PAUSED','INVALID_ARTIFACT','REPOSITORY_NOT_AUTHORIZED','REGISTRATION_DENIED','SERVICE_ARTIFACT_DENIED','REGISTRATION_CAP_REACHED','CONTROL_API_UNAVAILABLE'];
try {
  const endpoint=new URL(required('VELORA_REGISTRATION_ENDPOINT'));
  if(endpoint.protocol!=='https:' || endpoint.username || endpoint.password || endpoint.search || endpoint.hash
      || endpoint.pathname!=='/api/v1/ci/artifacts/register')throw Error('Invalid registration endpoint');
  // Validate source and immutable coordinates before requesting a credential.
  let metadata;
  try {
    if(process.env.CANDIDATE_FILE){
      if(process.env.ARTIFACT_ID || process.env.ARTIFACT_SHA256)throw Error('Ambiguous input');
      if(statSync(process.env.CANDIDATE_FILE).size>16384)throw Error('Oversized input');
      metadata=artifactMetadata(JSON.parse(readFileSync(process.env.CANDIDATE_FILE,'utf8')));
    }else{
      const checksum=required('ARTIFACT_SHA256'),artifactId=required('ARTIFACT_ID');
      if(!/^[a-f0-9]{64}$/.test(checksum) || !/^[1-9][0-9]{0,19}$/.test(artifactId))throw Error('Invalid coordinates');
      metadata=artifactMetadata({schema:1,repository:required('GITHUB_REPOSITORY'),service_id:'control-plane',
        git_sha:required('GITHUB_SHA'),git_ref:required('GITHUB_REF'),build_run_id:required('GITHUB_RUN_ID'),
        artifact_type:'worker_bundle',artifact_uri:`https://api.github.com/repos/${required('GITHUB_REPOSITORY')}/actions/artifacts/${artifactId}/zip`,
        sha256:checksum,version:'0.1.0',build_status:'PASSED',test_status:'PASSED'});
    }
    if(metadata.repository!==required('GITHUB_REPOSITORY').toLowerCase() || metadata.git_sha!==required('GITHUB_SHA')
      || metadata.git_ref!==required('GITHUB_REF') || metadata.build_run_id!==required('GITHUB_RUN_ID'))throw Error('Source mismatch');
  }catch{throw Error('Invalid immutable artifact receipt');}
  const tokenUrl=new URL(required('ACTIONS_ID_TOKEN_REQUEST_URL'));
  if(tokenUrl.protocol!=='https:' || !tokenUrl.hostname.endsWith('.actions.githubusercontent.com'))throw Error('Invalid GitHub OIDC endpoint');
  tokenUrl.searchParams.set('audience','velora-control-plane');
  const tokenResponse=await fetch(tokenUrl,{headers:{authorization:'Bearer '+required('ACTIONS_ID_TOKEN_REQUEST_TOKEN')},redirect:'error',signal:AbortSignal.timeout(10000)});
  if(!tokenResponse.ok)throw Error('GitHub OIDC request failed');
  const token=(await tokenResponse.json()).value;
  if(typeof token!=='string' || token.length>16384)throw Error('Invalid OIDC response');
  const response=await fetch(endpoint,{method:'POST',headers:{authorization:'Bearer '+token,'content-type':'application/json'},body:JSON.stringify(metadata),redirect:'error',signal:AbortSignal.timeout(15000)});
  if(![200,201].includes(response.status)){
    const errors=new Set(registrationErrors);
    let code='';try{const body=await response.json();if(errors.has(body.error))code=' '+body.error;}catch{}
    if(code===' CI_IDENTITY_DENIED'){
      // Diagnostic booleans only. Decoding is not verification or authorization;
      // the origin still verifies every signature and claim. Never emit values.
      try{
        const [header,payload]=token.split('.').slice(0,2).map(part=>JSON.parse(Buffer.from(part,'base64url').toString('utf8')));
        const fields=['sub','exp','iat','nbf','jti','repository','repository_id','repository_owner','repository_owner_id','sha','ref','run_id','run_attempt','workflow_ref','workflow_sha','event_name','runner_environment'];
        const lifetime=payload.exp-payload.iat;
        console.error(JSON.stringify({event:'ci_identity_diagnostic',rs256:header.alg==='RS256',jwt_type:header.typ==='JWT',issuer:payload.iss==='https://token.actions.githubusercontent.com',audience:payload.aud==='velora-control-plane',required_claims:fields.every(field=>payload[field]!==undefined),numeric_strings:['repository_id','repository_owner_id','run_id','run_attempt'].every(field=>typeof payload[field]==='string' && /^[1-9][0-9]{0,19}$/.test(payload[field])),bounded_lifetime:Number.isFinite(lifetime)&&lifetime>0&&lifetime<=600,no_environment:payload.environment===undefined,job_workflow_self:payload.job_workflow_ref===payload.workflow_ref&&payload.job_workflow_sha===payload.workflow_sha,github_hosted:payload.runner_environment==='github-hosted',source_matches:payload.sha===process.env.GITHUB_SHA,workflow_source_matches:payload.workflow_sha===process.env.GITHUB_SHA,ref_matches:payload.ref===process.env.GITHUB_REF}));
      }catch{}
    }
    throw Error('Candidate registration failed: HTTP '+response.status+code);
  }
  const receipt=await response.json();
  try{
    const candidate=artifactMetadata(receipt.candidate);
    if(receipt.candidate.candidate_id!==metadata.candidate_id || candidate.candidate_id!==metadata.candidate_id
      || typeof receipt.created!=='boolean')throw Error('Receipt mismatch');
  }catch{throw Error('Invalid candidate receipt');}
  console.log(JSON.stringify({candidate_id:receipt.candidate.candidate_id,sha256:metadata.sha256,created:receipt.created,deployment_performed:false}));
}catch(error){
  // Do not expose raw network errors, request URLs or credential-bearing objects.
  const safe=new Set(['Missing registration configuration','Invalid registration endpoint','Invalid GitHub OIDC endpoint','GitHub OIDC request failed','Invalid OIDC response','Invalid immutable artifact receipt','Invalid candidate receipt']);
  const registrationMessage=new RegExp('^Candidate registration failed: HTTP [1-5][0-9]{2}(?: ('+registrationErrors.join('|')+'))?$');
  console.error(error instanceof Error && (safe.has(error.message)||registrationMessage.test(error.message))?error.message:'Development registration failed');
  process.exitCode=1;
}
