import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import ts from 'typescript';
import { compileModule } from 'svelte/compiler';
import * as runtime from 'svelte/internal/client';
import { PreparationEvents } from '../src/lib/preparation-events.ts';
import { ContextDomain } from '../src/lib/context-domain.ts';
import { AccountDomain } from '../src/lib/account-domain.ts';
import { SettingsDomain } from '../src/lib/settings-domain.ts';

const source = fs.readFileSync(new URL('../src/lib/store.svelte.ts', import.meta.url), 'utf8');
const transpiled = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
}).outputText;
const compiled = compileModule(transpiled, { filename: 'store.svelte.js', generate: 'client' }).js.code;
const manifest = label => ({ branding: {}, instances: [{ id: 'alpha' }, { id: 'beta' }], label });
const account = id => ({ id, kind: 'offline', username: `synthetic-${id}`, uuid: id, panel_url: null, role: null });
const settle = async () => { for (let turn = 0; turn < 20; turn++) await Promise.resolve(); };

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((accept, deny) => { resolve = accept; reject = deny; });
  return { promise, resolve, reject };
}

async function fixture() {
  const calls = [];
  const timers = new Map();
  const contexts = [];
  let timerId = 0;
  let initialized = false;
  const boot = {
    settings: { selected_instance: 'alpha', check_updates: false, memory_max_mb: 2048, companion: { scale: 1 } },
    accounts: [account('first'), account('second')], active_account: 'first',
    manifest: manifest('initial'), panel_url: null, game_running: [],
  };
  const context = vm.createContext({
    console, performance,
    setTimeout: (callback, delay) => { const id = ++timerId; timers.set(id, { callback, delay }); return id; },
    clearTimeout: id => timers.delete(id),
  });
  const dependencies = {
    'svelte/internal/client': runtime,
    '@velora/experience': { pageEnabled: () => true },
    './preparation-events': { PreparationEvents },
    './context-domain': { ContextDomain },
    './account-domain': { AccountDomain },
    './settings-domain': { SettingsDomain },
    './theme': { applyTheme: () => {} },
    './tauri': {
      invoke: (command, args) => {
        if (!initialized && command === 'bootstrap') return Promise.resolve(boot);
        if (command === 'show_window') return Promise.resolve();
        const reply = deferred();
        calls.push({ command, args, ...reply });
        return reply.promise;
      },
      listen: () => Promise.resolve(() => {}),
      setExperienceContext: id => contexts.push(id),
      errorText: error => String(error),
    },
  };
  const module = new vm.SourceTextModule(compiled, { context });
  await module.link(specifier => {
    const values = dependencies[specifier];
    assert.ok(values, `unexpected dependency ${specifier}`);
    return new vm.SyntheticModule(Object.keys(values), function () {
      for (const [name, value] of Object.entries(values)) this.setExport(name, value);
    }, { context });
  });
  await module.evaluate();
  await module.namespace.init();
  initialized = true;
  return {
    store: module.namespace, calls, contexts, boot,
    flushSave: async () => {
      for (const [id, timer] of timers) if (timer.delay === 300) { timers.delete(id); timer.callback(); }
      await settle();
    },
  };
}

test('only the newest manifest response owns online state and content', async () => {
  const { store, calls } = await fixture();
  const older = store.refresh();
  const newer = store.refresh();
  await settle();
  assert.equal(calls.length, 1);
  calls[0].resolve(manifest('newer'));
  await Promise.all([older, newer]);
  assert.equal(store.app.manifest.label, 'newer');
  const stale = store.refresh();
  await settle();
  const latest = store.refresh();
  await settle();
  calls[2].resolve(manifest('latest'));
  await latest;
  calls[1].reject(new Error('stale outage'));
  await stale;
  assert.equal(store.app.manifest.label, 'latest');
  assert.equal(store.app.offline, false);
});

test('refresh preserves an instance selected while its reply is pending', async () => {
  const { store, calls, contexts } = await fixture();
  const refresh = store.refresh();
  await settle();
  store.selectInstance('beta');
  calls[0].resolve(manifest('refreshed'));
  await refresh;
  assert.equal(store.app.selected, 'beta');
  assert.equal(contexts.at(-1), 'beta');
});

