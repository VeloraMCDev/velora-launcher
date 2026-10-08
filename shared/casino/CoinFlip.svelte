<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Coins, Info } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import Burst from './Burst.svelte';
  import GameShell from './GameShell.svelte';
  import DoubleOrNothing from './DoubleOrNothing.svelte';
  import { cpost, money, mult, type CasinoState, type DoubleOffer, type Round, type Twist } from './casino';
  import { toast, errorText } from './host';
  import { dur } from './anim';
  import { sfx } from './sfx';

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.coinflip);
  let bet = $state(untrack(() => st.config.coinflip.min_bet));
  hotkeys(() => { const go = () => { if (!busy && cfg.enabled && !(balance != null && balance < bet)) void flip(); }; return { space: go, enter: go, h: () => { if (!busy) side = 'heads'; }, t: () => { if (!busy) side = 'tails'; } }; });
  let side = $state<'heads' | 'tails'>('heads');
  let busy = $state(false);
  let turns = $state(0); // total degrees the coin has turned, so every flip continues from where it stopped
  let result = $state<Round | null>(null);
  let offer = $state<DoubleOffer | null>(null);
  let recent = $state<{ win: boolean; landed: string }[]>([]);
  let coin: HTMLDivElement;
  let anim: Animation | null = null;
  const twist = $derived((result?.result?.twist ?? null) as Twist | null);

  async function flip() {
    if (busy) return;
    busy = true; result = null; offer = null;
    try {
      const r = await cpost<Round & { double: DoubleOffer | null }>(st.server.id, '/coinflip', { bet, side });
      onbalance(r.balance - r.payout);
      const landed = r.result.landed as 'heads' | 'tails';
      // Whole turns for drama (random, so no two flips look alike), ending on the right face.
      const from = turns;
      const base = Math.ceil(from / 360) * 360;
      const to = base + (3 + Math.floor(Math.random() * 4)) * 360 + (landed === 'heads' ? 0 : 180);
      turns = to;
      anim = coin.animate(
        [{ transform: `translateY(0) rotateX(${from}deg)` }, { transform: `translateY(-${90 + Math.random() * 40}px) rotateX(${(from + to) / 2}deg)`, offset: 0.5 }, { transform: `translateY(0) rotateX(${to}deg)` }],
        { duration: dur(1500 + Math.random() * 500), easing: 'cubic-bezier(0.25, 0.6, 0.35, 1)', fill: 'forwards' },
      );
      const ticker = setInterval(() => sfx.tick(1.4), 120);
      await anim.finished.catch(() => {});
      clearInterval(ticker);
      result = r; offer = r.double;
      recent = [{ win: r.payout > 0, landed }, ...recent].slice(0, 12);
      if (r.payout > 0) sfx.win(r.multiplier >= 3 ? 3 : 1); else sfx.lose();
      onbalance(r.balance);
      onplayed();
    } catch (e) {
      toast(errorText(e), 'error');
    } finally { busy = false; }
  }
  onMount(() => () => anim?.cancel());
</script>

