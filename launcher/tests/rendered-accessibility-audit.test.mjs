import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { composite, contrastRatio, textThreshold } from './rendered-accessibility-audit.mjs';

test('rendered audit composites translucent layers in display order', () => {
  const surface = composite([255, 255, 255, 0.1], [0, 0, 0, 1]);
  assert.deepEqual(surface, [25.5, 25.5, 25.5, 1]);
  assert.deepEqual(composite([200, 100, 0, 0], surface), surface);
  assert.deepEqual(composite([200, 100, 0, 1], surface), [200, 100, 0, 1]);
  assert.deepEqual(composite([100, 100, 100, 0.5], surface), [62.75, 62.75, 62.75, 1]);
});

test('rendered audit uses symmetric WCAG sRGB contrast, including the observed red count defect', () => {
  assert.equal(contrastRatio([0, 0, 0], [255, 255, 255]), 21);
  assert.equal(contrastRatio([255, 255, 255], [0, 0, 0]), 21);
  assert.equal(contrastRatio([45, 55, 65], [45, 55, 65]), 1);
  assert.ok(contrastRatio([255, 255, 255], [229, 72, 77]) < 4.5);
  assert.ok(contrastRatio([0, 0, 0], [229, 72, 77]) > 4.5);
});

test('large text relaxation requires 18pt or bold 14pt, not merely semibold text', () => {
  assert.equal(textThreshold(24, 400), 3);
  assert.equal(textThreshold(56 / 3, 700), 3);
  assert.equal(textThreshold(56 / 3, 650), 4.5);
  assert.equal(textThreshold(18.6, 700), 4.5);
  assert.equal(textThreshold(23.9, 400), 4.5);
});

test('fallback avatar white initials exceed normal text contrast for every generated hue', () => {
  const source = readFileSync(new URL('../src/components/Avatar.svelte', import.meta.url), 'utf8');
  assert.match(source, /style:background="hsl\(\{hue\} 45% 30%\)"/);
  for (let hue = 0; hue < 360; hue++) {
    const chroma = (1 - Math.abs(2 * 0.3 - 1)) * 0.45;
    const intermediate = chroma * (1 - Math.abs((hue / 60) % 2 - 1));
    const channels = [[chroma, intermediate, 0], [intermediate, chroma, 0], [0, chroma, intermediate], [0, intermediate, chroma], [intermediate, 0, chroma], [chroma, 0, intermediate]][Math.floor(hue / 60)]
      .map(channel => Math.round((channel + 0.3 - chroma / 2) * 255));
    assert.ok(contrastRatio([255, 255, 255], channels) >= 4.5, `${hue}: ${channels}`);
  }
});

test('editable keybindings retain full opacity even when application to Minecraft is off', () => {
  const source = readFileSync(new URL('../src/pages/Settings.svelte', import.meta.url), 'utf8');
  assert.match(source, /You can edit bindings below\. Enable this switch to apply them when Minecraft starts\./);
  assert.doesNotMatch(source, /class:disabled=\{!s.keybinds_enabled\}/);
  assert.doesNotMatch(source, /\.binds\.disabled\s*\{/);
});

test('semantic accent text clears normal-text contrast on the tested launcher surfaces', () => {
  const normalText = [236, 237, 241];
  for (const hex of ['6d6af5', '7c5cff', '22d3ee', '22c55e', 'f59e0b', 'f97316', 'ef4444', 'ec4899', 'a855f7', 'e5e7eb']) {
    const accent = [0, 2, 4].map(offset => parseInt(hex.slice(offset, offset + 2), 16));
    const foreground = composite([...accent, 0.65], normalText);
    for (const surface of [[13, 14, 18], [22, 24, 30], [35, 37, 42]]) {
      assert.ok(contrastRatio(foreground, surface) >= 4.5, `${hex}: ${surface}`);
    }
  }
});

test('observed social, marketplace, guide and casino contrast defects retain semantic wiring', () => {
  const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
  assert.match(read('../src/pages/Social.svelte'), /\.playing-on\s*\{\s*color: var\(--state-info-text\)/);
  assert.match(read('../src/pages/Market.svelte'), /\.kind \{[^}]*color: var\(--text\)/);
  assert.match(read('../src/pages/Market.svelte'), /\.kind\.a \{[^}]*color: var\(--state-info-text\)/);
  assert.match(read('../../shared/experience/PlayerGuide.svelte'), /a,code \{ color: var\(--state-info-text, color-mix/);
  assert.match(read('../../shared/casino/CasinoPage.svelte'), /\.tabs button\.active \{ color: var\(--text\); background: var\(--state-selected-surface, color-mix/);
});

test('fixed-dark HUD preview labels remain readable independently of global branding text', () => {
  const source = readFileSync(new URL('../src/components/CompanionSettings.svelte', import.meta.url), 'utf8');
  assert.match(source, /\.hud-module span\{color:color-mix\(in srgb,var\(--hud-accent\) 40%,#fff\)/);
  assert.match(source, /background:#27312b/);
  assert.match(source, /background:rgb\(23 27 32 \/ var\(--hud-opacity\)\)/);
  for (const hex of ['000000', 'ffffff', '6d6af5', '7c5cff', '22d3ee', '22c55e', 'f59e0b', 'f97316', 'ef4444', 'ec4899', 'a855f7', 'e5e7eb']) {
    const accent = [0, 2, 4].map(offset => parseInt(hex.slice(offset, offset + 2), 16));
    const foreground = composite([...accent, 0.4], [255, 255, 255]);
    for (let step = 2; step <= 10; step++) {
      const background = composite([23, 27, 32, step / 10], [39, 49, 43]);
      assert.ok(contrastRatio(foreground, background) >= 4.5, `${hex}, opacity ${step / 10}`);
    }
    const foregroundOverDarkBranding = composite([...accent, 0.65], [0, 0, 0]);
    if (hex === '22d3ee') {
      const background = composite([23, 27, 32, 0.88], [39, 49, 43]);
      assert.ok(contrastRatio(foregroundOverDarkBranding, background) < 4.5);
      assert.ok(contrastRatio(foreground, background) >= 4.5);
    }
  }
});
