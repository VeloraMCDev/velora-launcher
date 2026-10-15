import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import { compile } from 'svelte/compiler';
import * as serverRuntime from 'svelte/internal/server';
import { render } from 'svelte/server';
import { focusableElements, focusTrap } from '../src/lib/focus.ts';
import { settingsTabIndex } from '../src/lib/settings-navigation.ts';
import { actionContrast } from '../src/lib/action-contrast.ts';
import { BIND_GROUPS, defaults, keyLabel } from '../src/lib/keys.ts';
import { bytes, gb } from '../src/lib/format.ts';

class ElementFixture {
  constructor(document, name, tabIndex = -1) {
    this.ownerDocument = document;
    this.name = name;
    this.tabIndex = tabIndex;
    this.children = [];
    this.parentElement = null;
    this.inert = false;
    this.hidden = false;
    this.disabled = false;
    this.isConnected = true;
    this.visible = true;
    this.attributes = new Map();
  }
  append(...children) {
    for (const child of children) { child.parentElement = this; this.children.push(child); }
  }
  contains(node) { return node === this || this.children.some(child => child.contains(node)); }
  closest() { return this.inert || this.hidden ? this : this.parentElement?.closest() ?? null; }
  matches(selector) { assert.equal(selector, ':disabled'); return this.disabled; }
  getClientRects() { return this.visible ? [{}] : []; }
  querySelectorAll() { return this.children.flatMap(child => [child, ...child.querySelectorAll()]); }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
  setAttribute(name, value) { this.attributes.set(name, value); if (name === 'tabindex') this.tabIndex = Number(value); }
  removeAttribute(name) { this.attributes.delete(name); if (name === 'tabindex') this.tabIndex = -1; }
  focus() {
    if (this.closest() || this.disabled || !this.isConnected) return;
    this.ownerDocument.activeElement = this;
    this.ownerDocument.dispatch('focusin', { target: this });
  }
}

function keyboardFixture() {
  const listeners = new Map();
  const document = {
    activeElement: null,
    defaultView: { HTMLElement: ElementFixture, getComputedStyle: node => ({ display: node.visible ? 'block' : 'none', visibility: 'visible' }) },
    addEventListener(type, callback) {
      const callbacks = listeners.get(type) ?? new Set();
      callbacks.add(callback);
      listeners.set(type, callbacks);
    },
    removeEventListener(type, callback) { listeners.get(type)?.delete(callback); },
    dispatch(type, event) {
      for (const callback of listeners.get(type) ?? []) { callback(event); if (event.stopped) break; }
    },
  };
  const body = new ElementFixture(document, 'body');
  const background = new ElementFixture(document, 'background');
  const opener = new ElementFixture(document, 'opener', 0);
  const overlay = new ElementFixture(document, 'overlay');
  const dialog = new ElementFixture(document, 'dialog');
  const first = new ElementFixture(document, 'first', 0);
  const last = new ElementFixture(document, 'last', 0);
  background.append(opener);
  dialog.append(first, last);
  overlay.append(dialog);
  body.append(background, overlay);
  opener.focus();
  const press = (key, shiftKey = false, isComposing = false) => {
    const event = { key, shiftKey, isComposing, prevented: false, stopped: false,
      preventDefault() { this.prevented = true; }, stopImmediatePropagation() { this.stopped = true; } };
    document.dispatch('keydown', event);
    return event;
  };
  return { document, body, background, opener, overlay, dialog, first, last, press, listeners };
}

test('production focus trap enters, cycles both directions, closes on Escape and restores the opener', async () => {
  const fixture = keyboardFixture();
  let closed = 0;
  const trap = focusTrap(fixture.dialog, { onclose: () => closed++ });
  await Promise.resolve();
  assert.equal(fixture.document.activeElement, fixture.first);
  assert.equal(fixture.background.inert, true);
  assert.equal(fixture.press('Tab', true).prevented, true);
  assert.equal(fixture.document.activeElement, fixture.last);
  fixture.press('Tab');
  assert.equal(fixture.document.activeElement, fixture.first);
  fixture.press('Escape', false, true);
  assert.equal(closed, 0);
  assert.equal(fixture.press('Escape').stopped, true);
  assert.equal(closed, 1);
  trap.destroy();
  assert.equal(fixture.document.activeElement, fixture.opener);
  assert.equal(fixture.background.inert, false);
  assert.equal(fixture.dialog.getAttribute('tabindex'), null);
  assert.equal(fixture.listeners.get('keydown').size, 0);
  assert.equal(fixture.listeners.get('focusin').size, 0);
});

