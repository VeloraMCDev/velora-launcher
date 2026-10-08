// Keyboard shortcuts for the casino. A table registers the keys it answers to while it is the one on screen (tables stay mounted
// when you switch tabs, so each one asks the page whether it is the active pane).
import { getContext, onMount } from 'svelte';
import { cprefs } from './prefs.svelte';

export const ACTIVE = 'casino:active';
export type KeyMap = Record<string, (e: KeyboardEvent) => void>;

/** "Space" for the space bar, otherwise the lower-case key ("m", "arrowup", "1", "?"). */
export function keyName(e: KeyboardEvent): string {
  return e.key === ' ' ? 'space' : e.key.length === 1 ? e.key.toLowerCase() : e.key.toLowerCase();
}

function typing(e: KeyboardEvent): boolean {
  const t = e.target as HTMLElement | null;
  if (!t) return false;
  if (t.isContentEditable) return true;
  const tag = t.tagName;
  if (tag === 'TEXTAREA' || tag === 'SELECT') return true;
  // Number boxes and the like swallow keys; a range slider or a checkbox does not.
  return tag === 'INPUT' && !['range', 'checkbox', 'radio', 'button'].includes((t as HTMLInputElement).type);
}

/** Call during component setup. `keys` may be a function so it can read the table's current state. */
export function hotkeys(keys: KeyMap | (() => KeyMap), always = false) {
  const active = getContext<(() => boolean) | undefined>(ACTIVE);
  onMount(() => {
    const on = (e: KeyboardEvent) => {
      if (!cprefs.keys || e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented) return;
      if (!always && active && !active()) return;
      if (document.querySelector('[data-casino-modal]') && !always) return;
      if (typing(e)) return;
      const map = typeof keys === 'function' ? keys() : keys;
      const name = keyName(e);
      const run = map[name];
      if (!run) return;
      // A held key must not hammer a button that costs money.
      if (e.repeat && !['arrowup', 'arrowdown', '+', '-', '=', '_'].includes(name)) return;
      e.preventDefault();
      run(e);
    };
    window.addEventListener('keydown', on);
    return () => window.removeEventListener('keydown', on);
  });
}

/** The bets worth offering as one-tap chips between a table's minimum and what the player can afford. */
export function betPresets(min: number, cap: number, count = 5): number[] {
  const nice = [1, 2, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 25000, 50000, 100000, 250000, 1000000];
  const inside = nice.filter((v) => v >= min && v <= cap);
  if (inside.length <= count) return inside;
  const out: number[] = [];
  for (let i = 0; i < count; i++) out.push(inside[Math.round((i * (inside.length - 1)) / (count - 1))]);
  return [...new Set(out)];
}

/** The next bet up or down the ladder of chips (so the arrow keys move in sensible steps). */
export function stepBet(value: number, direction: 1 | -1, min: number, cap: number): number {
  const ladder = [min, ...[1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500, 1000, 2000, 2500, 5000, 10000, 25000, 50000, 100000, 250000, 1000000].filter((v) => v > min && v < cap), cap];
  if (direction > 0) return ladder.find((v) => v > value + 1e-9) ?? cap;
  return [...ladder].reverse().find((v) => v < value - 1e-9) ?? min;
}
