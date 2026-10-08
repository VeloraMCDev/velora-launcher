// Tiny synthesised sound effects (Web Audio, no files). Off until the player taps the speaker; the choice is remembered.
const KEY = 'scopenet.casino.sound';
let ctx: AudioContext | null = null;
let on = false;
try { on = localStorage.getItem(KEY) === '1'; } catch { /* storage unavailable */ }

export const soundOn = () => on;
export function setSound(v: boolean) {
  on = v;
  try { localStorage.setItem(KEY, v ? '1' : '0'); } catch { /* ignore */ }
  if (v) blip(660, 0.08, 'triangle', 0.06);
}

function audio(): AudioContext | null {
  if (!on || typeof AudioContext === 'undefined') return null;
  ctx ??= new AudioContext();
  if (ctx.state === 'suspended') void ctx.resume();
  return ctx;
}

function blip(freq: number, len: number, type: OscillatorType = 'sine', gain = 0.05, delay = 0, slideTo?: number) {
  const c = audio();
  if (!c) return;
  const t = c.currentTime + delay;
  const o = c.createOscillator(), g = c.createGain();
  o.type = type;
  o.frequency.setValueAtTime(freq, t);
  if (slideTo) o.frequency.exponentialRampToValueAtTime(slideTo, t + len);
  g.gain.setValueAtTime(0.0001, t);
  g.gain.exponentialRampToValueAtTime(gain, t + 0.008);
  g.gain.exponentialRampToValueAtTime(0.0001, t + len);
  o.connect(g).connect(c.destination);
  o.start(t);
  o.stop(t + len + 0.02);
}

let lastTick = 0;
export const sfx = {
  /** A wheel peg or reel notch. Throttled so a fast spin does not buzz. */
  tick(pitch = 1) { const n = performance.now(); if (n - lastTick < 38) return; lastTick = n; blip(900 * pitch, 0.035, 'square', 0.025); },
  stop() { blip(220, 0.12, 'triangle', 0.07, 0, 150); },
  coin(i = 0) { blip(1175 + i * 90, 0.12, 'triangle', 0.05, i * 0.07); blip(1568 + i * 90, 0.18, 'sine', 0.035, i * 0.07 + 0.04); },
  win(level = 1) { const n = Math.min(6, 2 + level * 2); for (let i = 0; i < n; i++) sfx.coin(i); },
  lose() { blip(220, 0.25, 'sawtooth', 0.04, 0, 110); },
  gem() { blip(988, 0.1, 'triangle', 0.05); blip(1319, 0.16, 'triangle', 0.045, 0.07); },
  boom() { blip(140, 0.45, 'sawtooth', 0.09, 0, 40); blip(90, 0.5, 'square', 0.05, 0.02, 30); },
  drop() { blip(520, 0.05, 'sine', 0.04); },
};
