import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import { actionContrast } from '../src/lib/action-contrast.ts';

test('filled action text maintains normal text contrast across default and supported custom accents', () => {
  for (const accent of ['#6d6af5', '#7c5cff', '#22d3ee', '#22c55e', '#f59e0b', '#f97316', '#ef4444', '#ec4899', '#a855f7', '#e5e7eb', '#fff', '#000']) {
    const pair = actionContrast(accent);
    assert.ok(pair.ratio >= 4.5, `${accent}: ${pair.ratio}`);
    assert.match(pair.foreground, /^#(?:000000|ffffff)$/);
  }
  for (let channel = 0; channel < 256; channel++) {
    const hex = channel.toString(16).padStart(2, '0');
    assert.ok(actionContrast('#' + hex.repeat(3)).ratio >= 4.5);
  }
});

test('unsupported, transparent or malformed accent data uses a safe opaque fallback', () => {
  for (const input of [null, {}, '#1234', '#ffffff00', 'transparent', 'red', 'url(https://synthetic.invalid)', '#gggggg', '#abcdef;']) {
    assert.deepEqual(actionContrast(input), actionContrast('#6d6af5'));
  }
});

test('notification count and selected command icons share contrast-safe filled colors', () => {
  for (const color of ['#e5484d', '#ef4444', '#ff0000', '#fff', '#000', '#6d6af5']) {
    assert.ok(actionContrast(color).ratio >= 4.5, color);
  }
  const theme = fs.readFileSync(new URL('../src/lib/theme.ts', import.meta.url), 'utf8');
  const bell = fs.readFileSync(new URL('../src/components/NotificationBell.svelte', import.meta.url), 'utf8');
  const palette = fs.readFileSync(new URL('../src/components/CommandPalette.svelte', import.meta.url), 'utf8');
  const news = fs.readFileSync(new URL('../src/components/NewsFeed.svelte', import.meta.url), 'utf8');
  assert.match(theme, /const filledDanger = actionContrast\(b.colors.danger\)/);
  assert.match(theme, /'--filled-danger-bg': filledDanger.background/);
  assert.match(theme, /'--filled-danger-fg': filledDanger.foreground/);
  assert.match(bell, /background: var\(--filled-danger-bg, #e5484d\); color: var\(--filled-danger-fg, #000000\)/);
  assert.match(palette, /\.item\.on \.ico \{ background: var\(--filled-action-bg, #6d6af5\); color: var\(--filled-action-fg, #000000\)/);
  assert.match(news, /\.tag \{[^}]*color: var\(--state-info-text\)/);
  assert.match(news, /\.more \{[^}]*color: var\(--state-info-text\)/);
});

test('filled action gradient moves away from its foreground for every supported accent', () => {
  for (const accent of ['#6d6af5', '#7c5cff', '#22d3ee', '#22c55e', '#f59e0b', '#f97316', '#ef4444', '#ec4899', '#a855f7', '#e5e7eb', '#000', '#fff']) {
    const pair = actionContrast(accent);
    const highlight = pair.foreground === '#000000' ? 255 : 0;
    for (let step = 0; step <= 12; step++) {
      const mixed = [1, 3, 5].map(offset => Math.round(parseInt(pair.background.slice(offset, offset + 2), 16) * (1 - step / 100) + highlight * step / 100));
      const channels = mixed.map(value => value / 255).map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
      const luminance = channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
      const ratio = pair.foreground === '#000000' ? (luminance + 0.05) / 0.05 : 1.05 / (luminance + 0.05);
      assert.ok(ratio >= 4.5, `${accent}, ${step}% highlight: ${ratio}`);
    }
  }
});

test('shared filled actions and selected swatches consume contrast-safe foregrounds', () => {
  const theme = fs.readFileSync(new URL('../src/lib/theme.ts', import.meta.url), 'utf8');
  const css = fs.readFileSync(new URL('../src/app.css', import.meta.url), 'utf8');
  const settings = fs.readFileSync(new URL('../src/pages/Settings.svelte', import.meta.url), 'utf8');
  assert.match(theme, /const filledAction = actionContrast\(accent\)/);
  assert.match(theme, /'--filled-action-fg': filledAction.foreground/);
  assert.match(css, /color: var\(--filled-action-fg, #000000\)/);
  assert.match(css, /var\(--filled-action-highlight, #ffffff\)/);
  assert.match(settings, /style:color=\{actionContrast\(c\).foreground\}/);
});
