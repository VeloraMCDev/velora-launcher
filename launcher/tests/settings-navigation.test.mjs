import assert from 'node:assert/strict';
import test from 'node:test';
import { settingsTabIndex } from '../src/lib/settings-navigation.ts';

test('settings keyboard moves and wraps both axes, with Home and End', () => {
  assert.equal(settingsTabIndex('ArrowUp', 0, 10), 9);
  assert.equal(settingsTabIndex('ArrowLeft', 0, 10), 9);
  assert.equal(settingsTabIndex('ArrowDown', 9, 10), 0);
  assert.equal(settingsTabIndex('ArrowRight', 9, 10), 0);
  assert.equal(settingsTabIndex('Home', 4, 10), 0);
  assert.equal(settingsTabIndex('End', 4, 10), 9);
  assert.equal(settingsTabIndex('ArrowRight', 0, 1), 0);
});

test('settings keyboard leaves other keys and invalid or removed controls untouched', () => {
  for (const key of ['Enter', 'Tab', 'Escape', 'a']) assert.equal(settingsTabIndex(key, 0, 10), null);
  for (const current of [-1, 10, NaN, 1.5]) assert.equal(settingsTabIndex('End', current, 10), null);
  for (const length of [0, -1, NaN, 1.5]) assert.equal(settingsTabIndex('Home', 0, length), null);
});
