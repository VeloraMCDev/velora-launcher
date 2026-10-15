import { pageEnabled } from '@velora/experience';
import { PreparationEvents, type OperationIdentity } from './preparation-events';
import { ContextDomain } from './context-domain';
import { AccountDomain } from './account-domain';
import { SettingsDomain } from './settings-domain';
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

const contextDomain = new ContextDomain();
const accountDomain = new AccountDomain();

export const settingsPersistence = $state({
  status: 'idle' as 'idle' | 'saved' | 'pending' | 'error',
  error: null as string | null,
});

const settingsDomain = new SettingsDomain();

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
  if (branding) applyTheme(branding, app.settings, app.panelUrl ?? '');
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
  settingsDomain.durable = $state.snapshot(boot.settings);
  app.accounts = boot.accounts;
  app.active = boot.active_account;
  app.manifest = boot.manifest;
  app.panelUrl = boot.panel_url;
  app.running = boot.game_running;
  consoleRunId = app.running[0]?.run_id ?? null;
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
  const revision = contextDomain.revision;
  try {
    const update = await invoke<UpdateInfo | null>('check_update');
    if (app.settings?.check_updates && panel === app.panelUrl && revision === contextDomain.revision) app.update = update;
  } catch { /* Keep the last known update during temporary outages. */ }
  finally {
    updateCheckBusy = false;
    if (panel !== app.panelUrl || revision !== contextDomain.revision) void refreshUpdates(true);
  }
}

export async function refresh() {
  const request = contextDomain.beginRefresh(app.panelUrl, app.active);
  const current = () => contextDomain.ownsRefresh(request, app.panelUrl, app.active);
  try {
    await contextDomain.tail;
    if (!current()) return;
    const manifest = await invoke<Manifest>('refresh_manifest');
    if (!current()) return;
    app.manifest = manifest;
    app.offline = false;
    if (request.selection === contextDomain.selection || !instances().some(instance => instance.id === app.selected)) pickSelected();
    else setExperienceContext(app.selected);
    theme();
  } catch (error) {
    if (!current()) return;
    app.offline = true;
    if (!app.manifest) throw error;
  }
}

async function reconcileContext(revision: number) {
  if (revision !== contextDomain.revision) return;
  contextDomain.refresh++;
  try {
    const boot = await contextDomain.enqueue(() => revision === contextDomain.revision ? invoke<Bootstrap>('bootstrap') : Promise.resolve(null));
    if (!boot || revision !== contextDomain.revision) return;
    contextDomain.uncertain = false;
    contextDomain.refresh++;
    app.accounts = boot.accounts;
    app.active = boot.active_account;
    app.panelUrl = boot.panel_url;
    app.manifest = null;
    app.update = null;
    app.offline = Boolean(app.panelUrl);
    setExperienceContext(null);
    if (app.panelUrl) await refresh().catch(() => {});
    else {
      pickSelected();
      theme();
    }
    if (revision === contextDomain.revision) void refreshUpdates(true);
  } catch (error) {
    if (revision !== contextDomain.revision) return;
    app.active = null;
    app.manifest = null;
    app.update = null;
    app.offline = true;
    setExperienceContext(null);
    toast(`Couldn't reconcile launcher context: ${errorText(error)}`, 'error');
  }
}

