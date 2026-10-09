// Velora Operations: health, deployment and release control for the production stack.
import { createServer } from 'node:http';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { extname, join, normalize, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as auth from './auth.mjs';
import * as actions from './actions.mjs';
import { containerLogs, listContainers } from './docker.mjs';
import { createMonitor, STACK_PROJECT } from './monitor.mjs';
import { applySettingsUpdate, audit, loadSettings, publicSettings, readAudit, saveSettings } from './store.mjs';
import { sendEmail } from './notify.mjs';

const PORT = Number(process.env.PORT ?? 8080);
const SECURE = process.env.OPS_INSECURE_COOKIES !== '1';
const WEB = resolve(fileURLToPath(new URL('../web/dist', import.meta.url)));
const VERSION = process.env.OPS_VERSION ?? 'development';
const monitor = createMonitor();

const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.woff2': 'font/woff2', '.json': 'application/json', '.ico': 'image/x-icon' };
const SECURITY_HEADERS = {
  'x-content-type-options': 'nosniff',
  'referrer-policy': 'same-origin',
  'x-frame-options': 'DENY',
  'content-security-policy': "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
  'permissions-policy': 'camera=(), microphone=(), geolocation=()',
};

function send(res, status, body, headers = {}) {
  const json = typeof body !== 'string' && !Buffer.isBuffer(body);
  res.writeHead(status, { ...SECURITY_HEADERS, 'cache-control': 'no-store', ...(json ? { 'content-type': 'application/json' } : {}), ...headers });
  res.end(json ? JSON.stringify(body) : body);
}
const fail = (res, e) => send(res, e.status && e.status < 600 ? e.status : 500, { error: e.message ?? 'Unexpected error' });

async function readBody(req, limit = 64 * 1024) {
  let size = 0; const chunks = [];
  for await (const chunk of req) {
    size += chunk.length;
    if (size > limit) throw Object.assign(new Error('Request too large'), { status: 413 });
    chunks.push(chunk);
  }
  if (!size) return {};
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8')); } catch { throw Object.assign(new Error('Invalid JSON'), { status: 400 }); }
}
const clientIp = req => String(req.headers['x-forwarded-for'] ?? '').split(',')[0].trim() || req.socket.remoteAddress;

function serveStatic(req, res) {
  const url = new URL(req.url, 'http://ops');
  let path = normalize(join(WEB, decodeURIComponent(url.pathname)));
  if (!path.startsWith(WEB)) return send(res, 404, 'Not found', { 'content-type': 'text/plain' });
  if (!existsSync(path) || statSync(path).isDirectory()) path = join(WEB, 'index.html');
  if (!existsSync(path)) return send(res, 503, 'Dashboard assets are not built', { 'content-type': 'text/plain' });
  const immutable = /\/assets\/.+-[A-Za-z0-9_-]{8,}\.\w+$/.test(path.replaceAll('\\', '/'));
  send(res, 200, readFileSync(path), { 'content-type': TYPES[extname(path)] ?? 'application/octet-stream', 'cache-control': immutable ? 'public, max-age=31536000, immutable' : 'no-cache' });
}

const routes = [];
const route = (method, pattern, handler, { open = false } = {}) => routes.push({ method, pattern: new RegExp(`^${pattern}$`), handler, open });

// ----- session -----
route('GET', '/healthz', (req, res) => send(res, 200, { ok: true, version: VERSION }), { open: true });
route('POST', '/api/login', async (req, res) => {
  const { username, password } = await readBody(req);
  const session = await auth.login(username, password, clientIp(req));
  audit({ user: session.username, action: 'sign-in', ip: clientIp(req), result: 'ok' });
  send(res, 200, { username: session.username, csrf: session.csrf, version: VERSION }, { 'set-cookie': auth.sessionCookie(session, SECURE) });
}, { open: true });
route('GET', '/api/session', async (req, res) => {
  const session = await auth.sessionFor(req);
  send(res, 200, session ? { username: session.username, csrf: session.csrf, version: VERSION } : { username: null, version: VERSION });
}, { open: true });
route('POST', '/api/logout', (req, res, session) => {
  auth.logout(session.id);
  send(res, 200, { ok: true }, { 'set-cookie': auth.clearCookie(SECURE) });
});

// ----- monitoring -----
route('GET', '/api/overview', (req, res) => send(res, 200, {
  snapshot: monitor.state.snapshot, alerts: Object.values(monitor.state.alerts.active), jobs: actions.listJobs(), project: STACK_PROJECT,
  updates: { checked: monitor.state.updates.checked, launcher_pending: (monitor.state.updates.launcher?.releases ?? []).filter(r => !r.approved && r.has_manifest && !r.prerelease).length,
    images: Object.fromEntries(Object.entries(monitor.state.updates.images ?? {}).map(([k, v]) => [k, Boolean(v.releases?.[0] && v.current && v.releases[0].digest !== v.current)])),
    containers: (monitor.state.updates.containers ?? []).filter(c => c.status === 'update-available').length },
}));
route('GET', '/api/history', (req, res) => send(res, 200, monitor.state.history));
route('GET', '/api/containers/([A-Za-z0-9_.-]+)/logs', async (req, res, session, [name]) => {
  const container = (await listContainers()).find(c => c.Names.includes(`/${name}`));
  if (!container || container.Labels['com.docker.compose.project'] !== STACK_PROJECT) throw Object.assign(new Error('Logs are available for Velora services only'), { status: 403 });
  const tail = new URL(req.url, 'http://ops').searchParams.get('tail');
  send(res, 200, { name, logs: await containerLogs(container.Id, tail) });
});

// ----- updates & releases -----
route('GET', '/api/updates', (req, res) => send(res, 200, { ...monitor.state.updates, dashboard_update: actions.dashboardUpdateStatus() }));
route('POST', '/api/updates/check', async (req, res) => send(res, 200, { ...await monitor.checkUpdates(true), dashboard_update: actions.dashboardUpdateStatus() }));

// ----- actions -----
const job = (res, session, kind, target, fn) => send(res, 202, actions.startJob(kind, session.username, target, fn, { notify: monitor.notify }));
route('POST', '/api/actions/restart', async (req, res, s) => { const { service } = await readBody(req); job(res, s, 'restart', service, actions.restart(service)); });
route('POST', '/api/actions/pull', async (req, res, s) => { const { service } = await readBody(req); job(res, s, 'pull-and-recreate', service, actions.pullService(service)); });
route('POST', '/api/actions/redeploy', (req, res, s) => job(res, s, 'redeploy-stack', STACK_PROJECT, actions.redeploy()));
route('POST', '/api/actions/backup', (req, res, s) => job(res, s, 'backup', 'panel-data', log => actions.requestBackup(log).then(last => `Archive ${last.archive}`)));
route('POST', '/api/actions/update-image', async (req, res, s) => {
  const { name, image } = await readBody(req);
  const known = monitor.state.updates.images?.[name]?.releases?.some(r => r.image === image);
  if (!known) throw Object.assign(new Error('Pick one of the listed release builds'), { status: 400 });
  job(res, s, 'update-image', `${name} → ${String(image).split('@')[1]?.slice(0, 19)}`, actions.updateImage(name, image));
});
route('POST', '/api/actions/rebuild-docs', async (req, res, s) => { const { ref = 'main' } = await readBody(req); job(res, s, 'rebuild-docs', ref, actions.rebuildDocs(ref)); });
route('POST', '/api/actions/approve-launcher', async (req, res, s) => {
  const { tag } = await readBody(req);
  job(res, s, 'approve-launcher', tag, async log => { const out = await actions.approveLauncher(tag, s.token)(log); monitor.checkUpdates().catch(() => {}); return out; });
});
route('GET', '/api/jobs', (req, res) => send(res, 200, actions.listJobs()));
route('GET', '/api/jobs/([0-9a-f-]{36})', (req, res, s, [id]) => {
  const j = actions.getJob(id);
  if (!j) throw Object.assign(new Error('Job not found'), { status: 404 });
  send(res, 200, j);
});

// ----- Panel activity (proxied with the admin's own Panel token) -----
route('GET', '/api/panel/summary', async (req, res, s) => send(res, 200, await auth.panelFetch('/api/admin/operations/summary', { token: s.token })));
route('GET', '/api/panel/activity', async (req, res, s) => {
  const q = new URL(req.url, 'http://ops').searchParams;
  const params = new URLSearchParams();
  for (const key of ['source', 'player', 'offset']) if (q.get(key)) params.set(key, q.get(key).slice(0, 64));
  send(res, 200, await auth.panelFetch(`/api/admin/activity?${params}`, { token: s.token }));
});
route('GET', '/api/audit', (req, res) => send(res, 200, readAudit(300)));

// ----- settings -----
route('GET', '/api/settings', (req, res) => send(res, 200, publicSettings()));
route('PUT', '/api/settings', async (req, res, s) => {
  const next = applySettingsUpdate(loadSettings(), await readBody(req));
  saveSettings(next);
  audit({ user: s.username, action: 'settings', result: 'saved' });
  send(res, 200, publicSettings(next));
});
route('POST', '/api/settings/test-email', async (req, res, s) => {
  await sendEmail(loadSettings(), { title: 'Test notification', severity: 'info', lines: [`${s.username} sent this test from the Velora operations dashboard.`, 'Alerts for the events you enabled will arrive like this.'] });
  audit({ user: s.username, action: 'test-email', result: 'sent' });
  send(res, 200, { ok: true });
});

export const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, 'http://ops');
    const match = routes.find(r => r.method === req.method && r.pattern.test(url.pathname));
    if (!match) {
      if (url.pathname.startsWith('/api/')) return send(res, 404, { error: 'Not found' });
      if (req.method !== 'GET' && req.method !== 'HEAD') return send(res, 405, { error: 'Method not allowed' });
      return serveStatic(req, res);
    }
    let session = null;
    if (!match.open) {
      session = await auth.sessionFor(req);
      if (!session) return send(res, 401, { error: 'Sign in again' });
      if (req.method !== 'GET' && !auth.csrfOk(req, session)) return send(res, 403, { error: 'Missing or invalid CSRF token' });
    }
    await match.handler(req, res, session, url.pathname.match(match.pattern).slice(1));
  } catch (e) {
    if (!(e.status < 500)) console.error(req.method, req.url, e);
    fail(res, e);
  }
});

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  monitor.start();
  server.listen(PORT, () => console.log(`Velora Operations ${VERSION} listening on :${PORT}`));
  for (const signal of ['SIGTERM', 'SIGINT']) process.on(signal, () => server.close(() => process.exit(0)));
}
