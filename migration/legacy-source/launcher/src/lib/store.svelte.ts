import { pageEnabled } from '@scopenet/experience';
import { errorText, invoke, listen, setExperienceContext } from './tauri';
import { applyTheme } from './theme';
import type { Account, Bootstrap, Instance, Manifest, ProgressEvent, Settings, Stage, UpdateInfo } from './types';

export interface LaunchState {
  state: 'idle' | 'preparing' | 'running' | 'error';
  stage: Stage | null;
  label: string;
  done: number;
  total: number;
  filesDone: number;
  filesTotal: number;
  speed: number;
  message: string | null;
}

export interface ExitInfo { instance_id: string; run_id: string; code: number | null; crashed: boolean; tail: string[]; crash_report: string | null }

const idle = (): LaunchState => ({ state: 'idle', stage: null, label: '', done: 0, total: 0, filesDone: 0, filesTotal: 0, speed: 0, message: null });

export const app = $state({
  ready: false,
  boot: null as Bootstrap | null,
  manifest: null as Manifest | null,
  settings: null as Settings | null,
  accounts: [] as Account[],
  active: null as string | null,
  panelUrl: null as string | null,
  offline: false,
  view: 'home' as 'home' | 'settings' | 'stats' | 'quests' | 'guilds' | 'social' | 'commands' | 'collections' | 'market' | 'casino' | 'experience',
  settingsTab: 'account',
  viewProfileUuid: null as string | null,
  /** Open this friend's conversation when the Social page loads. */
  chatWith: null as string | null,
  selected: null as string | null,
  launch: idle(),
  launchingId: null as string | null,
  running: [] as { run_id: string; instance_id: string }[],
  logs: [] as string[],
  consoleOpen: false,
  crash: null as ExitInfo | null,
  update: null as UpdateInfo | null,
  addAccount: false,
  instanceSettings: null as string | null,
  /** Bumped after a skin change so avatars reload. */
  skinVersion: 0,
  toasts: [] as { id: number; text: string; kind: 'ok' | 'error' | 'info' }[],
});

let toastId = 1;
export function toast(text: string, kind: 'ok' | 'error' | 'info' = 'ok') {
  const id = toastId++;
  app.toasts.push({ id, text, kind });
  setTimeout(() => {
    const i = app.toasts.findIndex((t) => t.id === id);
    if (i >= 0) app.toasts.splice(i, 1);
  }, kind === 'error' ? 7000 : 3500);
}

export const activeAccount = () => app.accounts.find((a) => a.id === app.active) ?? null;
export const instances = (): Instance[] => app.manifest?.instances ?? [];
export const selectedInstance = (): Instance | null => instances().find((i) => i.id === app.selected) ?? instances()[0] ?? null;

/** Resolve panel-relative media URLs ("/uploads/x.png"). */
export function abs(url: string | null | undefined): string | null {
  if (!url) return null;
  if (url.startsWith('/') && app.panelUrl) return app.panelUrl + url;
  return url;
}

export const experienceBranding = () => selectedInstance()?.experience?.branding ?? app.manifest?.branding;

function theme() {
  const branding = experienceBranding();
  if (branding) applyTheme(branding, app.settings);
}

function pickSelected() {
  const list = instances();
  const wanted = app.settings?.selected_instance;
  app.selected = list.find((i) => i.id === wanted)?.id ?? list.find((i) => i.featured)?.id ?? list[0]?.id ?? null;
  setExperienceContext(app.selected);
}

export async function init() {
  const boot = await invoke<Bootstrap>('bootstrap');
  app.boot = boot;
  app.settings = boot.settings;
  app.accounts = boot.accounts;
  app.active = boot.active_account;
  app.manifest = boot.manifest;
  app.panelUrl = boot.panel_url;
  app.running = boot.game_running;
  pickSelected();
  theme();
  subscribe();
  invoke('show_window').catch(() => {});
  void refreshUpdates(true);
  if (app.panelUrl && !app.manifest) {
    try {
      await refresh();
    } finally {
      app.ready = true;
    }
  } else {
    app.ready = true;
    if (app.panelUrl) await refresh();
  }
}

let updateCheckBusy = false;
let updateCheckTime = 0;
export async function refreshUpdates(force = false) {
  if (!app.settings?.check_updates || updateCheckBusy || (!force && Date.now() - updateCheckTime < 60_000)) return;
  updateCheckBusy = true;
  updateCheckTime = Date.now();
  const panel = app.panelUrl;
  try {
    const update = await invoke<UpdateInfo | null>('check_update');
    if (app.settings?.check_updates && panel === app.panelUrl) app.update = update;
  } catch { /* Keep the last known update during temporary outages. */ }
  finally {
    updateCheckBusy = false;
    if (panel !== app.panelUrl) void refreshUpdates(true);
  }
}

export async function refresh() {
  try {
    app.manifest = await invoke<Manifest>('refresh_manifest');
    app.offline = false;
    pickSelected();
    theme();
  } catch (e) {
    app.offline = true;
    if (!app.manifest) throw e;
  }
}

