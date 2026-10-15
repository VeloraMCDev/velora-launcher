import assert from 'node:assert/strict';
import test from 'node:test';
import { PreparationEvents } from '../src/lib/preparation-events.ts';

const identity = (operation_id, instance_id = 'synthetic-alpha') => ({ instance_id, operation_id });
const begin = (events, operationId = 'first-operation') => events.begin('synthetic-alpha', () => Promise.resolve(operationId));

test('busy admission does not invoke another request', () => {
  const events = new PreparationEvents();
  const request = begin(events);
  assert.equal(events.busy, true);
  assert.equal(events.begin('synthetic-beta', () => { throw new Error('must not invoke'); }), null);
  events.admit(request, 'first-operation');
  assert.equal(events.busy, true);
  assert.equal(events.request, request);
});

test('events before admission are replayed only for the admitted owner', () => {
  const events = new PreparationEvents();
  const request = begin(events);
  const applied = [];
  events.event(identity('stale-operation'), 'progress', () => applied.push('stale'));
  events.event(identity('first-operation', 'synthetic-beta'), 'state', () => applied.push('foreign'));
  events.event(identity('first-operation'), 'progress', () => applied.push('current'));
  assert.deepEqual(applied, []);
  events.admit(request, 'first-operation');
  assert.deepEqual(applied, ['current']);
  assert.equal(request.events.length, 0);
});

test('buffered terminal outcome completes even a fast repair before the IPC reply', () => {
  const events = new PreparationEvents();
  const request = begin(events);
  events.event(identity('first-operation'), 'state', () => events.transition('idle'));
  events.admit(request, 'first-operation');
  assert.equal(events.busy, false);
  assert.equal(request.phase, 'idle');
  assert.equal(events.transition('preparing'), false);
  assert.equal(events.transition('running'), false);
});

test('old same-instance events cannot overwrite a replacement', () => {
  const events = new PreparationEvents();
  const previous = begin(events);
  events.admit(previous, 'first-operation');
  events.transition('idle');
  const current = begin(events, 'second-operation');
  events.admit(current, 'second-operation');
  let stale = 0;
  for (const kind of ['progress', 'state', 'log']) events.event(identity('first-operation'), kind, () => stale++);
  assert.equal(stale, 0);
  assert.equal(current.phase, 'preparing');
  assert.equal(events.busy, true);
});

test('rejected request clears queued callbacks and restores the prior request', () => {
  const events = new PreparationEvents();
  const previous = begin(events);
  events.admit(previous, 'first-operation');
  events.transition('running');
  const current = begin(events, 'denied-operation');
  let applied = false;
  events.event(identity('denied-operation'), 'state', () => { applied = true; });
  assert.equal(events.reject(current, previous), true);
  assert.equal(events.request, previous);
  assert.equal(current.events.length, 0);
  assert.equal(applied, false);
});

test('obsolete admission or rejection cannot replace the current owner', () => {
  const events = new PreparationEvents();
  const previous = begin(events);
  events.admit(previous, 'first-operation');
  events.transition('idle');
  const current = begin(events, 'second-operation');
  events.admit(current, 'second-operation');
  assert.equal(events.reject(previous, null), false);
  events.admit(previous, 'first-operation');
  assert.equal(events.request, current);
  assert.equal(current.phase, 'preparing');
});

test('pending buffer is bounded and preserves terminal state under progress floods', () => {
  const events = new PreparationEvents();
  const request = begin(events);
  events.event(identity('first-operation'), 'state', () => events.transition('idle'));
  for (let index = 0; index < 1000; index++) events.event(identity('first-operation'), 'progress', () => {});
  assert.equal(request.events.length, 128);
  events.admit(request, 'first-operation');
  assert.equal(request.phase, 'idle');
  assert.equal(events.busy, false);
});

test('bounded all-state buffer rejects expendable progress without growing', () => {
  const events = new PreparationEvents();
  const request = begin(events);
  for (let index = 0; index < 128; index++) events.event(identity('first-operation'), 'state', () => {});
  events.event(identity('first-operation'), 'progress', () => { throw new Error('must not replay'); });
  assert.equal(request.events.length, 128);
  events.admit(request, 'first-operation');
});

test('a captured cancellation promise retains its token after replacement', async () => {
  const events = new PreparationEvents();
  const previous = begin(events);
  const captured = events.request;
  events.admit(previous, 'first-operation');
  events.transition('idle');
  const current = begin(events, 'second-operation');
  events.admit(current, 'second-operation');
  assert.equal(await captured.operation, 'first-operation');
  assert.equal(captured.instanceId, 'synthetic-alpha');
});