test('dynamic hidden, disabled, negative-tabindex and removed controls are excluded', async () => {
  const fixture = keyboardFixture();
  const preferred = new ElementFixture(fixture.document, 'preferred', 2);
  fixture.dialog.append(preferred);
  const trap = focusTrap(fixture.dialog, { onclose() {}, initialFocus: () => preferred });
  await Promise.resolve();
  assert.equal(fixture.document.activeElement, preferred);
  assert.deepEqual(focusableElements(fixture.dialog), [preferred, fixture.first, fixture.last]);
  fixture.first.disabled = true;
  fixture.last.visible = false;
  preferred.tabIndex = -1;
  fixture.press('Tab');
  assert.equal(fixture.document.activeElement, fixture.dialog);
  preferred.tabIndex = 0;
  preferred.isConnected = false;
  assert.deepEqual(focusableElements(fixture.dialog), []);
  trap.destroy();
});

test('programmatic focus escape is contained and action updates use the latest close callback', async () => {
  const fixture = keyboardFixture();
  const trap = focusTrap(fixture.dialog, { onclose() { assert.fail('stale callback'); } });
  await Promise.resolve();
  fixture.document.activeElement = fixture.opener;
  fixture.document.dispatch('focusin', { target: fixture.opener });
  assert.equal(fixture.document.activeElement, fixture.first);
  let closed = false;
  trap.update({ onclose: () => { closed = true; } });
  fixture.press('Escape');
  assert.equal(closed, true);
  trap.destroy();
});

test('nested overlays isolate only the top layer, preserve prior inert state and restore inner focus', async () => {
  const fixture = keyboardFixture();
  const existingInert = new ElementFixture(fixture.document, 'existing-inert');
  existingInert.inert = true;
  fixture.body.append(existingInert);
  let lowerClosed = false;
  const lower = focusTrap(fixture.dialog, { onclose: () => { lowerClosed = true; } });
  await Promise.resolve();
  fixture.last.focus();
  const upperDialog = new ElementFixture(fixture.document, 'upper-dialog');
  const upperControl = new ElementFixture(fixture.document, 'upper-control', 0);
  upperDialog.append(upperControl);
  fixture.body.append(upperDialog);
  let upperClosed = false;
  const upper = focusTrap(upperDialog, { onclose: () => { upperClosed = true; } });
  await Promise.resolve();
  assert.equal(fixture.overlay.inert, true);
  fixture.press('Escape');
  assert.equal(upperClosed, true);
  assert.equal(lowerClosed, false);
  upper.destroy();
  assert.equal(fixture.document.activeElement, fixture.last);
  assert.equal(fixture.overlay.inert, false);
  assert.equal(existingInert.inert, true);
  lower.destroy();
  assert.equal(fixture.document.activeElement, fixture.opener);
  assert.equal(existingInert.inert, true);
});

test('out-of-order teardown and destroy-before-microtask do not steal focus or leak inert state', async () => {
  const fixture = keyboardFixture();
  const lower = focusTrap(fixture.dialog, { onclose() {} });
  const upperDialog = new ElementFixture(fixture.document, 'upper');
  fixture.body.append(upperDialog);
  const upper = focusTrap(upperDialog, { onclose() {} });
  lower.destroy();
  await Promise.resolve();
  assert.equal(fixture.document.activeElement, upperDialog);
  upper.destroy();
  assert.equal(fixture.background.inert, false);
  const canceled = focusTrap(fixture.dialog, { onclose() {} });
  canceled.destroy();
  fixture.opener.focus();
  await Promise.resolve();
  assert.equal(fixture.document.activeElement, fixture.opener);
});

const source = filename => fs.readFileSync(new URL(`../src/${filename}`, import.meta.url), 'utf8');
const iconSource = '<svg aria-hidden="true"></svg>';

