// Periodic health collection, metric history and alert evaluation.
import { readFileSync, readdirSync, statSync, statfsSync } from 'node:fs';
import { connect } from 'node:tls';
import { join } from 'node:path';
import { containerStats, inspectImage, listContainers } from './docker.mjs';
import { imageUpdate, launcherReleases, releaseImages, commitInfo } from './updates.mjs';
import { loadSettings, loadState, saveState, audit } from './store.mjs';
import { sendEmail } from './notify.mjs';
import { PANEL_INTERNAL_URL } from './auth.mjs';

export const STACK_PROJECT = process.env.STACK_PROJECT ?? 'velora-platform';
export const STACK_DIR = process.env.STACK_DIR ?? '/home/ubuntu/velora-platform';
export const BACKUP_STATE_DIR = process.env.BACKUP_STATE_DIR ?? '/backup-state';
export const BACKUP_ARCHIVE_DIR = process.env.BACKUP_ARCHIVE_DIR ?? '/backup-archives';
export const PUBLIC_URL = process.env.OPS_PUBLIC_URL ?? 'https://ops.velora.scopedd.lol';

/** OPS_ENDPOINTS="Panel=https://velora.example/health,Docs=https://docs.velora.example/" */
export function parseEndpoints(value = process.env.OPS_ENDPOINTS ?? '') {
  return value.split(',').map(s => s.trim()).filter(Boolean).map(entry => {
    const at = entry.indexOf('=');
    const name = entry.slice(0, at).trim(), url = entry.slice(at + 1).trim();
    if (at < 1 || !/^https?:\/\//.test(url)) throw new Error(`Invalid OPS_ENDPOINTS entry: ${entry}`);
    return { name, url };
  });
}

// ---------- host metrics ----------
let lastCpu = null;
export function cpuPercent(stat, previous) {
  const fields = stat.split('\n')[0].trim().split(/\s+/).slice(1).map(Number);
  const idle = fields[3] + (fields[4] || 0);
  const total = fields.reduce((a, b) => a + b, 0);
  const sample = { idle, total };
  if (!previous || total <= previous.total) return { sample, percent: null };
  return { sample, percent: Math.max(0, Math.min(100, 100 * (1 - (idle - previous.idle) / (total - previous.total)))) };
}
export function memory(meminfo) {
  const kb = key => Number(meminfo.match(new RegExp(`^${key}:\\s+(\\d+)`, 'm'))?.[1] ?? 0) * 1024;
  const total = kb('MemTotal'), available = kb('MemAvailable');
  return { total, used: total - available, percent: total ? (100 * (total - available)) / total : 0 };
}
function host() {
  const cpu = cpuPercent(readFileSync('/proc/stat', 'utf8'), lastCpu);
  lastCpu = cpu.sample;
  const [l1, l5, l15] = readFileSync('/proc/loadavg', 'utf8').split(' ').map(Number);
  let disk = null;
  try {
    const s = statfsSync(STACK_DIR);
    const total = s.blocks * s.bsize, free = s.bavail * s.bsize;
    disk = { total, used: total - free, percent: (100 * (total - free)) / total };
  } catch { /* stack directory not mounted in development */ }
  return { cpu: cpu.percent, load: [l1, l5, l15], cores: (readFileSync('/proc/cpuinfo', 'utf8').match(/^processor/gm) ?? []).length || 1,
    memory: memory(readFileSync('/proc/meminfo', 'utf8')), disk, uptime: Number(readFileSync('/proc/uptime', 'utf8').split(' ')[0]) };
}

// ---------- endpoints & certificates ----------
async function probe({ name, url }) {
  const started = performance.now();
  try {
    const response = await fetch(url, { redirect: 'manual', signal: AbortSignal.timeout(10_000) });
    await response.arrayBuffer();
    return { name, url, ok: response.status < 400, status: response.status, ms: Math.round(performance.now() - started) };
  } catch (e) {
    return { name, url, ok: false, status: 0, ms: null, error: e.cause?.code ?? e.message };
  }
}
export function certificateExpiry(hostname, port = 443) {
  return new Promise(resolve => {
    const socket = connect({ host: hostname, port, servername: hostname, timeout: 10_000 }, () => {
      const cert = socket.getPeerCertificate();
      socket.end();
      resolve(cert?.valid_to ? { host: hostname, expires: new Date(cert.valid_to).toISOString(), issuer: cert.issuer?.O ?? cert.issuer?.CN } : { host: hostname, error: 'no certificate' });
    });
    socket.on('error', e => resolve({ host: hostname, error: e.message }));
    socket.on('timeout', () => { socket.destroy(); resolve({ host: hostname, error: 'timeout' }); });
  });
}

// ---------- backups ----------
function backups() {
  let last = null;
  try { last = JSON.parse(readFileSync(join(BACKUP_STATE_DIR, 'last.json'), 'utf8')); } catch { /* none yet */ }
  let archives = [];
  try {
    archives = readdirSync(BACKUP_ARCHIVE_DIR).filter(f => /^velora-panel-data-.*\.tar\.gz$/.test(f)).sort().reverse()
      .map(f => ({ name: f, bytes: statSync(join(BACKUP_ARCHIVE_DIR, f)).size, at: statSync(join(BACKUP_ARCHIVE_DIR, f)).mtime.toISOString() }));
  } catch { /* archive directory not mounted */ }
  let pending = false;
  try { statSync(join(BACKUP_STATE_DIR, 'request')); pending = true; } catch { /* no request */ }
  return { last, archives, pending };
}

// ---------- alerts ----------
/** Pure alert evaluation: returns the active problems for a snapshot. */
export function evaluate(snapshot, settings, now = Date.now()) {
  const problems = [];
  const add = (key, event, severity, title, detail) => problems.push({ key, event, severity, title, detail });
  for (const c of snapshot.containers.filter(c => c.project === STACK_PROJECT)) {
    if (c.state !== 'running') add(`container:${c.service}`, 'service_down', 'critical', `${c.service} is ${c.state}`, c.status);
    else if (c.health === 'unhealthy') add(`container:${c.service}`, 'service_down', 'critical', `${c.service} is unhealthy`, c.status);
  }
  for (const e of snapshot.endpoints) if (!e.ok) add(`endpoint:${e.name}`, 'endpoint_down', 'critical', `${e.name} is unreachable`, `${e.url} → ${e.status || e.error}`);
  const last = snapshot.backups.last;
  if (last?.status === 'failed') add('backup', 'backup_failed', 'critical', 'The latest backup failed', last.message);
  else if (!last || now - Date.parse(last.finished_at) > settings.alerts.backup_max_age_hours * 3600_000) {
    add('backup', 'backup_failed', 'warning', 'Backups are overdue', last ? `Last successful backup finished ${last.finished_at}` : 'No backup has run yet');
  } else if (!last.offsite) add('backup', 'backup_failed', 'warning', 'The latest backup did not reach Hermes', last.message);
  if (snapshot.host.disk?.percent >= settings.alerts.disk_percent) add('disk', 'disk_low', 'warning', `Disk ${snapshot.host.disk.percent.toFixed(0)}% full`, 'Free space on the VPS before backups or updates fail.');
  for (const c of snapshot.certificates ?? []) {
    const days = c.expires ? (Date.parse(c.expires) - now) / 86400_000 : null;
    if (days !== null && days < settings.alerts.certificate_days) add(`cert:${c.host}`, 'certificate_expiring', 'warning', `Certificate for ${c.host} expires in ${Math.max(0, Math.floor(days))} days`, `Issued by ${c.issuer ?? 'unknown'}`);
  }
  return problems;
}

/** One-off notices (new releases) are sent once per release identity. */
export function notices(updates, seen) {
  const out = [];
  for (const r of updates.launcher?.releases ?? []) {
    if (!r.approved && !r.prerelease && r.has_manifest && !seen.includes(`launcher:${r.tag}`)) {
      out.push({ id: `launcher:${r.tag}`, event: 'launcher_release', title: `Launcher ${r.version} is ready for approval`, detail: 'Review and approve it under Updates to publish it to players.' });
    }
  }
  for (const [name, image] of Object.entries(updates.images ?? {})) {
    const latest = image.releases?.[0];
    if (latest && image.current && latest.digest !== image.current && !seen.includes(`image:${name}:${latest.digest}`)) {
      out.push({ id: `image:${name}:${latest.digest}`, event: 'image_update', title: `A new ${name} build is available`, detail: latest.commitInfo?.message ?? latest.commit });
    }
  }
  return out;
}

// ---------- the monitor loop ----------
export function createMonitor({ images = { 'velora-panel': 'panel', 'velora-operations': 'ops' } } = {}) {
  const endpoints = parseEndpoints();
  const history = loadState('history', []);
  const alertState = loadState('alerts', { active: {}, seen: [] });
  const state = { snapshot: null, updates: { checked: null }, history, alerts: alertState, certificates: [], certChecked: 0, updatesChecked: 0, containerUpdatesChecked: 0 };

  async function collect() {
    const all = await listContainers();
    const containers = all.map(c => ({
      id: c.Id.slice(0, 12), name: c.Names[0]?.replace(/^\//, ''), image: c.Image, state: c.State, status: c.Status,
      health: c.Status.match(/\((healthy|unhealthy|health: starting)\)/)?.[1] ?? null, created: c.Created,
      project: c.Labels['com.docker.compose.project'] ?? null, service: c.Labels['com.docker.compose.service'] ?? null,
      ports: [...new Set((c.Ports ?? []).filter(p => p.PublicPort).map(p => `${p.PublicPort}→${p.PrivatePort}/${p.Type}`))],
      helper: c.Labels['org.velora.operations.helper'] === 'true',
    })).filter(c => !c.helper);
    await Promise.all(containers.filter(c => c.project === STACK_PROJECT && c.state === 'running').map(async c => {
      c.stats = await containerStats(c.id).catch(() => null);
    }));
    const probes = await Promise.all([{ name: 'Panel (internal)', url: `${PANEL_INTERNAL_URL}/health` }, ...endpoints].map(probe));
    if (Date.now() - state.certChecked > 3600_000) {
      const hosts = [...new Set(endpoints.map(e => new URL(e.url)).filter(u => u.protocol === 'https:').map(u => u.hostname))];
      state.certificates = await Promise.all(hosts.map(h => certificateExpiry(h)));
      state.certChecked = Date.now();
    }
    const snapshot = { at: new Date().toISOString(), host: host(), containers, endpoints: probes, backups: backups(), certificates: state.certificates };
    state.snapshot = snapshot;
    const panel = containers.find(c => c.project === STACK_PROJECT && c.service === 'panel');
    const last = history.at(-1);
    if (!last || Date.now() - Date.parse(last.t) >= 55_000) {
      history.push({ t: snapshot.at, cpu: snapshot.host.cpu, mem: snapshot.host.memory.percent, load: snapshot.host.load[0],
        panel_ms: probes.find(p => p.name !== 'Panel (internal)' && /health/.test(p.url))?.ms ?? null,
        panel_cpu: panel?.stats?.cpu ?? null, panel_mem: panel?.stats?.memory ?? null });
      while (history.length > 1440) history.shift();
      if (history.length % 10 === 0) saveState('history', history);
    }
    await alert(snapshot);
  }

  async function notify(event, message) {
    const settings = loadSettings();
    if (!settings.notifications.enabled || !settings.notifications.events[event]) return;
    try { await sendEmail(settings, { ...message, link: PUBLIC_URL }); }
    catch (e) { console.error('notification failed:', e.message); }
  }

  async function alert(snapshot) {
    const settings = loadSettings();
    const problems = evaluate(snapshot, settings);
    const active = state.alerts.active;
    const now = new Date().toISOString();
    for (const p of problems) {
      const existing = active[p.key];
      if (!existing) active[p.key] = { ...p, since: now, count: 1, notified: false };
      else Object.assign(existing, p, { count: existing.count + 1 });
      // Two consecutive failing checks before emailing, so a single blip is not paged.
      const a = active[p.key];
      if (!a.notified && a.count >= 2) {
        a.notified = true;
        audit({ user: 'monitor', action: 'alert', target: p.key, result: p.title });
        await notify(p.event, { title: p.title, severity: p.severity, lines: [p.detail ?? '', `Since ${a.since}`] });
      }
    }
    for (const [key, a] of Object.entries(active)) {
      if (problems.some(p => p.key === key)) continue;
      delete active[key];
      if (a.notified) {
        audit({ user: 'monitor', action: 'resolved', target: key, result: a.title });
        await notify(a.event, { title: `Resolved: ${a.title}`, severity: 'resolved', lines: [`Problem started ${a.since} and has cleared.`] });
      }
    }
    saveState('alerts', state.alerts);
  }

  async function checkUpdates(force = false) {
    const settings = loadSettings();
    const token = settings.github_token || undefined;
    const updates = { ...state.updates, checked: new Date().toISOString(), errors: [] };
    try {
      const releases = await launcherReleases(token);
      updates.launcher = { releases, error: null };
    } catch (e) { updates.errors.push(`Launcher releases: ${e.message}`); }
    updates.images = {};
    const all = await listContainers();
    for (const [name, service] of Object.entries(images)) {
      try {
        const container = all.find(c => c.Labels['com.docker.compose.project'] === STACK_PROJECT && c.Labels['com.docker.compose.service'] === service);
        const releases = await releaseImages(name);
        for (const r of releases.slice(0, 5)) r.commitInfo = await commitInfo(r.commit, token);
        updates.images[name] = { service, current: container?.Image?.split('@')[1] ?? null, running: container?.Image ?? null, releases };
      } catch (e) { updates.errors.push(`${name} images: ${e.message}`); }
    }
    if (force || Date.now() - state.containerUpdatesChecked > 6 * 3600_000) {
      const containers = [];
      for (const c of all.filter(c => c.Labels['org.velora.operations.helper'] !== 'true')) {
        const info = await inspectImage(c.ImageID).catch(() => null);
        containers.push({ name: c.Names[0]?.replace(/^\//, ''), project: c.Labels['com.docker.compose.project'] ?? null, service: c.Labels['com.docker.compose.service'] ?? null, ...(await imageUpdate(c, info)) });
      }
      updates.containers = containers;
      state.containerUpdatesChecked = Date.now();
    }
    // Mark the release the Panel currently serves (its public update feed needs no credentials).
    const served = await fetch(`${PANEL_INTERNAL_URL}/api/v1/launcher/update`, { signal: AbortSignal.timeout(10_000) })
      .then(r => (r.ok ? r.json() : null)).catch(() => null);
    updates.launcher_served = served?.version ?? null;
    for (const r of updates.launcher?.releases ?? []) r.approved = r.version === served?.version && Boolean(served?.signature);
    state.updates = updates;
    for (const n of notices(updates, state.alerts.seen)) {
      state.alerts.seen.push(n.id);
      await notify(n.event, { title: n.title, severity: 'info', lines: [n.detail] });
    }
    state.alerts.seen = state.alerts.seen.slice(-200);
    saveState('alerts', state.alerts);
    state.updatesChecked = Date.now();
    return updates;
  }

  function start() {
    const loop = async () => { try { await collect(); } catch (e) { console.error('collect failed:', e.message); } };
    loop();
    setInterval(loop, 30_000).unref();
    setTimeout(() => checkUpdates(true).catch(e => console.error('update check failed:', e.message)), 5_000).unref();
    setInterval(() => checkUpdates().catch(e => console.error('update check failed:', e.message)), 30 * 60_000).unref();
    setInterval(() => saveState('history', history), 10 * 60_000).unref();
  }

  return { state, start, collect, checkUpdates, notify };
}
