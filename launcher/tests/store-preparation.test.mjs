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
const identity = (operation_id, instance_id = 'synthetic-alpha') => ({ instance_id, operation_id });

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((accept, deny) => { resolve = accept; reject = deny; });
  return { promise, resolve, reject };
}

async function fixture(gameRunning = []) {
  const calls = [];
  const admissions = [];
  const listeners = new Map();
  const context = vm.createContext({ console, performance, setTimeout: () => 0, clearTimeout: () => {} });
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
        calls.push({ command, args });
        if (command === 'bootstrap') return Promise.resolve({ settings: {}, accounts: [], active_account: null, manifest: null, panel_url: null, game_running: gameRunning });
        if (command === 'launch' || command === 'repair_instance') {
          const admission = deferred();
          admissions.push(admission);
          return admission.promise;
        }
        return Promise.resolve();
      },
      listen: (event, handler) => { listeners.set(event, handler); return Promise.resolve(() => {}); },
      setExperienceContext: () => {},
      errorText: (error) => String(error),
    },
  };
  const module = new vm.SourceTextModule(compiled, { context });
  await module.link((specifier) => {
    const values = dependencies[specifier];
    assert.ok(values, `unexpected dependency ${specifier}`);
    return new vm.SyntheticModule(Object.keys(values), function () {
      for (const [name, value] of Object.entries(values)) this.setExport(name, value);
    }, { context });
  });
  await module.evaluate();
  await module.namespace.init();
  return { store: module.namespace, calls, admissions, emit: (event, payload) => listeners.get(event)(payload) };
}

const state = (operationId, phase, runId = null) => ({ ...identity(operationId), state: phase, run_id: runId, message: null });
const progress = (operationId, done) => ({ ...identity(operationId), type: 'progress', stage: 'files', done, total: 100, files_done: 1, files_total: 2 });

test('early repair completion is applied after its admission reply', async () => {
  const { store, admissions, emit } = await fixture();
  const repair = store.repair('synthetic-alpha');
  emit('launch://state', state('first-operation', 'idle'));
  assert.equal(store.app.launch.state, 'preparing');
  admissions[0].resolve('first-operation');
  await repair;
  assert.equal(store.app.launch.state, 'idle');
});

test('stale state progress and logs cannot reset another same-instance operation', async () => {
  const { store, admissions, emit } = await fixture();
  const previous = store.play('synthetic-alpha');
  admissions[0].resolve('first-operation');
  await previous;
  emit('launch://state', state('first-operation', 'running', 'first-run'));
  const current = store.play('synthetic-alpha');
  admissions[1].resolve('second-operation');
  await current;
  emit('launch://progress', progress('second-operation', 20));
  emit('launch://progress', progress('first-operation', 90));
  emit('game://log', { ...identity('first-operation'), run_id: 'first-run', lines: ['stale log'] });
  emit('launch://state', state('first-operation', 'idle', 'first-run'));
  assert.equal(store.app.launch.state, 'preparing');
  assert.equal(store.app.launch.done, 20);
  assert.equal(store.app.logs.length, 0);
});

test('wrong-instance event with the correct token is rejected', async () => {
  const { store, admissions, emit } = await fixture();
  const start = store.play('synthetic-alpha');
  admissions[0].resolve('first-operation');
  await start;
  emit('launch://progress', { ...progress('first-operation', 90), instance_id: 'synthetic-beta' });
  emit('launch://state', { ...state('first-operation', 'error'), instance_id: 'synthetic-beta' });
  assert.equal(store.app.launch.state, 'preparing');
  assert.equal(store.app.launch.done, 0);
});

test('progress and preparing after terminal state do not resurrect preparation', async () => {
  const { store, admissions, emit } = await fixture();
  const start = store.repair('synthetic-alpha');
  admissions[0].resolve('first-operation');
  await start;
  emit('launch://state', state('first-operation', 'idle'));
  emit('launch://state', state('first-operation', 'preparing'));
  emit('launch://progress', progress('first-operation', 90));
  assert.equal(store.app.launch.state, 'idle');
  assert.equal(store.app.launch.done, 0);
});

test('all store callers are blocked while admission is pending despite stale visible UI', async () => {
  const { store, admissions } = await fixture();
  const start = store.play('synthetic-alpha');
  store.app.launch.state = 'idle';
  await store.play('synthetic-beta');
  await store.repair('synthetic-beta');
  assert.equal(admissions.length, 1);
  admissions[0].resolve('first-operation');
  await start;
});

test('cancel snapshots a pending admission and supplies its exact token', async () => {
  const { store, calls, admissions } = await fixture();
  const start = store.play('synthetic-alpha');
  const cancel = store.cancel();
  admissions[0].resolve('first-operation');
  await start;
  await cancel;
  const cancellation = calls.find(({ command }) => command === 'cancel_launch');
  assert.equal(cancellation.args.instanceId, 'synthetic-alpha');
  assert.equal(cancellation.args.operationId, 'first-operation');
});

test('rejected admission never sends cancellation for an unidentified job', async () => {
  const { store, calls, admissions } = await fixture();
  const start = store.play('synthetic-alpha');
  const cancel = store.cancel();
  admissions[0].reject('synthetic denied');
  await start;
  await cancel;
  assert.equal(calls.filter(({ command }) => command === 'cancel_launch').length, 0);
  assert.equal(store.app.launch.state, 'error');
});

test('current owned logs are displayed and exit removes only its identified run', async () => {
  const { store, admissions, emit } = await fixture();
  const start = store.play('synthetic-alpha');
  admissions[0].resolve('first-operation');
  await start;
  emit('launch://state', state('first-operation', 'running', 'first-run'));
  emit('game://log', { ...identity('first-operation'), run_id: 'first-run', lines: ['current log'] });
  assert.equal(store.app.logs[0], 'current log');
  emit('launch://state', { ...state('other-operation', 'running', 'other-run'), instance_id: 'synthetic-beta' });
  emit('game://exit', { instance_id: 'synthetic-alpha', run_id: 'first-run', crashed: false });
  emit('launch://state', state('first-operation', 'idle', 'first-run'));
  assert.equal(store.app.running.length, 1);
  assert.equal(store.app.running[0].run_id, 'other-run');
  assert.equal(store.app.launch.state, 'idle');
});

test('bootstrapped run retains scoped live console logs without a preparation token', async () => {
  const { store, emit } = await fixture([{ instance_id: 'synthetic-alpha', run_id: 'restored-run' }]);
  emit('game://log', { ...identity('restored-operation'), run_id: 'restored-run', lines: ['restored live log'] });
  emit('game://log', { ...identity('restored-operation', 'synthetic-beta'), run_id: 'restored-run', lines: ['foreign log'] });
  emit('game://log', { ...identity('restored-operation'), run_id: 'unknown-run', lines: ['unknown log'] });
  assert.equal(store.app.logs.length, 1);
  assert.equal(store.app.logs[0], 'restored live log');
});

test('late running event cannot resurrect a run after its identified exit', async () => {
  const { store, admissions, emit } = await fixture();
  const start = store.play('synthetic-alpha');
  admissions[0].resolve('first-operation');
  await start;
  emit('launch://state', state('first-operation', 'running', 'first-run'));
  emit('game://exit', { instance_id: 'synthetic-alpha', run_id: 'first-run', crashed: false });
  emit('launch://state', state('first-operation', 'idle', 'first-run'));
  emit('launch://state', state('first-operation', 'running', 'first-run'));
  assert.equal(store.app.running.length, 0);
  assert.equal(store.app.launch.state, 'idle');
});
