// Minimal Docker Engine API client over the local socket. Only this module talks to Docker.
import { request } from 'node:http';

export const SOCKET = process.env.DOCKER_SOCKET ?? '/var/run/docker.sock';

export function docker(method, path, { body, timeout = 30_000, raw = false } = {}) {
  return new Promise((resolve, reject) => {
    const payload = body === undefined ? undefined : Buffer.from(JSON.stringify(body));
    const req = request({ socketPath: SOCKET, method, path, timeout, headers: payload ? { 'content-type': 'application/json', 'content-length': payload.length } : {} }, res => {
      const chunks = [];
      res.on('data', c => chunks.push(c));
      res.on('end', () => {
        const buffer = Buffer.concat(chunks);
        if (res.statusCode >= 400) {
          let message = buffer.toString('utf8');
          try { message = JSON.parse(message).message ?? message; } catch { /* plain text error */ }
          return reject(Object.assign(new Error(`Docker ${method} ${path.split('?')[0]}: ${message}`), { status: res.statusCode }));
        }
        if (raw) return resolve(buffer);
        if (!buffer.length) return resolve(null);
        try { resolve(JSON.parse(buffer.toString('utf8'))); } catch { resolve(buffer.toString('utf8')); }
      });
    });
    req.on('timeout', () => req.destroy(new Error(`Docker ${method} ${path} timed out`)));
    req.on('error', reject);
    if (payload) req.write(payload);
    req.end();
  });
}

/** Docker multiplexes stdout/stderr in non-TTY log streams: 8-byte headers before each frame. */
export function demux(buffer) {
  const lines = [];
  let offset = 0;
  // A frame header is [stream 0-2, 0, 0, 0, size u32]; anything else is a TTY (raw) stream.
  const header = at => buffer[at] <= 2 && buffer[at + 1] === 0 && buffer[at + 2] === 0 && buffer[at + 3] === 0;
  while (offset + 8 <= buffer.length && header(offset)) {
    const size = buffer.readUInt32BE(offset + 4);
    lines.push(buffer.subarray(offset + 8, offset + 8 + size).toString('utf8'));
    offset += 8 + size;
  }
  return offset === 0 ? buffer.toString('utf8') : lines.join('');
}

export const listContainers = () => docker('GET', '/containers/json?all=1');
export const inspectContainer = id => docker('GET', `/containers/${encodeURIComponent(id)}/json`);
export const inspectImage = ref => docker('GET', `/images/${encodeURIComponent(ref)}/json`);

export async function containerLogs(id, tail = 300) {
  const n = Math.max(1, Math.min(2000, Number(tail) || 300));
  const buffer = await docker('GET', `/containers/${encodeURIComponent(id)}/logs?stdout=1&stderr=1&timestamps=1&tail=${n}`, { raw: true });
  // Strip ANSI colour codes so logs render cleanly in the browser.
  return demux(buffer).replace(/\x1b\[[0-9;]*m/g, '');
}

/** CPU % and memory from a single non-streaming stats sample (Docker includes the previous sample). */
export async function containerStats(id) {
  const s = await docker('GET', `/containers/${encodeURIComponent(id)}/stats?stream=false`, { timeout: 10_000 });
  const cpuDelta = (s.cpu_stats?.cpu_usage?.total_usage ?? 0) - (s.precpu_stats?.cpu_usage?.total_usage ?? 0);
  const systemDelta = (s.cpu_stats?.system_cpu_usage ?? 0) - (s.precpu_stats?.system_cpu_usage ?? 0);
  const cpus = s.cpu_stats?.online_cpus || s.cpu_stats?.cpu_usage?.percpu_usage?.length || 1;
  const cache = s.memory_stats?.stats?.inactive_file ?? s.memory_stats?.stats?.cache ?? 0;
  return {
    cpu: systemDelta > 0 && cpuDelta >= 0 ? (cpuDelta / systemDelta) * cpus * 100 : 0,
    memory: Math.max(0, (s.memory_stats?.usage ?? 0) - cache),
    memoryLimit: s.memory_stats?.limit ?? 0,
  };
}

/** Runs a short-lived helper container to completion and returns its exit code and output. */
export async function runHelper({ image, cmd, binds = [], workingDir, env = [], name, user, timeout = 15 * 60_000, onLog }) {
  await ensureImage(image);
  const created = await docker('POST', `/containers/create${name ? `?name=${encodeURIComponent(name)}` : ''}`, {
    body: {
      Image: image, Cmd: cmd, WorkingDir: workingDir, Env: env, Tty: false, ...(user ? { User: user } : {}),
      Labels: { 'org.velora.operations.helper': 'true' },
      HostConfig: { Binds: binds, AutoRemove: false, NetworkMode: 'bridge' },
    },
  });
  const id = created.Id;
  try {
    await docker('POST', `/containers/${id}/start`);
    const started = Date.now();
    let seen = 0;
    for (;;) {
      const info = await inspectContainer(id);
      const logs = demux(await docker('GET', `/containers/${id}/logs?stdout=1&stderr=1`, { raw: true }));
      if (onLog && logs.length > seen) { onLog(logs.slice(seen)); seen = logs.length; }
      if (!info.State.Running) return { code: info.State.ExitCode, output: logs };
      if (Date.now() - started > timeout) {
        await docker('POST', `/containers/${id}/kill`).catch(() => {});
        throw new Error(`${cmd.join(' ')} timed out`);
      }
      await new Promise(r => setTimeout(r, 1500));
    }
  } finally {
    await docker('DELETE', `/containers/${id}?force=1`).catch(() => {});
  }
}

/** Starts a helper that outlives this process (used when the dashboard updates itself). */
export async function startDetachedHelper({ image, cmd, binds = [], workingDir }) {
  await ensureImage(image);
  const created = await docker('POST', '/containers/create', {
    body: { Image: image, Cmd: cmd, WorkingDir: workingDir, Labels: { 'org.velora.operations.helper': 'true' }, HostConfig: { Binds: binds, AutoRemove: true } },
  });
  await docker('POST', `/containers/${created.Id}/start`);
  return created.Id;
}

export async function ensureImage(image) {
  try { await inspectImage(image); return; } catch (e) { if (e.status !== 404) throw e; }
  const [repo, tag] = splitRef(image);
  await docker('POST', `/images/create?fromImage=${encodeURIComponent(repo)}&tag=${encodeURIComponent(tag)}`, { timeout: 10 * 60_000, raw: true });
}

export function splitRef(image) {
  const at = image.indexOf('@');
  if (at > 0) return [image.slice(0, at), image.slice(at + 1)];
  const slash = image.lastIndexOf('/');
  const colon = image.lastIndexOf(':');
  return colon > slash ? [image.slice(0, colon), image.slice(colon + 1)] : [image, 'latest'];
}
