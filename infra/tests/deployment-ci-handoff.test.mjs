import test from 'node:test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {artifactMetadata} from '../scripts/artifact-metadata.mjs';

const entry=new URL('../scripts/register-development-candidate.mjs',import.meta.url).href;
const configuration={SystemRoot:process.env.SystemRoot??'',VELORA_REGISTRATION_ENDPOINT:'https://control.example.com/api/v1/ci/artifacts/register',ACTIONS_ID_TOKEN_REQUEST_URL:'https://fixture.actions.githubusercontent.com/oidc',ACTIONS_ID_TOKEN_REQUEST_TOKEN:'synthetic-private-job-credential',GITHUB_REPOSITORY:'Example/infra',GITHUB_SHA:'b'.repeat(40),GITHUB_REF:'refs/heads/main',GITHUB_RUN_ID:'456'};
const probe={schema:1,repository:'Example/infra',service_id:'deployment-probe',git_sha:configuration.GITHUB_SHA,git_ref:configuration.GITHUB_REF,build_run_id:configuration.GITHUB_RUN_ID,artifact_type:'oci',artifact_uri:'ghcr.io/example/deployment-probe@sha256:'+'a'.repeat(64),sha256:'a'.repeat(64),oci_digest:'sha256:'+'a'.repeat(64),version:'0.1.0',build_status:'PASSED',test_status:'PASSED'};

function run(fixture,env){
  return execFileSync(process.execPath,['--input-type=module','-e',fixture+`;await import(${JSON.stringify(entry)});`],{env:{...configuration,...env},encoding:'utf8',stdio:'pipe'});
}

test('OCI and Worker handoffs send canonical metadata and accept new or duplicate immutable receipts',()=>{
  const directory=mkdtempSync(join(tmpdir(),'velora-handoff-'));
  try{
    const path=join(directory,'candidate.json');writeFileSync(path,JSON.stringify(artifactMetadata(probe)));
    const worker={...probe,service_id:'control-plane',artifact_type:'worker_bundle',artifact_uri:'https://api.github.com/repos/Example/infra/actions/artifacts/123/zip'};
    delete worker.oci_digest;
    for(const [input,env] of [[probe,{CANDIDATE_FILE:path}],[worker,{ARTIFACT_ID:'123',ARTIFACT_SHA256:'a'.repeat(64)}]]){
      const expected=artifactMetadata(input);
      for(const created of [true,false]){
        const fixture=`import assert from 'node:assert/strict';let calls=0;globalThis.fetch=async(url,options)=>{
          assert.equal(options.redirect,'error');
          if(++calls===1){assert.equal(new URL(url).searchParams.get('audience'),'velora-control-plane');return Response.json({value:'synthetic-oidc'});}
          assert.equal(options.headers.authorization,'Bearer synthetic-oidc');
          assert.deepEqual(JSON.parse(options.body),${JSON.stringify(expected)});
          return Response.json({candidate:${JSON.stringify(expected)},created:${created}},{status:${created?201:200}});
        };`;
        assert.deepEqual(JSON.parse(run(fixture,env)),{candidate_id:expected.candidate_id,sha256:expected.sha256,created,deployment_performed:false});
      }
    }
  }finally{rmSync(directory,{recursive:true,force:true});}
});

