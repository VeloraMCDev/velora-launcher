<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Gift, Clock } from '@lucide/svelte';
  import WheelSvg from './WheelSvg.svelte';
  import Burst from './Burst.svelte';
  import { cpost, money, segmentAt, segmentCenter, untilText, type CasinoState } from './casino';
  import { toast, errorText } from './host';
  import { dur, loop, wait, wheelController } from './anim';
  import { sfx } from './sfx';

  let { st, onbalance, onplayed }: { st: CasinoState; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.daily);
  let rotation = $state(0);
  let pointer = $state(0);
  let busy = $state(false);
  let landed = $state<number | null>(null);
  let prize = $state<number | null>(null);
  let left = $state(untrack(() => st.free.left));
  let now = $state(Date.now());
  $effect(() => { left = st.free.left; });

  let under = -1;
  const wheel = wheelController(0, (deg, speed) => {
    rotation = deg;
    const i = segmentAt(untrack(() => cfg.segments), -deg);
    if (i !== under) { if (under !== -1) { pointer = -Math.min(26, 9 + speed / 45); sfx.tick(); } under = i; }
  });
  const stopPointer = loop((dt) => { if (Math.abs(pointer) > 0.05) pointer *= Math.exp(-dt * 13); else if (pointer !== 0) pointer = 0; });
  onMount(() => { const t = setInterval(() => (now = Date.now()), 1000); return () => { clearInterval(t); wheel.cancel(); stopPointer(); }; });

  async function spin() {
    if (busy || left <= 0) return;
    busy = true; landed = null; prize = null;
    wheel.start();
    const began = performance.now();
    try {
      const r = await cpost<{ segment: number; payout: number; balance: number; left: number }>(st.server.id, '/daily');
      const segs = cfg.segments;
      const w = segs.reduce((a, s) => a + s.weight, 0);
      const jitter = (Math.random() - 0.5) * 0.6 * ((segs[r.segment].weight / w) * 360);
      onbalance(r.balance - r.payout);
      await wait(Math.max(0, dur(800) - (performance.now() - began)));
      await wheel.land(-segmentCenter(segs, r.segment) + jitter, 4000);
      landed = r.segment; prize = r.payout; left = r.left;
      sfx.stop(); sfx.win(2);
      onbalance(r.balance);
      onplayed();
    } catch (e) { wheel.cancel(); toast(errorText(e), 'error'); } finally { busy = false; }
  }
</script>

<section class="daily" class:ready={left > 0}>
  <div class="wheelbox">
    <WheelSvg segments={cfg.segments} {rotation} {pointer} size={240} highlight={landed} />
    {#if prize != null}{#key prize}<Burst />{/key}{/if}
  </div>
  <div class="copy">
    <span class="tag"><Gift size={13} /> Free daily spin</span>
    {#if prize != null}
      <h3>You won {money(prize)}!</h3>
      <p>It's in your balance already.</p>
    {:else}
      <h3>{left > 0 ? 'Your free spin is ready' : 'Come back tomorrow'}</h3>
      <p>Everyone gets {cfg.spins_per_day} free spin{cfg.spins_per_day === 1 ? '' : 's'} a day. Prizes go straight to your balance. Nothing to lose.</p>
    {/if}
    {#if left > 0}
      <button class="primary go" onclick={spin} disabled={busy || !cfg.enabled}>{busy ? 'Spinning…' : left > 1 ? `Spin free (${left} left)` : 'Spin for free'}</button>
    {:else}
      <p class="reset"><Clock size={14} /> Next free spin in <b>{untilText(st.free.resets_at, now)}</b></p>
    {/if}
  </div>
</section>

<style>
  .daily { position: relative; display: flex; gap: 1.6rem; align-items: center; padding: 1.3rem 1.6rem; border-radius: calc(var(--radius) + 4px); background: radial-gradient(120% 140% at 0% 0%, color-mix(in srgb, #ec4899 20%, transparent), transparent 60%), var(--panel, var(--surface)); border: 1px solid var(--line-strong); overflow: hidden; flex-wrap: wrap; }
  .daily.ready { border-color: color-mix(in srgb, #ec4899 55%, transparent); box-shadow: 0 0 0 1px color-mix(in srgb, #ec4899 25%, transparent), 0 20px 60px -30px #ec4899; }
  .wheelbox { position: relative; padding-top: 0.6rem; }
  .copy { display: flex; flex-direction: column; gap: 0.55rem; flex: 1; min-width: 14rem; align-items: flex-start; }
  .tag { display: inline-flex; gap: 0.35rem; align-items: center; font-size: 0.75rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.07em; color: #f9a8d4; }
  h3 { font-size: 1.5rem; } p { color: var(--muted); font-size: 0.9rem; max-width: 30rem; }
  .go { padding: 0.8rem 1.7rem; font-size: 1rem; font-weight: 700; border: none; border-radius: 99rem; background: linear-gradient(135deg, #ec4899, #8b5cf6); color: #fff; box-shadow: 0 8px 28px -6px #ec4899; animation: glow 2.4s infinite; }
  .go:disabled { animation: none; }
  @keyframes glow { 50% { box-shadow: 0 8px 40px -2px #ec4899; transform: translateY(-1px); } }
  .reset { display: inline-flex; gap: 0.4rem; align-items: center; } .reset b { color: var(--text); font-variant-numeric: tabular-nums; }
  @media (prefers-reduced-motion: reduce) { .go { animation: none; } }
</style>
