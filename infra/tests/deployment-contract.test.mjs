import test from 'node:test';
import assert from 'node:assert/strict';
import { artifactMetadata } from '../scripts/artifact-metadata.mjs';

const input = () => ({ schema:1, repository:'VeloraMCDev/infra', service_id:'deployment-probe', git_sha:'a'.repeat(40), git_ref:'refs/heads/main', build_run_id:'123', artifact_type:'oci', artifact_uri:`ghcr.io/veloramcdev/deployment-probe@sha256:${'b'.repeat(64)}`, oci_digest:`sha256:${'b'.repeat(64)}`, sha256:'b'.repeat(64), version:'0.1.0', build_status:'PASSED', test_status:'PASSED' });

test('candidate identity survives retry/promotion and changes when immutable target changes', () => {
  const first = artifactMetadata(input());
  const retry = artifactMetadata({ ...input(), repository:'veloramcdev/infra', build_run_id:'124' });
  assert.equal(first.candidate_id, retry.candidate_id);
  assert.equal(first.repository, 'veloramcdev/infra');
  assert.deepEqual(artifactMetadata(first), first);
  assert.throws(() => artifactMetadata({ ...input(), candidate_id:'rc_'+'c'.repeat(64) }));
  assert.notEqual(first.candidate_id, artifactMetadata({ ...input(), git_sha:'c'.repeat(40) }).candidate_id);
  assert.notEqual(first.candidate_id, artifactMetadata({ ...input(), artifact_uri:`ghcr.io/veloramcdev/deployment-probe@sha256:${'c'.repeat(64)}`, sha256:'c'.repeat(64), oci_digest:`sha256:${'c'.repeat(64)}` }).candidate_id);
});

test('tags, inconsistent digests, failed tests, short SHAs and schema extensions fail closed', () => {
  for (const change of [{artifact_uri:'ghcr.io/veloramcdev/deployment-probe:latest'}, {artifact_uri:'ghcr.io/veloramcdev/deployment-probe:sha-'+ 'a'.repeat(40)}, {oci_digest:'sha256:'+ 'c'.repeat(64)}, {test_status:'FAILED'}, {build_status:'UNKNOWN'}, {git_sha:'abcdef0'}, {schema:2}, {shell:'docker compose up'}, {environment:'production'}, {artifact_uri:`ghcr.io/veloramcdev/../probe@sha256:${'b'.repeat(64)}`}]) {
    assert.throws(() => artifactMetadata({ ...input(), ...change }));
  }
});

test('non-OCI sources require checksums and credential-free locators; other registered sources remain possible', () => {
  const { oci_digest, ...binary } = { ...input(), repository:'ExampleOrg/utility', artifact_type:'binary', artifact_uri:'https://downloads.example.invalid/utility/v0.1.0/utility-linux-amd64' };
  assert.equal(artifactMetadata(binary).repository, 'exampleorg/utility');
  for (const artifact_uri of ['http://example.invalid/file', 'https://user:password@example.invalid/file', 'https://example.invalid/file?token=synthetic', 'https://example.invalid/file#credential']) assert.throws(() => artifactMetadata({ ...binary, artifact_uri }));
  assert.throws(() => artifactMetadata({ ...binary, sha256:undefined }));
  assert.throws(() => artifactMetadata({ ...binary, provenance_uri:'https://example.invalid/provenance?token=synthetic' }));
});
