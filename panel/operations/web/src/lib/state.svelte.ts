import { api, setCsrf } from './api';
import type { Job, Overview } from './types';

export type Page = 'overview' | 'services' | 'activity' | 'updates' | 'backups' | 'settings' | 'audit';
const PAGES: Page[] = ['overview', 'services', 'activity', 'updates', 'backups', 'settings', 'audit'];

export const app = $state({
  ready: false,
  user: null as string | null,
  version: '',
  page: 'overview' as Page,
  overview: null as Overview | null,
  toasts: [] as { id: number; text: string; kind: 'ok' | 'error' }[],
  job: null as Job | null,
  jobOpen: false,
  confirm: null as null | { title: string; body: string; action: string; danger?: boolean; resolve: (ok: boolean) => void },
});

/** Asks before an operation that changes production. */
export function ask(title: string, body: string, action = 'Continue', danger = false): Promise<boolean> {
  return new Promise(resolve => (app.confirm = { title, body, action, danger, resolve: ok => { app.confirm = null; resolve(ok); } }));
}

function fromHash(): Page {
  const page = location.hash.replace(/^#\/?/, '') as Page;
  return PAGES.includes(page) ? page : 'overview';
}
export function navigate(page: Page) {
  location.hash = `/${page}`;
}
window.addEventListener('hashchange', () => (app.page = fromHash()));
window.addEventListener('ops:signed-out', () => { app.user = null; });

export async function boot() {
  app.page = fromHash();
  const session = await api<{ username: string | null; csrf?: string; version: string }>('/api/session').catch(() => ({ username: null, version: '' }));
  app.version = session.version;
  if (session.username && session.csrf) { setCsrf(session.csrf); app.user = session.username; }
  app.ready = true;
}

export async function signIn(username: string, password: string) {
  const session = await api<{ username: string; csrf: string; version: string }>('/api/login', { method: 'POST', body: { username, password } });
  setCsrf(session.csrf);
  app.user = session.username;
  app.version = session.version;
}
export async function signOut() {
  await api('/api/logout', { method: 'POST' }).catch(() => {});
  app.user = null;
  app.overview = null;
}

let toastId = 0;
export function toast(text: string, kind: 'ok' | 'error' = 'ok') {
  const id = ++toastId;
  app.toasts.push({ id, text, kind });
  setTimeout(() => (app.toasts = app.toasts.filter(t => t.id !== id)), kind === 'error' ? 7000 : 4000);
}

export async function refreshOverview() {
  app.overview = await api<Overview>('/api/overview');
}

/** Starts an action and follows its job log in the drawer until it finishes. */
export async function runAction(path: string, body?: unknown): Promise<Job | null> {
  try {
    const job = await api<Job>(path, { method: 'POST', body: body ?? {} });
    app.job = job;
    app.jobOpen = true;
    for (;;) {
      await new Promise(r => setTimeout(r, 1500));
      const current = await api<Job>(`/api/jobs/${job.id}`);
      app.job = current;
      if (current.status !== 'running') {
        toast(current.status === 'succeeded' ? `${current.kind} finished` : `${current.kind} failed`, current.status === 'succeeded' ? 'ok' : 'error');
        refreshOverview().catch(() => {});
        return current;
      }
    }
  } catch (e) {
    toast((e as Error).message, 'error');
    return null;
  }
}
