// Real-listener acceptance using only fresh synthetic data. Never opens operator data.
import assert from 'node:assert/strict';
import {spawn, execFileSync} from 'node:child_process';
import {mkdtempSync, mkdirSync, cpSync, readdirSync, lstatSync, readFileSync, writeFileSync} from 'node:fs';
import {join, resolve, relative, dirname} from 'node:path';
import {tmpdir} from 'node:os';
import {createHash} from 'node:crypto';
import {createServer} from 'node:net';
import {fileURLToPath} from 'node:url';

const application = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const binary = resolve(process.argv[2] || 'target/debug/velora-panel');
const web = resolve(application, 'panel/web/dist');
assert(lstatSync(binary).isFile(), 'Supply the freshly built backend binary');
const root = mkdtempSync(join(tmpdir(), 'velora-restore-acceptance-'));
const original = join(root, 'original'), backup = join(root, 'cold-backup'), restored = join(root, 'restored');
mkdirSync(original);
const password = 'Synthetic-Admin-Acceptance-4829!';
const playerPassword = 'Synthetic-Player-Acceptance-4829!';
const checks = [];
let active;

function confined(path) {
  const rel = relative(root, path);
  assert(rel && !rel.startsWith('..') && !resolve(path).includes('\n'), 'Fixture path escapes isolated root');
  return path;
}
async function stop() {
  if (!active) return;
  const previous = active; active = undefined;
  if (previous.exitCode !== null) return;
  await new Promise((ok, fail) => {
    const timeout = setTimeout(() => {previous.kill('SIGKILL'); fail(Error('Fixture backend did not stop'));}, 10000);
    previous.once('exit', () => {clearTimeout(timeout); ok();});
    previous.kill();
  });
}
async function start(data) {
  assert(!active, 'Only one fixture writer may run at a time');
  confined(data);
  const server = createServer();
  await new Promise((ok, fail) => server.once('error', fail).listen(0, '127.0.0.1', ok));
  const port = server.address().port;
  await new Promise(ok => server.close(ok));
  const origin = `http://127.0.0.1:${port}`;
  const env = {};
  for (const key of ['SystemRoot','WINDIR','PATH','TEMP','TMP','USERPROFILE','APPDATA','LOCALAPPDATA']) {
    if (process.env[key]) env[key] = process.env[key];
  }
  Object.assign(env, {VELORA_BIND:`127.0.0.1:${port}`, VELORA_DATA_DIR:data, VELORA_WEB_DIR:web,
    VELORA_ICONS_DIR:join(application, 'panel/icons/dist'), ADMIN_USERNAME:'FixtureAdmin',
    // Changing bootstrap settings must not replace an existing account on restart.
    ADMIN_PASSWORD:checks.length ? 'Different-Synthetic-Bootstrap-4829!' : password,
    JWT_SECRET:'synthetic-acceptance-jwt-secret-4829-4829', PUBLIC_URL:origin, RUST_LOG:'error'});
  active = spawn(binary, [], {cwd:application, env, windowsHide:true, stdio:'ignore'});
  active.once('error', () => {});
  for (let i=0; i<100; i++) {
    if (active.exitCode !== null) throw Error('Fixture backend exited');
    try {if ((await fetch(origin+'/healthz', {signal:AbortSignal.timeout(1000)})).ok) return origin;} catch {}
    await new Promise(ok => setTimeout(ok, 200));
  }
  throw Error('Fixture health did not become ready');
}
async function request(origin, path, {method='GET', body, token, status=200}={}) {
  const response = await fetch(origin+path, {method, signal:AbortSignal.timeout(30000),
    headers:{'Content-Type':'application/json', ...(token ? {Authorization:`Bearer ${token}`} : {})},
    body:body===undefined ? undefined : JSON.stringify(body)});
  assert.equal(response.status, status, `${method} ${path}`);
  if (status >= 400) return;
  return response.json();
}
async function login(origin, username, pass) {
  const result = await request(origin, '/api/v1/auth/login', {method:'POST',body:{username,password:pass}});
  assert.equal(typeof result.token, 'string');
  return result.token;
}
function inventory(directory, prefix='') {
  const files = [];
  for (const entry of readdirSync(directory, {withFileTypes:true})) {
    const path = join(directory, entry.name), name = prefix+entry.name;
    assert(!entry.isSymbolicLink(), 'Fixture backup must not follow symlinks');
    if (entry.isDirectory()) files.push(...inventory(path, name+'/'));
    else {
      assert(entry.isFile(), 'Unsupported fixture backup entry');
      files.push({path:name, sha256:createHash('sha256').update(readFileSync(path)).digest('hex')});
    }
  }
  return files.sort((a,b) => a.path.localeCompare(b.path));
}
try {
  let origin = await start(original);
  let admin = await login(origin, 'FixtureAdmin', password);
  assert.equal((await request(origin, '/api/v1/auth/me', {token:admin})).role, 'admin');
  await request(origin, '/api/admin/users', {method:'POST', token:admin,
    body:{username:'FixturePlayer',password:playerPassword}, status:200});
  const instance = await request(origin, '/api/admin/instances', {method:'POST', token:admin,
    body:{name:'Restore fixture',mc_version:'1.21.1',loader:'vanilla'}});
  let player = await login(origin, 'FixturePlayer', playerPassword);
  await request(origin, '/api/admin/users', {token:player,status:403});
  assert((await request(origin, '/api/v1/launcher/manifest', {token:player})).instances.some(item => item.id===instance.id));
  const html = await fetch(origin+'/');
  assert(html.ok && (await html.text()).includes('<script'), 'Production SPA is unavailable');
  checks.push('fresh install, real admin/player login, instance visibility, admin API isolation, production SPA');
  await stop();

  const expected = inventory(original);
  assert(expected.length > 0);
  cpSync(confined(original), confined(backup), {recursive:true, errorOnExist:true, force:false});
  assert.deepEqual(inventory(backup), expected, 'Cold backup differs from stopped source');
  checks.push('cold backup includes exact database and synthetic identity bytes');

  origin = await start(original);
  admin = await login(origin, 'FixtureAdmin', password);
  const later = await request(origin, '/api/admin/instances', {method:'POST',token:admin,
    body:{name:'After backup',mc_version:'1.21.1',loader:'vanilla'}});
  checks.push('restart preserves original credentials despite changed bootstrap settings');
  await stop();

  assert.deepEqual(inventory(backup), expected, 'Backup changed after new source writes');
  cpSync(confined(backup), confined(restored), {recursive:true,errorOnExist:true,force:false});
  assert.deepEqual(inventory(restored), expected, 'Restored fixture checksum mismatch');
  origin = await start(restored);
  admin = await login(origin, 'FixtureAdmin', password);
  player = await login(origin, 'FixturePlayer', playerPassword);
  assert.equal((await request(origin, '/api/v1/auth/me', {token:admin})).role, 'admin');
  await request(origin, '/api/admin/users', {token:player,status:403});
  const manifest = await request(origin, '/api/v1/launcher/manifest', {token:player});
  assert(manifest.instances.some(item => item.id===instance.id));
  assert(!manifest.instances.some(item => item.id===later.id), 'Restore contains writes made after backup');
  checks.push('restore preserves users, credentials, roles and instances at the backup point');
  await stop();
  const report = {source:execFileSync('git',['-C',application,'rev-parse','HEAD']).toString().trim(),
    checks, backedUpFiles:expected.length, verifiedAt:new Date().toISOString(),
    scope:'Fresh synthetic data, cold backup and separate restored directory; no existing installation or live data cutover'};
  writeFileSync(join(root, 'acceptance.json'), JSON.stringify(report,null,2)+'\n');
  console.log(JSON.stringify(report));
} finally {await stop();}
