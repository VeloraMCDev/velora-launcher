// Compatibility identities that installed launchers and credential stores depend on.
import {readFileSync, existsSync} from 'node:fs';
import {resolve} from 'node:path';
const root = resolve(import.meta.dirname, '..');
for (const path of ['panel/web/package-lock.json', 'launcher/package-lock.json', 'Cargo.lock', 'integrations/build.gradle']) {
  if (!existsSync(resolve(root, path))) throw Error(`Missing application input: ${path}`);
}
const config = JSON.parse(readFileSync(resolve(root, 'launcher/src-tauri/tauri.conf.json')));
if (config.identifier !== 'net.scopenet.launcher') throw Error('Installed launcher identity must remain compatible');
const secrets = readFileSync(resolve(root, 'launcher/src-tauri/src/secrets.rs'), 'utf8');
if (!secrets.includes('const SERVICE: &str = "net.scopenet.launcher";')) throw Error('Credential store identity changed');
console.log('Compatibility identities unchanged.');
