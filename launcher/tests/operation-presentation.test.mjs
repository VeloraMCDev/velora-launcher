import assert from 'node:assert/strict';
import test from 'node:test';
import { OperationPresentation, preparationProgress, safePhase, launchShortcutAllowed } from '../src/lib/operation-presentation.ts';

test('background launch shortcut ignores editable, consumed, composing, repeating, modal and inert events', () => {
  const event = { key: 'Enter', ctrlKey: true, metaKey: false, repeat: false, defaultPrevented: false, isComposing: false };
  assert.equal(launchShortcutAllowed(event, false, false, false), true);
  for (const flag of ['repeat', 'defaultPrevented', 'isComposing']) assert.equal(launchShortcutAllowed({ ...event, [flag]: true }, false, false, false), false);
  for (const flags of [[true, false, false], [false, true, false], [false, false, true]]) assert.equal(launchShortcutAllowed(event, ...flags), false);
  assert.equal(launchShortcutAllowed({ ...event, ctrlKey: false }, false, false, false), false);
  assert.equal(launchShortcutAllowed({ ...event, ctrlKey: false, metaKey: true }, false, false, false), true);
});

const launchState = (values = {}) => ({ state: 'idle', stage: null, label: '', done: 0, total: 0, filesDone: 0, filesTotal: 0, speed: 0, message: null, ...values });
const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((accept, deny) => { resolve = accept; reject = deny; });
  return { promise, resolve, reject };
};

function fixture() {
  const controller = new OperationPresentation();
  let context = { selectedId: 'synthetic-alpha', ownerId: null, launch: launchState(), hasAccount: true };
  const calls = [];
  const admissions = [];
  const current = () => context;
  const perform = (kind, instanceId) => {
    calls.push({ kind, instanceId });
    const admission = deferred();
    admissions.push(admission);
    context = { ...context, ownerId: instanceId, launch: launchState({ state: 'preparing' }) };
    return admission.promise;
  };
  const update = (values) => { context = { ...context, ...values }; return controller.observe(context); };
  const start = (kind = 'launch') => controller.start(kind, current(), current, perform);
  return { controller, calls, admissions, current, perform, update, start };
}

test('positive totals are phase-local and bounded, never overall completion', () => {
  const progress = preparationProgress(launchState({ done: 160, total: 100, filesDone: 2, filesTotal: 3 }));
  assert.equal(progress.percent, 100);
  assert.equal(progress.done, 160);
  assert.equal(progress.determinate, true);
  assert.match(progress.description, /not overall/);
  assert.equal(preparationProgress(launchState({ total: 100 })).percent, 0);
});

test('zero and missing totals remain indeterminate, including reported work', () => {
  for (const total of [0, -1, NaN, Infinity, undefined]) {
    const progress = preparationProgress(launchState({ done: 512, total }));
    assert.equal(progress.determinate, false);
    assert.equal(progress.percent, null);
    assert.match(progress.description, /not available/);
  }
  assert.match(preparationProgress(launchState()).description, /does not mean installation is complete/);
});

test('malformed counters stay finite and nonnegative', () => {
  const progress = preparationProgress(launchState({ done: -5, total: Infinity, filesDone: NaN, filesTotal: -1 }));
  assert.equal(progress.done, 0);
  assert.equal(progress.total, 0);
  assert.equal(progress.filesDone, 0);
  assert.equal(progress.filesTotal, 0);
});

test('safe phases never reflect labels, paths, URLs or token-bearing errors', () => {
  for (const label of ['https://synthetic.invalid/?token=synthetic-secret', '/synthetic/private/path', '__proto__', 'constructor']) {
    assert.equal(safePhase(launchState({ label, stage: 'files' })), 'Preparing instance files');
  }
  assert.equal(safePhase(launchState({ label: 'Checking Java', stage: 'java' })), 'Checking Java');
  assert.equal(safePhase(launchState({ stage: '__proto__' })), 'Preparing Minecraft');
  const { controller, update } = fixture();
  const view = update({ ownerId: 'synthetic-alpha', launch: launchState({ state: 'error', message: 'synthetic-secret https://synthetic.invalid /private/path' }) });
  assert.equal(controller.kind, null);
  assert.equal(view.canRetry, false);
  assert.doesNotMatch(JSON.stringify(view), /synthetic-secret|synthetic.invalid|private\/path/);
});

