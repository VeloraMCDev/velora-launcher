// Signs launcher installers for a release and writes release-manifest.json + SHA256SUMS.
//
//   VELORA_RELEASE_SIGNING_KEY=<pkcs8 pem> node scripts/sign-launcher-release.mjs <dir> <version> <commit>
//
// Each signature covers "velora-launcher-release:v1\n<version>\n<sha256>" (see
// crates/shared/src/updates.rs), so launchers and the Panel can verify an installer
// belongs to exactly this release. The public key is crates/shared/release-signing.pub.
import { createHash, createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';
import { readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export const signedMessage = (version, sha256) =>
  `velora-launcher-release:v1\n${version.trim().replace(/^v/, '')}\n${sha256.toLowerCase()}`;

export function platformOf(name) {
  const lower = name.toLowerCase();
  if (lower.endsWith('-setup.exe')) return 'windows';
  if (lower.endsWith('.dmg')) return 'mac';
  if (lower.endsWith('.appimage') || lower.endsWith('.deb')) return 'linux';
  return null;
}

export function buildManifest(dir, version, commit, privateKeyPem, publicKeyBase64) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw Error(`Release version must be x.y.z, got ${version}`);
  if (!/^[0-9a-f]{40}$/.test(commit)) throw Error('Commit must be a full SHA');
  const key = createPrivateKey(privateKeyPem);
  const publicKey = createPublicKey(key);
  const derived = publicKey.export({ type: 'spki', format: 'der' }).subarray(-32).toString('base64');
  if (publicKeyBase64 && derived !== publicKeyBase64.trim()) {
    throw Error('Signing key does not match crates/shared/release-signing.pub');
  }
  const assets = [];
  for (const name of readdirSync(dir).sort()) {
    const platform = platformOf(name);
    if (!platform) continue;
    if (!/^[A-Za-z0-9._+-]+$/.test(name)) throw Error(`Unexpected installer name ${name}`);
    const bytes = readFileSync(join(dir, name));
    const sha256 = createHash('sha256').update(bytes).digest('hex');
    const message = Buffer.from(signedMessage(version, sha256));
    const signature = sign(null, message, key);
    if (!verify(null, message, publicKey, signature)) throw Error(`Signature self-check failed for ${name}`);
    assets.push({ platform, name, size: statSync(join(dir, name)).size, sha256, signature: signature.toString('base64') });
  }
  for (const platform of ['windows', 'mac', 'linux']) {
    if (!assets.some((a) => a.platform === platform)) throw Error(`No ${platform} installer in ${dir}`);
  }
  return { schema: 1, version, commit, assets };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [dir, version, commit] = process.argv.slice(2);
  const pem = process.env.VELORA_RELEASE_SIGNING_KEY;
  if (!dir || !version || !commit || !pem) {
    console.error('usage: VELORA_RELEASE_SIGNING_KEY=... node scripts/sign-launcher-release.mjs <dir> <version> <commit>');
    process.exit(2);
  }
  const root = new URL('..', import.meta.url);
  const publicKey = readFileSync(new URL('crates/shared/release-signing.pub', root), 'utf8');
  const manifest = buildManifest(dir, version, commit, pem, publicKey);
  writeFileSync(join(dir, 'release-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  writeFileSync(join(dir, 'SHA256SUMS'), manifest.assets.map((a) => `${a.sha256}  ${a.name}`).join('\n') + '\n');
  console.log(JSON.stringify({ version, assets: manifest.assets.map(({ platform, name, size }) => ({ platform, name, size })) }, null, 2));
}
