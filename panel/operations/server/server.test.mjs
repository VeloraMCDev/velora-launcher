import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

process.env.OPS_DATA_DIR = mkdtempSync(join(tmpdir(), 'velora-ops-'));
const store = await import('./store.mjs');
const { setImageDigest, currentImage } = await import('./actions.mjs');
const monitor = await import('./monitor.mjs');
const { parseImage } = await import('./updates.mjs');
const { sendEmail, renderEmail } = await import('./notify.mjs');
const auth = await import('./auth.mjs');
const { demux } = await import('./docker.mjs');
test.after(() => rmSync(process.env.OPS_DATA_DIR, { recursive: true, force: true }));

const digest = c => `sha256:${c.repeat(64)}`;
const env = `STACK_DIR=/srv/velora
PANEL_IMAGE=ghcr.io/veloramcdev/velora-panel@${digest('a')}
OPS_IMAGE=ghcr.io/veloramcdev/velora-operations@${digest('d')}
`;

test('.env edits replace only the named pinned image', () => {
  const next = `ghcr.io/veloramcdev/velora-panel@${digest('b')}`;
  const updated = setImageDigest(env, 'velora-panel', next);
  assert.equal(currentImage(updated, 'velora-panel'), next);
  assert.equal(currentImage(updated, 'velora-operations'), `ghcr.io/veloramcdev/velora-operations@${digest('d')}`);
  assert.match(updated, /^STACK_DIR=\/srv\/velora$/m);
  assert.throws(() => setImageDigest(env, 'velora-panel', 'ghcr.io/veloramcdev/velora-panel:latest'), /Invalid/);
  assert.throws(() => setImageDigest(env, 'velora-panel', `ghcr.io/evil/other@${digest('c')}`), /Invalid/);
  assert.throws(() => setImageDigest('STACK_DIR=/x\n', 'velora-operations', `ghcr.io/veloramcdev/velora-operations@${digest('c')}`), /no pinned/);
  assert.throws(() => setImageDigest(env, 'other', 'x'), /Unknown/);
  assert.throws(() => setImageDigest(env, 'other', 'x'), /Unknown/);
});

test('settings updates validate input and never echo secrets', () => {
  const current = store.loadSettings();
  const next = store.applySettingsUpdate(current, { notifications: { enabled: true, resend_api_key: 're_abcdefgh12345', from: 'Velora <alerts@example.org>', recipients: ['a@example.org', 'a@example.org'], events: { disk_low: false, bogus: true } } });
  assert.equal(next.notifications.recipients.length, 1);
  assert.equal(next.notifications.events.disk_low, false);
  assert.ok(!('bogus' in next.notifications.events));
  const visible = store.publicSettings(next);
  assert.equal(visible.notifications.resend_api_key, '');
  assert.equal(visible.notifications.resend_api_key_set, true);
  assert.equal(store.applySettingsUpdate(next, { notifications: { resend_api_key: '' } }).notifications.resend_api_key, 're_abcdefgh12345');
  assert.throws(() => store.applySettingsUpdate(current, { notifications: { resend_api_key: 'sk-not-resend' } }), /Resend/);
  assert.throws(() => store.applySettingsUpdate(current, { notifications: { recipients: ['nope'] } }), /recipient/);
  assert.throws(() => store.applySettingsUpdate(current, { alerts: { disk_percent: 20 } }), /Disk/);
  store.saveSettings(next);
  assert.equal(store.loadSettings().notifications.from, 'Velora <alerts@example.org>');
});

test('alerts cover services, endpoints, backups, disk and certificates', () => {
  const settings = store.loadSettings();
  const now = Date.parse('2026-10-09T12:00:00Z');
  const healthy = {
    containers: [{ project: 'velora-platform', service: 'panel', state: 'running', health: 'healthy', status: 'Up' }],
    endpoints: [{ name: 'Panel', url: 'https://x/health', ok: true, status: 200 }],
    backups: { last: { status: 'ok', offsite: true, finished_at: '2026-10-09T09:40:00Z' } },
    host: { disk: { percent: 48 } },
    certificates: [{ host: 'x', expires: '2026-12-01T00:00:00Z' }],
  };
  assert.deepEqual(monitor.evaluate(healthy, settings, now), []);
  const broken = structuredClone(healthy);
  broken.containers[0].health = 'unhealthy';
  broken.endpoints[0] = { ...broken.endpoints[0], ok: false, status: 502 };
  broken.backups.last.finished_at = '2026-10-07T09:40:00Z';
  broken.host.disk.percent = 95;
  broken.certificates[0].expires = '2026-10-15T00:00:00Z';
  const keys = monitor.evaluate(broken, settings, now).map(p => p.key).sort();
  assert.deepEqual(keys, ['backup', 'cert:x', 'container:panel', 'disk', 'endpoint:Panel']);
  broken.backups.last = { status: 'failed', message: 'disk full', finished_at: '2026-10-09T09:40:00Z' };
  assert.equal(monitor.evaluate(broken, settings, now).find(p => p.key === 'backup').severity, 'critical');
});

