// What the player has switched on at the tables, remembered between visits.
const KEY = 'scopenet.casino.prefs';

function load(): { chaos: boolean; keys: boolean; last: string } {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    return { chaos: v.chaos === true, keys: v.keys !== false, last: typeof v.last === 'string' ? v.last : '' };
  } catch { return { chaos: false, keys: true, last: '' }; }
}

/** `chaos`: Chaos mode (a win can be surged or cursed). `keys`: keyboard shortcuts. `last`: the game played most recently. */
export const cprefs = $state(load());

export function savePrefs() {
  try { localStorage.setItem(KEY, JSON.stringify({ chaos: cprefs.chaos, keys: cprefs.keys, last: cprefs.last })); } catch { /* storage may be unavailable */ }
}
export function setChaos(on: boolean) { cprefs.chaos = on; savePrefs(); }
export function setKeys(on: boolean) { cprefs.keys = on; savePrefs(); }
export function setLast(game: string) { if (cprefs.last !== game) { cprefs.last = game; savePrefs(); } }
