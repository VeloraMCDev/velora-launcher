<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Dices, Info } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import Burst from './Burst.svelte';
  import GameShell from './GameShell.svelte';
  import DoubleOrNothing from './DoubleOrNothing.svelte';
  import { cpost, money, mult, type CasinoState, type DoubleOffer, type Round, type Twist } from './casino';
  import { toast, errorText } from './host';
  import { dur, tween, wait } from './anim';
  import { sfx } from './sfx';

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.dice);
  let bet = $state(untrack(() => st.config.dice.min_bet));
  hotkeys(() => { const go = () => { if (!busy && cfg.enabled && !(balance != null && balance < bet)) void roll(); }; return { space: go, enter: go, u: () => { if (!busy) over = false; }, o: () => { if (!busy) over = true; } }; });
  let chance = $state(untrack(() => Math.min(st.config.dice.max_chance, Math.max(st.config.dice.min_chance, 50))));
  let over = $state(false);
  let busy = $state(false);
  let shown = $state<number | null>(null); // the number on the die while it rolls
  let marker = $state<number | null>(null); // where the marker sits on the track, 0 to 100
  let result = $state<Round | null>(null);
  let offer = $state<DoubleOffer | null>(null);
  let recent = $state<{ roll: number; win: boolean }[]>([]);
  const twist = $derived((result?.result?.twist ?? null) as Twist | null);

  // Same maths as the server: the fair price for this chance, less the house edge.
  const pays = $derived(Math.floor((100 * (1 - cfg.house_edge) / chance) * 10000) / 10000);
  const target = $derived(over ? 100 - chance : chance);
  const clampChance = (v: number) => Math.round(Math.min(cfg.max_chance, Math.max(cfg.min_chance, v)) * 100) / 100;
  // The slider is steeper at the low end, where chances are small and every fraction matters.
  const sliderPos = $derived((Math.log(chance / cfg.min_chance) / Math.log(cfg.max_chance / cfg.min_chance)) * 100 || 0);
  const fromSlider = (p: number) => clampChance(cfg.min_chance * Math.pow(cfg.max_chance / cfg.min_chance, p / 100));
  let timer: Tween | null = null;
  type Tween = ReturnType<typeof tween>;

  async function roll() {
    if (busy) return;
    busy = true; result = null; offer = null; marker = null;
    try {
      const began = performance.now();
      const p = cpost<Round & { double: DoubleOffer | null }>(st.server.id, '/dice', { bet, chance, mode: over ? 'over' : 'under' });
      // The number tumbles while the server decides.
      const spinner = setInterval(() => { shown = Math.floor(Math.random() * 10000) / 100; sfx.tick(1.2); }, 70);
      let r: Round & { double: DoubleOffer | null };
      try { r = await p; } finally { clearInterval(spinner); }
      onbalance(r.balance - r.payout);
      await wait(Math.max(0, dur(900) - (performance.now() - began)));
      const roll: number = r.result.roll;
      const from = shown ?? 50;
      timer = tween(dur(650), (e) => { shown = from + (roll - from) * e; marker = shown; });
      await timer.done;
      shown = roll; marker = roll;
      result = r; offer = r.double;
      recent = [{ roll, win: r.payout > 0 }, ...recent].slice(0, 10);
      if (r.payout > 0) sfx.win(r.multiplier >= 10 ? 3 : r.multiplier >= 3 ? 2 : 1); else sfx.lose();
      onbalance(r.balance);
      onplayed();
    } catch (e) {
      toast(errorText(e), 'error');
    } finally { busy = false; }
  }
  onMount(() => () => timer?.cancel());
</script>

