import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { PageLoads } from '../src/lib/page-loads.ts';

test('secondary page imports are deferred, coalesced and cached without changing component identity', async () => {
  const cache = new PageLoads();
  const page = { default: () => 'synthetic component' };
  let calls = 0;
  const loader = async () => { calls++; return page; };
  const first = cache.load('settings', loader);
  const second = cache.load('settings', loader);
  assert.equal(calls, 0);
  assert.equal(first, second);
  assert.equal(await first, page);
  assert.equal(await cache.load('settings', loader), page);
  assert.equal(calls, 1);
});

test('a rejected or throwing import is evicted so explicit retry can succeed', async () => {
  for (const loader of [async () => { throw Error('synthetic failure'); }, () => { throw Error('synthetic failure'); }]) {
    const cache = new PageLoads();
    await assert.rejects(cache.load('settings', loader), /synthetic failure/);
    assert.equal(await cache.load('settings', async () => 'recovered'), 'recovered');
  }
});

test('out-of-order imports remain isolated by page and do not cache application state', async () => {
  const cache = new PageLoads();
  let finish;
  const settings = cache.load('settings', () => new Promise(resolve => { finish = resolve; }));
  assert.equal(await cache.load('market', async () => 'market'), 'market');
  finish('settings');
  assert.equal(await settings, 'settings');
  assert.equal(await cache.load('market', async () => 'wrong'), 'market');
});

test('only fixed local page modules are lazy; instance reset and entry pages remain wired', () => {
  const pages = readFileSync(new URL('../src/lib/pages.ts', import.meta.url), 'utf8');
  const root = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
  const expected = ['Settings', 'Stats', 'Quests', 'Guilds', 'Social', 'Commands', 'Collections', 'Market', 'Casino'];
  for (const page of expected) {
    assert.ok(pages.includes(`import('../pages/${page}.svelte')`));
    assert.ok(!root.includes(`from './pages/${page}.svelte'`));
  }
  assert.match(root, /\{#key \(app\.selected \?\? ''\) \+ app\.view\}/);
  for (const page of ['Home', 'Login', 'Setup']) assert.ok(root.includes(`from './pages/${page}.svelte'`));
  assert.ok(!pages.includes('fetch('));
});
