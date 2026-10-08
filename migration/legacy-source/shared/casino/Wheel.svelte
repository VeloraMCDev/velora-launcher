<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { CircleDollarSign, Info } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import WheelSvg from './WheelSvg.svelte';
  import Burst from './Burst.svelte';
  import { cpost, mult, money, segmentAt, segmentCenter, type CasinoState, type Round } from './casino';
  import { toast, errorText } from './host';
  import { dur, loop, tween, wait, wheelController } from './anim';
  import { sfx } from './sfx';

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.wheel);
  let bet = $state(untrack(() => st.config.wheel.min_bet));
  hotkeys(() => { const go = () => { if (!busy && cfg.enabled && !(balance != null && balance < bet)) void spin(); }; return { space: go, enter: go }; });
  let busy = $state(false);
  let rotation = $state(0);
  let pointer = $state(0);
  let landed = $state<number | null>(null);
  let result = $state<Round | null>(null);

  // The pointer is flicked each time a slice boundary passes under it, like a real wheel.
  let under = -1;
  const wheel = wheelController(0, (deg, speed) => {
    rotation = deg;
    const i = segmentAt(untrack(() => cfg.segments), -deg);
    if (i !== under) {
      if (under !== -1) { pointer = -Math.min(26, 9 + speed / 45); sfx.tick(1 + Math.min(0.6, speed / 1500)); }
      under = i;
    }
  });
  const stopPointer = loop((dt) => { if (Math.abs(pointer) > 0.05) pointer *= Math.exp(-dt * 13); else if (pointer !== 0) pointer = 0; });

  async function spin() {
    if (busy) return;
    busy = true; landed = null; result = null;
    wheel.start();
    const began = performance.now();
    try {
      const r = await cpost<Round>(st.server.id, '/wheel', { bet });
      const i: number = r.result.segment;
      onbalance(r.balance - r.payout);
      await wait(Math.max(0, dur(800) - (performance.now() - began)));
      // Stop with the winning slice under the pointer (a little off-centre looks more real).
      const total = cfg.segments.reduce((a, s) => a + s.weight, 0);
      const jitter = (Math.random() - 0.5) * 0.6 * (360 * cfg.segments[i].weight / total);
      await wheel.land(-segmentCenter(cfg.segments, i) + jitter);
      landed = i; result = r;
      sfx.stop();
      if (r.payout > 0) sfx.win(r.multiplier >= 10 ? 3 : r.multiplier >= 3 ? 2 : 1); else sfx.lose();
      // A little settle: the pointer rests in its slot.
      void tween(260, (e) => (pointer = -6 * (1 - e)));
      onbalance(r.balance);
      onplayed();
    } catch (e) {
      wheel.cancel();
      toast(errorText(e), 'error');
    } finally { busy = false; }
  }
  onMount(() => () => { wheel.cancel(); stopPointer(); });
</script>

<div class="game">
  <section class="panel controls">
    <h2><CircleDollarSign size={18} /> Wheel</h2>
    <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} disabled={busy} />
    <button class="spinbtn primary" onclick={spin} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>{busy ? 'Spinning…' : `Spin · ${money(bet)}`}<kbd class="k">Space</kbd></button>
    {#if !cfg.enabled}<p class="note">The wheel is closed right now.</p>{/if}
    <p class="note"><Info size={13} /> Pays back {(st.rtp.wheel * 100).toFixed(1)}% over time. Wider slices are likelier.</p>
    <div class="legend">
      {#each [...cfg.segments].sort((a, b) => b.value - a.value) as s}<span style="--c:{s.color}"><i></i>{mult(s.value)}</span>{/each}
    </div>
  </section>
  <section class="stage">
    <div class="holder">
      <WheelSvg segments={cfg.segments} {rotation} {pointer} size={340} highlight={landed} />
      {#if result}{#key result}
        <div class="banner" class:win={result.payout > 0} class:lose={result.payout === 0}>
          {#if result.payout > 0}<b>{mult(result.multiplier)}</b><span>+{money(result.payout)}</span>{:else}<b>0×</b><span>Better luck next spin</span>{/if}
          {#if result.multiplier >= 5}<Burst />{/if}
        </div>
      {/key}{/if}
    </div>
  </section>
</div>

<style>
  .game { display: grid; grid-template-columns: 19rem 1fr; gap: 1.1rem; align-items: start; }
  /* Phones: show the game first so you can watch it, with the main button pinned above the tab bar. */
  @media (max-width: 860px) {
    .game { grid-template-columns: 1fr; }
    .stage { order: -1; }
    .spinbtn { position: sticky; bottom: calc(var(--pl-tab, 0px) + env(safe-area-inset-bottom, 0px) + 10px); z-index: 4; }
  }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.9rem; }
  h2 { display: flex; gap: 0.5rem; align-items: center; font-size: 1.05rem; }
  .spinbtn { padding: 0.85rem; font-size: 1rem; font-weight: 700; border-radius: var(--radius); background: linear-gradient(135deg, #ec4899, #8b5cf6); border: none; color: #fff; box-shadow: 0 6px 22px -6px #c026d3; }
  .spinbtn:hover:not(:disabled) { filter: brightness(1.1); transform: translateY(-1px); }
  .note { font-size: 0.78rem; color: var(--muted); display: flex; gap: 0.35rem; align-items: flex-start; }
  .legend { display: flex; flex-wrap: wrap; gap: 0.35rem; }
  .legend span { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.75rem; padding: 0.15rem 0.5rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 6%, transparent); }
  .legend i { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: var(--c); }
  .stage { display: grid; place-items: center; padding: 1.2rem 0 2rem; }
  .holder { position: relative; }
  .banner { position: absolute; left: 50%; bottom: -1.7rem; transform: translateX(-50%); padding: 0.55rem 1.5rem; border-radius: 99rem; display: flex; gap: 0.7rem; align-items: baseline; white-space: nowrap; animation: pop 0.4s var(--ease); box-shadow: 0 10px 34px -8px #000; }
  .banner.win { background: linear-gradient(135deg, #16a34a, #15803d); box-shadow: 0 8px 30px -6px #22c55e; }
  .banner.lose { background: color-mix(in srgb, var(--surface) 90%, #fff 4%); border: 1px solid var(--line-strong); color: var(--muted); }
  .banner b { font-size: 1.4rem; } .banner span { font-weight: 700; }
  @keyframes pop { from { transform: translateX(-50%) scale(0.6); opacity: 0; } }
</style>
