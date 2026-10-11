import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { defaultExperience, navigation, pageEnabled } from './index.ts';
import { DEFAULT_GUIDE, guideFor, safeGuideLink } from './player-guide.ts';
import { logoLayout } from './logo-layout.ts';

test('Player Guide remains reachable without enabling admin command tools', () => {
  const experience = defaultExperience();
  experience.features = [];
  experience.navigation = [{ id: 'commands', label: 'Command guide' }];
  assert.equal(pageEnabled(experience, 'commands'), false);
  assert.deepEqual(navigation(experience, [{ id: 'commands', label: 'Player Guide' }], true), [{ id: 'commands', label: 'Player Guide' }]);
});
test('default guide covers pack, server and integrations; deliberately empty custom guide stays empty', () => {
  assert.equal(guideFor(null), DEFAULT_GUIDE);
  for (const category of ['Modpack', 'Server', 'Integrations']) assert.ok(DEFAULT_GUIDE.articles.some(a => a.category === category));
  const empty = { title: 'Custom', introduction: '', show_commands: false, articles: [] };
  assert.equal(guideFor(empty), empty);
  assert.equal(safeGuideLink('javascript:alert(1)'), false);
  assert.equal(safeGuideLink('https://example.com/help'), true);
});
test('custom wordmark dimensions are instance-owned and malformed values cannot break layout', () => {
  const experience = defaultExperience();
  experience.modules.launcher_presentation = { width: 900, max_height: 320, title_size: 72 };
  assert.deepEqual(logoLayout(experience), { width: 900, max_height: 320, title_size: 72 });
  experience.modules.launcher_presentation = { width: Infinity, max_height: -1, title_size: 10000 };
  assert.deepEqual(logoLayout(experience), { width: 420, max_height: 40, title_size: 96 });
});