test('failed repair retains last safe phase and retry calls repair, not play', async () => {
  const fixtureState = fixture();
  const first = fixtureState.start('repair');
  fixtureState.current().launch.stage = 'files';
  fixtureState.current().launch.label = 'Checking instance files';
  fixtureState.controller.observe(fixtureState.current());
  fixtureState.admissions[0].resolve();
  await first;
  const failed = fixtureState.update({ launch: launchState({ state: 'error', message: 'unsafe synthetic error' }) });
  assert.equal(failed.state, 'failed');
  assert.equal(failed.phase, 'Checking instance files');
  assert.equal(failed.canRetry, true);
  const retry = fixtureState.controller.retry(failed.scope, fixtureState.current(), fixtureState.current, fixtureState.perform);
  assert.deepEqual(fixtureState.calls.map(call => call.kind), ['repair', 'repair']);
  fixtureState.admissions[1].resolve();
  await retry;
});

test('launch retry requires an account, repair does not', async () => {
  for (const kind of ['launch', 'repair']) {
    const fixtureState = fixture();
    const start = fixtureState.start(kind);
    fixtureState.admissions[0].resolve();
    await start;
    const view = fixtureState.update({ hasAccount: false, launch: launchState({ state: 'error' }) });
    assert.equal(view.canRetry, kind === 'repair');
  }
});

test('cancel during admission invokes once and waits for terminal state', async () => {
  const fixtureState = fixture();
  const start = fixtureState.start();
  const cancellation = deferred();
  let calls = 0;
  const perform = () => { calls++; return cancellation.promise; };
  const scope = fixtureState.controller.scope;
  const expected = fixtureState.current();
  const cancel = fixtureState.controller.cancel(scope, expected, fixtureState.current, perform);
  assert.equal(calls, 1);
  assert.equal(await fixtureState.controller.cancel(scope, expected, fixtureState.current, perform), false);
  fixtureState.admissions[0].resolve();
  await start;
  assert.equal(fixtureState.controller.observe(fixtureState.current()).pending, 'cancel');
  cancellation.resolve();
  await cancel;
  const waiting = fixtureState.controller.observe(fixtureState.current());
  assert.equal(waiting.state, 'preparing');
  assert.equal(waiting.canStart, false);
  assert.equal(waiting.canCancel, false);
  const ended = fixtureState.update({ launch: launchState() });
  assert.equal(ended.state, 'cancelled');
  assert.match(ended.announcement, /after cancellation request/);
  assert.equal(ended.canRetry, true);
});

test('cancellation failure is safe, allows another cancel and does not invent cancellation', async () => {
  const fixtureState = fixture();
  const start = fixtureState.start();
  fixtureState.admissions[0].resolve();
  await start;
  await fixtureState.controller.cancel(fixtureState.controller.scope, fixtureState.current(), fixtureState.current, () => Promise.reject('synthetic-secret /private/path'));
  const view = fixtureState.controller.observe(fixtureState.current());
  assert.equal(view.state, 'preparing');
  assert.equal(view.canCancel, true);
  assert.doesNotMatch(view.actionError, /secret|private/);
});

test('cancellation racing launch success or failure preserves the actual store state', async () => {
  for (const state of ['running', 'error']) {
    const fixtureState = fixture();
    const start = fixtureState.start();
    fixtureState.admissions[0].resolve();
    await start;
    await fixtureState.controller.cancel(fixtureState.controller.scope, fixtureState.current(), fixtureState.current, () => Promise.resolve());
    const view = fixtureState.update({ launch: launchState({ state }) });
    assert.equal(view.state, state === 'error' ? 'failed' : 'running');
  }
});

