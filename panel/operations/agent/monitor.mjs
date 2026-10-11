// Portable, unprivileged monitoring agent. No Docker socket, remote commands or player identities.
import os from 'node:os';
import { readFile, statfs } from 'node:fs/promises';
import { connect } from 'node:net';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';

export function config(value) {
  const url = new URL(value.ops_url);
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash
    || !/^[a-f0-9-]{36}$/.test(value.id) || typeof value.token !== 'string' || value.token.length < 32 || value.token.length > 256) throw Error('HTTPS Ops URL and enrollment credentials required');
  const interval = value.interval_seconds ?? 30;
  if (!Number.isInteger(interval) || interval < 15 || interval > 60) throw Error('Report interval must be 15–60 seconds');
  const services = value.services ?? [], disks = value.disks ?? [];
  if (!Array.isArray(services) || services.length > 64 || !Array.isArray(disks) || disks.length > 16) throw Error('Too many monitoring targets');
  const names = new Set();
  const validText = (v, max) => typeof v === 'string' && v.trim().length > 0 && v.length <= max && !/[\x00-\x1f\x7f]/.test(v);
  const validHeaders = headers => headers && !Array.isArray(headers) && typeof headers === 'object' && Object.entries(headers).every(([k,v]) => /^[!#$%&'*+.^_`|~0-9a-z-]+$/i.test(k) && typeof v === 'string' && !/[\r\n]/.test(v));
  if (value.ops_headers && (!validHeaders(value.ops_headers) || Object.keys(value.ops_headers).some(k => /^(authorization|content-type|host)$/i.test(k)))) throw Error('Invalid Ops access headers');
  for (const s of services) {
    if (!validText(s.name, 80) || names.has(s.name) || !['http','tcp','minecraft'].includes(s.kind)) throw Error('Invalid monitoring target');
    names.add(s.name);
    if (s.kind === 'http') {
      const target = new URL(s.url);
      if (!['http:','https:'].includes(target.protocol) || target.username || target.password) throw Error('Invalid HTTP target');
      if (s.headers && !validHeaders(s.headers)) throw Error('Invalid HTTP headers');
    } else if (!validText(s.host, 253) || !Number.isInteger(s.port) || s.port < 1 || s.port > 65535 || (s.virtual_host !== undefined && !validText(s.virtual_host,253))) throw Error('Invalid TCP target');
  }
  for (const d of disks) if (!validText(d.name,80) || !validText(d.path,4096)) throw Error('Invalid disk target');
  return { ...value, ops_url: value.ops_url.replace(/\/$/, ''), interval_seconds: interval, services, disks };
}
function varint(value) {
  let n = value >>> 0; const bytes = [];
  do { let b = n & 127; n >>>= 7; if (n) b |= 128; bytes.push(b); } while (n);
  return Buffer.from(bytes);
}
function decode(buffer, offset = 0) {
  let value = 0;
  for (let i = 0; i < 5; i++) {
    if (offset + i >= buffer.length) return null;
    const b = buffer[offset + i]; value |= (b & 127) << (7 * i);
    if (!(b & 128)) return { value: value >>> 0, end: offset + i + 1 };
  }
  throw Error('Invalid VarInt');
}
export function tcpProbe(target) {
  return new Promise(resolve => {
    const socket = connect({ host: target.host, port: target.port });
    let buffer = Buffer.alloc(0), done = false;
    function finish(value) { if (done) return; done = true; socket.destroy(); resolve(value); }
    socket.setTimeout(3000, () => finish({ ok: false }));
    socket.on('error', () => finish({ ok: false })); socket.on('end', () => finish({ ok: false }));
    socket.on('close', () => finish({ ok: false }));
    socket.on('connect', () => {
      if (target.kind === 'tcp') return finish({ ok: true });
      const host = Buffer.from(target.virtual_host ?? target.host, 'utf8'), port = Buffer.alloc(2); port.writeUInt16BE(target.port);
      const handshake = Buffer.concat([Buffer.from([0]),varint(-1),varint(host.length),host,port,Buffer.from([1])]);
      socket.write(Buffer.concat([varint(handshake.length),handshake,Buffer.from([1,0])]));
    });
    socket.on('data', chunk => {
      try {
        if (buffer.length + chunk.length > 65536) return finish({ ok: false });
        buffer = Buffer.concat([buffer,chunk]); const packet = decode(buffer);
        if (!packet) return;
        if (packet.value > 65530) return finish({ ok: false });
        if (buffer.length < packet.end + packet.value) return;
        const id = decode(buffer,packet.end), size = id && decode(buffer,id.end);
        if (!size || id.value !== 0 || size.end + size.value !== packet.end + packet.value) return finish({ ok: false });
        const status = JSON.parse(buffer.subarray(size.end,size.end + size.value).toString('utf8'));
        const p = status.players;
        const players = p && Number.isInteger(p.online) && Number.isInteger(p.max) && p.online >= 0 && p.max >= 0 && p.online <= 1000000 && p.max <= 1000000 ? { online: p.online, max: p.max } : undefined;
        finish({ ok: true, ...(players ? { players } : {}) });
      } catch { finish({ ok: false }); }
    });
  });
}
export async function probe(target) {
  const start = performance.now(); let result;
  try {
    if (target.kind === 'http') {
      const r = await fetch(target.url, { headers: target.headers, redirect: 'manual', signal: AbortSignal.timeout(3000) });
      await r.body?.cancel(); result = { ok: r.ok };
    } else result = await tcpProbe(target);
  } catch { result = { ok: false }; }
  return { name: target.name, kind: target.kind, ...result, ms: Math.round(performance.now() - start) };
}
export function createCollector() {
  let previous;
  return async cfg => {
    const times = os.cpus().reduce((sum,c) => { sum.idle += c.times.idle; sum.total += Object.values(c.times).reduce((a,b) => a+b,0); return sum; }, {idle:0,total:0});
    const delta = previous && times.total - previous.total;
    const cpu = delta > 0 ? Math.max(0,Math.min(100,100*(1-(times.idle-previous.idle)/delta))) : null; previous=times;
    const services = [];
    for (let i=0;i<cfg.services.length;i+=8) services.push(...await Promise.all(cfg.services.slice(i,i+8).map(probe)));
    const disks = [];
    for (const d of cfg.disks) try { const s = await statfs(d.path); disks.push({name:d.name,total:s.blocks*s.bsize,used:(s.blocks-s.bavail)*s.bsize}); } catch { /* unavailable paths are omitted */ }
    return { platform:os.platform(),version:'1.0.0',uptime:os.uptime(),cpu,memory:{total:os.totalmem(),used:os.totalmem()-os.freemem()},services,disks };
  };
}
export async function start(filename) {
  const cfg = config(JSON.parse(await readFile(filename, 'utf8'))), collect = createCollector();
  let stopped = false; const shutdown = new AbortController();
  for (const signal of ['SIGINT','SIGTERM']) process.on(signal, () => { stopped = true; shutdown.abort(); });
  while (!stopped) {
    try {
      const r = await fetch(`${cfg.ops_url}/api/agents/${cfg.id}/report`, { method:'POST', redirect:'manual', headers:{...cfg.ops_headers,authorization:`Bearer ${cfg.token}`,'content-type':'application/json'},body:JSON.stringify(await collect(cfg)),signal:AbortSignal.any([shutdown.signal,AbortSignal.timeout(10000)]) });
      await r.body?.cancel(); if (!r.ok) console.error(`Monitoring report rejected (${r.status})`);
    } catch { console.error('Monitoring report could not reach Ops'); }
    if (!stopped) await sleep(cfg.interval_seconds*1000,undefined,{signal:shutdown.signal}).catch(()=>{});
  }
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!process.argv[2]) { console.error('Usage: node monitor.mjs /path/to/operator-config.json'); process.exitCode=1; }
  else await start(process.argv[2]).catch(() => { console.error('Invalid or unreadable monitoring configuration'); process.exitCode=1; });
}