async function settingsFixture(settingsTab, overrides = {}, accountKind = 'offline') {
  const account = { id: 'synthetic', username: 'Synthetic_Player', kind: accountKind };
  const settings = { theme_mode: 'custom', accent: '#6d6af5', reduce_motion: false, ui_scale: 100,
    background_video: true, glass: true, keybinds: {}, keybinds_enabled: true, vanilla_controls_enabled: true,
    auto_jump: false, sensitivity: 0.5, ...overrides };
  const app = { settingsTab, settings, accounts: [account], active: account.id, manifest: null, boot: null, update: null, running: [] };
  const context = vm.createContext({ console });
  const modules = new Map();
  const noOperation = () => {};
  const dependencies = {
    'svelte/internal/server': serverRuntime,
    '../lib/settings-navigation': { settingsTabIndex },
    '../lib/store.svelte': { app, activeAccount: () => account, abs: value => value, instances: () => [],
      connect: noOperation, refresh: noOperation, removeAccount: noOperation, saveSettings: noOperation,
      switchAccount: noOperation, toast: noOperation },
    '../lib/keys': { BIND_GROUPS, defaults, keyLabel, fromKeyboard: noOperation, fromMouse: noOperation },
    '../lib/format': { bytes, gb },
    '../lib/tauri': { errorText: String, invoke: noOperation, openUrl: noOperation, pickFile: noOperation },
    '../lib/action-contrast': { actionContrast },
    '@velora/experience/icon-components': { iconName: () => 'synthetic-icon' },
    '../../lib/focus': { focusTrap },
  };
  async function component(filename, content = source(filename)) {
    const compiled = compile(content, { filename, generate: 'server' }).js.code;
    const module = new vm.SourceTextModule(compiled, { context });
    await module.link(async specifier => {
      if (modules.has(specifier)) return modules.get(specifier);
      let dependency;
      if (dependencies[specifier]) {
        const values = dependencies[specifier];
        dependency = new vm.SyntheticModule(Object.keys(values), function () {
          for (const [name, value] of Object.entries(values)) this.setExport(name, value);
        }, { context });
      } else if (specifier === '../components/ui/SettingSwitch.svelte') {
        dependency = await component('components/ui/SettingSwitch.svelte');
      } else if (specifier === '../components/ui/StateMessage.svelte') {
        dependency = await component('components/ui/StateMessage.svelte');
      } else if (specifier === '../components/Modal.svelte') {
        dependency = await component('components/Modal.svelte');
      } else if (specifier === './ui/DialogSurface.svelte') {
        dependency = await component('components/ui/DialogSurface.svelte');
      } else if (specifier === '@lucide/svelte') {
        const icon = (await component('fixture-icon.svelte', iconSource)).namespace.default;
        const names = ['UserRound', 'Gamepad2', 'Cpu', 'Keyboard', 'Palette', 'Rocket', 'HardDrive', 'Info',
          'Plus', 'LogOut', 'Check', 'FolderOpen', 'Trash2', 'FileCode', 'RotateCcw', 'Download', 'LoaderCircle',
          'RefreshCw', 'MousePointerClick', 'Zap', 'Gauge', 'Feather', 'Link2', 'Shirt', 'Upload', 'Ban',
          'BookmarkPlus', 'Sparkles', 'X'];
        dependency = new vm.SyntheticModule(names, function () { for (const name of names) this.setExport(name, icon); }, { context });
      } else if (specifier.endsWith('.svelte')) {
        dependency = await component('fixture-child.svelte', iconSource);
      } else assert.fail(`Unexpected Settings dependency ${specifier}`);
      modules.set(specifier, dependency);
      return dependency;
    });
    await module.evaluate();
    return module;
  }
  const settingsModule = await component('pages/Settings.svelte');
  return render(settingsModule.namespace.default).body;
}

test('production Settings renders named categories and associated switch help in both reduced-motion states', async () => {
  for (const reduceMotion of [false, true]) {
    const html = await settingsFixture('appearance', { reduce_motion: reduceMotion });
    assert.match(html, /aria-label="Settings categories"/);
    assert.match(html, /role="tab"[^>]*id="settings-tab-appearance"[^>]*aria-selected="true"[^>]*tabindex="0"[^>]*aria-controls="settings-content"[^>]*>[\s\S]*?Appearance/);
    assert.match(html, /role="tabpanel"[^>]*aria-labelledby="settings-tab-appearance"[^>]*tabindex="0"/);
    const identifier = html.match(/<label for="([^"]+)"[^>]*>Reduce motion<\/label>/)?.[1];
    assert.ok(identifier);
    const input = html.match(new RegExp(`<input id="${identifier}"[^>]+>`))?.[0];
    assert.match(input, /role="switch"/);
    assert.match(input, new RegExp(`aria-describedby="${identifier}-help"`));
    assert.equal(/\schecked(?:[\s=>])/.test(input), reduceMotion, input);
    assert.match(html, /aria-label="Custom accent colour"/);
    assert.match(html, /aria-pressed="true"[^>]*>Custom accent/);
  }
});

