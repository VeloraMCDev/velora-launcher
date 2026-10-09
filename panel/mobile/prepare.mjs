import {execFileSync} from 'node:child_process';
import {existsSync, readFileSync, writeFileSync, copyFileSync, appendFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {createMobileConfig} from './config.mjs';
const platform = process.argv[2];
if (!['android', 'ios'].includes(platform)) throw Error('Choose android or ios');
if (platform === 'ios' && process.platform !== 'darwin') throw Error('iOS builds require macOS and Xcode');
createMobileConfig();
const pkg = JSON.parse(readFileSync('node_modules/@capacitor/cli/package.json'));
const entry = resolve('node_modules/@capacitor/cli', typeof pkg.bin === 'string' ? pkg.bin : pkg.bin.cap);
const run = args => execFileSync(process.execPath, [entry, ...args], {stdio: 'inherit'});
if (!existsSync(platform)) run(['add', platform]);
run(['sync', platform]);
if (platform === 'android') {
  copyFileSync('signing.gradle', 'android/velora-signing.gradle');
  const build = 'android/app/build.gradle';
  const version = JSON.parse(readFileSync('package.json')).version;
  const parts = version.split('.').map(Number);
  if (parts.length !== 3 || parts.some(p => !Number.isInteger(p) || p < 0 || p > 999)) throw Error('Mobile version must be x.y.z with parts below 1000');
  const code = parts[0] * 1_000_000 + parts[1] * 1_000 + parts[2];
  const source = readFileSync(build, 'utf8');
  const updated = source.replace(/versionCode\s+\d+/, `versionCode ${code}`).replace(/versionName\s+['"][^'"]+['"]/, `versionName '${version}'`);
  if (!updated.includes(`versionCode ${code}`) || !updated.includes(`versionName '${version}'`)) throw Error('Cannot set Android app version');
  writeFileSync(build, updated);
  const include = "apply from: '../velora-signing.gradle'";
  if (!readFileSync(build, 'utf8').includes(include)) appendFileSync(build, '\n' + include + '\n');
}
