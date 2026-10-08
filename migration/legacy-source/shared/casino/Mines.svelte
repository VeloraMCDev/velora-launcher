<script lang="ts">
  import { untrack } from 'svelte';
  import { Bomb, Gem, Info, Hand } from '@lucide/svelte';
  import BetInput from './BetInput.svelte';
  import { hotkeys } from './hotkeys';
  import Burst from './Burst.svelte';
  import { cpost, mult, money, sleep, type CasinoState, type MinesGame } from './casino';
  import { toast, errorText } from './host';
  import { sfx } from './sfx';

  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  const cfg = $derived(st.config.mines);
  let bet = $state(untrack(() => st.config.mines.min_bet));
  hotkeys(() => { const go = () => { if (!active && !busy && cfg.enabled && !(balance != null && balance < bet)) void start(); }; return { space: go, enter: go, c: () => { if (active && game && game.revealed.length > 0 && !busy) void cashout(); } }; });
  let mines = $state(untrack(() => Math.min(Math.max(3, st.config.mines.min_mines), st.config.mines.max_mines)));
  let game = $state<MinesGame | null>(untrack(() => st.mines));
  let busy = $state(false);
  let flipping = $state<number | null>(null);
  let won = $state<number | null>(null);
  const size = $derived(game?.size ?? cfg.size);
  const cells = $derived(size * size);
  const active = $derived(game?.status === 'active');
  const over = $derived(game != null && game.status !== 'active');
  const safeLeft = $derived(cells - (game?.mines ?? mines));

  // Same formula as the server, to preview the next payout before the first tile is turned.
  const preview = (m: number, safe: number) => {
    let x = 1 - cfg.house_edge;
    for (let i = 0; i < safe; i++) x *= (cells - i) / (cells - m - i);
    return Math.max(1, Math.min(cfg.max_multiplier, Math.round(x * 100) / 100));
  };

  async function start() {
    if (busy) return;
    busy = true; won = null;
    try {
      const r = await cpost<{ game: MinesGame; balance: number }>(st.server.id, '/mines/start', { bet, mines });
      game = r.game; onbalance(r.balance);
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }
  async function reveal(tile: number) {
    if (busy || !game || game.status !== 'active' || game.revealed.includes(tile)) return;
    busy = true; flipping = tile;
    try {
      const r = await cpost<{ game: MinesGame; balance: number | null; payout?: number }>(st.server.id, '/mines/reveal', { tile });
      await sleep(180);
      game = r.game;
      if (r.game.status === 'lost') sfx.boom(); else if (r.game.status === 'active') sfx.gem();
      if (r.game.status === 'cashed') { won = r.payout ?? r.game.cashout; if (r.balance != null) onbalance(r.balance); onplayed(); }
      if (r.game.status === 'lost') onplayed();
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; flipping = null; }
  }
  async function cashout() {
    if (busy || !game) return;
    busy = true;
    try {
      const r = await cpost<{ game: MinesGame; balance: number; payout: number }>(st.server.id, '/mines/cashout');
      game = r.game; won = r.payout; sfx.win(2); onbalance(r.balance); onplayed();
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }
  const tileState = (i: number) => {
    if (!game) return 'idle';
    if (game.revealed.includes(i)) return 'gem';
    if (game.status !== 'active' && game.layout?.includes(i)) return game.hit === i ? 'boom' : 'mine';
    return 'hidden';
  };
</script>

<div class="game">
  <section class="panel controls">
    <h2><Bomb size={18} /> Mines</h2>
    {#if !active}
      <BetInput bind:value={bet} min={cfg.min_bet} max={cfg.max_bet} {balance} disabled={busy} />
      <label class="opt">
        <span class="lbl">Mines <b>{mines}</b></span>
        <input class="fancy" type="range" min={cfg.min_mines} max={cfg.max_mines} step="1" bind:value={mines} style="--p:{cfg.max_mines > cfg.min_mines ? ((mines - cfg.min_mines) / (cfg.max_mines - cfg.min_mines)) * 100 : 100}%" />
        <small>More mines means each safe tile pays more. First tile pays {mult(preview(mines, 1))}.</small>
      </label>
      <button class="go primary" onclick={start} disabled={busy || !cfg.enabled || (balance != null && balance < bet)}>{over ? 'Play again' : 'Start'} · {money(bet)}<kbd class="k">Space</kbd></button>
      {#if !cfg.enabled}<p class="note">Mines is closed right now.</p>{/if}
    {:else if game}
      <div class="live">
        <div><span>Bet</span><b>{money(game.bet)}</b></div>
        <div><span>Mines</span><b>{game.mines}</b></div>
        <div><span>Safe left</span><b>{safeLeft - game.revealed.length}</b></div>
      </div>
      <div class="big"><span>Cash out now</span><b>{money(game.cashout)}</b><small>{mult(game.multiplier)}</small></div>
      <p class="note">Next safe tile: {mult(game.next_multiplier)} → {money(Math.min(st.config.max_payout, game.bet * game.next_multiplier))}</p>
      <button class="cash primary" onclick={cashout} disabled={busy || game.revealed.length === 0}><Hand size={16} /> {game.revealed.length ? `Cash out ${money(game.cashout)}` : 'Turn over a tile first'}</button>
    {/if}
    {#if over && game}
      <div class="end" class:win={game.status === 'cashed'}>
        {#if game.status === 'cashed'}<b>Cashed out {money(won ?? game.cashout)}</b><small>{mult(game.multiplier)} with {game.revealed.length} gems</small>{:else}<b>Boom! You lost {money(game.bet)}</b><small>You found {game.revealed.length} gem{game.revealed.length === 1 ? '' : 's'} first</small>{/if}
      </div>
    {/if}
    <p class="note"><Info size={13} /> Pays back {(st.rtp.mines * 100).toFixed(1)}% over time. Leave any time; your game waits for you.</p>
  </section>

  <section class="stage">
    <div class="board" class:boom={game?.status === 'lost'} style="--n:{size}">
      {#each Array(cells) as _, i}
        {@const s = tileState(i)}
        <button class="tile {s}" class:flip={flipping === i} disabled={!active || busy || s === 'gem'} onclick={() => reveal(i)} aria-label="Tile {i + 1}">
          <span class="face back"></span>
          <span class="face front">{#if s === 'gem'}<Gem size={30} />{:else if s === 'mine' || s === 'boom'}<Bomb size={30} />{/if}</span>
        </button>
      {/each}
      {#if won != null && over}{#key won}<Burst />{/key}{/if}
    </div>
  </section>
</div>

<style>
  .game { display: grid; grid-template-columns: 19rem 1fr; gap: 1.1rem; align-items: start; }
  /* Phones: show the game first so you can watch it, with the main button pinned above the tab bar. */
  @media (max-width: 860px) {
    .game { grid-template-columns: 1fr; }
    .stage { order: -1; }
    .go, .cash { position: sticky; bottom: calc(var(--pl-tab, 0px) + env(safe-area-inset-bottom, 0px) + 10px); z-index: 4; }
  }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.9rem; }
  h2 { display: flex; gap: 0.5rem; align-items: center; font-size: 1.05rem; }
  .opt { display: flex; flex-direction: column; gap: 0.35rem; } .lbl { font-size: 0.82rem; font-weight: 600; } .lbl b { color: var(--accent-2); margin-left: 0.3rem; }
  .opt small { color: var(--muted); font-size: 0.75rem; }
  input.fancy { height: 1.2rem; background: linear-gradient(90deg, #ef4444 var(--p), color-mix(in srgb, var(--text) 12%, transparent) var(--p)) center / 100% 0.3rem no-repeat; }
  .go, .cash { padding: 0.85rem; font-size: 1rem; font-weight: 700; border-radius: var(--radius); border: none; color: #fff; display: inline-flex; justify-content: center; gap: 0.5rem; align-items: center; }
  .go { background: linear-gradient(135deg, #22c55e, #15803d); box-shadow: 0 6px 22px -6px #22c55e; }
  .cash { background: linear-gradient(135deg, #f59e0b, #d97706); color: #1a1205; box-shadow: 0 6px 22px -6px #f59e0b; }
  .go:hover:not(:disabled), .cash:hover:not(:disabled) { filter: brightness(1.1); transform: translateY(-1px); }
  .note { font-size: 0.78rem; color: var(--muted); display: flex; gap: 0.35rem; align-items: flex-start; }
  .live { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.4rem; }
  .live div { background: color-mix(in srgb, var(--text) 5%, transparent); border-radius: var(--radius-sm); padding: 0.45rem 0.6rem; display: flex; flex-direction: column; }
  .live span { font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; } .live b { font-variant-numeric: tabular-nums; }
  .big { display: flex; flex-direction: column; padding: 0.8rem 1rem; border-radius: var(--radius); background: linear-gradient(135deg, color-mix(in srgb, #22c55e 18%, transparent), transparent); border: 1px solid color-mix(in srgb, #22c55e 40%, transparent); }
  .big span { font-size: 0.75rem; color: var(--muted); } .big b { font-size: 1.7rem; font-variant-numeric: tabular-nums; } .big small { color: #4ade80; font-weight: 700; }
  .end { padding: 0.7rem 1rem; border-radius: var(--radius); background: color-mix(in srgb, #ef4444 16%, transparent); border: 1px solid color-mix(in srgb, #ef4444 45%, transparent); display: flex; flex-direction: column; }
  .end.win { background: color-mix(in srgb, #22c55e 16%, transparent); border-color: color-mix(in srgb, #22c55e 45%, transparent); }
  .end small { color: var(--muted); }
  .stage { display: grid; place-items: center; padding: 0.6rem; }
  .board { position: relative; display: grid; grid-template-columns: repeat(var(--n), 1fr); gap: 0.55rem; padding: 0.9rem; border-radius: 1.2rem; background: radial-gradient(120% 100% at 50% 0%, #17261c, #0a0f0c 70%); border: 1px solid var(--line); width: min(100%, 31rem); aspect-ratio: 1; }
  .board.boom { animation: quake 0.5s ease-out; }
  @keyframes quake { 10%, 90% { transform: translate(-2px, 1px); } 20%, 80% { transform: translate(4px, -2px); } 30%, 50%, 70% { transform: translate(-6px, 2px); } 40%, 60% { transform: translate(6px, -2px); } }
  .tile { position: relative; border: none; padding: 0; background: none; perspective: 600px; border-radius: 0.7rem; cursor: pointer; }
  .tile:disabled { cursor: default; }
  .face { position: absolute; inset: 0; border-radius: 0.7rem; display: grid; place-items: center; backface-visibility: hidden; transition: transform 0.35s var(--ease), box-shadow 0.15s; }
  .back { background: linear-gradient(160deg, #2c3a4a, #1b2530); box-shadow: inset 0 2px 0 #ffffff14, 0 4px 0 #0c1219, 0 6px 12px #0008; }
  .tile.hidden:not(:disabled):hover .back { transform: translateY(-3px); box-shadow: inset 0 2px 0 #ffffff22, 0 7px 0 #0c1219, 0 10px 18px #0009, 0 0 0 2px var(--accent); }
  .front { transform: rotateY(180deg); background: #0b1410; color: #4ade80; box-shadow: inset 0 0 22px #22c55e44; }
  .tile.gem .back, .tile.mine .back, .tile.boom .back { transform: rotateY(180deg); }
  .tile.gem .front, .tile.mine .front, .tile.boom .front { transform: rotateY(0); }
  .tile.gem .front { animation: gem 0.5s var(--ease); }
  .tile.mine .front { color: #f87171; background: #140b0b; box-shadow: inset 0 0 22px #ef444433; opacity: 0.85; }
  .tile.boom .front { color: #fff; background: radial-gradient(circle, #f97316, #b91c1c); box-shadow: 0 0 30px #ef4444; animation: boom 0.55s ease-out; }
  .tile.flip .back { transform: scale(0.92); }
  @keyframes gem { 0% { transform: rotateY(0) scale(0.8); } 60% { transform: rotateY(0) scale(1.12); } 100% { transform: rotateY(0) scale(1); } }
  @keyframes boom { 0% { transform: scale(0.7); } 30% { transform: scale(1.25) rotate(-4deg); } 60% { transform: scale(1.05) rotate(3deg); } 100% { transform: scale(1); } }
</style>