<GameShell title="Coin Flip" icon={Coins}>
  {#snippet controls()}
    <div class="seg2" role="radiogroup" aria-label="Your call">
      <button class:on={side === 'heads'} role="radio" aria-checked={side === 'heads'} onclick={() => (side = 'heads')} disabled={busy}>🪙 Heads</button>
      <button class:on={side === 'tails'} role="radio" aria-checked={side === 'tails'} onclick={() => (side = 'tails')} disabled={busy}>🌑 Tails</button>
    </div>
    <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} disabled={busy} />
    <p class="note"><Info size={13} /> Call it right and win {mult(cfg.payout)} your bet ({money(bet * cfg.payout)}). A pure 50/50 toss{#if st.config.chaos.enabled}, with the odd lucky surge or curse on a win{/if}.</p>
    {#if recent.length}<div class="chips" aria-label="Recent flips">{#each recent as r}<span class={r.win ? 'w' : 'l'}>{r.landed === 'heads' ? 'H' : 'T'}</span>{/each}</div>{/if}
  {/snippet}
  {#snippet action()}
    <button class="big go" onclick={flip} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>{busy ? 'Flipping…' : `Flip · ${money(bet)}`}<kbd class="k">Space</kbd></button>
    {#if !cfg.enabled}<p class="note">Coin Flip is closed right now.</p>{/if}
  {/snippet}
  {#snippet stage()}
    <div class="arena">
      <div class="shadow" aria-hidden="true"></div>
      <div class="holder" aria-live="polite">
        <div class="coin" bind:this={coin} role="img" aria-label={result ? `The coin landed on ${result.result.landed}` : 'A coin'}>
          <span class="face h"><b>H</b></span>
          <span class="face t"><b>T</b></span>
        </div>
        {#if result}{#key result}
          <div class="banner" class:win={result.payout > 0} class:lose={result.payout === 0}>
            {#if result.payout > 0}<b>{result.result.landed === 'heads' ? 'Heads' : 'Tails'}!</b><span>+{money(result.payout)}</span>{:else}<b>{result.result.landed === 'heads' ? 'Heads' : 'Tails'}</b><span>Not this time</span>{/if}
            {#if twist}<i class="twist {twist.kind}">{twist.kind === 'surge' ? `⚡ Lucky surge ${twist.x}×` : '☠ Cursed win ½×'}</i>{/if}
            {#if result.payout > 0 && (result.multiplier >= 3 || twist?.kind === 'surge')}<Burst />{/if}
          </div>
        {/key}{/if}
      </div>
    </div>
    {#if offer}{#key offer.id}<DoubleOrNothing {st} {offer} {onbalance} {onplayed} />{/key}{/if}
  {/snippet}
</GameShell>

<style>
  .go { background: linear-gradient(135deg, #f5b942, #d97706); color: #1a1205 !important; box-shadow: 0 6px 22px -6px #f59e0b; }
  .arena { position: relative; display: grid; place-items: center; min-height: 22rem; padding: 3rem 1rem 4.5rem; border-radius: var(--radius); background: radial-gradient(90% 70% at 50% 35%, #2a1d07, #0c0a06 75%); border: 1px solid var(--line); overflow: hidden; margin-bottom: 1rem; }
  .holder { position: relative; width: 11rem; height: 11rem; perspective: 900px; }
  .coin { position: absolute; inset: 0; transform-style: preserve-3d; will-change: transform; }
  .face { position: absolute; inset: 0; display: grid; place-items: center; border-radius: 50%; backface-visibility: hidden; border: 0.55rem solid #b8892b; box-shadow: inset 0 0 0 0.3rem #fff3b044, inset 0 0 2.2rem #0005, 0 0 40px -8px #f59e0b88; }
  .face b { font-size: 4.6rem; font-weight: 900; text-shadow: 0 2px 0 #fff6, 0 -2px 0 #0005; }
  .h { background: radial-gradient(circle at 32% 28%, #fff7d1, #f5c542 55%, #b8892b); color: #7a5410; }
  .t { background: radial-gradient(circle at 32% 28%, #f1f5f9, #94a3b8 55%, #475569); color: #1e293b; transform: rotateX(180deg); border-color: #64748b; }
  .shadow { position: absolute; bottom: 2.4rem; width: 8rem; height: 1.1rem; border-radius: 50%; background: radial-gradient(#000a, transparent 70%); }
  .banner { position: absolute; left: 50%; bottom: -3.4rem; transform: translateX(-50%); padding: 0.55rem 1.4rem; border-radius: 99rem; display: flex; gap: 0.7rem; align-items: center; width: max-content; white-space: nowrap; animation: pop 0.4s var(--ease); box-shadow: 0 10px 34px -8px #000; max-width: 94vw; flex-wrap: wrap; justify-content: center; }
  .banner.win { background: linear-gradient(135deg, #16a34a, #15803d); box-shadow: 0 8px 30px -6px #22c55e; }
  .banner.lose { background: color-mix(in srgb, var(--surface) 90%, #fff 4%); border: 1px solid var(--line-strong); color: var(--muted); }
  .banner b { font-size: 1.3rem; } .banner span { font-weight: 700; }
  @keyframes pop { from { transform: translateX(-50%) scale(0.6); opacity: 0; } }
  @media (max-width: 520px) { .holder { width: 9rem; height: 9rem; } .face b { font-size: 3.6rem; } .arena { min-height: 19rem; } }
</style>