<GameShell title="Dice" icon={Dices}>
  {#snippet controls()}
    <div class="seg2" role="radiogroup" aria-label="Roll under or over">
      <button class:on={!over} role="radio" aria-checked={!over} onclick={() => (over = false)} disabled={busy}>Roll under</button>
      <button class:on={over} role="radio" aria-checked={over} onclick={() => (over = true)} disabled={busy}>Roll over</button>
    </div>
    <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} disabled={busy} />
    <div class="chance">
      <div class="top"><span>Win chance</span><label><input type="number" min={cfg.min_chance} max={cfg.max_chance} step="0.01" bind:value={chance} onchange={() => (chance = clampChance(Number(chance) || cfg.min_chance))} disabled={busy} aria-label="Win chance in percent" />%</label></div>
      <input class="fancy" type="range" min="0" max="100" step="0.1" value={sliderPos} oninput={(e) => (chance = fromSlider(Number(e.currentTarget.value)))} disabled={busy} style="--p:{sliderPos}%" aria-label="Win chance" />
      <div class="stats"><div><span>Pays</span><b>{mult(pays)}</b></div><div><span>{over ? 'Roll over' : 'Roll under'}</span><b>{target.toFixed(2)}</b></div><div><span>Win</span><b class="g">+{money(bet * pays - bet)}</b></div></div>
    </div>
    <p class="note"><Info size={13} /> You choose the odds: the smaller the chance, the bigger the payout. Pays back {(st.rtp.dice * 100).toFixed(0)}% over time{#if st.config.chaos.enabled}, plus the odd surge or curse on wins{/if}.</p>
    {#if recent.length}<div class="chips" aria-label="Recent rolls">{#each recent as r}<span class={r.win ? 'w' : 'l'}>{r.roll.toFixed(2)}</span>{/each}</div>{/if}
  {/snippet}
  {#snippet action()}
    <button class="big go" onclick={roll} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>{busy ? 'Rolling…' : `Roll · ${money(bet)}`}<kbd class="k">Space</kbd></button>
    {#if !cfg.enabled}<p class="note">Dice is closed right now.</p>{/if}
  {/snippet}
  {#snippet stage()}
    <div class="table">
      <div class="num" class:win={result && result.payout > 0} class:lose={result && result.payout === 0} class:rolling={busy} aria-live="polite">
        {shown == null ? '--.--' : shown.toFixed(2)}
        {#if result}{#key result}{#if result.payout > 0 && (result.multiplier >= 5 || twist?.kind === 'surge')}<Burst />{/if}{/key}{/if}
      </div>
      <div class="track" role="img" aria-label="Win zone {over ? 'above' : 'below'} {target.toFixed(2)}">
        <div class="zone win" class:over style:left={over ? `${target}%` : '0'} style:width="{chance}%"></div>
        {#if marker != null}<div class="pin" class:hit={result && result.payout > 0} class:miss={result && result.payout === 0} style:left="{marker}%"><i></i><b>{marker.toFixed(2)}</b></div>{/if}
        <div class="ticks"><span>0</span><span>25</span><span>50</span><span>75</span><span>100</span></div>
      </div>
      <div class="legend"><span class="w"><i></i> Win</span><span class="l"><i></i> Lose</span></div>
      {#if result}{#key result}
        <div class="banner" class:win={result.payout > 0} class:lose={result.payout === 0}>
          {#if result.payout > 0}<b>{mult(result.multiplier)}</b><span>+{money(result.payout)}</span>{:else}<b>Missed</b><span>Better luck next roll</span>{/if}
          {#if twist}<i class="twist {twist.kind}">{twist.kind === 'surge' ? `⚡ Lucky surge ${twist.x}×` : '☠ Cursed win ½×'}</i>{/if}
        </div>
      {/key}{/if}
    </div>
    {#if offer}{#key offer.id}<DoubleOrNothing {st} {offer} {onbalance} {onplayed} />{/key}{/if}
  {/snippet}
</GameShell>

<style>
  .go { background: linear-gradient(135deg, #38bdf8, #6366f1); box-shadow: 0 6px 22px -6px #38bdf8; }
  .chance { display: flex; flex-direction: column; gap: 0.5rem; }
  .chance .top { display: flex; justify-content: space-between; align-items: center; font-size: 0.82rem; font-weight: 600; }
  .chance label { display: inline-flex; align-items: center; gap: 0.25rem; color: var(--muted); font-weight: 700; }
  .chance label input { width: 5.4rem; text-align: right; font-weight: 800; font-variant-numeric: tabular-nums; padding: 0.3rem 0.5rem; }
  input.fancy { height: 1.2rem; background: linear-gradient(90deg, var(--accent) var(--p), color-mix(in srgb, var(--text) 12%, transparent) var(--p)) center / 100% 0.3rem no-repeat; }
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.4rem; }
  .stats div { background: color-mix(in srgb, var(--text) 5%, transparent); border-radius: var(--radius-sm); padding: 0.4rem 0.6rem; display: flex; flex-direction: column; min-width: 0; }
  .stats span { font-size: 0.64rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; } .stats b { font-variant-numeric: tabular-nums; font-size: 0.92rem; overflow: hidden; text-overflow: ellipsis; } .g { color: #4ade80; }
  .table { position: relative; display: flex; flex-direction: column; gap: 1.6rem; align-items: stretch; padding: 2rem 1.6rem 4.5rem; border-radius: var(--radius); background: radial-gradient(100% 80% at 50% 0%, #162a4a, #090c14 75%); border: 1px solid var(--line); margin-bottom: 1rem; }
  .num { position: relative; align-self: center; font-size: clamp(3.4rem, 14vw, 6rem); font-weight: 900; font-variant-numeric: tabular-nums; letter-spacing: -0.02em; line-height: 1; transition: color 0.2s, text-shadow 0.2s; text-shadow: 0 0 30px #38bdf866; }
  .num.rolling { color: #93c5fd; } .num.win { color: #4ade80; text-shadow: 0 0 36px #22c55e99; } .num.lose { color: #f87171; text-shadow: 0 0 36px #ef444488; }
  .track { position: relative; height: 1.5rem; border-radius: 99rem; background: linear-gradient(90deg, #7f1d1d, #b91c1c); box-shadow: inset 0 2px 8px #000a; margin: 1.6rem 0 1.3rem; }
  .zone { position: absolute; top: 0; bottom: 0; transition: left 0.15s, width 0.15s; } .zone.win { background: linear-gradient(90deg, #16a34a, #4ade80); box-shadow: 0 0 22px #22c55e66; z-index: 1; border-radius: 99rem; }
  .pin { position: absolute; top: -1.3rem; transform: translateX(-50%); display: flex; flex-direction: column; align-items: center; z-index: 3; transition: left 0.05s linear; }
  .pin i { width: 0; height: 0; border-left: 0.55rem solid transparent; border-right: 0.55rem solid transparent; border-top: 0.8rem solid #f5d97a; filter: drop-shadow(0 2px 4px #000a); order: 2; }
  .pin b { font-size: 0.72rem; background: #f5d97a; color: #1a1205; padding: 0.05rem 0.4rem; border-radius: 99rem; order: 1; font-variant-numeric: tabular-nums; }
  .pin.hit b { background: #4ade80; } .pin.miss b { background: #f87171; }
  .ticks { position: absolute; left: 0; right: 0; top: 1.8rem; display: flex; justify-content: space-between; font-size: 0.68rem; color: var(--muted); }
  .legend { display: flex; justify-content: center; gap: 1rem; font-size: 0.74rem; color: var(--muted); } .legend i { display: inline-block; width: 0.6rem; height: 0.6rem; border-radius: 50%; margin-right: 0.3rem; } .legend .w i { background: #22c55e; } .legend .l i { background: #b91c1c; }
  .banner { position: absolute; left: 50%; bottom: -1.5rem; transform: translateX(-50%); padding: 0.55rem 1.4rem; border-radius: 99rem; display: flex; gap: 0.7rem; align-items: center; width: max-content; white-space: nowrap; animation: pop 0.4s var(--ease); box-shadow: 0 10px 34px -8px #000; max-width: 94vw; flex-wrap: wrap; justify-content: center; }
  .banner.win { background: linear-gradient(135deg, #16a34a, #15803d); box-shadow: 0 8px 30px -6px #22c55e; } .banner.lose { background: color-mix(in srgb, var(--surface) 90%, #fff 4%); border: 1px solid var(--line-strong); color: var(--muted); }
  .banner b { font-size: 1.3rem; } .banner span { font-weight: 700; }
  @keyframes pop { from { transform: translateX(-50%) scale(0.6); opacity: 0; } }
  @media (max-width: 520px) { .table { padding: 1.4rem 1rem 4.2rem; } }
</style>