test('untrusted candidate inputs fail before requesting any OIDC credential',()=>{
  const directory=mkdtempSync(join(tmpdir(),'velora-handoff-'));
  try{
    const path=join(directory,'candidate.json');
    for(const input of [
      {...probe,repository:'Other/infra'}, {...probe,git_sha:'c'.repeat(40)},
      {...probe,git_ref:'refs/heads/other'}, {...probe,build_run_id:'999'},
      {...probe,artifact_uri:'ghcr.io/example/deployment-probe:latest'},
      {...probe,oci_digest:'sha256:'+'c'.repeat(64)}, {...probe,candidate_id:'rc_'+'c'.repeat(64)},
      {...probe,test_status:'FAILED'}, {...probe,unknown:true}, 'x'.repeat(17000)
    ]){
      writeFileSync(path,typeof input==='string'?input:JSON.stringify(input));
      assert.throws(()=>run(`globalThis.fetch=()=>{console.error('UNEXPECTED_CREDENTIAL_REQUEST');throw Error('Unexpected request');}`,{CANDIDATE_FILE:path}),error=>{
        assert.equal(error.status,1);assert.equal(error.stdout,'');assert.equal(error.stderr.trim(),'Invalid immutable artifact receipt');return true;
      });
    }
    writeFileSync(path,JSON.stringify(probe));
    assert.throws(()=>run(`globalThis.fetch=()=>{console.error('UNEXPECTED_CREDENTIAL_REQUEST');}`,{CANDIDATE_FILE:path,ARTIFACT_ID:'123'}),error=>error.stderr.trim()==='Invalid immutable artifact receipt');
  }finally{rmSync(directory,{recursive:true,force:true});}
});

test('successful HTTP responses must identify the exact candidate and a boolean creation result',()=>{
  const expected=artifactMetadata({...probe,service_id:'control-plane',artifact_type:'worker_bundle',artifact_uri:'https://api.github.com/repos/Example/infra/actions/artifacts/123/zip',oci_digest:undefined});
  for(const receipt of [
    {candidate:{...expected,candidate_id:'rc_'+'c'.repeat(64)},created:true},
    {candidate:artifactMetadata({...expected,service_id:'other',candidate_id:undefined}),created:true},
    {candidate:{...expected,candidate_id:undefined},created:true},
    {candidate:expected,created:'true'}, {candidate:null,created:true}
  ]){
    assert.throws(()=>run(`let calls=0;globalThis.fetch=async()=>++calls===1?Response.json({value:'synthetic-oidc'}):Response.json(${JSON.stringify(receipt)});`,{ARTIFACT_ID:'123',ARTIFACT_SHA256:'a'.repeat(64)}),error=>{
      assert.equal(error.stdout,'');assert.equal(error.stderr.trim(),'Invalid candidate receipt');return true;
    });
  }
});

test('CI handoff reports only allowlisted errors and never prints token or network error contents',()=>{
  const token='synthetic-private-job-credential';
  const entry=new URL('../scripts/register-development-candidate.mjs',import.meta.url).href;
  const env={SystemRoot:process.env.SystemRoot??'',VELORA_REGISTRATION_ENDPOINT:'https://control.example.com/api/v1/ci/artifacts/register',ACTIONS_ID_TOKEN_REQUEST_URL:'https://fixture.actions.githubusercontent.com/oidc',ACTIONS_ID_TOKEN_REQUEST_TOKEN:token,ARTIFACT_SHA256:'a'.repeat(64),ARTIFACT_ID:'123',GITHUB_REPOSITORY:'Example/infra',GITHUB_SHA:'b'.repeat(40),GITHUB_REF:'refs/heads/main',GITHUB_RUN_ID:'456'};
  for(const [error,expected,network] of [
    ['CI_IDENTITY_DENIED','Candidate registration failed: HTTP 403 CI_IDENTITY_DENIED',false],
    [token,'Candidate registration failed: HTTP 403',false],
    [token,'Development registration failed',true],
    ['Invalid '+token,'Development registration failed',true]
  ]){
    const fixture=`let calls=0;globalThis.fetch=async()=>{if(++calls===1)return Response.json({value:${JSON.stringify(token)}});${network?`throw Error(${JSON.stringify(error)});`:`return Response.json({error:${JSON.stringify(error)}},{status:403});`}};await import(${JSON.stringify(entry)});`;
    try{execFileSync(process.execPath,['--input-type=module','-e',fixture],{env,encoding:'utf8',stdio:'pipe'});assert.fail('Expected registration failure');}
    catch(result){assert.equal(result.status,1);assert.equal(result.stdout,'');assert.equal(result.stderr.trim(),expected);assert.ok(!result.stderr.includes(token));}
  }
});