test('release notices are sent once per release', () => {
  const updates = {
    launcher: { releases: [{ tag: 'launcher-v1.3.0', version: '1.3.0', approved: false, prerelease: false, has_manifest: true }] },
    images: { 'velora-panel': { current: digest('a'), releases: [{ digest: digest('b'), commit: 'c'.repeat(40) }] } },
  };
  const first = monitor.notices(updates, []);
  assert.deepEqual(first.map(n => n.event), ['launcher_release', 'image_update']);
  assert.deepEqual(monitor.notices(updates, first.map(n => n.id)), []);
});

test('host metrics parse /proc formats', () => {
  const a = monitor.cpuPercent('cpu  100 0 100 800 0 0 0 0 0 0\n', null);
  const b = monitor.cpuPercent('cpu  150 0 150 900 0 0 0 0 0 0\n', a.sample);
  assert.equal(Math.round(b.percent), 50);
  const m = monitor.memory('MemTotal:       1000 kB\nMemFree: 100 kB\nMemAvailable:    250 kB\n');
  assert.equal(m.used, 750 * 1024);
  assert.deepEqual(monitor.parseEndpoints('Panel=https://a/health, Docs=https://b/'), [{ name: 'Panel', url: 'https://a/health' }, { name: 'Docs', url: 'https://b/' }]);
  assert.throws(() => monitor.parseEndpoints('broken'), /Invalid/);
});

test('image references resolve to registries', () => {
  assert.deepEqual(parseImage('redis:8.6.5-alpine'), { registry: 'registry-1.docker.io', repository: 'library/redis', tag: '8.6.5-alpine', pinned: false });
  assert.equal(parseImage('lscr.io/linuxserver/sonarr:latest').registry, 'lscr.io');
  assert.equal(parseImage('fnsys/dockhand').tag, 'latest');
  assert.equal(parseImage(`ghcr.io/veloramcdev/velora-panel@${digest('a')}`).pinned, true);
});

test('Resend emails carry the configured sender and recipients', async () => {
  let sent;
  const settings = { notifications: { resend_api_key: 're_test12345678', from: 'ops@example.org', recipients: ['me@example.org'] } };
  await sendEmail(settings, { title: 'Panel down', severity: 'critical', lines: ['<b>502</b>'] }, async (url, init) => {
    sent = { url, init };
    return new Response(JSON.stringify({ id: 'x' }), { status: 200 });
  });
  const body = JSON.parse(sent.init.body);
  assert.equal(sent.url, 'https://api.resend.com/emails');
  assert.equal(sent.init.headers.authorization, 'Bearer re_test12345678');
  assert.deepEqual(body.to, ['me@example.org']);
  assert.equal(body.subject, '[Velora] Panel down');
  assert.match(body.html, /&lt;b&gt;502/);
  assert.match(renderEmail({ title: 't', lines: [] }).text, /Velora Operations/);
  await assert.rejects(sendEmail({ notifications: { resend_api_key: '', from: '', recipients: [] } }, { title: 'x' }), /Configure/);
  await assert.rejects(sendEmail(settings, { title: 'x' }, async () => new Response('{"message":"invalid from"}', { status: 422 })), /422: invalid from/);
});

test('sign-in throttling and cookie parsing', () => {
  const now = Date.now();
  for (let i = 0; i < 10; i++) assert.ok(auth.allowAttempt('203.0.113.9', now));
  assert.ok(!auth.allowAttempt('203.0.113.9', now));
  assert.ok(auth.allowAttempt('203.0.113.9', now + 16 * 60_000));
  assert.deepEqual(auth.parseCookies('a=1; velora_ops=abc%3D; b=2'), { a: '1', velora_ops: 'abc=', b: '2' });
  const cookie = auth.sessionCookie({ id: 'x', expires: Date.now() + 60_000 });
  assert.match(cookie, /HttpOnly; SameSite=Strict; Max-Age=\d+; Secure/);
});

test('docker log frames are demultiplexed', () => {
  const frame = (stream, text) => { const b = Buffer.alloc(8 + text.length); b[0] = stream; b.writeUInt32BE(text.length, 4); b.write(text, 8); return b; };
  assert.equal(demux(Buffer.concat([frame(1, 'out\n'), frame(2, 'err\n')])), 'out\nerr\n');
  assert.equal(demux(Buffer.from('plain tty text')), 'plain tty text');
});
