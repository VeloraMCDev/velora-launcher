// Operator actions on the Velora stack. Each runs as a tracked job with a live log and an audit entry.
import { randomUUID } from 'node:crypto';
import { readFileSync, renameSync, writeFileSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';
import { runHelper, startDetachedHelper, inspectContainer, listContainers } from './docker.mjs';
import { audit } from './store.mjs';
import { STACK_DIR, STACK_PROJECT, BACKUP_STATE_DIR } from './monitor.mjs';
import { PANEL_INTERNAL_URL, panelFetch } from './auth.mjs';

const CLI_IMAGE = process.env.DOCKER_CLI_IMAGE ?? 'docker:29-cli';
const NODE_IMAGE = process.env.NODE_IMAGE ?? 'node:24-alpine';
const GIT_IMAGE = process.env.GIT_IMAGE ?? 'alpine/git:2.49.1';
export const SERVICES = ['panel', 'docs', 'ops'];
const IMAGE_SERVICES = { 'velora-panel': 'panel', 'velora-operations': 'ops' };
const jobs = new Map();

export const listJobs = () => [...jobs.values()].sort((a, b) => b.started.localeCompare(a.started)).slice(0, 30).map(({ log, ...j }) => ({ ...j, lines: log.length }));
export const getJob = id => jobs.get(id) ?? null;
export const busy = () => [...jobs.values()].some(j => j.status === 'running');

export function startJob(kind, user, target, fn, { notify } = {}) {
  if (busy()) throw Object.assign(new Error('Another operation is still running. Wait for it to finish.'), { status: 409 });
  const job = { id: randomUUID(), kind, target, user, status: 'running', started: new Date().toISOString(), finished: null, log: [] };
  const log = line => { for (const l of String(line).split('\n')) if (l.trim()) job.log.push(`${new Date().toISOString().slice(11, 19)}  ${l}`); if (job.log.length > 4000) job.log.splice(0, job.log.length - 4000); };
  jobs.set(job.id, job);
  while (jobs.size > 50) jobs.delete(jobs.keys().next().value);
  audit({ user, action: kind, target, result: 'started', job: job.id });
  (async () => {
    try {
      const result = await fn(log);
      job.status = 'succeeded';
      log(result ?? 'Done.');
    } catch (e) {
      job.status = 'failed';
      log(`FAILED: ${e.message}`);
      await notify?.('action_failed', { title: `${kind} failed`, severity: 'critical', lines: [`${target ?? ''} — ${e.message}`, `Started by ${user}`] });
    } finally {
      job.finished = new Date().toISOString();
      audit({ user, action: kind, target, result: job.status, job: job.id });
    }
  })();
  return job;
}

const stackBinds = () => ['/var/run/docker.sock:/var/run/docker.sock', `${STACK_DIR}:${STACK_DIR}`];
async function compose(args, log) {
  log(`$ docker compose ${args.join(' ')}`);
  const { code, output } = await runHelper({ image: CLI_IMAGE, cmd: ['docker', 'compose', '-p', STACK_PROJECT, ...args], binds: stackBinds(), workingDir: STACK_DIR, onLog: log });
  if (code !== 0) throw new Error(`docker compose ${args[0]} exited with ${code}${output ? '' : ''}`);
}

function checkService(service) {
  if (!SERVICES.includes(service)) throw Object.assign(new Error(`Unknown service ${service}`), { status: 400 });
}

export const restart = service => log => { checkService(service); return compose(['restart', service], log); };
export const redeploy = () => async log => { await compose(['pull', '--ignore-pull-failures', 'docs'], log); await compose(['up', '-d'], log); return 'Stack is up to date with compose.yaml.'; };
export const pullService = service => async log => { checkService(service); await compose(['pull', service], log); await compose(['up', '-d', service], log); };

// ---------- backups ----------
export async function requestBackup(log, timeout = 15 * 60_000) {
  const requested = Date.now();
  writeFileSync(join(BACKUP_STATE_DIR, 'request'), `${new Date(requested).toISOString()}\n`);
  log('Backup requested; waiting for the host backup service…');
  for (;;) {
    await new Promise(r => setTimeout(r, 5000));
    let last = null;
    try { last = JSON.parse(readFileSync(join(BACKUP_STATE_DIR, 'last.json'), 'utf8')); } catch { /* not yet */ }
    if (last && Date.parse(last.started_at) >= requested - 2000 && last.finished_at) {
      if (last.status !== 'ok') throw new Error(`Backup failed: ${last.message}`);
      log(`Backup ${last.archive} complete (${(last.bytes / 1048576).toFixed(0)} MB, offsite: ${last.offsite ? 'yes' : 'no'}).`);
      return last;
    }
    if (Date.now() - requested > timeout) {
      try { unlinkSync(join(BACKUP_STATE_DIR, 'request')); } catch { /* consumed */ }
      throw new Error('The backup did not finish in time; is velora-backup-request.path enabled on the host?');
    }
  }
}

// ---------- image updates ----------
// Release images are pinned in the deployment's .env (PANEL_IMAGE=…@sha256:…), so compose.yaml
// stays identical to infra/vps/compose.yaml.
const IMAGE_VARIABLES = { 'velora-panel': 'PANEL_IMAGE', 'velora-operations': 'OPS_IMAGE' };
const pinned = name => new RegExp(`^(${IMAGE_VARIABLES[name]}=)(ghcr\\.io/[a-z0-9-]+/${name}@sha256:[0-9a-f]{64})\\s*$`, 'm');

/** Replaces the pinned digest of one Velora image in .env text. */
export function setImageDigest(text, name, image) {
  if (!IMAGE_VARIABLES[name]) throw new Error(`Unknown release image ${name}`);
  if (!/^ghcr\.io\/[a-z0-9-]+\/[a-z0-9-]+@sha256:[0-9a-f]{64}$/.test(image) || !image.split('@')[0].endsWith(`/${name}`)) throw new Error('Invalid release image reference');
  if (!pinned(name).test(text)) throw new Error(`.env has no pinned ${IMAGE_VARIABLES[name]}`);
  return text.replace(pinned(name), `$1${image}`);
}
export function currentImage(text, name) {
  return IMAGE_VARIABLES[name] ? text.match(pinned(name))?.[2] ?? null : null;
}

async function waitHealthy(service, log, timeout = 150_000) {
  const started = Date.now();
  for (;;) {
    await new Promise(r => setTimeout(r, 3000));
    const container = (await listContainers()).find(c => c.Labels['com.docker.compose.project'] === STACK_PROJECT && c.Labels['com.docker.compose.service'] === service);
    const info = container && await inspectContainer(container.Id).catch(() => null);
    const health = info?.State?.Health?.Status ?? (info?.State?.Running ? 'running' : 'stopped');
    if (health === 'healthy' || (health === 'running' && !info?.State?.Health)) {
      if (service === 'panel') {
        const ok = await fetch(`${PANEL_INTERNAL_URL}/health`, { signal: AbortSignal.timeout(5000) }).then(r => r.ok).catch(() => false);
        if (!ok) { if (Date.now() - started > timeout) throw new Error('Panel /health did not respond'); continue; }
      }
      log(`${service} is ${health}.`);
      return;
    }
    if (Date.now() - started > timeout) throw new Error(`${service} did not become healthy (last state: ${health})`);
  }
}

export const updateImage = (name, image) => async log => {
  const service = IMAGE_SERVICES[name];
  if (!service) throw new Error(`Unknown release image ${name}`);
  const file = join(STACK_DIR, '.env');
  const original = readFileSync(file, 'utf8');
  const previous = currentImage(original, name);
  if (previous === image) return `${service} already runs ${image}.`;
  const updated = setImageDigest(original, name, image);
  if (service === 'ops') {
    // The dashboard cannot supervise its own restart: hand the update to a helper that outlives it.
    writeFileSync(file + '.tmp', updated); renameSync(file + '.tmp', file);
    log(`.env now pins ${image}. Restarting the dashboard; reload the page in a minute.`);
    await startDetachedHelper({ image: CLI_IMAGE, cmd: ['sh', '-c', `sleep 3 && docker compose -p ${STACK_PROJECT} up -d ops`], binds: stackBinds(), workingDir: STACK_DIR });
    return 'Self-update handed off.';
  }
  await requestBackup(log);
  writeFileSync(file + '.tmp', updated); renameSync(file + '.tmp', file);
  log(`.env: ${previous} → ${image}`);
  try {
    await compose(['pull', service], log);
    await compose(['up', '-d', service], log);
    await waitHealthy(service, log);
    return `${service} updated to ${image.split('@')[1].slice(0, 19)}…`;
  } catch (e) {
    log(`Update failed (${e.message}); rolling back to ${previous}.`);
    writeFileSync(file + '.tmp', original); renameSync(file + '.tmp', file);
    await compose(['up', '-d', service], log);
    await waitHealthy(service, log).catch(err => log(`Rollback health check: ${err.message}`));
    throw new Error(`${e.message}; rolled back to the previous image`);
  }
};

// ---------- documentation ----------
export const rebuildDocs = (ref = 'main') => async log => {
  if (!/^[A-Za-z0-9._/-]{1,100}$/.test(ref)) throw new Error('Invalid git reference');
  const src = join(STACK_DIR, 'src');
  const user = process.env.STACK_UID ?? '1000:1000';
  log(`Fetching ${ref}…`);
  let r = await runHelper({ image: GIT_IMAGE, cmd: ['-c', `safe.directory=${src}`, '-C', src, 'fetch', '--quiet', 'origin', ref], binds: [`${src}:${src}`], onLog: log, user });
  if (r.code) throw new Error('git fetch failed');
  r = await runHelper({ image: GIT_IMAGE, cmd: ['-c', `safe.directory=${src}`, '-C', src, 'checkout', '--quiet', '--detach', 'FETCH_HEAD'], binds: [`${src}:${src}`], onLog: log, user });
  if (r.code) throw new Error('git checkout failed');
  const site = process.env.DOCS_SITE_URL ?? '';
  r = await runHelper({
    image: NODE_IMAGE, user,
    cmd: ['sh', '-c', `cd docs && export HOME=/tmp npm_config_cache=/tmp/npm && npm ci --no-audit --no-fund --omit=dev >/dev/null && npm test && GITHUB_SHA=$(cat ../.git/HEAD | grep -Eo '^[0-9a-f]{40}') DOCS_SITE_URL='${site.replace(/'/g, '')}' npm run build && rm -rf ../../docs-site.new && cp -r dist ../../docs-site.new && rm -rf ../../docs-site.old && { [ -d ../../docs-site ] && mv ../../docs-site ../../docs-site.old; true; } && mv ../../docs-site.new ../../docs-site`],
    binds: [`${STACK_DIR}:${STACK_DIR}`], workingDir: src, onLog: log,
  });
  if (r.code) throw new Error('documentation build failed');
  await compose(['restart', 'docs'], log);
  return 'Documentation rebuilt and published.';
};

// ---------- launcher releases ----------
export const approveLauncher = (tag, token) => async log => {
  if (!/^launcher-v\d+\.\d+\.\d+$/.test(tag)) throw new Error('Invalid launcher release tag');
  log(`Asking the Panel to verify and publish ${tag} (downloads every installer and checks its signature)…`);
  const approval = await panelFetch(`/api/admin/launcher/releases/${tag}/approve`, { token, method: 'POST', timeout: 20 * 60_000 });
  for (const a of approval.assets ?? []) log(`✓ ${a.platform}: ${a.name} (${(a.size / 1048576).toFixed(1)} MB, sha256 ${a.sha256.slice(0, 12)}…)`);
  return `Launcher ${approval.version} is now offered to players.`;
};
