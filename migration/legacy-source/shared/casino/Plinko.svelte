<script lang="ts">
  import { untrack } from 'svelte';
  import { onMount } from 'svelte';
  import { Triangle, Info } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import { cpost, mult, multColor, money, type CasinoState, type Round } from './casino';
  import { toast, errorText } from './host';
  import { sfx } from './sfx';

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.plinko);
  let bet = $state(untrack(() => st.config.plinko.min_bet));
  hotkeys(() => { const go = () => { if (!busy && cfg.enabled && !(balance != null && balance < bet)) void drop(); }; return { space: go, enter: go, r: () => { if (!busy) risk = cfg.risks[(cfg.risks.indexOf(risk) + 1) % cfg.risks.length]; } }; });
  let risk = $state(untrack(() => (st.config.plinko.risks.includes('medium') ? 'medium' : st.config.plinko.risks[0])));
  let rows = $state(untrack(() => Math.min(Math.max(12, st.config.plinko.min_rows), st.config.plinko.max_rows)));
  const table = $derived(st.plinko_tables[risk]?.[String(rows)] ?? []);

  // The board is drawn in a fixed 600-wide space; the peg spacing shrinks as rows are added.
  const W = 600, TOP = 28, BOTTOM = 56;
  const dx = $derived(Math.min(46, (W - 60) / rows));
  const dy = $derived(Math.min(34, 400 / rows));
  const H = $derived(TOP + rows * dy + BOTTOM);
  const pegPos = (k: number, j: number) => ({ x: W / 2 + (j - k / 2) * dx, y: TOP + k * dy });
  const pegs = $derived(Array.from({ length: rows }, (_, k) => Array.from({ length: k + 1 }, (_, j) => ({ k, j, ...pegPos(k, j) }))).flat());
  const slotX = (s: number) => W / 2 + (s - rows / 2) * dx;

  type Ball = { id: number; path: number[]; t: number; x: number; y: number; slot: number; r: Round; doneAt: number };
  let balls = $state<Ball[]>([]);
  let flash = $state<Record<number, number>>({});
  let hits = $state<Record<string, number>>({});
  let recent = $state<{ m: number; profit: number }[]>([]);
  let nextId = 1;
  let busy = $state(false);
  const STEP_MS = 135;
  const START = { x: W / 2, y: TOP - 22 };

  async function drop() {
    if (busy) return;
    busy = true;
    try {
      const r = await cpost<Round>(st.server.id, '/plinko', { bet, rows, risk });
      onbalance((balance ?? 0) - bet);
      sfx.drop();
      balls.push({ id: nextId++, path: r.result.path, t: 0, x: START.x, y: START.y, slot: r.result.slot, r, doneAt: 0 });
      ensureLoop();
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }

  let raf = 0, last = 0;
  function ensureLoop() { if (!raf) { last = performance.now(); raf = requestAnimationFrame(tick); } }

  /** Where the ball rests after `k` pegs: the start, a peg in row k-1, or (past the last row) its slot. */
  function rest(b: Ball, k: number) {
    if (k <= 0) return START;
    if (k > rows) return { x: slotX(b.slot), y: TOP + rows * dy + 22 };
    const rights = b.path.slice(0, k - 1).reduce((a, v) => a + v, 0);
    return pegPos(k - 1, rights);
  }

  function tick(now: number) {
    const dt = Math.min(64, now - last); last = now;
    for (const b of balls) {
      if (b.doneAt) continue;
      b.t += dt / STEP_MS;
      const k = Math.floor(b.t);
      if (k > rows) { // landed
        const end = rest(b, rows + 1);
        b.x = end.x; b.y = end.y; b.doneAt = now;
        flash[b.slot] = now;
        recent = [{ m: b.r.multiplier, profit: b.r.profit }, ...recent].slice(0, 12);
        if (b.r.payout > b.r.bet) sfx.win(b.r.multiplier >= 10 ? 3 : b.r.multiplier >= 3 ? 2 : 1); else sfx.stop();
        onbalance(b.r.balance);
        onplayed();
        continue;
      }
      const u = b.t - k;
      const from = rest(b, k), to = rest(b, k + 1);
      const e = u * u * (3 - 2 * u);
      b.x = from.x + (to.x - from.x) * e;
      b.y = from.y + (to.y - from.y) * u * u - Math.sin(u * Math.PI) * dy * 0.4;
      if (u > 0.9 && k >= 1 && k <= rows) {
        const rights = b.path.slice(0, k).reduce((a, v) => a + v, 0);
        if (k < rows && !hits[`${k}:${rights}`]) { hits[`${k}:${rights}`] = now; sfx.tick(1.1 + Math.random() * 0.5); }
      }
    }
    balls = balls.filter((b) => !b.doneAt || now - b.doneAt < 1300);
    if (balls.length) raf = requestAnimationFrame(tick);
    else { raf = 0; hits = {}; }
  }
  onMount(() => () => cancelAnimationFrame(raf));
  const glow = (k: number, j: number, now: number) => { const t = hits[`${k}:${j}`]; return t ? Math.max(0, 1 - (now - t) / 320) : 0; };
</script>

<div class="game">
  <section class="panel controls">
    <h2><Triangle size={18} /> Plinko</h2>
    <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} />
    <div class="opt">
      <span class="lbl">Risk</span>
      <div class="segmented">{#each cfg.risks as r}<button class:active={risk === r} onclick={() => (risk = r)}>{r[0].toUpperCase() + r.slice(1)}</button>{/each}</div>
    </div>
    <label class="opt">
      <span class="lbl">Rows <b>{rows}</b></span>
      <input class="fancy" type="range" min={cfg.min_rows} max={cfg.max_rows} step="1" bind:value={rows} style="--p:{cfg.max_rows > cfg.min_rows ? ((rows - cfg.min_rows) / (cfg.max_rows - cfg.min_rows)) * 100 : 100}%" />
    </label>
    <button class="drop primary" onclick={drop} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>Drop ball · {money(bet)}<kbd class="k">Space</kbd></button>
    {#if !cfg.enabled}<p class="note">Plinko is closed right now.</p>{/if}
    <p class="note"><Info size={13} /> Pays back {(cfg.rtp * 100).toFixed(1)}% over time. Higher risk means bigger edges and smaller middles.</p>
    {#if recent.length}
      <div class="recent">{#each recent as r}<span style="--c:{multColor(r.m)}">{mult(r.m)}</span>{/each}</div>
    {/if}
  </section>

  <section class="stage">
    <svg viewBox="0 0 {W} {H}" role="img" aria-label="Plinko board">
      <defs><radialGradient id="ball" cx="35%" cy="30%"><stop offset="0" stop-color="#fff" /><stop offset="0.45" stop-color="#7dd3fc" /><stop offset="1" stop-color="#0284c7" /></radialGradient></defs>
      {#each pegs as p}
        {@const l = glow(p.k, p.j, performance.now())}
        <circle cx={p.x} cy={p.y} r={3.2 + l * 2.2} fill={l > 0 ? '#e0f2fe' : '#94a3b8'} opacity={0.55 + l * 0.45} />
      {/each}
      {#each table as m, s}
        {@const w = Math.max(14, dx - 4)}
        <g transform="translate({slotX(s) - w / 2}, {TOP + rows * dy + 8})" class="slot" class:flash={!!flash[s] && performance.now() - flash[s] < 900}>
          <rect width={w} height="30" rx="7" fill={multColor(m)} opacity="0.92" />
          <text x={w / 2} y="19.5" text-anchor="middle" font-size={rows > 13 ? 8.2 : 9.6} font-weight="800" fill="#06070a">{mult(m).replace('×', '')}</text>
        </g>
      {/each}
      {#each balls as b (b.id)}<circle cx={b.x} cy={b.y} r="7.5" fill="url(#ball)" stroke="#0007" stroke-width="1" class:land={!!b.doneAt} />{/each}
    </svg>
  </section>
</div>

<style>
  .game { display: grid; grid-template-columns: 19rem 1fr; gap: 1.1rem; align-items: start; }
  /* Phones: show the game first so you can watch it, with the main button pinned above the tab bar. */
  @media (max-width: 860px) {
    .game { grid-template-columns: 1fr; }
    .stage { order: -1; }
    .drop { position: sticky; bottom: calc(var(--pl-tab, 0px) + env(safe-area-inset-bottom, 0px) + 10px); z-index: 4; }
  }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.9rem; }
  h2 { display: flex; gap: 0.5rem; align-items: center; font-size: 1.05rem; }
  .opt { display: flex; flex-direction: column; gap: 0.35rem; } .lbl { font-size: 0.82rem; font-weight: 600; } .lbl b { color: var(--accent-2); margin-left: 0.3rem; }
  .segmented { display: flex; } .segmented button { flex: 1; text-transform: capitalize; }
  input.fancy { height: 1.2rem; background: linear-gradient(90deg, var(--accent) var(--p), color-mix(in srgb, var(--text) 12%, transparent) var(--p)) center / 100% 0.3rem no-repeat; }
  .drop { padding: 0.85rem; font-size: 1rem; font-weight: 700; border-radius: var(--radius); background: linear-gradient(135deg, #38bdf8, #6366f1); border: none; color: #fff; box-shadow: 0 6px 22px -6px #38bdf8; }
  .drop:hover:not(:disabled) { filter: brightness(1.1); transform: translateY(-1px); }
  .note { font-size: 0.78rem; color: var(--muted); display: flex; gap: 0.35rem; align-items: flex-start; }
  .recent { display: flex; flex-wrap: wrap; gap: 0.3rem; } .recent span { font-size: 0.72rem; font-weight: 700; padding: 0.15rem 0.5rem; border-radius: 99rem; color: #06070a; background: var(--c); }
  .stage { padding: 0.5rem; border-radius: var(--radius); background: radial-gradient(120% 90% at 50% 0%, #1a2a44, #0b0e16 70%); border: 1px solid var(--line); }
  svg { width: 100%; max-height: 72vh; display: block; }
  @media (max-width: 860px) { svg { max-height: 54vh; } }
  .slot rect { transition: transform 0.2s; transform-box: fill-box; transform-origin: center; }
  .slot.flash rect { animation: bump 0.7s ease-out; filter: brightness(1.4) drop-shadow(0 0 8px currentColor); }
  @keyframes bump { 0% { transform: translateY(0); } 25% { transform: translateY(6px) scale(1.08); } 100% { transform: translateY(0); } }
  circle.land { opacity: 0; transition: opacity 0.5s 0.9s; }
</style>
