import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { defaultExperience, preset, navigation, pageEnabled } from './index.ts';
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

test('persisted Guild navigation displays Factions without changing IDs or configuration', () => {
  const experience = defaultExperience();
  experience.navigation = [{ id: 'guilds', label: 'Guilds & Claims' }, { id: 'home', label: 'Home' }];
  const pages = navigation(experience, [{ id: 'home', label: 'Overview' }, { id: 'guilds', label: 'Factions' }]);
  assert.deepEqual(pages, [{ id: 'guilds', label: 'Factions & Claims' }, { id: 'home', label: 'Home' }]);
  assert.equal(experience.navigation[0].label, 'Guilds & Claims');
  assert.equal(pageEnabled(experience, 'guilds'), true);
});