test('connect invalidates earlier refresh and serializes panel mutations', async () => {
  const { store, calls, boot } = await fixture();
  const refresh = store.refresh();
  await settle();
  const first = store.connect('https://first.invalid');
  const second = store.connect('https://second.invalid');
  await settle();
  assert.equal(calls.length, 2);
  calls[0].resolve(manifest('stale'));
  await refresh;
  calls[1].resolve(manifest('first'));
  await settle();
  calls[2].resolve({ ...boot, panel_url: 'https://first.invalid' });
  await first;
  await settle();
  assert.equal(store.app.panelUrl, null);
  calls[3].resolve(manifest('second'));
  await settle();
  calls[4].resolve({ ...boot, panel_url: 'https://second.invalid' });
  await second;
  assert.equal(store.app.panelUrl, 'https://second.invalid');
  assert.equal(store.app.manifest.label, 'second');
});

test('failed context mutation does not poison later queued account selection', async () => {
  const { store, calls, boot } = await fixture();
  const first = store.switchAccount('first');
  const failure = assert.rejects(first, /denied/);
  const second = store.switchAccount('second');
  await settle();
  calls[0].reject(new Error('denied'));
  await failure;
  await settle();
  assert.equal(calls[1].command, 'select_account');
  calls[1].resolve();
  await settle();
  assert.equal(calls[2].command, 'bootstrap');
  calls[2].resolve({ ...boot, active_account: 'second', panel_url: 'https://recovered.invalid' });
  await settle();
  calls[3].resolve(manifest('second-account'));
  await second;
  assert.equal(store.app.active, 'second');
});

test('superseded partial connect failure is reconciled by later successful account mutation', async () => {
  const { store, calls, boot } = await fixture();
  const connection = store.connect('https://accepted-partial.invalid');
  const rejected = assert.rejects(connection, /superseded bootstrap failed/);
  const selection = store.switchAccount('second');
  await settle();
  calls[0].resolve(manifest('accepted-partial'));
  await settle();
  calls[1].reject(new Error('superseded bootstrap failed'));
  await rejected;
  await settle();
  assert.equal(calls[2].command, 'select_account');
  calls[2].resolve();
  await settle();
  assert.equal(calls[3].command, 'bootstrap');
  calls[3].resolve({ ...boot, active_account: 'second', panel_url: 'https://accepted-partial.invalid' });
  await settle();
  calls[4].resolve(manifest('authoritative-partial-second-account'));
  await selection;
  assert.equal(store.app.panelUrl, 'https://accepted-partial.invalid');
  assert.equal(store.app.active, 'second');
  assert.equal(store.app.manifest.label, 'authoritative-partial-second-account');
});

test('successful latest connect reconciles uncertain account mutation from its authoritative bootstrap', async () => {
  const { store, calls, boot } = await fixture();
  const selection = store.switchAccount('second');
  const rejected = assert.rejects(selection, /uncertain account mutation/);
  const connection = store.connect('https://newest.invalid');
  await settle();
  calls[0].reject(new Error('uncertain account mutation'));
  await rejected;
  await settle();
  calls[1].resolve(manifest('newest-second-account'));
  await settle();
  calls[2].resolve({ ...boot, active_account: 'second', panel_url: 'https://newest.invalid' });
  await connection;
  assert.equal(store.app.active, 'second');
  assert.equal(store.app.panelUrl, 'https://newest.invalid');
  assert.equal(store.app.manifest.label, 'newest-second-account');
});

test('older successful account choice cannot overwrite newer choice', async () => {
  const { store, calls } = await fixture();
  const first = store.switchAccount('second');
  const second = store.switchAccount('first');
  await settle();
  calls[0].resolve();
  await first;
  await settle();
  assert.equal(store.app.active, 'first');
  calls[1].resolve();
  await settle();
  calls[2].resolve(manifest('first-account'));
  await second;
  assert.equal(store.app.active, 'first');
});

test('older account success followed by latest failure reconciles host account and fresh manifest', async () => {
  const { store, calls, boot } = await fixture();
  const older = store.switchAccount('second');
  const latest = store.switchAccount('first');
  const rejected = assert.rejects(latest, /latest account failed/);
  await settle();
  calls[0].resolve();
  await older;
  await settle();
  calls[1].reject(new Error('latest account failed'));
  await settle();
  assert.equal(calls[2].command, 'bootstrap');
  calls[2].resolve({ ...boot, active_account: 'second', panel_url: 'https://recovered.invalid', manifest: manifest('cached-old-account') });
  await settle();
  assert.equal(store.app.active, 'second');
  assert.equal(store.app.manifest, null);
  assert.equal(calls[3].command, 'refresh_manifest');
  calls[3].resolve(manifest('authoritative-second-account'));
  await rejected;
  assert.equal(store.app.active, 'second');
  assert.equal(store.app.manifest.label, 'authoritative-second-account');
  assert.equal(store.app.offline, false);
});

