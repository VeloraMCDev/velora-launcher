import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { defaultExperience, preset, navigation, pageEnabled, coreFeatures } from './index.ts';
test('Velora SMP exposes supported modules and excludes legacy systems', () => {
  const e = preset('velora-smp');
  assert.equal(e.modules.velora_core.starting_balance_cents, 100_000);
  assert.equal(e.modules.velora_core.faction_max_members, 8);
  for (const page of ['casino', 'market', 'guilds', 'map', 'stats', 'commands']) assert.equal(pageEnabled(e, page), true);
  for (const page of ['quests', 'achievements', 'collections', 'events', 'companion', 'items']) assert.equal(pageEnabled(e, page), false);
  const pages = navigation(e, [{ id: 'guilds', label: 'Guilds' }, { id: 'quests', label: 'Quests' }]);
  assert.deepEqual(pages, [{ id: 'guilds', label: 'Factions' }]);
  e.modules.velora_core.modules.economy = false;
  assert.equal(coreFeatures(e.modules.velora_core).includes('economy'), false);
});
test('legacy SMP retains every existing capability', () => {
  for (const page of ['casino', 'market', 'guilds', 'quests', 'leveling', 'map']) assert.equal(pageEnabled(defaultExperience(), page), true);
});
test('Frontiers navigation uses its identity and excludes unrelated systems', () => {
  const pages = navigation(preset('frontiers'), [{ id: 'casino', label: 'Casino' }, { id: 'home', label: 'Home' }, { id: 'experience', label: 'Overview' }, { id: 'map', label: 'Map' }]);
  assert.deepEqual(pages.map(p => p.id), ['home', 'experience', 'map']);
  assert.equal(pages[0].label, 'Frontier overview');
  assert.equal(pageEnabled(preset('frontiers'), 'guilds'), false);
  assert.equal(pageEnabled(preset('frontiers'), 'settings'), true);
});
test('configuration cannot add unknown or disabled navigation pages', () => {
  const experience = preset('frontiers');
  experience.navigation = [{ id: 'casino', label: 'Override' }, { id: 'unknown', label: 'Remote script' }];
  assert.deepEqual(navigation(experience, [{ id: 'home', label: 'Home' }, { id: 'casino', label: 'Casino' }]), [{ id: 'home', label: 'Home' }]);
});
