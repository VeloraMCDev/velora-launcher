import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
import {parse} from 'yaml';
import {artifactMetadata} from '../scripts/artifact-metadata.mjs';

test('Panel release workflow emits metadata accepted by the deployment authority',()=>{
 const workflow=parse(readFileSync(new URL('../../.github/workflows/panel-release.yml',import.meta.url),'utf8'));
 const run=workflow.jobs.image.steps.find(step=>step.name==='Record the candidate').run;
 const source=run.match(/<<'JS'\r?\n([\s\S]*?)\r?\nJS(?:\r?\n|$)/)?.[1];
 assert(source,'Candidate generator must remain inspectable');
 const env={ARTIFACT_URI:'ghcr.io/veloramcdev/velora-panel@sha256:'+'b'.repeat(64),GITHUB_REPOSITORY:'VeloraMCDev/velora-launcher',GITHUB_SHA:'a'.repeat(40),GITHUB_REF:'refs/heads/main',GITHUB_RUN_ID:'123',PANEL_VERSION:'0.5.0'};
 let candidate;
 const generate=values=>runInNewContext(source.replace(/^import \{writeFileSync\} from 'node:fs';\r?\n/,''),{process:{env:values},writeFileSync:(path,data)=>{assert.equal(path,'panel-candidate.json');candidate=JSON.parse(data);}},{timeout:1000});
 generate(env);
 const validated=artifactMetadata(candidate);
 assert.equal(validated.repository,'veloramcdev/velora-launcher');
 assert.equal(validated.service_id,'velora-panel');
 assert.equal(validated.sha256,'b'.repeat(64));
 assert.equal(validated.oci_digest,'sha256:'+'b'.repeat(64));
 assert.throws(()=>generate({...env,ARTIFACT_URI:'ghcr.io/veloramcdev/velora-panel:latest'}));
});