test('older panel success followed by latest failure reconciles accepted host panel', async () => {
  const { store, calls, boot } = await fixture();
  const older = store.connect('https://accepted.invalid');
  const latest = store.connect('https://rejected.invalid');
  const rejected = assert.rejects(latest, /latest panel failed/);
  await settle();
  calls[0].resolve(manifest('accepted-panel'));
  await settle();
  calls[1].resolve({ ...boot, panel_url: 'https://accepted.invalid' });
  await older;
  await settle();
  calls[2].reject(new Error('latest panel failed'));
  await settle();
  assert.equal(calls[3].command, 'bootstrap');
  calls[3].resolve({ ...boot, panel_url: 'https://accepted.invalid' });
  await settle();
  assert.equal(calls[4].command, 'refresh_manifest');
  calls[4].resolve(manifest('authoritative-accepted-panel'));
  await rejected;
  assert.equal(store.app.panelUrl, 'https://accepted.invalid');
  assert.equal(store.app.manifest.label, 'authoritative-accepted-panel');
});

test('accepted panel mutation with failed bootstrap reconciles instead of assuming no mutation', async () => {
  const { store, calls, boot } = await fixture();
  const connection = store.connect('https://partial.invalid');
  const rejected = assert.rejects(connection, /initial bootstrap failed/);
  await settle();
  calls[0].resolve(manifest('partial-panel'));
  await settle();
  calls[1].reject(new Error('initial bootstrap failed'));
  await settle();
  assert.equal(calls[2].command, 'bootstrap');
  calls[2].resolve({ ...boot, panel_url: 'https://partial.invalid' });
  await settle();
  calls[3].resolve(manifest('authoritative-partial-panel'));
  await rejected;
  assert.equal(store.app.panelUrl, 'https://partial.invalid');
  assert.equal(store.app.manifest.label, 'authoritative-partial-panel');
});

test('delayed reconciliation cannot resurrect an account after a newer request starts', async () => {
  const { store, calls, boot } = await fixture();
  const failed = store.switchAccount('second');
  const rejected = assert.rejects(failed, /failed account/);
  await settle();
  calls[0].reject(new Error('failed account'));
  await settle();
  assert.equal(calls[1].command, 'bootstrap');
  const newer = store.switchAccount('first');
  calls[1].resolve({ ...boot, active_account: 'second', panel_url: 'https://stale.invalid' });
  await rejected;
  await settle();
  assert.equal(store.app.active, 'first');
  assert.equal(store.app.panelUrl, null);
  assert.equal(calls[2].command, 'select_account');
  calls[2].resolve();
  await settle();
  assert.equal(calls[3].command, 'bootstrap');
  calls[3].resolve({ ...boot, panel_url: 'https://latest.invalid' });
  await settle();
  calls[4].resolve(manifest('latest-account'));
  await newer;
  assert.equal(store.app.active, 'first');
  assert.equal(store.app.panelUrl, 'https://latest.invalid');
  assert.equal(store.app.manifest.label, 'latest-account');
});

test('reconciliation manifest cannot replace a later panel connection', async () => {
  const { store, calls, boot } = await fixture();
  const failed = store.connect('https://failed.invalid');
  const rejected = assert.rejects(failed, /failed panel/);
  await settle();
  calls[0].reject(new Error('failed panel'));
  await settle();
  calls[1].resolve({ ...boot, panel_url: 'https://recovered.invalid' });
  await settle();
  assert.equal(calls[2].command, 'refresh_manifest');
  const newer = store.connect('https://newest.invalid');
  await settle();
  calls[3].resolve(manifest('newest-panel'));
  await settle();
  calls[4].resolve({ ...boot, panel_url: 'https://newest.invalid' });
  await newer;
  calls[2].resolve(manifest('stale-recovery'));
  await rejected;
  assert.equal(store.app.panelUrl, 'https://newest.invalid');
  assert.equal(store.app.manifest.label, 'newest-panel');
});

test('recovery bootstrap cannot replace a panel requested after recovery started', async () => {
  const { store, calls, boot } = await fixture();
  const failed = store.connect('https://failed.invalid');
  const rejected = assert.rejects(failed, /failed panel/);
  await settle();
  calls[0].reject(new Error('failed panel'));
  await settle();
  assert.equal(calls[1].command, 'bootstrap');
  const newest = store.connect('https://newest.invalid');
  calls[1].resolve({ ...boot, panel_url: 'https://stale-recovery.invalid' });
  await rejected;
  await settle();
  assert.equal(store.app.panelUrl, null);
  calls[2].resolve(manifest('newest-panel'));
  await settle();
  calls[3].resolve({ ...boot, panel_url: 'https://newest.invalid' });
  await newest;
  assert.equal(store.app.panelUrl, 'https://newest.invalid');
  assert.equal(store.app.manifest.label, 'newest-panel');
});

