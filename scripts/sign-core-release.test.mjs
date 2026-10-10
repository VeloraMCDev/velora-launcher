import assert from 'node:assert/strict';
import { generateKeyPairSync, createPublicKey, verify } from 'node:crypto';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { buildManifest, jarName, signedMessage } from './sign-core-release.mjs';
import { signedMessage as launcherMessage } from './sign-launcher-release.mjs';

function release(version = '0.6.0') {
  const dir = mkdtempSync(join(tmpdir(), 'core-release-'));
  for (const role of ['server', 'client']) writeFileSync(join(dir, jarName(role, '1.20.1', version)), Buffer.concat([Buffer.from('PK\x03\x04'), Buffer.from(role)]));
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  return { dir, privateKey: privateKey.export({ type: 'pkcs8', format: 'pem' }), publicKey };
}

test('signs the server and client jar for exactly this version and checksum', () => {
  const { dir, privateKey, publicKey } = release();
  const manifest = buildManifest(dir, '0.6.0', '1.20.1', 'a'.repeat(40), privateKey);
  assert.equal(manifest.schema, 1);
  assert.deepEqual(manifest.assets.map((a) => a.role), ['server', 'client']);
  for (const a of manifest.assets) {
    assert.equal(a.name, jarName(a.role, '1.20.1', '0.6.0'));
    assert.ok(verify(null, Buffer.from(signedMessage('0.6.0', a.sha256)), publicKey, Buffer.from(a.signature, 'base64')));
    assert.ok(!verify(null, Buffer.from(signedMessage('0.6.1', a.sha256)), publicKey, Buffer.from(a.signature, 'base64')), 'version is bound');
  }
});

test('mod signatures cannot be replayed as launcher signatures', () => {
  assert.notEqual(signedMessage('0.6.0', 'ab'.repeat(32)), launcherMessage('0.6.0', 'ab'.repeat(32)));
});

test('refuses a key that is not the published release key and files that are not jars', () => {
  const { dir, privateKey } = release();
  const other = createPublicKey(generateKeyPairSync('ed25519').privateKey).export({ type: 'spki', format: 'der' }).subarray(-32).toString('base64');
  assert.throws(() => buildManifest(dir, '0.6.0', '1.20.1', 'a'.repeat(40), privateKey, other), /does not match/);
  writeFileSync(join(dir, jarName('client', '1.20.1', '0.6.0')), 'not a jar');
  assert.throws(() => buildManifest(dir, '0.6.0', '1.20.1', 'a'.repeat(40), privateKey), /not a jar/);
  assert.throws(() => buildManifest(dir, '0.6', '1.20.1', 'a'.repeat(40), privateKey), /x\.y\.z/);
});
