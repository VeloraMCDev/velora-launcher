<script lang="ts">
  import { untrack } from 'svelte';
  import { Spade, Info } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import Burst from './Burst.svelte';
  import GameShell from './GameShell.svelte';
  import DoubleOrNothing from './DoubleOrNothing.svelte';
  import { card, cpost, money, type BlackjackGame, type CasinoState, type DoubleOffer } from './casino';
  import { toast, errorText } from './host';
  import { dur, wait } from './anim';
  import { sfx } from './sfx';

  type Reply = { game: BlackjackGame; balance: number; payout: number; double: DoubleOffer | null };

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.blackjack);
  let bet = $state(untrack(() => st.config.blackjack.min_bet));
  hotkeys(() => {
    const deal = () => { if (!active && !busy && cfg.enabled && !(balance != null && balance < bet)) void act('/blackjack/start', { bet }); };
    return {
      space: deal, enter: deal,
      h: () => { if (active && !busy) void act('/blackjack/hit'); },
      s: () => { if (active && !busy) void act('/blackjack/stand'); },
      d: () => { if (active && !busy && game?.can_double && !(balance != null && balance < game.bet) && game.bet * 2 <= cfg.max_bet) void act('/blackjack/double'); },
    };
  });
  let game = $state<BlackjackGame | null>(untrack(() => st.blackjack));
  let busy = $state(false);
  let offer = $state<DoubleOffer | null>(null);
  let payout = $state(0);
  // Cards deal one at a time: `dealt` counts how many of each hand are on the table so far.
  let shownPlayer = $state(untrack(() => st.blackjack?.player.length ?? 0));
  let shownDealer = $state(untrack(() => st.blackjack?.dealer.length ?? 0));
  const active = $derived(game?.status === 'active');
  const over = $derived(game != null && game.status === 'done');
  const outcome = $derived(game?.outcome);

  async function reveal(next: BlackjackGame) {
    // Show new cards with a short beat between them, so each deal is something to watch.
    const prevP = shownPlayer;
    game = next;
    for (let i = prevP; i < next.player.length; i++) { shownPlayer = i + 1; sfx.drop(); await wait(dur(240)); }
    shownPlayer = next.player.length;
    for (let i = shownDealer; i < next.dealer.length; i++) { shownDealer = i + 1; sfx.drop(); await wait(dur(300)); }
    shownDealer = next.dealer.length;
  }

  async function act(path: string, body: Record<string, unknown> = {}) {
    if (busy) return;
    busy = true;
    try {
      const fresh = path === '/blackjack/start';
      if (fresh) { game = null; offer = null; payout = 0; shownPlayer = 0; shownDealer = 0; }
      const r = await cpost<Reply>(st.server.id, path, body);
      onbalance(fresh ? r.balance : r.balance - r.payout);
      await reveal(r.game);
      if (r.game.status === 'done') {
        payout = r.payout; offer = r.double;
        if (r.payout > r.game.bet) sfx.win(r.game.outcome === 'blackjack' ? 3 : 2); else if (r.payout === 0) sfx.lose();
        onbalance(r.balance); onplayed();
      }
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }

  const verdict: Record<string, string> = { blackjack: 'Blackjack!', win: 'You win', push: 'Push', lose: 'Dealer wins', bust: 'Bust' };
  const hint = $derived.by(() => {
    if (!game || game.status !== 'active') return '';
    const d = game.dealer[0] == null ? 0 : (game.dealer[0] % 13) + 1;
    return d >= 7 || d === 1 ? 'The dealer is showing strength.' : 'The dealer is showing a weak card.';
  });
</script>

<GameShell title="Blackjack" icon={Spade}>
  {#snippet controls()}
    <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} disabled={active || busy} />
    <p class="note"><Info size={13} /> Beat the dealer to 21 without going over. Blackjack pays {cfg.blackjack_pay + 1 === 2.5 ? '3 to 2' : `${cfg.blackjack_pay} to 1`}, the dealer stands on {cfg.dealer_hits_soft_17 ? 'hard 17 and hits soft 17' : 'all 17s'}, and you may double down on your first two cards. Cards come from an endless shoe, so nothing can be counted.</p>
    {#if active}<p class="note">{hint}</p>{/if}
  {/snippet}
  {#snippet action()}
    {#if active && game}
      <div class="row">
        <button class="big hit" onclick={() => act('/blackjack/hit')} disabled={busy}>Hit<kbd class="k">H</kbd></button>
        <button class="big stand" onclick={() => act('/blackjack/stand')} disabled={busy}>Stand<kbd class="k">S</kbd></button>
      </div>
      <button class="big dbl" onclick={() => act('/blackjack/double')} disabled={busy || !game.can_double || (balance != null && balance < game.bet) || game.bet * 2 > cfg.max_bet}>Double down · {money(game.bet)} more<kbd class="k">D</kbd></button>
    {:else}
      <button class="big go" onclick={() => act('/blackjack/start', { bet })} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>{busy ? 'Dealing…' : over ? `Deal again · ${money(bet)}` : `Deal · ${money(bet)}`}<kbd class="k">Space</kbd></button>
    {/if}
    {#if !cfg.enabled}<p class="note">Blackjack is closed right now.</p>{/if}
  {/snippet}
  {#snippet stage()}
    <div class="felt" class:win={over && (outcome === 'win' || outcome === 'blackjack')} class:lose={over && (outcome === 'lose' || outcome === 'bust')}>
      <div class="hand dealer">
        <div class="who"><span>Dealer</span>{#if game && shownDealer}<b class="tot">{game.status === 'done' ? game.dealer_total : game.dealer_total + (game.hidden ? ' + ?' : '')}</b>{/if}</div>
        <div class="cards">
          {#if !game}<div class="slot"></div><div class="slot"></div>{/if}
          {#each game?.dealer.slice(0, shownDealer) ?? [] as c, i (i)}{@render face(c)}{/each}
          {#if game && game.hidden && shownDealer >= 1}<div class="card back" aria-label="Hidden card"></div>{/if}
        </div>
      </div>
      <div class="mid">
        {#if over && outcome}
          {#key game}
            <div class="verdict {outcome}"><b>{verdict[outcome]}</b>{#if payout > 0}<span>{payout > (game?.bet ?? 0) ? '+' : ''}{money(payout - (game?.bet ?? 0))}</span>{:else}<span>-{money(game?.bet ?? 0)}</span>{/if}</div>
            {#if outcome === 'blackjack' || (outcome === 'win' && payout >= (game?.bet ?? 0) * 2)}<Burst count={outcome === 'blackjack' ? 40 : 24} />{/if}
          {/key}
        {:else if active && game}
          <div class="pot">Bet <b>{money(game.bet)}</b>{#if game.doubled} · doubled{/if}</div>
        {:else}
          <div class="pot idle">Place a bet and deal</div>
        {/if}
      </div>
      <div class="hand">
        <div class="cards">
          {#if !game}<div class="slot"></div><div class="slot"></div>{/if}
          {#each game?.player.slice(0, shownPlayer) ?? [] as c, i (i)}{@render face(c)}{/each}
        </div>
        <div class="who"><span>You</span>{#if game && shownPlayer}<b class="tot" class:bust={game.player_total > 21}>{game.player_total}{#if game.soft && game.player_total <= 21 && game.status === 'active'}<small> soft</small>{/if}</b>{/if}</div>
      </div>
    </div>
    {#if offer}{#key offer.id}<DoubleOrNothing {st} {offer} {onbalance} {onplayed} />{/key}{/if}
  {/snippet}
</GameShell>

{#snippet face(id: number)}
  {@const c = card(id)}
  <div class="card" class:red={c.red} aria-label="{c.rank} of {c.suit}">
    <span class="r">{c.rank}<i>{c.suit}</i></span>
    <span class="s">{c.suit}</span>
    <span class="r b">{c.rank}<i>{c.suit}</i></span>
  </div>
{/snippet}

<style>
  .go { background: linear-gradient(135deg, #22c55e, #15803d); box-shadow: 0 6px 22px -6px #22c55e; }
  .hit { background: linear-gradient(135deg, #38bdf8, #2563eb); } .stand { background: linear-gradient(135deg, #f59e0b, #d97706); color: #1a1205 !important; } .dbl { background: linear-gradient(135deg, #a855f7, #7c3aed); padding: 0.7rem !important; font-size: 0.92rem !important; }
  .felt { position: relative; display: flex; flex-direction: column; justify-content: space-between; gap: 0.8rem; min-height: 27rem; padding: 1.2rem 1rem; border-radius: 1.4rem; background: radial-gradient(120% 100% at 50% 40%, #14633a, #0a3a22 60%, #06261a); border: 2px solid #8a6a1f; box-shadow: inset 0 0 70px #000a, 0 20px 50px -24px #000; margin-bottom: 1rem; transition: box-shadow 0.3s; }
  .felt.win { box-shadow: inset 0 0 70px #000a, 0 0 50px -6px #22c55e; } .felt.lose { box-shadow: inset 0 0 70px #000a, 0 0 50px -10px #ef4444; }
  .who { display: flex; align-items: center; gap: 0.6rem; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.1em; color: #ffffffa8; font-weight: 700; }
  .tot { background: #000a; color: #fff; padding: 0.12rem 0.65rem; border-radius: 99rem; font-size: 0.95rem; letter-spacing: 0; font-variant-numeric: tabular-nums; } .tot.bust { background: #b91c1c; } .tot small { font-weight: 500; opacity: 0.7; }
  .hand { display: flex; flex-direction: column; gap: 0.5rem; align-items: center; } .hand .who { justify-content: center; }
  .cards { display: flex; justify-content: center; min-height: 6.6rem; padding-left: 2rem; }
  .card, .slot { width: clamp(3.9rem, 17vw, 5rem); aspect-ratio: 5 / 7; margin-left: -2rem; border-radius: 0.55rem; flex: none; }
  .slot { border: 2px dashed #ffffff30; margin-left: 0.4rem; }
  .card { position: relative; background: linear-gradient(160deg, #fff, #e8e8ee); color: #111827; box-shadow: 0 6px 14px #0009, inset 0 0 0 1px #0002; animation: deal 0.38s cubic-bezier(0.2, 1.1, 0.4, 1); display: grid; place-items: center; }
  .card.red { color: #dc2626; } .card .s { font-size: clamp(1.8rem, 8vw, 2.4rem); line-height: 1; }
  .card .r { position: absolute; top: 0.25rem; left: 0.35rem; display: flex; flex-direction: column; align-items: center; font-weight: 800; font-size: clamp(0.85rem, 3.4vw, 1.05rem); line-height: 1; } .card .r i { font-style: normal; font-size: 0.8em; } .card .r.b { top: auto; left: auto; bottom: 0.25rem; right: 0.35rem; transform: rotate(180deg); }
  .card.back { background: repeating-linear-gradient(45deg, #1e3a8a, #1e3a8a 6px, #1d4ed8 6px, #1d4ed8 12px); border: 3px solid #fff; animation: none; }
  @keyframes deal { from { transform: translate(40px, -60px) rotate(14deg) scale(0.7); opacity: 0; } }
  .mid { position: relative; min-height: 4.2rem; display: grid; place-items: center; }
  .pot { color: #ffffffbb; font-size: 0.9rem; background: #0006; padding: 0.35rem 1rem; border-radius: 99rem; } .pot b { color: #f5d97a; } .pot.idle { opacity: 0.7; }
  .verdict { display: flex; flex-direction: column; align-items: center; padding: 0.5rem 1.6rem; border-radius: 1rem; animation: pop 0.4s var(--ease); box-shadow: 0 10px 34px -8px #000; background: #000b; border: 1px solid #ffffff22; }
  .verdict b { font-size: 1.5rem; } .verdict span { font-weight: 800; }
  .verdict.blackjack, .verdict.win { background: linear-gradient(135deg, #16a34a, #15803d); } .verdict.blackjack { background: linear-gradient(135deg, #f59e0b, #ec4899); }
  .verdict.push { background: #334155; } .verdict.lose, .verdict.bust { background: linear-gradient(135deg, #7f1d1d, #b91c1c); }
  @keyframes pop { from { transform: scale(0.6); opacity: 0; } }
  @media (max-width: 520px) { .felt { min-height: 24rem; padding: 1rem 0.6rem; } }
</style>
