<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Dices, Info } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import Burst from './Burst.svelte';
  import { cpost, GLYPH, mult, money, type CasinoState, type Round, type Sym } from './casino';
  import { toast, errorText } from './host';
  import { dur, ease, loop, tween, wait, type Tween } from './anim';
  import { sfx } from './sfx';

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.slots);

  // Each reel is a ring of symbols that scrolls past a three-row window. `pos` is in cells, so it scales with the window.
  const RING = 48, MAX_SPEED = 21, ACCEL = 70;
  type Reel = { pos: number; v: number; mode: 'idle' | 'spin' | 'stop'; ring: Sym[] };
  const pick = (): Sym => {
    const syms = untrack(() => cfg.symbols);
    const total = syms.reduce((a, s) => a + s.weight, 0);
    let r = Math.random() * total;
    for (const s of syms) if ((r -= s.weight) < 0) return s;
    return syms[0];
  };
  const makeReel = (): Reel => ({ pos: Math.floor(Math.random() * RING), v: 0, mode: 'idle', ring: Array.from({ length: RING }, pick) });

  let bet = $state(untrack(() => st.config.slots.min_bet));
  hotkeys(() => { const go = () => { if (!busy && cfg.enabled && !(balance != null && balance < bet)) void spin(); }; return { space: go, enter: go }; });
  let busy = $state(false);
  let reels = $state<Reel[]>([makeReel(), makeReel(), makeReel()]);
  let win = $state<{ amount: number; mult: number; key: number; profit: boolean; line: string } | null>(null);
  let lit = $state<number[]>([]);
  let shake = $state(false);
  const live: Tween[] = [];
  let stopLoop: (() => void) | null = null;

  const view = (reel: Reel) => {
    const base = Math.floor(reel.pos), frac = reel.pos - base;
    return [-1, 0, 1, 2, 3].map((r) => ({ k: base + r, y: r - frac, sym: reel.ring[(((base + r) % RING) + RING) % RING], row: r }));
  };

  function startLoop() {
    stopLoop?.();
    let lastNotch = -1;
    stopLoop = loop((dt) => {
      let any = false;
      reels.forEach((r, i) => {
        if (r.mode !== 'spin') return;
        any = true;
        r.v = Math.min(MAX_SPEED, r.v + ACCEL * dt);
        r.pos = (r.pos + r.v * dt) % RING;
        if (i === 0 && Math.floor(r.pos) !== lastNotch) { lastNotch = Math.floor(r.pos); sfx.tick(0.8); }
      });
      if (!any && !reels.some((r) => r.mode === 'stop')) { stopLoop = null; return false; }
    });
  }

  /** Slow reel `i` down onto the symbol the server rolled; resolves when it has settled. */
  async function land(i: number, target: number | null) {
    const r = reels[i];
    const D = dur(1250);
    const dist = Math.max(4, Math.ceil((Math.max(r.v, 8) * (D / 1000)) / 4.2));
    const from = r.pos;
    const stop = Math.ceil(from) + dist;
    // The cells the reel is about to show are far above the window, so changing them now is invisible.
    r.ring[(stop + 1) % RING] = target == null ? pick() : cfg.symbols[target] ?? pick();
    r.ring[stop % RING] = pick();
    r.ring[(stop + 2) % RING] = pick();
    r.mode = 'stop';
    let lastNotch = Math.floor(from);
    const t = tween(D, (e) => {
      r.pos = from + (stop - from) * e;
      r.v = Math.max(0, (1 - e) * 12);
      const n = Math.floor(r.pos);
      if (n !== lastNotch) { lastNotch = n; if (i === 0 || e > 0.6) sfx.tick(0.7 + e * 0.5); }
    }, ease.outBack);
    live.push(t);
    await t.done;
    r.pos = stop % RING;
    r.v = 0;
    r.mode = 'idle';
    sfx.stop();
  }

  async function spin() {
    if (busy) return;
    busy = true; win = null; lit = []; shake = false;
    reels.forEach((r) => { r.mode = 'spin'; r.v = 0; });
    startLoop();
    const began = performance.now();
    try {
      const round = await cpost<Round>(st.server.id, '/slots', { bet });
      const target: number[] = round.result.reels;
      onbalance(round.balance - round.payout);
      // Keep them spinning a moment so a fast answer still feels like a spin.
      await wait(Math.max(0, dur(950) - (performance.now() - began)));
      await Promise.all(reels.map(async (_, i) => { await wait(i * dur(430)); await land(i, target[i]); }));
      onbalance(round.balance);
      reveal(round, target);
      onplayed();
    } catch (e) {
      reels.forEach((r) => { if (r.mode === 'spin') r.mode = 'idle'; r.v = 0; });
      toast(errorText(e), 'error');
    } finally { busy = false; }
  }

  function reveal(r: Round, target: number[]) {
    if (r.payout <= 0) { sfx.lose(); return; }
    const triple = target[0] === target[1] && target[1] === target[2];
    lit = triple ? [0, 1, 2] : [0, 1, 2].filter((i) => target.filter((t) => t === target[i]).length > 1);
    win = { amount: r.payout, mult: r.multiplier, key: Date.now(), profit: r.payout > r.bet, line: triple ? 'Three in a row!' : 'Two of a kind' };
    sfx.win(r.multiplier >= 10 ? 3 : r.multiplier >= 3 ? 2 : 1);
    if (r.multiplier >= 10) { shake = true; setTimeout(() => (shake = false), 700); }
  }

  const sparkle = $derived(win && win.mult >= 10);
  onMount(() => () => { stopLoop?.(); live.forEach((t) => t.cancel()); });