test('recovery preserves instance selection and local settings edits made during bootstrap', async () => {
  const { store, calls, boot, contexts } = await fixture();
  const failed = store.switchAccount('second');
  const rejected = assert.rejects(failed, /failed selection/);
  await settle();
  calls[0].reject(new Error('failed selection'));
  await settle();
  store.selectInstance('beta');
  store.app.settings.memory_max_mb = 8192;
  calls[1].resolve({ ...boot, active_account: 'second', panel_url: 'https://recovered.invalid' });
  await settle();
  calls[2].resolve(manifest('recovered-account'));
  await rejected;
  assert.equal(store.app.selected, 'beta');
  assert.equal(store.app.settings.memory_max_mb, 8192);
  assert.equal(store.settingsPersistence.status, 'pending');
  assert.equal(contexts.at(-1), 'beta');
});

test('failed removal reconciles authoritative inventory even if the host accepted its side effect', async () => {
  const { store, calls, boot } = await fixture();
  const removal = store.removeAccount('first');
  const rejected = assert.rejects(removal, /uncertain removal/);
  await settle();
  calls[0].reject(new Error('uncertain removal'));
  await settle();
  calls[1].resolve({ ...boot, accounts: [account('second')], active_account: 'second', panel_url: null });
  await rejected;
  assert.equal(store.app.accounts.length, 1);
  assert.equal(store.app.accounts[0].id, 'second');
  assert.equal(store.app.active, 'second');
  assert.equal(store.app.manifest, null);
});

test('failed recovery hides unverified context and preserves the original mutation error', async () => {
  const { store, calls } = await fixture();
  const selection = store.switchAccount('second');
  const rejected = assert.rejects(selection, /original mutation failure/);
  await settle();
  calls[0].reject(new Error('original mutation failure'));
  await settle();
  calls[1].reject(new Error('recovery unavailable'));
  await rejected;
  assert.equal(store.app.active, null);
  assert.equal(store.app.manifest, null);
  assert.equal(store.app.offline, true);
  assert.match(store.app.toasts.at(-1).text, /recovery unavailable/);
});

test('successful removal still updates inventory when newer selection is queued', async () => {
  const { store, calls } = await fixture();
  const removal = store.removeAccount('first');
  const selection = store.switchAccount('second');
  await settle();
  calls[0].resolve();
  await removal;
  await settle();
  assert.equal(store.app.accounts.some(existing => existing.id === 'first'), false);
  calls[1].resolve();
  await settle();
  calls[2].resolve(manifest('remaining-account'));
  await selection;
  assert.equal(store.app.active, 'second');
});

test('account addition invalidates an older account refresh', async () => {
  const { store, calls } = await fixture();
  const old = store.refresh();
  await settle();
  const added = store.accountAdded(account('third'));
  await settle();
  calls[1].resolve(manifest('third-account'));
  await added;
  calls[0].resolve(manifest('old-account'));
  await old;
  assert.equal(store.app.active, 'third');
  assert.equal(store.app.manifest.label, 'third-account');
});

test('settings failure restores persisted fields and reports error rather than saved', async () => {
  const { store, calls, flushSave } = await fixture();
  store.app.settings.memory_max_mb = 4096;
  store.saveSettings();
  assert.equal(store.settingsPersistence.status, 'pending');
  await flushSave();
  calls[0].reject(new Error('synthetic disk full'));
  await settle();
  assert.equal(store.app.settings.memory_max_mb, 2048);
  assert.equal(store.settingsPersistence.status, 'error');
  assert.match(store.settingsPersistence.error, /disk full/);
});

test('bootstrap is not advertised as a confirmed frontend save', async () => {
  const { store } = await fixture();
  assert.equal(store.settingsPersistence.status, 'idle');
});

test('old removal cannot erase a newly added replacement of the same account', async () => {
  const { store, calls } = await fixture();
  const removal = store.removeAccount('first');
  await settle();
  const added = store.accountAdded({ ...account('first'), username: 'synthetic-replacement' });
  calls[0].resolve();
  await removal;
  await settle();
  calls[1].resolve(manifest('replacement-account'));
  await added;
  assert.equal(store.app.accounts.find(existing => existing.id === 'first').username, 'synthetic-replacement');
  assert.equal(store.app.active, 'first');
});