test('foreign selection cannot cancel/retry and blocks preparation across instances', async () => {
  const fixtureState = fixture();
  const start = fixtureState.start();
  const stale = fixtureState.current();
  const view = fixtureState.update({ selectedId: 'synthetic-beta' });
  assert.equal(view.canStart, false);
  assert.equal(view.canCancel, false);
  assert.equal(view.mine, false);
  assert.equal(await fixtureState.controller.cancel(view.scope, stale, fixtureState.current, () => { throw new Error('must not cancel'); }), false);
  fixtureState.admissions[0].resolve();
  await start;
  const failed = fixtureState.update({ launch: launchState({ state: 'error' }) });
  assert.equal(failed.canRetry, false);
});

test('same-instance replacement rejects stale actions and ignores late cancellation failure', async () => {
  const fixtureState = fixture();
  const first = fixtureState.start();
  fixtureState.admissions[0].resolve();
  await first;
  const oldScope = fixtureState.controller.scope;
  const oldContext = fixtureState.current();
  const cancellation = deferred();
  const cancel = fixtureState.controller.cancel(oldScope, oldContext, fixtureState.current, () => cancellation.promise);
  fixtureState.update({ launch: launchState() });
  fixtureState.update({ launch: launchState({ state: 'preparing', stage: 'java' }) });
  cancellation.reject('synthetic-secret');
  await cancel;
  assert.equal(fixtureState.controller.actionError, null);
  assert.equal(fixtureState.controller.cancellationRequested, false);
  assert.equal(await fixtureState.controller.cancel(oldScope, oldContext, fixtureState.current, () => { throw new Error('must not cancel replacement'); }), false);
  fixtureState.update({ launch: launchState({ state: 'error' }) });
  assert.equal(await fixtureState.controller.retry(oldScope, oldContext, fixtureState.current, fixtureState.perform), false);
});

test('normal idle completion is not labelled cancelled; phase announcements ignore byte ticks', async () => {
  const fixtureState = fixture();
  const start = fixtureState.start('repair');
  const before = fixtureState.controller.observe(fixtureState.current()).announcement;
  Object.assign(fixtureState.current().launch, { done: 12, total: 100 });
  assert.equal(fixtureState.controller.observe(fixtureState.current()).announcement, before);
  fixtureState.admissions[0].resolve();
  await start;
  assert.equal(fixtureState.update({ launch: launchState() }).state, 'idle');
});

test('duplicate starts and stale selected-instance callbacks cannot admit another operation', async () => {
  const fixtureState = fixture();
  const stale = fixtureState.current();
  fixtureState.update({ selectedId: 'synthetic-beta' });
  assert.equal(await fixtureState.controller.start('launch', stale, fixtureState.current, fixtureState.perform), false);
  const first = fixtureState.start();
  assert.equal(await fixtureState.start('repair'), false);
  assert.equal(fixtureState.calls.length, 1);
  fixtureState.admissions[0].resolve();
  await first;
});

test('admission failure stays failed and allows retry of the requested action', async () => {
  const fixtureState = fixture();
  const first = fixtureState.start('repair');
  fixtureState.update({ launch: launchState({ state: 'error', message: 'synthetic denied' }) });
  fixtureState.admissions[0].resolve();
  await first;
  const view = fixtureState.controller.observe(fixtureState.current());
  assert.equal(view.state, 'failed');
  assert.equal(view.kind, 'repair');
  assert.equal(view.canRetry, true);
});

test('unexpected action rejection is bounded and cannot leak transport diagnostics', async () => {
  const fixtureState = fixture();
  await fixtureState.controller.start('launch', fixtureState.current(), fixtureState.current, () => { throw new Error('synthetic-secret https://synthetic.invalid /private/path'); });
  const view = fixtureState.controller.observe(fixtureState.current());
  assert.equal(view.pending, null);
  assert.equal(view.canStart, true);
  assert.equal(view.actionError, 'Could not request preparation. Please try again.');
});
