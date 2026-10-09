// Compatibility identities that installed launchers and credential stores depend on.
import {readFileSync, existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
const root = resolve(import.meta.dirname, '..');
for (const path of ['panel/web/package-lock.json', 'launcher/package-lock.json', 'Cargo.lock', 'integrations/build.gradle']) {
  if (!existsSync(resolve(root, path))) throw Error(`Missing application input: ${path}`);
}
const config = JSON.parse(readFileSync(resolve(root, 'launcher/src-tauri/tauri.conf.json')));
if (config.productName !== 'Velora Launcher' || config.bundle.publisher !== 'Velora') throw Error('Launcher packaging must use Velora branding');
if (config.identifier !== 'net.scopenet.launcher') throw Error('Installed launcher identity must remain compatible');
const secrets = readFileSync(resolve(root, 'launcher/src-tauri/src/secrets.rs'), 'utf8');
if (!secrets.includes('const SERVICE: &str = "net.scopenet.launcher";')) throw Error('Credential store identity changed');
const template = readFileSync(resolve(root, 'launcher/src-tauri/installer/installer.nsi'), 'utf8');
const upstream = template
  .replace('!define UNINSTKEY "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SCOPENET Launcher"', '!define UNINSTKEY "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${PRODUCTNAME}"')
  .replace('!define MANUPRODUCTKEY "Software\\SCOPENET\\SCOPENET Launcher"', '!define MANUPRODUCTKEY "${MANUKEY}\\${PRODUCTNAME}"');
if (createHash('sha256').update(upstream.replaceAll('\r\n','\n')).digest('hex') !== 'dabed59013b1d78b879a1a85bc7f2eed2993b33a9a90cdabe5946de3d3950597') throw Error('Unexpected Windows installer template changes');
const lock = JSON.parse(readFileSync(resolve(root, 'launcher/package-lock.json')));
if (lock.packages['node_modules/@tauri-apps/cli'].version !== '2.12.0' || config.bundle.windows.nsis.template !== 'installer/installer.nsi') throw Error('Review the installer template when upgrading Tauri CLI');
console.log('Compatibility identities unchanged.');
