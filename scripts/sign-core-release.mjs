// Signs the Velora Core jars for a release and writes core-manifest.json + SHA256SUMS.
//
//   VELORA_RELEASE_SIGNING_KEY=<pkcs8 pem> node scripts/sign-core-release.mjs <dir> <version> <minecraft> <commit>
//
// Each signature covers "velora-core-release:v1\n<version>\n<sha256>" (see crates/shared/src/updates.rs). That is a
// different domain from the launcher's signatures, so neither can be replayed as the other. The Panel verifies
// them against crates/shared/release-signing.pub before it serves a jar to game servers.
import { createHash, createPrivateKey, createPublicKey, sign, verify } from 'node:crypto';
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

export const signedMessage = (version, sha256) =>
  `velora-core-release:v1\n${version.trim().replace(/^v/, '')}\n${sha256.toLowerCase()}`;

export const jarName = (role, minecraft, version) => `velora-core-${role}-${minecraft}-${version}.jar`;

export function buildManifest(dir, version, minecraft, commit, privateKeyPem, publicKeyBase64) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw Error(`Release version must be x.y.z, got ${version}`);
  if (!/^\d+\.\d+(\.\d+)?$/.test(minecraft)) throw Error(`Minecraft version must look like 1.20.1, got ${minecraft}`);
  if (!/^[0-9a-f]{40}$/.test(commit)) throw Error('Commit must be a full SHA');
  const key = createPrivateKey(privateKeyPem);
  const publicKey = createPublicKey(key);
  const derived = publicKey.export({ type: 'spki', format: 'der' }).subarray(-32).toString('base64');
  if (publicKeyBase64 && derived !== publicKeyBase64.trim()) {
    throw Error('Signing key does not match crates/shared/release-signing.pub');
  }
  const assets = ['server', 'client'].map((role) => {
    const name = jarName(role, minecraft, version);
    const path = join(dir, name);
    const bytes = readFileSync(path);
    if (bytes.subarray(0, 4).toString('latin1') !== 'PK\x03\x04') throw Error(`${name} is not a jar`);
    const sha256 = createHash('sha256').update(bytes).digest('hex');
    const message = Buffer.from(signedMessage(version, sha256));
    const signature = sign(null, message, key);
    if (!verify(null, message, publicKey, signature)) throw Error(`Signature self-check failed for ${name}`);
    return { role, name, size: statSync(path).size, sha256, signature: signature.toString('base64') };
  });
  return { schema: 1, version, minecraft, commit, assets };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [dir, version, minecraft, commit] = process.argv.slice(2);
  const pem = process.env.VELORA_RELEASE_SIGNING_KEY;
  if (!dir || !version || !minecraft || !commit || !pem) {
    console.error('usage: VELORA_RELEASE_SIGNING_KEY=... node scripts/sign-core-release.mjs <dir> <version> <minecraft> <commit>');
    process.exit(2);
  }
  const root = new URL('..', import.meta.url);
  const publicKey = readFileSync(new URL('crates/shared/release-signing.pub', root), 'utf8');
  const manifest = buildManifest(dir, version, minecraft, commit, pem, publicKey);
  writeFileSync(join(dir, 'core-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  writeFileSync(join(dir, 'SHA256SUMS'), manifest.assets.map((a) => `${a.sha256}  ${a.name}`).join('\n') + '\n');
  console.log(JSON.stringify({ version, minecraft, assets: manifest.assets.map(({ role, name, size }) => ({ role, name, size })) }, null, 2));
}