test('production Settings exposes the persisted activity preference with associated disclosure', async () => {
  for (const sendStats of [false, true]) {
    const html = await settingsFixture('launcher', { send_stats: sendStats });
    const identifier = html.match(/<label for="([^"]+)"[^>]*>Share launcher activity<\/label>/)?.[1];
    assert.ok(identifier);
    const input = html.match(new RegExp(`<input id="${identifier}"[^>]+>`))?.[0];
    assert.match(input, /role="switch"/);
    assert.match(input, new RegExp(`aria-describedby="${identifier}-help"`));
    assert.equal(/\schecked(?:[\s=>])/.test(input), sendStats, input);
    assert.match(html, /your username and instance ID/);
    assert.match(html, /After this setting is saved/);
    assert.match(html, /Reports already admitted or in flight may still be sent/);
    assert.match(html, /A failed save does not change the saved preference/);
    assert.match(html, /not authentication, server connections or gameplay data/);
  }
  const settings = fs.readFileSync(new URL('../src/pages/Settings.svelte', import.meta.url), 'utf8');
  assert.match(settings, /bind:checked=\{s\.send_stats\} onchange=\{saveSettings\}/);
});

test('Settings categories retain a scrollable keyboard path at increased interface scale', () => {
  const settings = fs.readFileSync(new URL('../src/pages/Settings.svelte', import.meta.url), 'utf8');
  assert.match(settings, /\.tabs\s*\{[^}]*min-height:\s*0;[^}]*overflow-y:\s*auto;/);
  assert.match(settings, /\.tab\s*\{[^}]*flex-shrink:\s*0;/);
});

test('production Settings keyboard fixture names every key binding and preserves account-dependent categories', async () => {
  const html = await settingsFixture('controls');
  for (const group of BIND_GROUPS) for (const binding of group.binds) {
    assert.ok(html.includes(`aria-label="${binding.label}: `), binding.id);
  }
  assert.match(html, /role="tabpanel" aria-labelledby="settings-tab-controls" tabindex="0"/);
  const panel = await settingsFixture('appearance', {}, 'panel');
  assert.match(panel, /Skin &amp; cape/);
  assert.doesNotMatch(await settingsFixture('appearance'), /Skin &amp; cape/);
  assert.match(await settingsFixture('skin', {}, 'panel'), /data-state="busy" role="status" aria-atomic="true"[^>]*>[\s\S]*?Loading skin and cape/);
  assert.match(await settingsFixture('storage'), /data-state="busy" role="status" aria-atomic="true"[^>]*>[\s\S]*?Measuring storage/);
});

test('reduced-motion and compact layout stylesheet fixtures cover delays, scrolling and bounded overlays', () => {
  const css = source('app.css');
  const manual = css.slice(css.indexOf('.reduce-motion *'), css.indexOf('.sr-only'));
  const system = css.slice(css.indexOf('@media (prefers-reduced-motion: reduce)'), css.indexOf('/* ---------- atmosphere'));
  for (const rules of [manual, system]) {
    assert.match(rules, /animation-duration: 0\.001ms !important/);
    assert.match(rules, /animation-delay: 0ms !important/);
    assert.match(rules, /transition-delay: 0ms !important/);
    assert.match(rules, /scroll-behavior: auto !important/);
  }
  assert.match(css, /--state-focus-ring: color-mix/);
  assert.doesNotMatch(system, /casino-motion/);
  assert.match(source('pages/Settings.svelte'), /@media \(max-width: 760px\)/);
  for (const filename of ['components/Modal.svelte', 'components/CommandPalette.svelte']) {
    assert.match(source(filename), /max-height: calc\(100dvh - 1rem\)/);
    assert.match(source(filename), /DialogSurface/);
  }
});
