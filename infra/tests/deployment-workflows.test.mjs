import test from 'node:test';
import assert from 'node:assert/strict';
import { workflowFindings } from '../scripts/check-workflows.mjs';
import { readFileSync } from 'node:fs';

const valid = `name: Synthetic check
on: [push, pull_request]
permissions:
  contents: read
jobs:
  check:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@${'a'.repeat(40)}
        with:
          persist-credentials: false
`;

test('workflow guard catches floating actions, checkout credentials and unbounded PR jobs', () => {
  assert.deepEqual(workflowFindings(valid),[]);
  for (const unsafe of [valid.replace('a'.repeat(40),'v4'),valid.replace('persist-credentials: false','persist-credentials: true'),valid.replace('timeout-minutes: 10','timeout-minutes: 0'),valid.replace('ubuntu-latest','self-hosted'),valid.replace('permissions:\n  contents: read','permissions: write-all')]) assert.ok(workflowFindings(unsafe).length);
});

test('elevated release job requires main gate and transient assets require finite retention', () => {
  const elevated = valid.replace('    steps:',"    permissions:\n      packages: write\n    if: github.ref == 'refs/heads/main'\n    steps:");
  assert.deepEqual(workflowFindings(elevated),[]);
  assert.ok(workflowFindings(elevated.replace("    if: github.ref == 'refs/heads/main'\n",'')).length);
  const upload = valid+`      - uses: actions/upload-artifact@${'b'.repeat(40)}\n        with:\n          retention-days: 14\n`;
  assert.deepEqual(workflowFindings(upload),[]);
  assert.ok(workflowFindings(upload.replace('retention-days: 14','retention-days: 90')).length);
});

test('Development OIDC exception requires the exact opt-in, branch, dependencies and narrow permissions',()=>{
  for(const file of ['deployment-baseline.yml','release-probe.yml']){
  const source=readFileSync(new URL('../.github/workflows/'+file,import.meta.url),'utf8');
  assert.deepEqual(workflowFindings(source),[]);
  for(const unsafe of [
    source.replace("github.event_name != 'pull_request' && ",''),
    source.replaceAll('refs/heads/main','refs/heads/other'),
    source.replace(" && vars.VELORA_DEVELOPMENT_REGISTRATION_ENABLED == 'true'",''),
    source.replace(/needs: (contracts-and-container|candidate)/,'needs: unverified'),
    source.replace("github.event_name != 'pull_request' && github.ref == 'refs/heads/main' && vars.VELORA_DEVELOPMENT_REGISTRATION_ENABLED == 'true'","github.ref == 'refs/heads/main'"),
    source.replace('id-token: write','id-token: write\n      packages: write')
  ])assert.ok(workflowFindings(unsafe).length);
  }
});