test('panel connection is retained when a newer account selection arrives', async () => {
  const { store, calls, boot } = await fixture();
  const connection = store.connect('https://panel.invalid');
  await settle();
  const selection = store.switchAccount('second');
  calls[0].resolve(manifest('old-account'));
  await settle();
  calls[1].resolve({ ...boot, panel_url: 'https://panel.invalid' });
  await settle();
  assert.equal(store.app.panelUrl, 'https://panel.invalid');
  assert.equal(store.app.manifest, null);
  calls[2].resolve();
  await settle();
  const refreshes = calls.filter(call => call.command === 'refresh_manifest');
  assert.equal(refreshes.length, 1);
  refreshes[0].resolve(manifest('second-account'));
  await Promise.all([connection, selection]);
  assert.equal(store.app.active, 'second');
  assert.equal(store.app.manifest.label, 'second-account');
});

test('current refresh failure marks offline without destroying cached manifest', async () => {
  const { store, calls } = await fixture();
  const refresh = store.refresh();
  await settle();
  calls[0].reject(new Error('current outage'));
  await refresh;
  assert.equal(store.app.offline, true);
  assert.equal(store.app.manifest.label, 'initial');
});

test('settings debounce sends only the latest snapshot', async () => {
  const { store, calls, flushSave } = await fixture();
  store.app.settings.memory_max_mb = 4096;
  store.saveSettings();
  store.app.settings.memory_max_mb = 8192;
  store.saveSettings();
  await flushSave();
  assert.equal(calls.length, 1);
  assert.equal(calls[0].args.settings.memory_max_mb, 8192);
  calls[0].resolve();
  await settle();
  assert.equal(store.settingsPersistence.status, 'saved');
});

test('settings rollback preserves unscheduled later nested edits', async () => {
  const { store, calls, flushSave } = await fixture();
  store.app.settings.memory_max_mb = 4096;
  store.app.settings.companion.scale = 2;
  store.saveSettings();
  await flushSave();
  store.app.settings.companion.scale = 3;
  calls[0].reject(new Error('failed'));
  await settle();
  assert.equal(store.app.settings.memory_max_mb, 2048);
  assert.equal(store.app.settings.companion.scale, 3);
  assert.equal(store.settingsPersistence.status, 'error');
});

test('successful older snapshot does not advertise later unscheduled edits as saved', async () => {
  const { store, calls, flushSave } = await fixture();
  store.app.settings.memory_max_mb = 4096;
  store.saveSettings();
  await flushSave();
  store.app.settings.memory_max_mb = 8192;
  calls[0].resolve();
  await settle();
  assert.equal(store.app.settings.memory_max_mb, 8192);
  assert.equal(store.settingsPersistence.status, 'pending');
});

test('settings saves serialize and stale failure cannot erase newer edits', async () => {
  const { store, calls, flushSave } = await fixture();
  store.app.settings.memory_max_mb = 4096;
  store.saveSettings();
  await flushSave();
  store.app.settings.memory_max_mb = 8192;
  store.saveSettings();
  await flushSave();
  assert.equal(calls.length, 1);
  assert.equal(calls[0].args.settings.memory_max_mb, 4096);
  calls[0].reject(new Error('older failure'));
  await settle();
  assert.equal(calls.length, 2);
  assert.equal(store.app.settings.memory_max_mb, 8192);
  assert.equal(store.settingsPersistence.status, 'pending');
  calls[1].resolve();
  await settle();
  assert.equal(store.settingsPersistence.status, 'saved');
});

test('later save failure rolls back to last successful save, not bootstrap', async () => {
  const { store, calls, flushSave } = await fixture();
  store.app.settings.memory_max_mb = 4096;
  store.saveSettings();
  await flushSave();
  calls[0].resolve();
  await settle();
  store.app.settings.memory_max_mb = 8192;
  store.saveSettings();
  await flushSave();
  calls[1].reject(new Error('later failure'));
  await settle();
  assert.equal(store.app.settings.memory_max_mb, 4096);
  assert.equal(store.settingsPersistence.status, 'error');
});

test('failed selected-instance save reconciles context without losing newer selection', async () => {
  const { store, calls, flushSave, contexts } = await fixture();
  store.selectInstance('beta');
  await flushSave();
  calls[0].reject(new Error('failed selection save'));
  await settle();
  assert.equal(store.app.selected, 'alpha');
  assert.equal(contexts.at(-1), 'alpha');
  store.selectInstance('beta');
  await flushSave();
  store.selectInstance('alpha');
  calls[1].reject(new Error('stale selection failure'));
  await settle();
  assert.equal(store.app.selected, 'alpha');
  assert.equal(store.settingsPersistence.status, 'pending');
});
