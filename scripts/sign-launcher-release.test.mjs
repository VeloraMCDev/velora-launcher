import test from 'node:test';
import assert from 'node:assert/strict';
import { generateKeyPairSync, verify } from 'node:crypto';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildManifest, platformOf, signedMessage } from './sign-launcher-release.mjs';

const commit = 'a'.repeat(40);
function fixture() {
  const dir = mkdtempSync(join(tmpdir(), 'velora-release-'));
  for (const name of ['Velora-Launcher_1.3.0_x64-setup.exe', 'Velora-Launcher_1.3.0_universal.dmg', 'Velora-Launcher_1.3.0_amd64.AppImage', 'notes.txt']) {
    writeFileSync(join(dir, name), `fixture ${name}`);
  }
  return dir;
}

test('signs each installer for exactly its version and checksum', () => {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const pem = privateKey.export({ type: 'pkcs8', format: 'pem' });
  const pub = publicKey.export({ type: 'spki', format: 'der' }).subarray(-32).toString('base64');
  const dir = fixture();
  try {
    const manifest = buildManifest(dir, '1.3.0', commit, pem, pub);
    assert.deepEqual(manifest.assets.map((a) => a.platform).sort(), ['linux', 'mac', 'windows']);
    for (const asset of manifest.assets) {
      const sig = Buffer.from(asset.signature, 'base64');
      assert.ok(verify(null, Buffer.from(signedMessage('1.3.0', asset.sha256)), publicKey, sig));
      assert.ok(!verify(null, Buffer.from(signedMessage('1.3.1', asset.sha256)), publicKey, sig));
    }
    const other = generateKeyPairSync('ed25519').publicKey.export({ type: 'spki', format: 'der' }).subarray(-32).toString('base64');
    assert.throws(() => buildManifest(dir, '1.3.0', commit, pem, other), /does not match/);
    assert.throws(() => buildManifest(dir, '1.3', commit, pem, pub), /x\.y\.z/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('matches the Rust message format and recognises installers', () => {
  assert.equal(signedMessage('v1.3.0', 'AB'), 'velora-launcher-release:v1\n1.3.0\nab');
  assert.equal(platformOf('x_amd64.deb'), 'linux');
  assert.equal(platformOf('x.msi'), null);
});
