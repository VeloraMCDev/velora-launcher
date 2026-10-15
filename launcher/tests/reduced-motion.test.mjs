import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import test from 'node:test';
import { runInNewContext } from 'node:vm';
import { prefersReducedMotion } from '../src/lib/reduced-motion.ts';

function environment(context, values = {}) {
  for (const name of ['document', 'window']) {
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, writable: true, value: values[name] });
    context.after(() => {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else delete globalThis[name];
    });
  }
}

function preferences(context) {
  const state = { manual: false, system: false, queries: 0 };
  const document = { hidden: false, documentElement: { classList: { contains(name) {
    assert.equal(name, 'reduce-motion');
    return state.manual;
  } } } };
  const window = { matchMedia(query) {
    assert.equal(query, '(prefers-reduced-motion: reduce)');
    state.queries += 1;
    return { matches: state.system };
  } };
  environment(context, { document, window });
  return { state, document, window };
}

test('actual helper is safe without document or window (SSR)', (context) => {
  environment(context);
  delete globalThis.document;
  delete globalThis.window;
  assert.equal(prefersReducedMotion(), false);
});

test('actual helper tolerates missing root and matchMedia', (context) => {
  environment(context, { document: {}, window: {} });
  assert.equal(prefersReducedMotion(), false);
  globalThis.document.documentElement = null;
  assert.equal(prefersReducedMotion(), false);
});

test('manual and system preferences combine with OR precedence', (context) => {
  const { state } = preferences(context);
  for (const [manual, system, expected] of [[false, false, false], [false, true, true], [true, false, true], [true, true, true]]) {
    Object.assign(state, { manual, system, queries: 0 });
    assert.equal(prefersReducedMotion(), expected);
    assert.equal(state.queries, manual ? 0 : 1);
  }
  delete globalThis.window;
  assert.equal(prefersReducedMotion(), true);
});

test('helper reads live classes, media results and replacement document roots', (context) => {
  const { state, document } = preferences(context);
  assert.equal(prefersReducedMotion(), false);
  state.manual = true;
  assert.equal(prefersReducedMotion(), true);
  state.manual = false;
  state.system = true;
  assert.equal(prefersReducedMotion(), true);
  state.system = false;
  assert.equal(prefersReducedMotion(), false);
  document.documentElement = { classList: { contains: () => true } };
  assert.equal(prefersReducedMotion(), true);
});

const source = await readFile(new URL('../src/components/GuildWallet.svelte', import.meta.url), 'utf8');
const effectSource = source.slice(source.indexOf('  let frame = 0;'), source.indexOf('  async function load()'));

function balanceEffect(document, target = 123.45) {
  const pending = new Map();
  let nextFrame = 0;
  let effect;
  let cleanup;
  const sandbox = {
    document, prefersReducedMotion, wallet: { balance: target }, shown: 0,
    performance: { now: () => 0 },
    untrack: (callback) => callback(),
    $effect: (callback) => { effect = callback; },
    requestAnimationFrame: (callback) => { pending.set(++nextFrame, callback); return nextFrame; },
    cancelAnimationFrame: (handle) => pending.delete(handle),
  };
  runInNewContext(stripTypeScriptTypes(effectSource), sandbox);
  const run = () => { cleanup?.(); cleanup = effect(); };
  run();
  return {
    sandbox, pending, run,
    dispose: () => cleanup?.(),
    tick(now) {
      const [handle, callback] = pending.entries().next().value;
      pending.delete(handle);
      callback(now);
    },
  };
}

test('GuildWallet effect source uses the actual helper at admission; not a rendered component test', (context) => {
  assert.match(source, /import \{ prefersReducedMotion \} from '\.\.\/lib\/reduced-motion';/);
  const { state, document } = preferences(context);
  for (const preference of ['manual', 'system']) {
    state[preference] = true;
    const fixture = balanceEffect(document);
    assert.equal(fixture.sandbox.shown, 123.45);
    assert.equal(fixture.pending.size, 0);
    state[preference] = false;
  }
});

test('GuildWallet effect source settles exactly on live changes without scheduling more frames; not rendered', (context) => {
  const { state, document } = preferences(context);
  for (const preference of ['manual', 'system']) {
    const fixture = balanceEffect(document);
    fixture.tick(100);
    assert.ok(fixture.sandbox.shown > 0 && fixture.sandbox.shown < 123.45);
    state[preference] = true;
    fixture.tick(200);
    assert.equal(fixture.sandbox.shown, 123.45);
    assert.equal(fixture.pending.size, 0);
    state[preference] = false;
  }
});

test('GuildWallet effect source preserves hidden-document settlement and frame cleanup; not rendered', (context) => {
  const { document } = preferences(context);
  document.hidden = true;
  const hidden = balanceEffect(document);
  assert.equal(hidden.sandbox.shown, 123.45);
  assert.equal(hidden.pending.size, 0);
  document.hidden = false;
  const fixture = balanceEffect(document);
  fixture.tick(100);
  fixture.sandbox.wallet = { balance: 50.25 };
  fixture.run();
  assert.equal(fixture.pending.size, 1);
  fixture.tick(550);
  assert.equal(fixture.sandbox.shown, 50.25);
  assert.equal(fixture.pending.size, 0);
  fixture.sandbox.wallet = { balance: 75 };
  fixture.run();
  assert.equal(fixture.pending.size, 1);
  fixture.dispose();
  assert.equal(fixture.pending.size, 0);
});
