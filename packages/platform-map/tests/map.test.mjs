import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import ts from 'typescript';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';

// Compile the shipped TypeScript, not a second implementation of the renderer.
const root = new URL('../', import.meta.url);
const runtime = new URL('.test-runtime/', root);
mkdirSync(runtime, { recursive: true });
for (const name of ['engine', 'feed', 'types', 'index']) {
  const source = readFileSync(new URL(`${name}.ts`, root), 'utf8');
  const js = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
  writeFileSync(new URL(`${name}.mjs`, runtime), js.replaceAll(/(['"])(\.\/\w+)\1/g, '$1$2.mjs$1'));
}
const { MapView, startFeed, dimSlug, withAlpha } = await import(new URL('index.mjs', runtime));
const dimension = (id = 'minecraft:overworld') => ({ id, slug: dimSlug(id), label: 'Synthetic world', available: true, tiles: 1, bytes: 64, bounds: { min_x: 0, max_x: 0, min_y: 0, max_y: 0 } });
const info = (overrides = {}) => ({ enabled: true, ready: true, message: '', tile_size: 256, max_zoom: 0, tile_base: '/synthetic/tiles', token: 'synthetic-key', token_ttl_secs: 3600, dimensions: [dimension()], players: 0, ...overrides });
const overlay = () => ({ players: [], pins: [], claims: [] });
const flush = async () => { for (let i = 0; i < 6; i++) await Promise.resolve(); };

function globals(t, values) {
  for (const [key, value] of Object.entries(values)) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, key);
    Object.defineProperty(globalThis, key, { value, writable: true, configurable: true });
    t.after(() => previous ? Object.defineProperty(globalThis, key, previous) : delete globalThis[key]);
  }
}

function browser(t) {
  const frames = new Map(), images = [], listeners = new Map(), draws = [];
  let next = 0, disconnected = false;
  const ctx = new Proxy({}, { get: (_, name) => name === 'measureText' ? text => ({ width: text.length * 7 }) : (...args) => draws.push([name, ...args]) });
  const canvas = {
    style: {}, width: 0, height: 0, getContext: () => ctx,
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 400, height: 300 }),
    addEventListener: (name, fn) => listeners.set(name, fn),
    removeEventListener: (name, fn) => { if (listeners.get(name) === fn) listeners.delete(name); },
    setPointerCapture() {},
  };
  globals(t, {
    window: { devicePixelRatio: 2 },
    document: { hidden: false },
    ResizeObserver: class { observe() {} disconnect() { disconnected = true; } },
    Image: class { constructor() { images.push(this); } },
    Path2D: class { moveTo() {} lineTo() {} closePath() {} },
    requestAnimationFrame: fn => { frames.set(++next, fn); return next; },
    cancelAnimationFrame: id => frames.delete(id),
  });
  return { canvas, images, draws, listeners, frames, get disconnected() { return disconnected; },
    frame(now = performance.now() + 1000) { const pending = [...frames.values()]; frames.clear(); for (const fn of pending) fn(now); },
    click(x = 200, y = 150) { const e = { pointerId: 1, clientX: x, clientY: y }; listeners.get('pointerdown')(e); listeners.get('pointerup')(e); },
  };
}

function timers(t) {
  const pending = new Map(); let next = 0;
  globals(t, { document: { hidden: false }, setTimeout: (fn, delay) => { pending.set(++next, { fn, delay }); return next; }, clearTimeout: id => pending.delete(id) });
  return { pending, async run(delay) { const entry = [...pending].find(([, job]) => job.delay === delay); assert.ok(entry, `timer ${delay} exists`); pending.delete(entry[0]); await entry[1].fn(); await flush(); } };
}

test('extracted runtime retains frozen source bytes and only the viewer comment changes', () => {
  const provenance = JSON.parse(readFileSync(new URL('PROVENANCE.json', root)));
  for (const input of provenance.inputs) {
    let bytes = readFileSync(new URL(input.destination, root));
    if (input.destination === 'MapViewer.svelte') bytes = Buffer.from(bytes.toString().replace('The Velora map: supplied', 'The SCOPENET Map:'));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), input.sha256, input.destination);
  }
});

test('dimension aliases, custom namespaces and color alpha remain compatible', () => {
  assert.equal(dimSlug('minecraft:nether'), 'the_nether');
  assert.equal(dimSlug('minecraft:end'), 'the_end');
  assert.equal(dimSlug('synthetic:moon'), 'synthetic__moon');
  assert.equal(withAlpha('#8b6cff', 0.5), 'rgba(139, 108, 255, 0.5)');
  assert.equal(withAlpha('var(--custom)', 0.5), 'var(--custom)');
});

test('real canvas host frames bounds, switches dimensions and tears down listeners', t => {
  const b = browser(t), selected = [];
  const view = new MapView(b.canvas, { tileUrl: () => '/synthetic/tile', onselect: s => selected.push(s) });
  view.setInfo(info({ dimensions: [dimension(), dimension('minecraft:the_nether')] }));
  assert.deepEqual(view.getView(), { x: 128, z: 128, scale: 300 / 256 * 0.9 });
  assert.equal(b.canvas.width, 800);
  view.setDimension('nether');
  assert.equal(view.currentDimension.slug, 'the_nether');
  assert.deepEqual(selected, [null]);
  view.setDimension('missing');
  assert.equal(view.currentDimension.slug, 'the_nether');
  view.destroy();
  assert.equal(b.listeners.size, 0); assert.equal(b.frames.size, 0); assert.equal(b.disconnected, true);
});

