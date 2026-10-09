// Sign-in with Velora Panel administrator accounts. The Panel stays the identity authority:
// sessions hold the admin's Panel token and are re-validated against /auth/me regularly.
import { randomBytes, timingSafeEqual } from 'node:crypto';

export const PANEL_INTERNAL_URL = (process.env.PANEL_INTERNAL_URL ?? 'http://velora-platform-panel:8080').replace(/\/$/, '');
const SESSION_HOURS = 12;
const REVALIDATE_MS = 5 * 60_000;
export const COOKIE = 'velora_ops';

const sessions = new Map();
const attempts = new Map();

export async function panelFetch(path, { token, method = 'GET', body, timeout = 15_000 } = {}) {
  const response = await fetch(PANEL_INTERNAL_URL + path, {
    method,
    headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), ...(body ? { 'content-type': 'application/json' } : {}) },
    body: body ? JSON.stringify(body) : undefined,
    signal: AbortSignal.timeout(timeout),
  });
  const text = await response.text();
  let data = null;
  try { data = text ? JSON.parse(text) : null; } catch { data = { error: text }; }
  if (!response.ok) throw Object.assign(new Error(data?.error ?? data?.message ?? `Panel returned ${response.status}`), { status: response.status });
  return data;
}

/** Ten attempts per address per 15 minutes; the Panel applies its own per-account throttle too. */
export function allowAttempt(ip, now = Date.now()) {
  const recent = (attempts.get(ip) ?? []).filter(t => now - t < 15 * 60_000);
  if (recent.length >= 10) return false;
  recent.push(now);
  attempts.set(ip, recent);
  return true;
}

export async function login(username, password, ip) {
  if (!allowAttempt(ip)) throw Object.assign(new Error('Too many sign-in attempts. Try again in a few minutes.'), { status: 429 });
  if (typeof username !== 'string' || typeof password !== 'string' || !username.trim() || !password) {
    throw Object.assign(new Error('Enter your Velora username and password'), { status: 400 });
  }
  const auth = await panelFetch('/api/v1/auth/login', { method: 'POST', body: { username: username.trim(), password } }).catch(e => {
    throw Object.assign(new Error(e.status === 401 ? 'Wrong username or password' : e.message), { status: e.status === 401 ? 401 : e.status ?? 502 });
  });
  if (auth?.user?.role !== 'admin') throw Object.assign(new Error('Only Velora administrators can use the operations dashboard'), { status: 403 });
  const id = randomBytes(32).toString('base64url');
  const csrf = randomBytes(24).toString('base64url');
  const session = { id, csrf, token: auth.token, username: auth.user.username, created: Date.now(), checked: Date.now(), expires: Date.now() + SESSION_HOURS * 3600_000 };
  sessions.set(id, session);
  return session;
}

export function logout(id) {
  sessions.delete(id);
}

export function parseCookies(header = '') {
  return Object.fromEntries(header.split(';').map(p => p.trim().split('=')).filter(([k, v]) => k && v).map(([k, ...v]) => [k, decodeURIComponent(v.join('='))]));
}

/** Resolves the request's session, re-checking with the Panel that the user is still an active admin. */
export async function sessionFor(req) {
  const id = parseCookies(req.headers.cookie)[COOKIE];
  const session = id && sessions.get(id);
  if (!session) return null;
  if (Date.now() > session.expires) { sessions.delete(id); return null; }
  if (Date.now() - session.checked > REVALIDATE_MS) {
    try {
      const me = await panelFetch('/api/v1/auth/me', { token: session.token });
      if (me?.role !== 'admin') { sessions.delete(id); return null; }
      session.checked = Date.now();
    } catch (e) {
      if (e.status === 401 || e.status === 403) { sessions.delete(id); return null; }
      // Panel briefly unavailable (e.g. during its own redeploy): keep the session until it expires.
    }
  }
  return session;
}

export function csrfOk(req, session) {
  const sent = Buffer.from(String(req.headers['x-csrf-token'] ?? ''));
  const expected = Buffer.from(session.csrf);
  return sent.length === expected.length && timingSafeEqual(sent, expected);
}

export function sessionCookie(session, secure = true) {
  const maxAge = Math.floor((session.expires - Date.now()) / 1000);
  return `${COOKIE}=${session.id}; Path=/; HttpOnly; SameSite=Strict; Max-Age=${maxAge}${secure ? '; Secure' : ''}`;
}
export const clearCookie = (secure = true) => `${COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0${secure ? '; Secure' : ''}`;

export const _test = { sessions, attempts };
