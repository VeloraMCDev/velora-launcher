import {execFileSync} from 'node:child_process';
import {existsSync, readFileSync, copyFileSync, appendFileSync} from 'node:fs';
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
  const include = "apply from: '../velora-signing.gradle'";
  if (!readFileSync(build, 'utf8').includes(include)) appendFileSync(build, '\n' + include + '\n');
}
