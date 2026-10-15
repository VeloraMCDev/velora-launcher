import assert from 'node:assert/strict';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import { appendFileSync, closeSync, openSync } from 'node:fs';
import { mkdir, readFile, realpath, readdir, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';

const instanceId = 'synthetic-native-vanilla';
const manifestRoute = '/api/v1/launcher/manifest';
const instanceRoute = `/api/v1/launcher/instances/${instanceId}`;
const maxRequests = 2048;
const maxCommandBytes = 4096;

function offlineUuid(username) {
  const digest = createHash('md5').update(`OfflinePlayer:${username}`).digest();
  digest[6] = (digest[6] & 0x0f) | 0x30;
  digest[8] = (digest[8] & 0x3f) | 0x80;
  const hex = digest.toString('hex');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function seedManifest() {
  return {
    api_version: 1,
    panel_version: 'synthetic-native-acceptance',
    user: null,
    auth: { panel_accounts: false, registration: 'closed', offline_local: true, yggdrasil_url: null },
    branding: {
      name: 'Synthetic Native Fixture', tagline: 'Local acceptance only; no game downloads.',
      logo_url: null, icon_url: null, player_guide: null, interface_icons: {},
      background: { kind: 'gradient', url: null, blur: 0, dim: 55 },
      colors: {
        accent: '#6d6af5', accent_2: '#8b88f8', background: '#0d0e12', surface: '#16181e',
        text: '#ecedf1', muted: '#8b8f9a', success: '#3fb97c', danger: '#e5484d',
      },
      font: 'Inter', radius: 10, glass: false, version_label: 'Synthetic', news: [], links: [],
      about: { title: 'Synthetic fixture', icon_url: null, body: 'No real accounts or stores.', links: [] },
      features: { news: false, server_status: false, allow_user_theme: true, allow_advanced_java: true },
      custom_css: '',
    },
    instances: [{
      id: instanceId, name: 'Synthetic Vanilla', description: 'Repair intentionally fails; cancellation is safe.',
      icon_url: null, banner_url: null, logo_url: null, mc_version: '1.21.1',
      loader: 'vanilla', loader_version: null, revision: 1, clean_epoch: 0, server: null,
      memory: { min_mb: 1024, max_mb: 2048 }, jvm_args: '', featured: true,
      source_label: 'Synthetic native acceptance', file_count: 0, total_size: 0,
      experience: { kind: 'vanilla', branding: null, features: [], navigation: [], widgets: [], modules: {} },
    }],
  };
}

async function createFixture() {
  const id = randomUUID();
  const root = join(await realpath(tmpdir()), `velora-native-${id}`);
  await mkdir(root, { mode: 0o700 });
  const privateFile = (name, content) => writeFile(join(root, name), content, { flag: 'wx', mode: 0o600 });
  const privateJson = (name, value) => privateFile(name, `${JSON.stringify(value, null, 2)}\n`);
  await privateJson('.velora-native-acceptance.json', { version: 1, id, purpose: 'synthetic-native-acceptance' });
  await privateFile('secrets.key', randomBytes(32).toString('base64'));
  const accountId = randomUUID();
  const username = 'NativeFixture';
  await privateJson('accounts.json', {
    accounts: [{ id: accountId, kind: 'offline', username, uuid: offlineUuid(username), panel_url: null, role: null }],
    active: accountId,
  });
  const manifest = seedManifest();
  await privateJson('manifest.json', manifest);
  const logDescriptor = openSync(join(root, 'requests.ndjson'), 'wx', 0o600);
  const timers = new Set();
  const responses = {
    manifest: { status: 200, delay_ms: 0 },
    instance: { status: 503, delay_ms: 0 },
  };
  let loggedRequests = 0;
  let stopped = false;
  const server = createServer((request, response) => {
    const path = (request.url ?? '').split('?', 1)[0];
    const knownRoute = path === manifestRoute || path === instanceRoute;
    if (loggedRequests < maxRequests) {
      appendFileSync(logDescriptor, `${JSON.stringify({ timestamp: new Date().toISOString(), route: knownRoute ? path : '/unmatched' })}\n`);
      loggedRequests += 1;
    }
    request.resume();
    const policy = request.method === 'GET' && knownRoute
      ? { ...(path === manifestRoute ? responses.manifest : responses.instance) }
      : { status: 404, delay_ms: 0 };
    const reply = () => {
      if (stopped || response.destroyed) return;
      response.writeHead(policy.status, { 'content-type': 'application/json', 'cache-control': 'no-store' });
      response.end(JSON.stringify(path === manifestRoute && policy.status === 200
        ? manifest : { error: policy.status === 404 ? 'not_found' : 'synthetic_fixture_error' }));
    };
    if (policy.delay_ms === 0) {
      reply();
    } else {
      const timer = setTimeout(() => { timers.delete(timer); reply(); }, policy.delay_ms);
      timers.add(timer);
      response.once('close', () => { clearTimeout(timer); timers.delete(timer); });
    }
  });
  server.requestTimeout = 5000;
  server.headersTimeout = 5000;
  server.maxConnections = 32;
  server.on('clientError', (_error, socket) => socket.destroy());
  const stop = () => {
    if (stopped) return;
    stopped = true;
    for (const timer of timers) clearTimeout(timer);
    timers.clear();
    server.closeAllConnections();
    server.close();
    closeSync(logDescriptor);
  };
  try {
    await new Promise((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '127.0.0.1', resolve);
    });
    const serverURL = `http://127.0.0.1:${server.address().port}`;
    await privateJson('settings.json', {
      panel_url: serverURL, selected_instance: instanceId, show_social_sidebar: false,
      check_updates: false, send_stats: false, ui_scale: 100,
    });
    return {
      root, serverURL, manifest, stop, timers,
      command(value) {
        const target = value?.action === 'instance-response' ? 'instance'
          : value?.action === 'manifest-response' ? 'manifest' : null;
        if (!target) throw new Error('invalid_command');
        const status = value.status ?? responses[target].status;
        const delay = value.delay_ms ?? responses[target].delay_ms;
        if (!Number.isInteger(status) || !((target === 'manifest' && status === 200) || (status >= 400 && status <= 599))
          || !Number.isInteger(delay) || delay < 0 || delay > 120000) throw new Error('invalid_command');
        responses[target] = { status, delay_ms: delay };
        return { action: value.action, ...responses[target] };
      },
    };
  } catch (error) {
    stop();
    throw error;
  }
}

async function selfTest(fixture) {
  const jsonFile = async (name) => JSON.parse(await readFile(join(fixture.root, name), 'utf8'));
  const marker = await jsonFile('.velora-native-acceptance.json');
  assert.equal(basename(fixture.root), `velora-native-${marker.id}`);
  assert.match(marker.id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.deepEqual(marker, { version: 1, id: marker.id, purpose: 'synthetic-native-acceptance' });
  assert.equal(await realpath(fixture.root), fixture.root);
  assert.equal(Buffer.from(await readFile(join(fixture.root, 'secrets.key'), 'utf8'), 'base64').length, 32);
  assert.deepEqual((await readdir(fixture.root)).sort(), [
    '.velora-native-acceptance.json', 'accounts.json', 'manifest.json', 'requests.ndjson', 'secrets.key', 'settings.json',
  ]);
  if (process.platform !== 'win32') {
    assert.equal((await stat(fixture.root)).mode & 0o777, 0o700);
    for (const file of await readdir(fixture.root)) assert.equal((await stat(join(fixture.root, file))).mode & 0o777, 0o600);
  }
  const accounts = await jsonFile('accounts.json');
  assert.equal(accounts.accounts.length, 1);
  assert.equal(accounts.active, accounts.accounts[0].id);
  assert.equal(accounts.accounts[0].kind, 'offline');
  assert.equal(accounts.accounts[0].uuid, offlineUuid(accounts.accounts[0].username));
  assert.equal(offlineUuid('Notch'), 'b50ad385-829d-3141-a216-7e7d7539ba7f');
  assert.deepEqual(await jsonFile('settings.json'), {
    panel_url: fixture.serverURL, selected_instance: instanceId, show_social_sidebar: false,
    check_updates: false, send_stats: false, ui_scale: 100,
  });
  assert.deepEqual(await jsonFile('manifest.json'), fixture.manifest);
  const fetchRoute = (route, options) => fetch(`${fixture.serverURL}${route}`, options);
  assert.deepEqual(await (await fetchRoute(manifestRoute)).json(), fixture.manifest);
  assert.equal((await fetchRoute(instanceRoute)).status, 503);
  assert.equal((await fetchRoute('/secrets.key?token=not-logged')).status, 404);
  assert.equal((await fetchRoute(manifestRoute, { method: 'POST', body: 'not-logged' })).status, 404);
  fixture.command({ action: 'manifest-response', status: 503 });
  assert.equal((await fetchRoute(manifestRoute)).status, 503);
  fixture.command({ action: 'manifest-response', status: 200 });
  assert.deepEqual(await (await fetchRoute(manifestRoute)).json(), fixture.manifest);
  fixture.command({ action: 'instance-response', status: 502, delay_ms: 40 });
  const started = performance.now();
  assert.equal((await fetchRoute(instanceRoute)).status, 502);
  assert.ok(performance.now() - started >= 30);
  for (const command of [
    { action: 'instance-response', status: 200 }, { action: 'instance-response', status: 302 },
    { action: 'manifest-response', status: 201 }, { action: 'instance-response', delay_ms: -1 },
    { action: 'instance-response', delay_ms: 120001 }, { action: 'unknown' },
  ]) assert.throws(() => fixture.command(command), /invalid_command/);
  fixture.command({ action: 'instance-response', delay_ms: 120000 });
  await assert.rejects(fetchRoute(instanceRoute, { signal: AbortSignal.timeout(40) }));
  await new Promise((resolve) => setTimeout(resolve, 40));
  assert.equal(fixture.timers.size, 0);
  const log = await readFile(join(fixture.root, 'requests.ndjson'), 'utf8');
  assert.ok(!log.includes('not-logged'));
  for (const line of log.trim().split('\n')) {
    const entry = JSON.parse(line);
    assert.deepEqual(Object.keys(entry), ['timestamp', 'route']);
    assert.ok(Number.isFinite(Date.parse(entry.timestamp)));
  }
  fixture.stop();
  assert.equal(fixture.timers.size, 0);
  assert.ok((await readdir(fixture.root)).includes('manifest.json'));
  process.stdout.write(`${JSON.stringify({ selftest: 'passed', root: fixture.root, serverURL: fixture.serverURL })}\n`);
}

async function main() {
  if (process.argv.includes('--help')) {
    process.stdout.write('Node 24+: node launcher/scripts/native-fixture.mjs [--selftest]\n'
      + 'Prints {root,serverURL}; retains private fixture on SIGINT/SIGTERM. No artifact downloads.\n'
      + 'stdin JSON lines: {"action":"instance-response","status":503,"delay_ms":10000}\n'
      + 'or {"action":"manifest-response","status":503} (200 restores online).\n'
      + 'Instance status must be 400-599; delays 0-120000 ms apply to new requests.\n');
    return;
  }
  if (Number(process.versions.node.split('.')[0]) < 24) throw new Error('Node 24 or newer required');
  if (process.argv.slice(2).some((argument) => argument !== '--selftest')) throw new Error('unknown option');
  const fixture = await createFixture();
  if (process.argv.includes('--selftest')) {
    try { await selfTest(fixture); } finally { fixture.stop(); }
    return;
  }
  let pending = '';
  let oversized = false;
  const onData = (chunk) => {
    for (const character of chunk) {
      if (character === '\n') {
        try {
          if (oversized) throw new Error('invalid_command');
          if (pending.trim()) process.stderr.write(`${JSON.stringify(fixture.command(JSON.parse(pending)))}\n`);
        } catch {
          process.stderr.write('{"error":"invalid_command"}\n');
        }
        pending = '';
        oversized = false;
      } else if (!oversized) {
        pending += character;
        if (Buffer.byteLength(pending) > maxCommandBytes) { pending = ''; oversized = true; }
      }
    }
  };
  const shutdown = () => {
    process.stdin.off('data', onData);
    process.stdin.pause();
    fixture.stop();
    process.off('SIGINT', shutdown);
    process.off('SIGTERM', shutdown);
  };
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', onData);
  process.stdout.write(`${JSON.stringify({ root: fixture.root, serverURL: fixture.serverURL })}\n`);
}

main().catch(() => {
  process.stderr.write('{"error":"fixture_failed"}\n');
  process.exitCode = 1;
});