</script>

<div class="game">
  <section class="panel controls">
    <h2><Dices size={18} /> Slots</h2>
    <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} disabled={busy} />
    <button class="spinbtn primary" onclick={spin} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>{busy ? 'Spinning…' : `Spin · ${money(bet)}`}<kbd class="k">Space</kbd></button>
    {#if !cfg.enabled}<p class="note">Slots is closed right now.</p>{/if}
    <p class="note"><Info size={13} /> Pays back {(st.rtp.slots * 100).toFixed(1)}% over time. {(st.rtp.slots_hit * 100).toFixed(0)}% of spins pay something.</p>
  </section>

  <section class="stage">
    <div class="cabinet" class:shake>
      <div class="marquee">{#each Array(14) as _, i}<i class:run={busy} style="animation-delay:{i * 0.09}s"></i>{/each}</div>
      <div class="window" class:busy>
        {#each reels as reel, ri}
          <div class="reel" class:spinning={reel.mode !== 'idle'} style="--blur:{Math.min(3.2, reel.v / 7).toFixed(2)}px">
            {#each view(reel) as c (c.k)}
              <div class="cell" class:hit={lit.includes(ri) && c.row === 1 && reel.mode === 'idle'} style="--c:{c.sym.color};transform:translate3d(0,calc(var(--cell) * {c.y.toFixed(3)}),0)">
                <span>{GLYPH[c.sym.id] ?? c.sym.label.slice(0, 1)}</span>
              </div>
            {/each}
          </div>
        {/each}
        <div class="line" class:hit={!!win}></div>
        <i class="arrow l"></i><i class="arrow r"></i>
        <div class="shade"></div>
      </div>
      {#if win}{#key win.key}
        <div class="result" class:soft={!win.profit}><b>{win.line}</b><span>{win.profit ? '+' : ''}{money(win.amount)}{win.profit ? '' : ' back'} <small>({mult(win.mult)})</small></span>{#if sparkle}<Burst count={44} />{/if}</div>
      {/key}{/if}
    </div>
    <div class="paytable">
      {#each [...cfg.symbols].sort((a, b) => b.pay - a.pay) as s}
        <div class="pay" style="--c:{s.color}"><span class="g">{GLYPH[s.id] ?? s.label.slice(0, 1)}</span><span>×3</span><b>{mult(s.pay)}</b></div>
      {/each}
      <div class="pay pair"><span class="g">2×</span><span>any two</span><b>{mult(cfg.pair_pay)}</b></div>
    </div>
  </section>
</div>

<style>
  .game { --cell: 5.6rem; display: grid; grid-template-columns: 19rem 1fr; gap: 1.1rem; align-items: start; }
  /* Phones: show the game first so you can watch it, with the main button pinned above the tab bar. */
  @media (max-width: 860px) {
    .game { grid-template-columns: 1fr; }
    .stage { order: -1; }
    .spinbtn { position: sticky; bottom: calc(var(--pl-tab, 0px) + env(safe-area-inset-bottom, 0px) + 10px); z-index: 4; }
  }
  @media (max-width: 480px) { .game { --cell: 4.3rem; } }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.9rem; }
  h2 { display: flex; gap: 0.5rem; align-items: center; font-size: 1.05rem; }
  .spinbtn { padding: 0.85rem; font-size: 1rem; font-weight: 700; border-radius: var(--radius); background: linear-gradient(135deg, #f5b942, #d97706); color: #1a1205; border: none; box-shadow: 0 6px 22px -6px #f59e0b; }
  .spinbtn:hover:not(:disabled) { filter: brightness(1.08); transform: translateY(-1px); }
  .spinbtn:disabled { filter: grayscale(0.4) brightness(0.9); }
  .note { font-size: 0.78rem; color: var(--muted); display: flex; gap: 0.35rem; align-items: flex-start; }
  .stage { display: flex; flex-direction: column; gap: 1rem; align-items: center; min-width: 0; }
  .cabinet { position: relative; max-width: 100%; padding: 1.2rem 1.4rem 1.4rem; border-radius: 1.4rem; background: linear-gradient(160deg, #3a2a0b, #1a1206 55%, #2b1d07); border: 2px solid #8a6a1f; box-shadow: 0 20px 60px -20px #000, inset 0 0 40px #0008, 0 0 50px -10px #f59e0b55; }
  .cabinet.shake { animation: shake 0.6s ease-out; }
  @keyframes shake { 10%, 90% { transform: translateX(-2px); } 20%, 80% { transform: translateX(4px); } 30%, 50%, 70% { transform: translateX(-6px); } 40%, 60% { transform: translateX(6px); } }
  .marquee { display: flex; justify-content: space-between; margin-bottom: 0.8rem; }
  .marquee i { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: #fbbf24; box-shadow: 0 0 8px #fbbf24; opacity: 0.55; }
  .marquee i.run { animation: blink 0.5s infinite alternate; }
  @keyframes blink { from { opacity: 1; } to { opacity: 0.2; box-shadow: none; } }
  .window { position: relative; display: flex; gap: 0.5rem; padding: 0.5rem; border-radius: 1rem; background: #07070a; box-shadow: inset 0 0 22px #000; }
  .reel { position: relative; width: calc(var(--cell) * 1.08); height: calc(var(--cell) * 3); overflow: hidden; border-radius: 0.7rem; background: linear-gradient(#e9e4d4, #fff 20%, #fff 80%, #e9e4d4); }
  .reel.spinning .cell span { filter: blur(var(--blur)); }
  .cell { position: absolute; left: 0; right: 0; top: 0; height: var(--cell); display: grid; place-items: center; will-change: transform; }
  .cell span { line-height: 1; font-size: calc(var(--cell) * 0.56); filter: drop-shadow(0 3px 2px #0004); }
  .cell.hit { background: radial-gradient(circle, color-mix(in srgb, var(--c) 45%, #fff0) 0%, transparent 70%); }
  .cell.hit span { animation: pop 0.55s ease-in-out 4; }
  @keyframes pop { 50% { transform: scale(1.28) rotate(-6deg); } }
  .shade { position: absolute; inset: 0.5rem; border-radius: 0.7rem; pointer-events: none; background: linear-gradient(#0009, transparent 28%, transparent 72%, #0009); }
  .line { position: absolute; left: 0.2rem; right: 0.2rem; top: 50%; height: 3px; margin-top: -1px; background: #ef4444aa; box-shadow: 0 0 10px #ef4444; border-radius: 3px; pointer-events: none; z-index: 2; }
  .line.hit { background: #fde68a; box-shadow: 0 0 18px 3px #fbbf24; animation: flash 0.5s 4; }
  @keyframes flash { 50% { opacity: 0.3; } }
  .arrow { position: absolute; top: 50%; margin-top: -0.45rem; border: 0.45rem solid transparent; z-index: 2; }
  .arrow.l { left: -0.1rem; border-left-color: #fbbf24; } .arrow.r { right: -0.1rem; border-right-color: #fbbf24; }
  .result { position: absolute; left: 50%; bottom: -1.1rem; transform: translateX(-50%); padding: 0.55rem 1.3rem; border-radius: 99rem; background: linear-gradient(135deg, #16a34a, #15803d); box-shadow: 0 8px 30px -6px #22c55e; display: flex; flex-direction: column; align-items: center; line-height: 1.2; animation: rise 0.45s cubic-bezier(0.2, 1.4, 0.4, 1); white-space: nowrap; z-index: 6; }
  .result.soft { background: linear-gradient(135deg, #b45309, #92400e); box-shadow: 0 8px 30px -8px #f59e0b; }
  .result b { font-size: 0.78rem; opacity: 0.9; } .result span { font-size: 1.25rem; font-weight: 800; } .result small { font-weight: 500; opacity: 0.8; }
  @keyframes rise { from { transform: translateX(-50%) translateY(14px) scale(0.5); opacity: 0; } }
  .paytable { display: flex; flex-wrap: wrap; gap: 0.5rem; justify-content: center; margin-top: 0.8rem; }
  .pay { display: flex; align-items: center; gap: 0.4rem; padding: 0.35rem 0.7rem; border-radius: 99rem; background: color-mix(in srgb, var(--c, #888) 14%, var(--panel-solid, var(--surface))); border: 1px solid color-mix(in srgb, var(--c, #888) 40%, transparent); font-size: 0.82rem; }
  .pay .g { font-size: 1.1rem; } .pay span:nth-child(2) { color: var(--muted); } .pay b { font-variant-numeric: tabular-nums; }
  .pair { --c: #94a3b8; }
</style>
