import { test } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync, writeFileSync, unlinkSync } from 'node:fs';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
import { defaultExperience, enabled, navigation, safeLink } from './experience.ts';

async function component(source, name) {
  const result = compile(source, { generate: 'server', filename: `${name}.svelte` });
  const file = new URL(`.rendered-${name}.mjs`, import.meta.url);
  writeFileSync(file, result.js.code.replaceAll("'./experience'", "'./experience.ts'"));
  try { return (await import(file.href)).default; } finally { unlinkSync(file); }
}

test('generic hosts enable no private capabilities or presets by default', () => {
  assert.equal(defaultExperience().kind, 'generic');
  assert.deepEqual(defaultExperience().features, []);
  assert.equal(enabled(undefined, 'economy'), false);
});
test('host availability rejects unregistered and disabled navigation', () => {
  const experience = defaultExperience();
  experience.navigation = [{ id: 'remote-script', label: 'Run' }, { id: 'hidden', label: 'Hidden' }, { id: 'home', label: 'My home' }];
  assert.deepEqual(navigation(experience, [{ id: 'home', label: 'Home' }, { id: 'hidden', label: 'Hidden' }], id => id !== 'hidden'), [{ id: 'home', label: 'My home' }]);
});
test('links preserve HTTP(S) behavior and refuse executable schemes', () => {
  assert.equal(safeLink('https://example.invalid/help'), 'https://example.invalid/help');
  for (const value of ['javascript:alert(1)', 'data:text/html,test', null, {}]) assert.equal(safeLink(value), null);
});
test('standalone widget host renders text, links and injected synthetic components without private imports', async () => {
  const Host = await component(readFileSync(new URL('ExperienceWidgets.svelte', import.meta.url), 'utf8'), 'host');
  const Reference = await component('<script>let { widget } = $props();</script><b>Count: {widget.config.count}</b>', 'reference');
  const experience = defaultExperience();
  experience.widgets = [
    { id: 'text', component: 'constructor', title: '<script>unsafe</script>', body: 'Fallback body', config: {} },
    { id: 'links', component: 'links', title: 'Links', body: '', config: { links: [{ label: 'Help', url: 'https://example.invalid/help' }, { label: 'Do not render', url: 'javascript:alert(1)' }] } },
    { id: 'registered', component: 'reference', title: 'Synthetic extension', body: '', config: { count: 7 } },
  ];
  const html = render(Host, { props: { experience, components: { reference: Reference } } }).body;
  assert.match(html, /Fallback body/);
  assert.match(html, /&lt;script(?:>|&gt;)unsafe&lt;\/script(?:>|&gt;)/);
  assert.match(html, /Count: 7/);
  assert.match(html, /https:\/\/example.invalid\/help/);
  assert.doesNotMatch(html, /Do not render|javascript:|<script>unsafe/);
});