test('selection honors layer priority and claim holes without supplying claim policy', t => {
  const b = browser(t), selections = [], blocks = [];
  const view = new MapView(b.canvas, { tileUrl: () => '', onselect: s => selections.push(s), onblock: p => blocks.push(p) });
  view.setInfo(info());
  const player = { uuid: 'synthetic-player', name: 'Example', dimension: 'minecraft:overworld', x: 128, y: 64, z: 128, yaw: 0 };
  const pin = { id: 'synthetic-pin', kind: 'custom', label: 'Example place', dimension: player.dimension, x: 128, y: 64, z: 128, lines: [] };
  const claim = { id: 'synthetic-region', guild_id: '', name: 'Example region', tag: '', icon: '', color: '#8b6cff', dimension: player.dimension, chunks: 1, label_x: 128, label_z: 128, outer: [[0,0],[256,0],[256,256],[0,256]], holes: [[[120,120],[140,120],[140,140],[120,140]]], lines: [] };
  view.setOverlay({ players: [player], pins: [pin], claims: [claim] });
  b.click(); assert.equal(selections.at(-1).type, 'player');
  view.setLayers({ players: false, pins: true, claims: true }); b.click(); assert.equal(selections.at(-1).type, 'pin');
  view.setLayers({ players: false, pins: false, claims: true }); b.click(); assert.equal(selections.at(-1), null);
  b.click(100, 150); assert.equal(selections.at(-1).claim.id, claim.id);
  assert.deepEqual(blocks[0], { dimension: player.dimension, x: 128, z: 128 });
  view.destroy();
});

test('tile loading uses the injected URL and new tokens retry failed tiles while retaining loaded tiles', t => {
  const b = browser(t); let token = 'one';
  const view = new MapView(b.canvas, { tileUrl: (slug,z,x,y) => `/synthetic/${slug}/${z}/${x}/${y}?key=${token}` });
  view.setInfo(info({ token })); b.frame();
  assert.equal(b.images.length, 1); assert.equal(b.images[0].src, '/synthetic/overworld/0/0/0?key=one');
  b.images[0].onerror(); token = 'two'; view.setInfo(info({ token })); b.frame();
  assert.equal(b.images.length, 2); assert.match(b.images[1].src, /key=two$/);
  b.images[1].onload(); token = 'three'; view.setInfo(info({ token })); b.frame();
  assert.equal(b.images.length, 2); assert.ok(b.draws.some(([name]) => name === 'drawImage'));
  view.destroy();
});

test('empty host requests no tiles; following changes dimension with supplied player movement', t => {
  const b = browser(t), view = new MapView(b.canvas, { tileUrl: () => '/synthetic/tile' });
  view.setInfo(info({ token: null, dimensions: [] })); b.frame(); assert.equal(b.images.length, 0);
  view.setInfo(info({ token: null, dimensions: [dimension(), dimension('synthetic:moon')] }));
  view.follow('example'); view.setOverlay({ ...overlay(), players: [{ uuid: 'example', name: 'Example', dimension: 'synthetic:moon', x: -300, y: 70, z: 512, yaw: 0 }] });
  b.frame(); assert.equal(view.currentDimension.slug, 'synthetic__moon');
  assert.equal(view.getView().x, -300); assert.equal(view.getView().z, 512);
  assert.equal(b.images.length, 0); view.destroy();
});

test('feed renews tile keys, respects visibility and stops pending timers', async t => {
  const clock = timers(t), seen = []; let loads = 0;
  const stop = startFeed({ loadInfo: async () => info(), loadOverlay: async () => { loads++; return overlay(); }, onInfo: x => seen.push(x), onOverlay: x => seen.push(x) });
  await flush(); assert.equal(seen.length, 1); assert.ok([...clock.pending.values()].some(x => x.delay === 1200000));
  await clock.run(600); assert.equal(loads, 1);
  document.hidden = true; await clock.run(2500); assert.equal(loads, 1);
  document.hidden = false; await clock.run(2500); assert.equal(loads, 2);
  stop(); assert.equal(clock.pending.size, 0);
});

test('feed reports errors, retries unavailable info and ignores a stopped info response', async t => {
  const clock = timers(t), errors = []; let resolve, infos = 0;
  const stop = startFeed({ loadInfo: async () => { if (++infos === 1) throw new Error('synthetic outage'); return new Promise(r => { resolve = r; }); }, loadOverlay: async () => overlay(), onInfo: () => assert.fail('stopped response'), onOverlay: () => assert.fail('not ready'), onError: e => errors.push(e) });
  await flush(); assert.deepEqual(errors, ['synthetic outage']);
  const retry = clock.run(10000); await flush(); stop(); resolve(info()); await retry;
  assert.equal(clock.pending.size, 0);
});

test('standalone Svelte viewer renders waiting/error/layer states and escaped synthetic labels', async () => {
  const source = readFileSync(new URL('MapViewer.svelte', root), 'utf8');
  const result = compile(source, { generate: 'server', filename: 'MapViewer.svelte' });
  writeFileSync(new URL('MapViewer.mjs', runtime), result.js.code.replaceAll("'./engine'", "'./engine.mjs'").replaceAll("'./types'", "'./types.mjs'"));
  const Viewer = (await import(new URL('MapViewer.mjs', runtime))).default;
  const waiting = render(Viewer, { props: { info: null, overlay: null, tileUrl: () => '', error: 'Synthetic outage', available: { players: false, pins: false, claims: false } } }).body;
  assert.match(waiting, /Synthetic outage/); assert.doesNotMatch(waiting, /Guild land|Players online|Spawn, warps/);
  const ready = render(Viewer, { props: { info: info({ dimensions: [{ ...dimension(), label: '<script>synthetic</script>' }] }), overlay: overlay(), tileUrl: () => '' } }).body;
  assert.match(ready, /&lt;script(?:>|&gt;)synthetic/); assert.doesNotMatch(ready, /<script>synthetic/);
});
