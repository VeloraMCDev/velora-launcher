// Monitoring-only enrollment. These credentials cannot create dashboard sessions or run actions.
import { randomBytes, randomUUID, createHash, timingSafeEqual } from 'node:crypto';
import { loadState, saveState } from './store.mjs';

const hash = token => createHash('sha256').update(token).digest();
const invalid = message => Object.assign(new Error(message), { status: 400 });
const text = (value, max) => typeof value === 'string' && value.trim().length > 0 && value.length <= max && !/[\x00-\x1f\x7f]/.test(value);
const number = (value, max = Number.MAX_SAFE_INTEGER) => Number.isFinite(value) && value >= 0 && value <= max;
export function validateReport(value) {
  if (!value || !['linux', 'win32', 'darwin', 'freebsd', 'openbsd', 'aix', 'sunos'].includes(value.platform)
    || !text(value.version, 40) || !number(value.uptime) || !(value.cpu === null || number(value.cpu, 100))
    || !number(value.memory?.total) || !number(value.memory?.used) || value.memory.used > value.memory.total
    || !Array.isArray(value.services) || value.services.length > 64 || !Array.isArray(value.disks) || value.disks.length > 16) throw invalid('Invalid host report');
  const names = new Set();
  const services = value.services.map(s => {
    if (!s || !text(s.name, 80) || names.has(s.name) || !['http', 'tcp', 'minecraft'].includes(s.kind) || typeof s.ok !== 'boolean'
      || !(s.ms === null || number(s.ms, 60000)) || (s.players !== undefined && (!s.players || !number(s.players.online, 1000000) || !number(s.players.max, 1000000)))) throw invalid('Invalid service report');
    names.add(s.name);
    return { name: s.name, kind: s.kind, ok: s.ok, ms: s.ms, ...(s.players ? { players: { online: s.players.online, max: s.players.max } } : {}) };
  });
  // Explicitly copy safe fields. URLs, paths, headers, player samples and arbitrary data are discarded.
  const disks = value.disks.map(d => {
    if (!d || !text(d.name, 80) || !number(d.total) || !number(d.used) || d.used > d.total) throw invalid('Invalid disk report');
    return { name: d.name, total: d.total, used: d.used };
  });
  return { platform: value.platform, version: value.version, uptime: value.uptime, cpu: value.cpu,
    memory: { total: value.memory.total, used: value.memory.used }, services, disks };
}
export function createAgentRegistry({ read = () => loadState('monitor-agents', []), write = value => saveState('monitor-agents', value) } = {}) {
  const agents = read();
  function list(now = Date.now()) {
    return agents.map(({ token_hash, ...a }) => ({ ...a, online: Boolean(a.received_at && now - Date.parse(a.received_at) < 90000) }));
  }
  function enroll(name) {
    if (!text(name, 80) || agents.length >= 32) throw invalid('Enter a host name (up to 80 characters); maximum 32 agents');
    const token = randomBytes(32).toString('base64url');
    const a = { id: randomUUID(), name: name.trim(), token_hash: hash(token).toString('hex'), created_at: new Date().toISOString(), received_at: null, report: null };
    write([...agents, a]); agents.push(a);
    return { id: a.id, name: a.name, token }; // Only the enrollment response reveals the credential.
  }
  function authenticate(id, token) {
    const a = agents.find(a => a.id === id);
    const expected = a && Buffer.from(a.token_hash, 'hex');
    if (!expected || expected.length !== 32 || typeof token !== 'string' || token.length > 256 || !timingSafeEqual(expected, hash(token))) throw Object.assign(new Error('Invalid monitoring agent credential'), { status: 401 });
    return a;
  }
  function report(id, token, value, now = Date.now()) {
    const a = authenticate(id, token);
    if (a.received_at && now - Date.parse(a.received_at) < 5000) throw Object.assign(new Error('Agent reports are limited to one every five seconds'), { status: 429 });
    const next = validateReport(value);
    const updated = { ...a, report: next, received_at: new Date(now).toISOString() };
    write(agents.map(item => item.id === id ? updated : item)); Object.assign(a, updated);
  }
  function revoke(id) {
    const at = agents.findIndex(a => a.id === id);
    if (at < 0) throw Object.assign(new Error('Agent not found'), { status: 404 });
    write(agents.filter(a => a.id !== id)); agents.splice(at, 1);
  }
  return { list, enroll, authenticate, report, revoke };
}
