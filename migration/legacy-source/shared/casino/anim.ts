// Casino motion is driven from JavaScript (requestAnimationFrame), not CSS transitions. A system-wide "reduce animations"
// setting turns every CSS transition into an instant jump, which used to leave the reels and wheel frozen. Only the in-app
// "Reduce motion" switch (a class on <html>) shortens these.

export const reducedMotion = () => typeof document !== 'undefined' && document.documentElement.classList.contains('reduce-motion');
/** Durations shrink to a quick fade when the player asked for less motion in the app. */
export const dur = (ms: number) => (reducedMotion() ? Math.min(ms, 280) : ms);

export const ease = {
  linear: (t: number) => t,
  outQuad: (t: number) => 1 - (1 - t) * (1 - t),
  outCubic: (t: number) => 1 - Math.pow(1 - t, 3),
  outQuart: (t: number) => 1 - Math.pow(1 - t, 4),
  inCubic: (t: number) => t * t * t,
  inOutCubic: (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2),
  outBack: (t: number) => 1 + 2.2 * Math.pow(t - 1, 3) + 1.2 * Math.pow(t - 1, 2),
};

export interface Tween { done: Promise<void>; cancel(): void }

/** Calls `step(eased 0→1)` every frame for `ms`. `done` resolves at the end (also when cancelled). */
export function tween(ms: number, step: (t: number) => void, easing: (t: number) => number = ease.outCubic): Tween {
  let raf = 0, stop = false;
  const total = Math.max(1, dur(ms));
  let finish!: () => void;
  const done = new Promise<void>((r) => (finish = r));
  const t0 = performance.now();
  const frame = (now: number) => {
    if (stop) return finish();
    const t = Math.min(1, (now - t0) / total);
    step(easing(t));
    if (t < 1) raf = requestAnimationFrame(frame);
    else finish();
  };
  raf = requestAnimationFrame(frame);
  return { done, cancel: () => { stop = true; cancelAnimationFrame(raf); finish(); } };
}

/** A per-frame loop that hands you the elapsed seconds since the last frame. Returns a function that stops it. */
export function loop(frame: (dt: number, now: number) => boolean | void): () => void {
  let raf = 0, stop = false, last = performance.now();
  const tick = (now: number) => {
    if (stop) return;
    const dt = Math.min(0.064, (now - last) / 1000);
    last = now;
    if (frame(dt, now) === false) return;
    raf = requestAnimationFrame(tick);
  };
  raf = requestAnimationFrame(tick);
  return () => { stop = true; cancelAnimationFrame(raf); };
}

export const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** A spinning prize wheel: spins up the moment you call `start()`, then `land()` slows it onto an exact resting angle. */
export function wheelController(initial: number, onFrame: (deg: number, speed: number) => void) {
  const TOP_SPEED = 900, ACCEL = 1500; // degrees per second, and per second squared
  let angle = initial, v = 0, mode: 'idle' | 'spin' | 'land' = 'idle';
  let stopLoop: (() => void) | null = null;
  let active: Tween | null = null;
  return {
    get angle() { return angle; },
    start() {
      stopLoop?.();
      mode = 'spin';
      v = 0;
      stopLoop = loop((dt) => {
        if (mode !== 'spin') return false;
        v = Math.min(TOP_SPEED, v + ACCEL * dt);
        angle += v * dt;
        onFrame(angle, v);
      });
    },
    /** Come to rest with `base` (degrees, any turn) under the pointer after a few more turns. */
    async land(base: number, extraMs = 3600) {
      stopLoop?.();
      mode = 'land';
      const from = angle, v0 = Math.max(v, 420);
      const minDist = (v0 * dur(extraMs)) / 1000 / 3;
      const final = base + 360 * Math.ceil((from + minDist - base) / 360);
      const D = (3 * (final - from)) / v0 * 1000;
      let prev = from;
      active = tween(D, (e) => {
        angle = from + (final - from) * e;
        onFrame(angle, Math.abs(angle - prev) * 60);
        prev = angle;
      }, ease.outCubic);
      await active.done;
      angle = final;
      v = 0;
      mode = 'idle';
      onFrame(angle, 0);
    },
    cancel() { stopLoop?.(); active?.cancel(); mode = 'idle'; },
  };
}
