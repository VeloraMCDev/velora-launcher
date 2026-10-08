<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Rocket, Info } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import Burst from './Burst.svelte';
  import GameShell from './GameShell.svelte';
  import DoubleOrNothing from './DoubleOrNothing.svelte';
  import { cpost, cget, money, mult, multColor, type CasinoState, type CrashGame, type DoubleOffer } from './casino';
  import { toast, errorText } from './host';
  import { loop } from './anim';
  import { sfx } from './sfx';

  type Settled = { game: CrashGame | null; balance?: number; payout?: number; double?: DoubleOffer | null };

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.crash);
  let bet = $state(untrack(() => st.config.crash.min_bet));
  hotkeys(() => { const go = () => { if (running) void cashout(); else if (!busy && cfg.enabled && !(balance != null && balance < bet)) void start(); }; return { space: go, enter: go }; });
  let useAuto = $state(false);
  let auto = $state(2);
  let game = $state<CrashGame | null>(untrack(() => st.crash));
  let busy = $state(false);
  let m = $state(1); // the multiplier on screen
  let elapsed = $state(0);
  let final = $state<Settled | null>(null);
  let offer = $state<DoubleOffer | null>(null);
  let recent = $state<{ point: number; win: boolean }[]>([]);
  const running = $derived(game?.status === 'active' && !final);
  const busted = $derived(final?.game?.status === 'busted');
  const cashed = $derived(final?.game?.status === 'cashed');

  // The multiplier follows the same curve as the server, from the moment the round began.
  let t0 = 0;
  let nextTick = 1;
  let finishing = false;
  let stopLoop: (() => void) | null = null;
  let poller: ReturnType<typeof setInterval> | null = null;

  function begin(g: CrashGame) {
    game = g; final = null; offer = null; finishing = false; m = g.multiplier; elapsed = g.elapsed_ms; nextTick = Math.floor(m) + 1;
    t0 = performance.now() - g.elapsed_ms;
    stopLoop?.();
    stopLoop = loop(() => {
      if (finishing || !game) return false;
      elapsed = performance.now() - t0;
      m = Math.min(cfg.max_multiplier, Math.exp((game.rate * elapsed) / 1000));
      if (m >= nextTick) { sfx.tick(1 + Math.min(1, nextTick / 20)); nextTick = Math.floor(m) + 1; }
    });
    poller && clearInterval(poller);
    poller = setInterval(poll, 280);
  }
  function stop() { stopLoop?.(); stopLoop = null; if (poller) clearInterval(poller); poller = null; }

  function settle(r: Settled) {
    if (finishing || !r.game || r.game.status === 'active') return;
    finishing = true; stop();
    final = r; game = r.game;
    const g = r.game;
    // Snap to the number the server settled on: the crash point, or what the player cashed out at.
    m = g.status === 'busted' ? (g.crash_point ?? m) : Math.round(((r.payout ?? 0) / g.bet) * 100) / 100;
    elapsed = Math.max(elapsed, Math.log(Math.max(1, g.crash_point ?? m)) / g.rate * 1000);
    recent = [{ point: g.crash_point ?? m, win: g.status === 'cashed' }, ...recent].slice(0, 10);
    if (g.status === 'busted') sfx.boom(); else sfx.win((r.payout ?? 0) / g.bet >= 5 ? 3 : 2);
    if (r.balance != null) onbalance(r.balance);
    offer = r.double ?? null;
    onplayed();
  }
  async function poll() {
    if (finishing || !game) return;
    try {
      const r = await cget<Settled>(st.server.id, '/crash/status');
      if (!r.game) { finishing = true; stop(); game = null; onplayed(); return; }
      settle(r);
    } catch { /* a missed poll is retried on the next tick */ }
  }

  async function start() {
    if (busy || running) return;
    busy = true; final = null; offer = null; m = 1; elapsed = 0;
    try {
      const r = await cpost<{ game: CrashGame; balance: number }>(st.server.id, '/crash/start', useAuto ? { bet, auto } : { bet });
      onbalance(r.balance);
      begin(r.game);
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }
  async function cashout() {
    if (busy || !running) return;
    busy = true;
    try {
      const r = await cpost<Settled>(st.server.id, '/crash/cashout');
      settle(r);
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }

  onMount(() => { if (game?.status === 'active') begin(game); return stop; });

  // ---- the graph ----
  const W = 600, H = 320, PAD = 34;
  const xMax = $derived(Math.max(10000, elapsed * 1.08));
  const yMax = $derived(Math.max(2, m * 1.18));
  const rate = $derived(game?.rate ?? 0.12);
  const px = (t: number) => PAD + (t / xMax) * (W - PAD - 14);
  const py = (v: number) => H - PAD - ((v - 1) / (yMax - 1)) * (H - PAD - 16);
  const curve = $derived.by(() => {
    const end = Math.max(1, elapsed);
    const pts: string[] = [];
    const n = 70;
    for (let i = 0; i <= n; i++) { const t = (end * i) / n; pts.push(`${px(t).toFixed(1)},${py(Math.exp((rate * t) / 1000)).toFixed(1)}`); }
    return pts;
  });
  const head = $derived({ x: px(Math.max(1, elapsed)), y: py(Math.min(m, yMax)) });
  const yTicks = $derived.by(() => { const step = yMax > 20 ? 10 : yMax > 8 ? 2 : yMax > 4 ? 1 : 0.5; const out: number[] = []; for (let v = 1 + step; v < yMax; v += step) out.push(Math.round(v * 100) / 100); return out; });
  const color = $derived(busted ? '#ef4444' : multColor(m));
</script>

<GameShell title="Crash" icon={Rocket}>
  {#snippet controls()}
    <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} disabled={running || busy} />
    <div class="autoc">
      <label class="chk"><input type="checkbox" bind:checked={useAuto} disabled={running || busy} /> Auto cash out</label>
      <label class="at" class:off={!useAuto}>at <input type="number" min="1.01" max={cfg.max_multiplier} step="0.1" bind:value={auto} disabled={!useAuto || running || busy} aria-label="Auto cash out multiplier" />×</label>
    </div>
    <p class="note"><Info size={13} /> The multiplier climbs until it crashes, and nobody knows where. Cash out before it does to win your bet times the number. Pays back {(st.rtp.crash * 100).toFixed(0)}% over time, whatever you aim for.</p>
    {#if recent.length}<div class="chips" aria-label="Recent rounds">{#each recent as r}<span class={r.win ? 'w' : 'l'}>{mult(r.point)}</span>{/each}</div>{/if}
  {/snippet}
  {#snippet action()}
    {#if running}
      <button class="big cash" onclick={cashout} disabled={busy}>Cash out · {money(bet * m)}<kbd class="k">Space</kbd></button>
    {:else}
      <button class="big go" onclick={start} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>{busy ? 'Launching…' : `Launch · ${money(bet)}`}<kbd class="k">Space</kbd></button>
    {/if}
    {#if !cfg.enabled}<p class="note">Crash is closed right now.</p>{/if}
  {/snippet}
  {#snippet stage()}
    <div class="sky" class:bust={busted} class:win={cashed}>
      <svg viewBox="0 0 {W} {H}" role="img" aria-label="Multiplier graph" preserveAspectRatio="xMidYMid meet">
        <defs>
          <linearGradient id="crfill" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color={color} stop-opacity="0.35" /><stop offset="1" stop-color={color} stop-opacity="0" /></linearGradient>
        </defs>
        {#each yTicks as v}<g><line x1={PAD} x2={W - 10} y1={py(v)} y2={py(v)} stroke="#ffffff10" /><text x={PAD - 6} y={py(v) + 4} text-anchor="end" font-size="11" fill="#94a3b8">{mult(v)}</text></g>{/each}
        <line x1={PAD} x2={PAD} y1="10" y2={H - PAD} stroke="#ffffff22" /><line x1={PAD} x2={W - 10} y1={H - PAD} y2={H - PAD} stroke="#ffffff22" />
        {#if running || final}
          <polygon points="{PAD},{H - PAD} {curve.join(' ')} {head.x},{H - PAD}" fill="url(#crfill)" />
          <polyline points={curve.join(' ')} fill="none" stroke={color} stroke-width="4" stroke-linecap="round" stroke-linejoin="round" style="filter: drop-shadow(0 0 8px {color})" />
          {#if !busted}<g transform="translate({head.x} {head.y})"><circle r="14" fill={color} opacity="0.25" /><circle r="6" fill="#fff" /></g>
          {:else}<g transform="translate({head.x} {head.y})" class="boom"><circle r="26" fill="#ef4444" opacity="0.35" /><circle r="14" fill="#f97316" /><circle r="6" fill="#fff" /></g>{/if}
        {/if}
      </svg>
      <div class="read" aria-live="polite">
        <div class="mult" style:color>{running || final ? mult(m) : '1.00×'}</div>
        {#if running}
          <div class="sub">{useAuto && game?.auto ? `auto at ${mult(game.auto)} · ` : ''}worth {money(bet * m)}</div>
        {:else if busted}
          <div class="sub bad">Crashed at {mult(final?.game?.crash_point ?? m)}</div>
        {:else if cashed}
          <div class="sub good">Cashed out · +{money((final?.payout ?? 0) - bet)}</div>
        {:else}
          <div class="sub">Ready for launch</div>
        {/if}
      </div>
      {#if cashed}{#key final}<Burst />{/key}{/if}
    </div>
    {#if offer}{#key offer.id}<DoubleOrNothing {st} {offer} {onbalance} {onplayed} />{/key}{/if}
  {/snippet}
</GameShell>

<style>
  .go { background: linear-gradient(135deg, #f43f5e, #8b5cf6); box-shadow: 0 6px 22px -6px #f43f5e; }
  .cash { background: linear-gradient(135deg, #f59e0b, #d97706); color: #1a1205 !important; box-shadow: 0 6px 22px -6px #f59e0b; animation: pulse 1s ease-in-out infinite; }
  @keyframes pulse { 50% { box-shadow: 0 6px 34px -2px #f59e0b; } }
  .autoc { display: flex; align-items: center; justify-content: space-between; gap: 0.6rem; flex-wrap: wrap; }
  .chk { display: inline-flex; gap: 0.45rem; align-items: center; font-size: 0.86rem; font-weight: 600; }
  .at { display: inline-flex; gap: 0.3rem; align-items: center; color: var(--muted); font-weight: 700; } .at.off { opacity: 0.5; } .at input { width: 5.2rem; text-align: right; font-weight: 800; padding: 0.3rem 0.5rem; }
  .sky { position: relative; border-radius: var(--radius); background: radial-gradient(110% 90% at 50% 0%, #1b1740, #080a14 72%); border: 1px solid var(--line); overflow: hidden; margin-bottom: 1rem; transition: box-shadow 0.3s, border-color 0.3s; }
  .sky.bust { border-color: #ef444488; box-shadow: 0 0 60px -14px #ef4444 inset; animation: shake 0.45s; } .sky.win { border-color: #22c55e77; box-shadow: 0 0 60px -14px #22c55e inset; }
  @keyframes shake { 20% { transform: translateX(-6px); } 40% { transform: translateX(6px); } 60% { transform: translateX(-4px); } 80% { transform: translateX(3px); } }
  svg { width: 100%; height: auto; display: block; max-height: 62vh; }
  .read { position: absolute; inset: 0; display: grid; place-content: start start; text-align: left; padding: 1rem 1.2rem 0 3.4rem; pointer-events: none; }
  .mult { font-size: clamp(2.4rem, 10vw, 4.4rem); font-weight: 900; font-variant-numeric: tabular-nums; letter-spacing: -0.03em; text-shadow: 0 0 40px currentColor; line-height: 1; }
  .sub { font-size: 0.95rem; font-weight: 700; color: var(--muted); margin-top: 0.4rem; } .sub.bad { color: #f87171; } .sub.good { color: #4ade80; }
  .boom circle:first-child { animation: ring 0.6s ease-out; transform-origin: center; } @keyframes ring { from { transform: scale(0.2); opacity: 1; } }
</style>
