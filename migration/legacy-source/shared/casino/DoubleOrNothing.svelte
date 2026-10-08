<script lang="ts">
  // The gamble offered after a win: risk the winnings for a chance to double them. Taking it flips a coin on the server; a win
  // can be doubled again, up to the streak limit. Declining costs nothing, the winnings are already in the balance.
  import { onMount, untrack } from 'svelte';
  import { Coins, Flame, X } from '@lucide/svelte';
  import Burst from './Burst.svelte';
  import { cpost, money, untilText, type CasinoState, type DoubleOffer } from './casino';
  import { toast, errorText } from './host';
  import { dur, wait } from './anim';
  import { sfx } from './sfx';

  let { st, offer, onbalance, onplayed }: { st: CasinoState; offer: DoubleOffer; onbalance: (v: number) => void; onplayed: () => void } = $props();
  let cur = $state<DoubleOffer | null>(untrack(() => offer));
  let busy = $state(false);
  let spin = $state(0);
  let verdict = $state<'won' | 'lost' | null>(null);
  let lastLost = $state(0);
  let lastWon = $state(0);
  let hidden = $state(false);
  let now = $state(Date.now());
  let coin = $state<HTMLDivElement | null>(null);

  onMount(() => { const t = setInterval(() => (now = Date.now()), 1000); return () => clearInterval(t); });
  const left = $derived(cur ? Math.max(0, new Date(cur.expires_at).getTime() - now) : 0);
  const expired = $derived(cur != null && left <= 0);

  async function take() {
    if (!cur || busy || expired) return;
    busy = true; verdict = null;
    const offerNow = cur;
    try {
      const r = await cpost<{ won: boolean; stake: number; payout: number; balance: number; double: DoubleOffer | null }>(st.server.id, '/double', { id: offerNow.id });
      onbalance(r.balance - r.payout + offerNow.stake);
      // The coin tumbles for a moment, then settles on the answer the server already gave.
      spin += 1440 + (r.won ? 0 : 180);
      coin?.animate([{ transform: `rotateY(${spin - 1440 - (r.won ? 0 : 180)}deg) translateY(0)` }, { transform: `rotateY(${spin - 720}deg) translateY(-70px)`, offset: 0.5 }, { transform: `rotateY(${spin}deg) translateY(0)` }], { duration: dur(1400), easing: 'cubic-bezier(0.3, 0.7, 0.3, 1)', fill: 'forwards' });
      for (let i = 0; i < 6; i++) { sfx.tick(1 + i * 0.1); await wait(dur(200)); }
      await wait(dur(120));
      verdict = r.won ? 'won' : 'lost';
      if (r.won) { sfx.win(2); lastWon = r.payout; cur = r.double; } else { sfx.boom(); lastLost = r.stake; cur = null; }
      onbalance(r.balance);
      onplayed();
    } catch (e) {
      toast(errorText(e), 'error');
      cur = null;
    } finally { busy = false; }
  }
</script>

{#if !hidden && (cur || verdict)}
  <div class="don" class:won={verdict === 'won'} class:lost={verdict === 'lost'} role="group" aria-label="Double or Nothing">
    <div class="coinwrap" aria-hidden="true">
      <div class="coin" bind:this={coin}><span class="face f">2×</span><span class="face b">0</span></div>
      {#key verdict}{#if verdict === 'won'}<Burst count={22} />{/if}{/key}
    </div>
    <div class="txt">
      {#if verdict === 'lost' && !cur}
        <b>Gone.</b><span>Double or Nothing took {money(lastLost)}. The house thanks you.</span>
      {:else if verdict === 'won' && !cur}
        <b>Doubled to {money(lastWon)}!</b><span>That's the end of the streak. Cash it in, the money is yours.</span>
      {:else if cur}
        <b>{verdict === 'won' ? `Doubled to ${money(lastWon)}! Again?` : 'Double or Nothing?'}</b>
        <span>Risk <em>{money(cur.stake)}</em> to win <em class="g">{money(cur.payout)}</em> · {(cur.win_chance * 100).toFixed(0)}% chance{#if cur.streak > 0} · streak {cur.streak}/{cur.max_streak}{/if}</span>
        <small>{expired ? 'Offer expired' : `Offer closes in ${untilText(cur.expires_at, now)}`}</small>
      {/if}
    </div>
    {#if cur}
      <div class="btns">
        <button class="go" onclick={take} disabled={busy || expired}><Flame size={16} /> {busy ? 'Flipping…' : 'Double'}</button>
        <button class="no ghost" onclick={() => { cur = null; hidden = true; }} disabled={busy} aria-label="Keep the winnings"><Coins size={15} /> Keep it</button>
      </div>
    {:else}
      <button class="x ghost icon" onclick={() => (hidden = true)} aria-label="Dismiss"><X size={15} /></button>
    {/if}
  </div>
{/if}

<style>
  .don { position: relative; display: flex; align-items: center; gap: 0.9rem; padding: 0.8rem 1rem; border-radius: var(--radius); background: linear-gradient(135deg, #3b1d6a55, #7c2d1255), var(--panel-solid, var(--surface)); border: 1px solid #f59e0b66; box-shadow: 0 10px 34px -14px #f59e0b; flex-wrap: wrap; animation: slide 0.4s cubic-bezier(0.2, 1.2, 0.4, 1); }
  .don.won { border-color: #22c55e88; box-shadow: 0 10px 34px -14px #22c55e; }
  .don.lost { border-color: #ef444466; box-shadow: 0 10px 34px -14px #ef4444; }
  @keyframes slide { from { transform: translateY(12px) scale(0.96); opacity: 0; } }
  .coinwrap { position: relative; width: 3.4rem; height: 3.4rem; flex: none; perspective: 500px; }
  .coin { position: absolute; inset: 0; transform-style: preserve-3d; }
  .face { position: absolute; inset: 0; display: grid; place-items: center; border-radius: 50%; font-weight: 900; font-size: 1.05rem; backface-visibility: hidden; border: 3px solid #b8892b; box-shadow: inset 0 0 0 3px #fff3b055, 0 4px 12px #0007; }
  .f { background: radial-gradient(circle at 35% 30%, #fff5c9, #f5b942 60%, #b8892b); color: #6b4c10; }
  .b { background: radial-gradient(circle at 35% 30%, #cbd5e1, #64748b 60%, #334155); color: #1e293b; transform: rotateY(180deg); }
  .txt { flex: 1; min-width: 10rem; display: flex; flex-direction: column; gap: 0.1rem; }
  .txt b { font-size: 1.02rem; } .txt span { font-size: 0.85rem; color: color-mix(in srgb, var(--text) 80%, transparent); } .txt small { color: var(--muted); font-size: 0.72rem; }
  em { font-style: normal; font-weight: 800; color: var(--text); } em.g { color: #4ade80; }
  .btns { display: flex; gap: 0.5rem; margin-left: auto; }
  .go { display: inline-flex; gap: 0.4rem; align-items: center; padding: 0.65rem 1.2rem; border: none; border-radius: 99rem; font-weight: 800; color: #fff; background: linear-gradient(135deg, #f59e0b, #ef4444); box-shadow: 0 6px 22px -6px #f97316; touch-action: manipulation; }
  .go:hover:not(:disabled) { filter: brightness(1.12); transform: translateY(-1px); }
  .no { display: inline-flex; gap: 0.35rem; align-items: center; padding: 0.65rem 0.9rem; }
  .x { margin-left: auto; padding: 0.3rem; }
  @media (max-width: 520px) { .btns { width: 100%; } .btns button { flex: 1; justify-content: center; } }
</style>