export async function connect(url: string) {
  app.manifest = await invoke<Manifest>('set_panel_url', { url });
  const boot = await invoke<Bootstrap>('bootstrap');
  app.panelUrl = boot.panel_url;
  app.update = null;
  void refreshUpdates(true);
  app.offline = false;
  pickSelected();
  theme();
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;
/** Persist settings (debounced) and re-apply the theme immediately. */
export function saveSettings() {
  theme();
  if (!app.settings?.check_updates) app.update = null;
  else void refreshUpdates();
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    if (app.settings) invoke('save_settings', { settings: $state.snapshot(app.settings) }).catch((e) => toast(errorText(e), 'error'));
  }, 300);
}

export function selectInstance(id: string) {
  if (!instances().some(i => i.id === id)) return;
  app.selected = id;
  setExperienceContext(id);
  app.view = 'home';
  theme();
  if (app.settings) {
    app.settings.selected_instance = id;
    saveSettings();
  }
}

export async function accountAdded(account: Account) {
  const i = app.accounts.findIndex((a) => a.id === account.id);
  if (i >= 0) app.accounts[i] = account;
  else app.accounts.push(account);
  app.active = account.id;
  app.addAccount = false;
  await refresh().catch(() => {});
}

export async function switchAccount(id: string) {
  await invoke('select_account', { id });
  app.active = id;
  await refresh().catch(() => {});
}

export async function removeAccount(id: string) {
  await invoke('remove_account', { id });
  app.accounts = app.accounts.filter((a) => a.id !== id);
  if (app.active === id) app.active = app.accounts[0]?.id ?? null;
  await refresh().catch(() => {});
}

export async function play(id: string) {
  app.launch = { ...idle(), state: 'preparing', label: 'Getting ready…' };
  app.launchingId = id;
  app.logs = [];
  try {
    await invoke('launch', { instanceId: id });
  } catch (e) {
    app.launch = { ...idle(), state: 'error', message: errorText(e) };
  }
}

export async function repair(id: string) {
  app.launch = { ...idle(), state: 'preparing', label: 'Verifying files…' };
  app.launchingId = id;
  try {
    await invoke('repair_instance', { instanceId: id });
  } catch (e) {
    app.launch = { ...idle(), state: 'error', message: errorText(e) };
  }
}

export async function cancel() {
  if (app.launchingId) await invoke('cancel_launch', { instanceId: app.launchingId });
}

export async function kill(runId: string) {
  await invoke('kill_game', { runId });
}

let subscribed = false;
let last = { t: 0, done: 0, stage: '' as string };

function subscribe() {
  if (subscribed) return;
  subscribed = true;

  listen<ProgressEvent>('launch://progress', (ev) => {
    const l = app.launch;
    if (ev.type === 'stage') {
      l.stage = ev.stage;
      l.label = ev.label;
      l.done = l.total = l.filesDone = l.filesTotal = 0;
      l.speed = 0;
    } else if (ev.type === 'progress') {
      const now = performance.now();
      if (last.stage !== ev.stage || ev.done < last.done) last = { t: now, done: ev.done, stage: ev.stage };
      else if (now - last.t > 500) {
        const inst = ((ev.done - last.done) / (now - last.t)) * 1000;
        l.speed = l.speed ? l.speed * 0.6 + inst * 0.4 : inst;
        last = { t: now, done: ev.done, stage: ev.stage };
      }
      l.stage = ev.stage;
      l.done = ev.done;
      l.total = ev.total;
      l.filesDone = ev.files_done;
      l.filesTotal = ev.files_total;
    } else if (ev.type === 'log') {
      pushLogs([ev.line]);
    }
  });

  listen<{ instance_id: string; run_id: string | null; state: LaunchState['state']; message: string | null }>('launch://state', (ev) => {
    if (ev.state === 'running') {
      if (ev.run_id && !app.running.some((g) => g.run_id === ev.run_id)) app.running.push({ run_id: ev.run_id, instance_id: ev.instance_id });
      app.launch = { ...idle(), state: 'running' };
      if (app.settings?.show_console) app.consoleOpen = true;
    } else if (ev.state === 'error') {
      app.launch = { ...idle(), state: 'error', message: ev.message };
    } else if (ev.state === 'idle') {
      if (app.launch.state !== 'error') app.launch = idle();
      if (ev.message && ev.message !== 'Cancelled') toast(ev.message);
    }
  });

  listen<string[]>('game://log', (lines) => pushLogs(lines));

  listen<ExitInfo>('game://exit', (ev) => {
    app.running = app.running.filter((g) => g.run_id !== ev.run_id);
    if (ev.crashed) app.crash = ev;
  });
}

function pushLogs(lines: string[]) {
  app.logs.push(...lines);
  if (app.logs.length > 3000) app.logs.splice(0, app.logs.length - 3000);
}