export async function connect(url: string) {
  const revision = ++contextDomain.revision;
  const panelRequest = ++contextDomain.panel;
  const selection = contextDomain.selection;
  const result = await contextDomain.mutate(revision, async () => {
    const manifest = await invoke<Manifest>('set_panel_url', { url });
    const boot = await invoke<Bootstrap>('bootstrap');
    return { manifest, boot };
  }, reconcileContext);
  if (panelRequest !== contextDomain.panel) return;
  if (revision === contextDomain.revision) {
    if (contextDomain.uncertain) {
      app.accounts = result.boot.accounts;
      app.active = result.boot.active_account;
    }
    contextDomain.uncertain = false;
  }
  app.manifest = revision === contextDomain.revision ? result.manifest : null;
  app.panelUrl = result.boot.panel_url;
  app.update = null;
  void refreshUpdates(true);
  app.offline = false;
  if (selection === contextDomain.selection || !instances().some(instance => instance.id === app.selected)) pickSelected();
  else setExperienceContext(app.selected);
  theme();
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;
export function saveSettings() {
  theme();
  if (!app.settings?.check_updates) app.update = null;
  else void refreshUpdates();
  if (!app.settings) return;
  const revision = ++settingsDomain.revision;
  settingsPersistence.status = 'pending';
  settingsPersistence.error = null;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    if (!app.settings) return;
    const candidate = $state.snapshot(app.settings);
    settingsDomain.persist(candidate, revision, () => invoke('save_settings', { settings: candidate }), () => {
      settingsPersistence.status = app.settings && JSON.stringify($state.snapshot(app.settings)) === JSON.stringify(candidate) ? 'saved' : 'pending';
      settingsPersistence.error = null;
    }, (error, current) => {
      if (current) {
        if (app.settings) {
          const selected = app.settings.selected_instance;
          settingsDomain.rollback(app.settings, candidate);
          if (selected !== app.settings.selected_instance && app.selected === selected) pickSelected();
          theme();
        }
        settingsPersistence.status = 'error';
        settingsPersistence.error = errorText(error);
      }
      toast(errorText(error), 'error');
    });
  }, 300);
}

export function selectInstance(id: string) {
  if (!instances().some(instance => instance.id === id)) return;
  contextDomain.selection++;
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
  accountDomain.revision++;
  const contextRevision = ++contextDomain.revision;
  app.accounts = accountDomain.added(app.accounts, account);
  app.active = account.id;
  app.addAccount = false;
  await contextDomain.tail;
  if (contextRevision !== contextDomain.revision) return;
  if (contextDomain.uncertain) {
    await reconcileContext(contextRevision);
    return;
  }
  await refresh().catch(() => {});
}

export async function switchAccount(id: string) {
  const revision = ++accountDomain.revision;
  const contextRevision = ++contextDomain.revision;
  await contextDomain.mutate(contextRevision, () => invoke('select_account', { id }), reconcileContext);
  if (revision !== accountDomain.revision) return;
  if (contextDomain.uncertain && contextRevision === contextDomain.revision) {
    await reconcileContext(contextRevision);
    return;
  }
  app.active = id;
  await refresh().catch(() => {});
}

export async function removeAccount(id: string) {
  const revision = ++accountDomain.revision;
  const addition = accountDomain.addition(id);
  const contextRevision = ++contextDomain.revision;
  await contextDomain.mutate(contextRevision, () => invoke('remove_account', { id }), reconcileContext);
  if (addition !== accountDomain.addition(id)) return;
  if (contextDomain.uncertain && contextRevision === contextDomain.revision) {
    await reconcileContext(contextRevision);
    return;
  }
  app.accounts = accountDomain.removed(app.accounts, id);
  if (app.active === id) app.active = app.accounts[0]?.id ?? null;
  if (revision !== accountDomain.revision) return;
  await refresh().catch(() => {});
}

const preparationEvents = new PreparationEvents();
const exitedRuns = new Set<string>();
let consoleRunId: string | null = null;

export async function play(id: string) {
  if (preparationEvents.busy) return;
  consoleRunId = null;
  last = { t: 0, done: 0, stage: '' };
  app.launch = { ...idle(), state: 'preparing', label: 'Getting ready…' };
  app.launchingId = id;
  app.logs = [];
  const previousRequest = preparationEvents.request;
  const request = preparationEvents.begin(id, () => invoke<string>('launch', { instanceId: id }));
  if (!request) return;
  try {
    preparationEvents.admit(request, await request.operation);
  } catch (e) {
    if (preparationEvents.reject(request, previousRequest)) {
      app.launch = { ...idle(), state: 'error', message: errorText(e) };
    }
  }
}

