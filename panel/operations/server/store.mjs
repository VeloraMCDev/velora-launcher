// Settings, audit trail and metric history persisted in the dashboard's data directory.
import { appendFileSync, existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export const DATA_DIR = process.env.OPS_DATA_DIR ?? '/data';

const DEFAULT_SETTINGS = {
  notifications: {
    enabled: false,
    resend_api_key: '',
    from: '',
    recipients: [],
    events: {
      service_down: true,
      endpoint_down: true,
      backup_failed: true,
      disk_low: true,
      certificate_expiring: true,
      launcher_release: true,
      image_update: true,
      action_failed: true,
    },
  },
  github_token: '',
  alerts: { disk_percent: 90, backup_max_age_hours: 26, certificate_days: 14 },
};

function readJson(file, fallback) {
  try { return JSON.parse(readFileSync(join(DATA_DIR, file), 'utf8')); } catch { return fallback; }
}
function writeJson(file, value) {
  mkdirSync(DATA_DIR, { recursive: true });
  const target = join(DATA_DIR, file);
  writeFileSync(target + '.tmp', JSON.stringify(value, null, 1), { mode: 0o600 });
  renameSync(target + '.tmp', target);
}

const merge = (base, over) => {
  if (Array.isArray(base) || typeof base !== 'object' || base === null) return over ?? base;
  const out = { ...base };
  for (const [k, v] of Object.entries(over ?? {})) out[k] = k in base ? merge(base[k], v) : v;
  return out;
};

export function loadSettings() {
  return merge(DEFAULT_SETTINGS, readJson('settings.json', {}));
}
export function saveSettings(settings) {
  writeJson('settings.json', settings);
}

/** Settings as sent to the browser: secrets are reported as set/unset, never returned. */
export function publicSettings(settings = loadSettings()) {
  return {
    ...settings,
    notifications: { ...settings.notifications, resend_api_key: '', resend_api_key_set: Boolean(settings.notifications.resend_api_key) },
    github_token: '',
    github_token_set: Boolean(settings.github_token),
  };
}

const EMAIL = /^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+$/;
/** Validates a settings update from the browser; empty secret fields keep the stored value. */
export function applySettingsUpdate(current, update) {
  const next = structuredClone(current);
  const n = update?.notifications ?? {};
  if (typeof n.enabled === 'boolean') next.notifications.enabled = n.enabled;
  if (typeof n.resend_api_key === 'string' && n.resend_api_key.trim()) {
    if (!/^re_[A-Za-z0-9_]{8,}$/.test(n.resend_api_key.trim())) throw new Error('That does not look like a Resend API key (re_…)');
    next.notifications.resend_api_key = n.resend_api_key.trim();
  }
  if (n.clear_resend_api_key === true) next.notifications.resend_api_key = '';
  if (typeof n.from === 'string') {
    const from = n.from.trim();
    if (from && !EMAIL.test(from.replace(/^.*<([^>]+)>$/, '$1'))) throw new Error('Sender must be an email address, optionally "Name <address>"');
    next.notifications.from = from;
  }
  if (Array.isArray(n.recipients)) {
    const list = [...new Set(n.recipients.map(r => String(r).trim()).filter(Boolean))];
    if (list.length > 20 || list.some(r => !EMAIL.test(r))) throw new Error('Enter up to 20 valid recipient addresses');
    next.notifications.recipients = list;
  }
  for (const [k, v] of Object.entries(n.events ?? {})) if (k in next.notifications.events && typeof v === 'boolean') next.notifications.events[k] = v;
  if (typeof update?.github_token === 'string' && update.github_token.trim()) next.github_token = update.github_token.trim();
  if (update?.clear_github_token === true) next.github_token = '';
  const a = update?.alerts ?? {};
  const bound = (v, lo, hi) => Number.isFinite(v) && v >= lo && v <= hi;
  if (a.disk_percent !== undefined) { if (!bound(a.disk_percent, 50, 99)) throw new Error('Disk alert must be 50–99%'); next.alerts.disk_percent = a.disk_percent; }
  if (a.backup_max_age_hours !== undefined) { if (!bound(a.backup_max_age_hours, 2, 168)) throw new Error('Backup age must be 2–168 hours'); next.alerts.backup_max_age_hours = a.backup_max_age_hours; }
  if (a.certificate_days !== undefined) { if (!bound(a.certificate_days, 1, 60)) throw new Error('Certificate warning must be 1–60 days'); next.alerts.certificate_days = a.certificate_days; }
  return next;
}

export function audit(entry) {
  mkdirSync(DATA_DIR, { recursive: true });
  appendFileSync(join(DATA_DIR, 'audit.jsonl'), JSON.stringify({ at: new Date().toISOString(), ...entry }) + '\n', { mode: 0o600 });
}
export function readAudit(limit = 200) {
  const file = join(DATA_DIR, 'audit.jsonl');
  if (!existsSync(file)) return [];
  const lines = readFileSync(file, 'utf8').trim().split('\n').filter(Boolean);
  return lines.slice(-limit).reverse().map(l => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);
}

export const loadState = (name, fallback) => readJson(`${name}.json`, fallback);
export const saveState = (name, value) => writeJson(`${name}.json`, value);
