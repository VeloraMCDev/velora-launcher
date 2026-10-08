// CI only. The operator supplies the key; this never creates/replaces an identity.
import {writeFileSync, appendFileSync} from 'node:fs';
import {join} from 'node:path';
const encoded = (process.env.VELORA_ANDROID_KEYSTORE_BASE64 || '').trim();
const root = process.env.RUNNER_TEMP;
if (!root || !process.env.GITHUB_ENV || !/^[A-Za-z0-9+/]+={0,2}$/.test(encoded) || encoded.length > 131072) {
  throw Error('Android signing key is missing or invalid');
}
const bytes = Buffer.from(encoded, 'base64');
if (bytes.length < 100 || bytes.toString('base64') !== encoded) throw Error('Android signing key is invalid');
const path = join(root, 'velora-player-signing.jks');
try {
  writeFileSync(path, bytes, {mode:0o600, flag:'wx'});
  appendFileSync(process.env.GITHUB_ENV, `VELORA_ANDROID_KEYSTORE=${path}\n`);
} finally {bytes.fill(0);}