export async function repair(id: string) {
  if (preparationEvents.busy) return;
  last = { t: 0, done: 0, stage: '' };
  app.launch = { ...idle(), state: 'preparing', label: 'Verifying files…' };
  app.launchingId = id;
  const previousRequest = preparationEvents.request;
  const request = preparationEvents.begin(id, () => invoke<string>('repair_instance', { instanceId: id }));
  if (!request) return;
  try {
    preparationEvents.admit(request, await request.operation);
  } catch (e) {
    if (preparationEvents.reject(request, previousRequest)) {
      app.launch = { ...idle(), state: 'error', message: errorText(e) };
    }
  }
}

export async function cancel() {
  const request = preparationEvents.request;
  if (!request) return;
  const operationId = await request.operation.catch(() => null);
  if (operationId) await invoke('cancel_launch', { instanceId: request.instanceId, operationId });
}

export async function kill(runId: string) {
  await invoke('kill_game', { runId });
}

let subscribed = false;
let last = { t: 0, done: 0, stage: '' as string };

function subscribe() {
  if (subscribed) return;
  subscribed = true;

  listen<ProgressEvent & OperationIdentity>('launch://progress', (ev) => {
    preparationEvents.event(ev, 'progress', () => {
      if (!preparationEvents.busy) return;
      const launch = app.launch;
      if (ev.type === 'stage') {
        launch.stage = ev.stage;
        launch.label = ev.label;
        launch.done = launch.total = launch.filesDone = launch.filesTotal = 0;
        launch.speed = 0;
      } else if (ev.type === 'progress') {
        const now = performance.now();
        if (last.stage !== ev.stage || ev.done < last.done) last = { t: now, done: ev.done, stage: ev.stage };
        else if (now - last.t > 500) {
          const speed = ((ev.done - last.done) / (now - last.t)) * 1000;
          launch.speed = launch.speed ? launch.speed * 0.6 + speed * 0.4 : speed;
          last = { t: now, done: ev.done, stage: ev.stage };
        }
        launch.stage = ev.stage;
        launch.done = ev.done;
        launch.total = ev.total;
        launch.filesDone = ev.files_done;
        launch.filesTotal = ev.files_total;
      } else if (ev.type === 'log') {
        pushLogs([ev.line]);
      }
    });
  });

  listen<OperationIdentity & { run_id: string | null; state: LaunchState['state']; message: string | null }>('launch://state', (ev) => {
    if (ev.state === 'running' && ev.run_id && exitedRuns.has(ev.run_id)) return;
    if (ev.state === 'running' && ev.run_id && !app.running.some((game) => game.run_id === ev.run_id)) {
      app.running.push({ run_id: ev.run_id, instance_id: ev.instance_id });
    }
    preparationEvents.event(ev, 'state', () => {
      if (!preparationEvents.transition(ev.state)) return;
      if (ev.state === 'running') {
        consoleRunId = ev.run_id;
        app.launch = { ...idle(), state: 'running' };
        if (app.settings?.show_console) app.consoleOpen = true;
      } else if (ev.state === 'error') {
        app.launch = { ...idle(), state: 'error', message: ev.message };
      } else if (ev.state === 'idle') {
        if (app.launch.state !== 'error') app.launch = app.running.some((game) => game.instance_id === ev.instance_id) ? { ...idle(), state: 'running' } : idle();
        if (ev.message && ev.message !== 'Cancelled') toast(ev.message);
      }
    });
  });

  listen<OperationIdentity & { run_id: string; lines: string[] }>('game://log', (ev) => {
    const knownRun = app.running.some((game) => game.run_id === ev.run_id && game.instance_id === ev.instance_id);
    if (!knownRun || exitedRuns.has(ev.run_id)) return;
    if (!preparationEvents.busy && consoleRunId === ev.run_id) pushLogs(ev.lines);
    else preparationEvents.event(ev, 'log', () => {
      if (consoleRunId === ev.run_id) pushLogs(ev.lines);
    });
  });

  listen<ExitInfo>('game://exit', (ev) => {
    exitedRuns.add(ev.run_id);
    app.running = app.running.filter((g) => g.run_id !== ev.run_id);
    if (ev.crashed) app.crash = ev;
  });
}

function pushLogs(lines: string[]) {
  app.logs.push(...lines);
  if (app.logs.length > 3000) app.logs.splice(0, app.logs.length - 3000);
}
