<script lang="ts">
  import { onMount } from 'svelte';
  import { Dice5, Gem, CircleDollarSign, Bomb, Gift, Crosshair, Swords, Save, RotateCcw, TrendingUp, TrendingDown, Coins, Users2, Trash2, Ban, Plus, X, Sparkles, Spade, Rocket, Dices, Flame, Shuffle } from '@lucide/svelte';
  import Toggle from '../components/Toggle.svelte';
  import WheelPreview from '../components/WheelPreview.svelte';
  import SegmentsEditor from '../components/SegmentsEditor.svelte';
  import BarChart from '../components/BarChart.svelte';
  import { get, post, put, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Seg = { label: string; value: number; weight: number; color: string };
  type Sym = { id: string; label: string; weight: number; pay: number; color: string };
  type Cfg = {
    enabled: boolean; max_payout: number; daily_loss_limit: number; feed_min_win: number;
    slots: { enabled: boolean; min_bet: number; max_bet: number; pair_pay: number; symbols: Sym[] };
    wheel: { enabled: boolean; min_bet: number; max_bet: number; segments: Seg[] };
    daily: { enabled: boolean; spins_per_day: number; segments: Seg[] };
    plinko: { enabled: boolean; min_bet: number; max_bet: number; min_rows: number; max_rows: number; rtp: number; risks: string[] };
    mines: { enabled: boolean; min_bet: number; max_bet: number; size: number; min_mines: number; max_mines: number; house_edge: number; max_multiplier: number };
    blackjack: { enabled: boolean; min_bet: number; max_bet: number; blackjack_pay: number; dealer_hits_soft_17: boolean };
    crash: { enabled: boolean; min_bet: number; max_bet: number; house_edge: number; max_multiplier: number };
    dice: { enabled: boolean; min_bet: number; max_bet: number; house_edge: number; min_chance: number; max_chance: number };
    coinflip: { enabled: boolean; min_bet: number; max_bet: number; payout: number };
    roulette: { enabled: boolean; min_bet: number; max_bet: number };
    burst: { enabled: boolean; min_bet: number; max_bet: number; survival: number; house_edge: number; max_steps: number };
    double: { enabled: boolean; win_chance: number; max_streak: number; offer_minutes: number };
    chaos: { enabled: boolean; surge_chance: number; curse_chance: number };
    bounties: { enabled: boolean; min_amount: number; max_amount: number; tax_percent: number; expire_days: number; max_active_per_player: number; claim_cooldown_minutes: number; allow_anonymous: boolean; allow_cancel: boolean };
    betting: { enabled: boolean; creators: string; metrics: string[]; windows_minutes: number[]; lock_minutes: number; min_stake: number; max_stake: number; rake_percent: number; max_open_per_player: number; max_threshold: number };
  };
  type Preview = {
    slots: { rtp: number; hit: number; max: number };
    wheel: { rtp: number; max: number };
    wheel_odds: number[];
    daily: { expected: number; per_day: number; odds: number[] };
    plinko: Record<string, Record<string, { rtp: number; max: number; table: number[] }>>;
    mines: { rtp: number; samples: { mines: number; steps: number[] }[] };
    blackjack: { rtp: number; blackjack_pay: number };
    crash: { rtp: number; max: number };
    dice: { rtp: number; samples: { chance: number; pays: number }[] };
    coinflip: { rtp: number };
    double: { rtp: number };
    chaos: { factor: number };
  };
  type Game = { game: string; rounds: number; wagered: number; paid: number; profit: number; rounds_7d: number; profit_7d: number };
  type Stats = {
    games: Game[]; days: { day: string; wagered: number; paid: number; profit: number; players: number }[];
    big_wins: { name: string; game: string; bet: number; payout: number; at: string }[];
    top_players: { name: string; wagered: number; paid: number; net: number; rounds: number }[];
    bounties: { active: number; active_total: number; claimed: number; claimed_total: number; tax_taken: number };
    betting: { open: number; markets: number; volume: number; rake_taken: number };
    free_spins_today: number;
  };

  const TABS = [
    { id: 'overview', label: 'Overview', icon: TrendingUp },
    { id: 'slots', label: 'Slots', icon: Gem },
    { id: 'wheel', label: 'Wheel', icon: CircleDollarSign },
    { id: 'plinko', label: 'Plinko', icon: Dice5 },
    { id: 'mines', label: 'Mines', icon: Bomb },
    { id: 'blackjack', label: 'Blackjack', icon: Spade },
    { id: 'crash', label: 'Crash', icon: Rocket },
    { id: 'dice', label: 'Dice', icon: Dices },
    { id: 'coinflip', label: 'Coin Flip', icon: Coins },
    { id: 'roulette', label: 'Roulette', icon: Coins },
    { id: 'burst', label: 'Burst', icon: Flame },
    { id: 'double', label: 'Double or Nothing', icon: Flame },
    { id: 'chaos', label: 'Chaos', icon: Shuffle },
    { id: 'daily', label: 'Daily spin', icon: Gift },
    { id: 'bounties', label: 'Bounties', icon: Crosshair },
    { id: 'betting', label: 'Betting', icon: Swords },
  ] as const;
  type Tab = (typeof TABS)[number]['id'];

  let tab = $state<Tab>('overview');
  let cfg = $state<Cfg | null>(null);
  let saved = $state('');
  let preview = $state<Preview | null>(null);
  let stats = $state<Stats | null>(null);
  let metrics = $state<[string, string][]>([]);
  let defaults = $state<Cfg | null>(null);
  let bounties = $state<any[]>([]);
  let markets = $state<any[]>([]);
  let busy = $state(false);
  let plinkoRisk = $state('medium');
  let plinkoRows = $state(12);
  const dirty = $derived(cfg ? JSON.stringify(cfg) !== saved : false);

  async function load() {
    try {
      const d = await get<{ config: Cfg; preview: Preview; metrics: [string, string][]; defaults: Cfg }>('/api/admin/casino');
      cfg = d.config; preview = d.preview; metrics = d.metrics; defaults = d.defaults; saved = JSON.stringify(d.config);
      plinkoRows = Math.min(Math.max(plinkoRows, cfg.plinko.min_rows), cfg.plinko.max_rows);
      await Promise.all([loadStats(), loadLists()]);
    } catch (e) { toastError(e); }
  }
  async function loadStats() { try { stats = await get<Stats>('/api/admin/casino/stats'); } catch (e) { toastError(e); } }
  async function loadLists() {
    try {
      bounties = (await get<{ bounties: any[] }>('/api/admin/casino/bounties')).bounties;
      markets = (await get<{ markets: any[] }>('/api/admin/casino/markets')).markets;
    } catch (e) { toastError(e); }
  }
  onMount(load);

  // The odds shown next to every control come from the server, so they always match what players get.
  let timer: ReturnType<typeof setTimeout>;
  $effect(() => {
    if (!cfg) return;
    const body = JSON.stringify(cfg);
    clearTimeout(timer);
    timer = setTimeout(async () => {
      try { preview = (await post<{ preview: Preview }>('/api/admin/casino/preview', JSON.parse(body))).preview; } catch { /* the last good preview stays */ }
    }, 250);
    return () => clearTimeout(timer);
  });

  async function save() {
    if (!cfg) return;
    busy = true;
    try {
      const d = await put<{ config: Cfg; preview: Preview }>('/api/admin/casino', cfg);
      cfg = d.config; preview = d.preview; saved = JSON.stringify(d.config);
      toast('Casino settings saved. Players see them right away.');
    } catch (e) { toastError(e); } finally { busy = false; }
  }
  function discard() { if (saved) cfg = JSON.parse(saved); }
  function resetGame(key: 'slots' | 'wheel' | 'daily' | 'plinko' | 'mines' | 'blackjack' | 'crash' | 'dice' | 'coinflip' | 'double' | 'chaos' | 'bounties' | 'betting') {
    if (cfg && defaults && confirm(`Put ${key} back to the standard settings?`)) (cfg as any)[key] = structuredClone($state.snapshot(defaults)[key]);
  }

  const money = (v: number) => (v < 0 ? '-$' : '$') + Math.abs(v).toLocaleString(undefined, { maximumFractionDigits: 2 });
  const pct = (v: number) => (v * 100).toFixed(1) + '%';
  const rtpTone = (v: number) => (v > 1 ? 'bad' : v >= 0.9 ? 'good' : v >= 0.8 ? 'warn' : 'bad');
  const GAMEMETA: Record<string, string> = { slots: 'Slots', wheel: 'Wheel', plinko: 'Plinko', mines: 'Mines', blackjack: 'Blackjack', crash: 'Crash', dice: 'Dice', coinflip: 'Coin Flip', double: 'Double or Nothing', daily: 'Daily spin' };

  const totals = $derived(stats ? stats.games.reduce((a, g) => ({ rounds: a.rounds + g.rounds, wagered: a.wagered + g.wagered, paid: a.paid + g.paid, profit: a.profit + g.profit, week: a.week + g.profit_7d }), { rounds: 0, wagered: 0, paid: 0, profit: 0, week: 0 }) : null);
  const chartData = $derived((stats?.days ?? []).map((d) => ({ day: d.day, launches: Math.round(d.wagered) })));

  const WINDOWS = [15, 30, 60, 180, 360, 720, 1440, 4320, 10080];
  const windowText = (m: number) => { const [n, u] = m < 60 ? [m, 'minute'] : m < 1440 ? [m / 60, 'hour'] : [m / 1440, 'day']; return `${n} ${u}${n === 1 ? '' : 's'}`; };
  function toggleIn(list: string[], v: string) { const i = list.indexOf(v); if (i >= 0) { if (list.length > 1) list.splice(i, 1); } else list.push(v); }
  function toggleWindow(list: number[], v: number) { const i = list.indexOf(v); if (i >= 0) { if (list.length > 1) list.splice(i, 1); } else { list.push(v); list.sort((a, b) => a - b); } }

  async function cancelBounty(id: number) {
    if (!confirm('Take this bounty down and give the money back to whoever placed it?')) return;
    try { await post(`/api/admin/casino/bounties/${id}/cancel`, {}); toast('Bounty removed'); await Promise.all([loadLists(), loadStats()]); } catch (e) { toastError(e); }
  }
  async function voidMarket(id: number) {
    if (!confirm('Cancel this bet and give every stake back?')) return;
    try { await post(`/api/admin/casino/markets/${id}/void`, {}); toast('Bet cancelled, stakes returned'); await Promise.all([loadLists(), loadStats()]); } catch (e) { toastError(e); }
  }
  const plinkoTable = $derived(preview?.plinko?.[plinkoRisk]?.[String(plinkoRows)]?.table ?? []);
  const plinkoPeak = $derived(Math.max(1, ...plinkoTable));
  const metricName = (id: string) => metrics.find((m) => m[0] === id)?.[1] ?? id;
</script>

<div class="page">
<header class="head">
  <div>
    <h1><Dice5 size={26} /> Casino</h1>
    <p>Slots, Wheel, Plinko, Mines, Blackjack, Crash, Dice, Coin Flip, Double or Nothing, a free daily spin, bounties and player bets. All of it is paid in the Velora economy and shows up for players in the launcher and the player panel.</p>
  </div>
  {#if cfg}<Toggle bind:checked={cfg.enabled} label="Casino open" help="Close everything at once" />{/if}
</header>

{#if !cfg}
  <p class="muted">Loading…</p>
{:else}
  <div class="tabs seg">
    {#each TABS as t}<button type="button" class:on={tab === t.id} onclick={() => (tab = t.id)}><t.icon size={14} /> {t.label}</button>{/each}
  </div>

  <div class="stage">
    {#if tab === 'overview'}
      {#if totals && stats}
        <div class="tiles">
          <div class="tile"><span><Coins size={14} /> Wagered</span><b>{money(totals.wagered)}</b><small>{totals.rounds.toLocaleString()} rounds</small></div>
          <div class="tile"><span>Paid out</span><b>{money(totals.paid)}</b><small>{totals.wagered ? pct(totals.paid / totals.wagered) : '—'} returned</small></div>
          <div class="tile" class:pos={totals.profit >= 0} class:neg={totals.profit < 0}><span>{#if totals.profit >= 0}<TrendingUp size={14} />{:else}<TrendingDown size={14} />{/if} House profit</span><b>{money(totals.profit)}</b><small>{money(totals.week)} in the last 7 days</small></div>
          <div class="tile"><span><Gift size={14} /> Free spins today</span><b>{stats.free_spins_today}</b><small>{cfg.daily.spins_per_day} per player per day</small></div>
        </div>
        <section class="card">
          <h2>Wagered per day</h2>
          {#if chartData.length}<BarChart data={chartData} label="Wagered per day" />{:else}<p class="muted">No rounds yet. Once players start playing, takings show up here.</p>{/if}
        </section>
        <div class="two">
          <section class="card">
            <h2>By game</h2>
            <table>
              <thead><tr><th>Game</th><th class="n">Rounds</th><th class="n">Wagered</th><th class="n">Paid</th><th class="n">Profit</th></tr></thead>
              <tbody>
                {#each stats.games as g}<tr><td>{GAMEMETA[g.game] ?? g.game}</td><td class="n">{g.rounds.toLocaleString()}</td><td class="n">{money(g.wagered)}</td><td class="n">{money(g.paid)}</td><td class="n" class:goodtxt={g.profit >= 0} class:badtxt={g.profit < 0}>{money(g.profit)}</td></tr>{:else}<tr><td colspan="5" class="muted">Nothing played yet</td></tr>{/each}
              </tbody>
            </table>
          </section>
          <section class="card">
            <h2>Biggest wins</h2>
            <table>
              <tbody>
                {#each stats.big_wins as w}<tr><td><b>{w.name}</b><small class="muted"> {GAMEMETA[w.game] ?? w.game} · {timeAgo(w.at)}</small></td><td class="n goodtxt">+{money(w.payout - w.bet)}</td></tr>{:else}<tr><td class="muted">No wins yet</td></tr>{/each}
              </tbody>
            </table>
          </section>
        </div>
        <div class="two">
          <section class="card">
            <h2><Users2 size={16} /> Top players by wagered</h2>
            <table>
              <thead><tr><th>Player</th><th class="n">Rounds</th><th class="n">Wagered</th><th class="n">Net</th></tr></thead>
              <tbody>{#each stats.top_players as p}<tr><td>{p.name}</td><td class="n">{p.rounds}</td><td class="n">{money(p.wagered)}</td><td class="n" class:goodtxt={p.net >= 0} class:badtxt={p.net < 0}>{money(p.net)}</td></tr>{:else}<tr><td colspan="4" class="muted">No players yet</td></tr>{/each}</tbody>
            </table>
          </section>
          <section class="card">
            <h2>Bounties &amp; bets</h2>
            <dl>
              <div><dt>Active bounties</dt><dd>{stats.bounties.active} · {money(stats.bounties.active_total)}</dd></div>
              <div><dt>Bounties collected</dt><dd>{stats.bounties.claimed} · {money(stats.bounties.claimed_total)}</dd></div>
              <div><dt>Bounty tax taken</dt><dd>{money(stats.bounties.tax_taken)}</dd></div>
              <div><dt>Open bets</dt><dd>{stats.betting.open}</dd></div>
              <div><dt>Bet volume</dt><dd>{money(stats.betting.volume)}</dd></div>
              <div><dt>Betting rake taken</dt><dd>{money(stats.betting.rake_taken)}</dd></div>
            </dl>
          </section>
        </div>
      {/if}
      <section class="card">
        <h2>Safety limits</h2>
        <div class="fields">
          <label>Largest payout per round ($)<input type="number" min="1" bind:value={cfg.max_payout} /></label>
          <label>Daily loss limit per player ($)<input type="number" min="0" bind:value={cfg.daily_loss_limit} /><small>0 turns it off. A player who has lost this much today can't bet until tomorrow (UTC).</small></label>
          <label>Show wins of at least ($)<input type="number" min="0" bind:value={cfg.feed_min_win} /><small>Shown in the launcher's big-wins ticker.</small></label>
        </div>
      </section>

    {:else if tab === 'slots' && preview}
      <section class="card">
        <header class="sec"><h2><Gem size={18} /> Slots</h2><Toggle bind:checked={cfg.slots.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.slots.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.slots.max_bet} /></label>
          <label>Any two matching pays (× bet)<input type="number" min="0" step="0.1" bind:value={cfg.slots.pair_pay} /></label>
        </div>
        <div class="rtp {rtpTone(preview.slots.rtp)}"><b>{pct(preview.slots.rtp)}</b> returned to players · house edge {pct(1 - preview.slots.rtp)} · {pct(preview.slots.hit)} of spins pay · top prize {preview.slots.max}×</div>
        <div class="symbols">
          <div class="head"><span></span><span>Symbol</span><span>3 in a row pays (×)</span><span>Weight</span><span></span></div>
          {#each cfg.slots.symbols as s, i}
            <div class="row">
              <input type="color" bind:value={s.color} aria-label="Colour" />
              <input bind:value={s.label} maxlength="16" aria-label="Name" />
              <input type="number" min="0" step="1" bind:value={s.pay} />
              <input type="number" min="0.01" step="1" bind:value={s.weight} />
              <button type="button" class="ghost" disabled={cfg.slots.symbols.length <= 2} onclick={() => cfg && cfg.slots.symbols.splice(i, 1)} aria-label="Remove"><Trash2 size={14} /></button>
            </div>
          {/each}
          <button type="button" class="ghost" disabled={cfg.slots.symbols.length >= 10} onclick={() => cfg && cfg.slots.symbols.push({ id: 's' + (cfg.slots.symbols.length + 1) + Math.random().toString(36).slice(2, 5), label: 'New', weight: 10, pay: 10, color: '#38bdf8' })}><Plus size={14} /> Add symbol</button>
        </div>
        <button type="button" class="ghost reset" onclick={() => resetGame('slots')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'wheel' && preview}
      <section class="card">
        <header class="sec"><h2><CircleDollarSign size={18} /> Wheel</h2><Toggle bind:checked={cfg.wheel.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.wheel.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.wheel.max_bet} /></label>
        </div>
        <div class="rtp {rtpTone(preview.wheel.rtp)}"><b>{pct(preview.wheel.rtp)}</b> returned to players · house edge {pct(1 - preview.wheel.rtp)} · top prize {preview.wheel.max}×</div>
        <div class="wheelrow">
          <WheelPreview segments={cfg.wheel.segments} size={210} />
          <SegmentsEditor bind:segments={cfg.wheel.segments} valueLabel="Pays (×)" odds={preview.wheel_odds} />
        </div>
        <button type="button" class="ghost reset" onclick={() => resetGame('wheel')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'plinko' && preview}
      <section class="card">
        <header class="sec"><h2><Dice5 size={18} /> Plinko</h2><Toggle bind:checked={cfg.plinko.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.plinko.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.plinko.max_bet} /></label>
          <label>Fewest rows<input type="number" min="6" max="16" bind:value={cfg.plinko.min_rows} /></label>
          <label>Most rows<input type="number" min="6" max="16" bind:value={cfg.plinko.max_rows} /></label>
        </div>
        <label class="slider">Return to players <b class="rtpval {rtpTone(cfg.plinko.rtp)}">{pct(cfg.plinko.rtp)}</b>
          <input class="fancy" type="range" min="0.5" max="0.995" step="0.005" bind:value={cfg.plinko.rtp} style="--p:{((cfg.plinko.rtp - 0.5) / 0.495) * 100}%" />
          <small>The payout tables below are scaled so the average round returns exactly this much. The house keeps {pct(1 - cfg.plinko.rtp)}.</small>
        </label>
        <div class="chips">
          <span class="lbl">Risk levels players can pick</span>
          {#each ['low', 'medium', 'high'] as r}<button type="button" class="chip" class:on={cfg.plinko.risks.includes(r)} onclick={() => cfg && toggleIn(cfg.plinko.risks, r)}>{r}</button>{/each}
        </div>
        <div class="plinko-preview">
          <div class="controls">
            <div class="seg">{#each ['low', 'medium', 'high'] as r}<button type="button" class:on={plinkoRisk === r} onclick={() => (plinkoRisk = r)}>{r}</button>{/each}</div>
            <label class="inline">Rows {plinkoRows}<input class="fancy" type="range" min={cfg.plinko.min_rows} max={cfg.plinko.max_rows} bind:value={plinkoRows} style="--p:{cfg.plinko.max_rows > cfg.plinko.min_rows ? ((plinkoRows - cfg.plinko.min_rows) / (cfg.plinko.max_rows - cfg.plinko.min_rows)) * 100 : 100}%" /></label>
          </div>
          <div class="bars" aria-label="Payout multiplier for each landing slot">
            {#each plinkoTable as m}
              <div class="bar" title="{m}×"><i style="height:{Math.max(4, (Math.log10(m + 1) / Math.log10(plinkoPeak + 1)) * 100)}%; --c:{m >= 1 ? (m >= 10 ? '#ec4899' : '#22c55e') : '#64748b'}"></i><span>{m >= 100 ? Math.round(m) : m}×</span></div>
            {/each}
          </div>
          <small class="muted">Bars use a log scale so the small payouts stay visible. Edge slots pay the most.</small>
        </div>
        <button type="button" class="ghost reset" onclick={() => resetGame('plinko')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'mines' && preview}
      <section class="card">
        <header class="sec"><h2><Bomb size={18} /> Mines</h2><Toggle bind:checked={cfg.mines.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.mines.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.mines.max_bet} /></label>
          <label>Board size<input type="number" min="3" max="7" bind:value={cfg.mines.size} /><small>{cfg.mines.size} × {cfg.mines.size} = {cfg.mines.size * cfg.mines.size} tiles</small></label>
          <label>Fewest mines<input type="number" min="1" bind:value={cfg.mines.min_mines} /></label>
          <label>Most mines<input type="number" min="1" bind:value={cfg.mines.max_mines} /></label>
          <label>Top multiplier cap (×)<input type="number" min="1" bind:value={cfg.mines.max_multiplier} /></label>
        </div>
        <label class="slider">House edge <b class="rtpval {rtpTone(1 - cfg.mines.house_edge)}">{pct(cfg.mines.house_edge)}</b>
          <input class="fancy" type="range" min="0" max="0.2" step="0.005" bind:value={cfg.mines.house_edge} style="--p:{(cfg.mines.house_edge / 0.2) * 100}%" />
          <small>Every multiplier is the fair odds of surviving that far, less this edge.</small>
        </label>
        <table class="mini">
          <thead><tr><th>Mines</th><th class="n">1 tile</th><th class="n">3 tiles</th><th class="n">5 tiles</th></tr></thead>
          <tbody>{#each preview.mines.samples as s}<tr><td>{s.mines}</td>{#each s.steps as v}<td class="n">{v}×</td>{/each}</tr>{/each}</tbody>
        </table>
        <button type="button" class="ghost reset" onclick={() => resetGame('mines')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'blackjack' && preview}
      <section class="card">
        <header class="sec"><h2><Spade size={18} /> Blackjack</h2><Toggle bind:checked={cfg.blackjack.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.blackjack.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.blackjack.max_bet} /></label>
          <label>Blackjack pays (× the bet, on top of it)<input type="number" min="1" max="3" step="0.1" bind:value={cfg.blackjack.blackjack_pay} /><small>1.5 is the classic 3 to 2. 1.2 is 6 to 5, which favours the house.</small></label>
        </div>
        <Toggle bind:checked={cfg.blackjack.dealer_hits_soft_17} label="Dealer hits on a soft 17" />
        <p class="muted">Cards come from an endless shoe, so nothing can be counted. With sensible play the game returns about <b>{pct(preview.blackjack.rtp)}</b>; lowering the blackjack payout or having the dealer hit soft 17 widens the house's edge.</p>
        <button type="button" class="ghost reset" onclick={() => resetGame('blackjack')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'crash' && preview}
      <section class="card">
        <header class="sec"><h2><Rocket size={18} /> Crash</h2><Toggle bind:checked={cfg.crash.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.crash.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.crash.max_bet} /></label>
          <label>Top multiplier (×)<input type="number" min="2" bind:value={cfg.crash.max_multiplier} /><small>A round that gets this far crashes here.</small></label>
        </div>
        <label class="slider">House edge <b class="rtpval {rtpTone(1 - cfg.crash.house_edge)}">{pct(cfg.crash.house_edge)}</b>
          <input class="fancy" type="range" min="0" max="0.2" step="0.005" bind:value={cfg.crash.house_edge} style="--p:{(cfg.crash.house_edge / 0.2) * 100}%" />
          <small>Every round crashes at a random point. Whatever multiplier a player aims for, they get back {pct(preview.crash.rtp)} on average, so there is no safe target to exploit.</small>
        </label>
        <button type="button" class="ghost reset" onclick={() => resetGame('crash')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'dice' && preview}
      <section class="card">
        <header class="sec"><h2><Dices size={18} /> Dice</h2><Toggle bind:checked={cfg.dice.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.dice.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.dice.max_bet} /></label>
          <label>Lowest win chance (%)<input type="number" min="0.01" max="98" step="0.01" bind:value={cfg.dice.min_chance} /></label>
          <label>Highest win chance (%)<input type="number" min="0.01" max="98" step="0.01" bind:value={cfg.dice.max_chance} /></label>
        </div>
        <label class="slider">House edge <b class="rtpval {rtpTone(1 - cfg.dice.house_edge)}">{pct(cfg.dice.house_edge)}</b>
          <input class="fancy" type="range" min="0" max="0.2" step="0.005" bind:value={cfg.dice.house_edge} style="--p:{(cfg.dice.house_edge / 0.2) * 100}%" />
          <small>Players pick their own odds; each chance pays the fair price less this edge.</small>
        </label>
        <table class="mini">
          <thead><tr><th>Win chance</th><th class="n">Pays</th></tr></thead>
          <tbody>{#each preview.dice.samples as d}<tr><td>{d.chance}%</td><td class="n">{d.pays}×</td></tr>{/each}</tbody>
        </table>
        <button type="button" class="ghost reset" onclick={() => resetGame('dice')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'roulette' || tab === 'burst'}
      <section class="panel"><header class="sec"><h2>{tab==='roulette'?'Roulette':'Burst'}</h2><Toggle bind:checked={cfg[tab].enabled} label="Open"/></header>
      <div class="fields"><label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg[tab].min_bet}/></label><label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg[tab].max_bet}/></label>
      {#if tab==='burst'}<label>Step survival probability<input type="number" min="0.1" max="0.95" step="0.01" bind:value={cfg.burst.survival}/></label><label>House edge<input type="number" min="0" max="0.25" step="0.01" bind:value={cfg.burst.house_edge}/></label><label>Maximum steps<input type="number" min="1" max="20" bind:value={cfg.burst.max_steps}/></label>{/if}</div>
      <p>{tab==='roulette'?'Single-zero European wheel: 37 equal pockets, 36× straight bets and 2×/3× outside bets.':'A round keeps the odds and payout cap it started with. Closing Burst still allows existing players to cash out.'}</p></section>
    {:else if tab === 'coinflip' && preview}
      <section class="card">
        <header class="sec"><h2><Coins size={18} /> Coin Flip</h2><Toggle bind:checked={cfg.coinflip.enabled} label="Open" /></header>
        <div class="fields">
          <label>Smallest bet ($)<input type="number" min="0.01" bind:value={cfg.coinflip.min_bet} /></label>
          <label>Biggest bet ($)<input type="number" min="0.01" bind:value={cfg.coinflip.max_bet} /></label>
          <label>Pays (× the bet)<input type="number" min="1" max="2" step="0.01" bind:value={cfg.coinflip.payout} /><small>The coin is always a fair 50/50. A fair payout is 2; {cfg.coinflip.payout} returns {pct(cfg.coinflip.payout / 2)}.</small></label>
        </div>
        <button type="button" class="ghost reset" onclick={() => resetGame('coinflip')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'double' && preview}
      <section class="card">
        <header class="sec"><h2><Flame size={18} /> Double or Nothing</h2><Toggle bind:checked={cfg.double.enabled} label="On" /></header>
        <p class="muted">After any win (slots, wheel, plinko, mines, blackjack, crash, dice or coin flip) the player is offered a gamble on those winnings: win and they double, lose and they're gone. Declining costs nothing, the winnings are already in their balance.</p>
        <label class="slider">Chance the double comes off <b class="rtpval {rtpTone(cfg.double.win_chance * 2)}">{pct(cfg.double.win_chance)}</b>
          <input class="fancy" type="range" min="0.05" max="0.95" step="0.005" bind:value={cfg.double.win_chance} style="--p:{((cfg.double.win_chance - 0.05) / 0.9) * 100}%" />
          <small>Returns {pct(preview.double.rtp)} of what's risked. Below 50% the house keeps an edge; each extra double compounds it.</small>
        </label>
        <div class="fields">
          <label>Most doubles in a row<input type="number" min="1" max="20" bind:value={cfg.double.max_streak} /></label>
          <label>Offer stays open for (minutes)<input type="number" min="1" max="60" bind:value={cfg.double.offer_minutes} /></label>
        </div>
        <button type="button" class="ghost reset" onclick={() => resetGame('double')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'chaos' && preview}
      <section class="card">
        <header class="sec"><h2><Shuffle size={18} /> Chaos</h2><Toggle bind:checked={cfg.chaos.enabled} label="On" /></header>
        <p class="muted">Makes instant games (slots, wheel, plinko, dice, coin flip) less predictable than their tables. Every winning round can be hit by a <b>lucky surge</b> (the win is multiplied by 1.5×, 2× or 3×) or a <b>curse</b> (the win is halved). Players see which one hit.</p>
        <label class="slider">Lucky surge chance <b class="rtpval good">{pct(cfg.chaos.surge_chance)}</b>
          <input class="fancy" type="range" min="0" max="0.5" step="0.005" bind:value={cfg.chaos.surge_chance} style="--p:{(cfg.chaos.surge_chance / 0.5) * 100}%" />
        </label>
        <label class="slider">Curse chance <b class="rtpval warn">{pct(cfg.chaos.curse_chance)}</b>
          <input class="fancy" type="range" min="0" max="0.5" step="0.005" bind:value={cfg.chaos.curse_chance} style="--p:{(cfg.chaos.curse_chance / 0.5) * 100}%" />
        </label>
        <p class="muted">Net effect on winning payouts: <b class="rtpval {preview.chaos.factor > 1.005 ? 'bad' : 'good'}">{preview.chaos.factor >= 1 ? '+' : ''}{((preview.chaos.factor - 1) * 100).toFixed(1)}%</b>. {preview.chaos.factor > 1.005 ? 'Surges outweigh curses, so players come out slightly ahead of the tables.' : 'Roughly even, so the tables still describe what players get back.'}</p>
        <button type="button" class="ghost reset" onclick={() => resetGame('chaos')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'daily' && preview}
      <section class="card">
        <header class="sec"><h2><Gift size={18} /> Daily free spin</h2><Toggle bind:checked={cfg.daily.enabled} label="Open" /></header>
        <p class="muted">Every player gets free spins of this wheel each day (UTC). Prizes are paid straight into their balance, so this is the casino's gift to the economy. Expected prize per spin: <b>{money(preview.daily.expected)}</b>, about {money(preview.daily.expected * cfg.daily.spins_per_day)} per player per day.</p>
        <label class="slider">Free spins per player per day <b class="rtpval">{cfg.daily.spins_per_day}</b>
          <input class="fancy" type="range" min="0" max="10" step="1" bind:value={cfg.daily.spins_per_day} style="--p:{cfg.daily.spins_per_day * 10}%" />
        </label>
        <div class="wheelrow">
          <WheelPreview segments={cfg.daily.segments} size={210} />
          <SegmentsEditor bind:segments={cfg.daily.segments} valueLabel="Prize ($)" step={5} odds={preview.daily.odds} />
        </div>
        <button type="button" class="ghost reset" onclick={() => resetGame('daily')}><RotateCcw size={14} /> Standard settings</button>
      </section>

    {:else if tab === 'bounties'}
      <section class="card">
        <header class="sec"><h2><Crosshair size={18} /> Bounties</h2><Toggle bind:checked={cfg.bounties.enabled} label="Open" /></header>
        <p class="muted">Players put money on another player's head. Whoever kills that player in PvP collects every bounty on them. The money is held until then.</p>
        <div class="fields">
          <label>Smallest bounty ($)<input type="number" min="0.01" bind:value={cfg.bounties.min_amount} /></label>
          <label>Biggest bounty ($)<input type="number" min="0.01" bind:value={cfg.bounties.max_amount} /></label>
          <label>House takes (%)<input type="number" min="0" max="90" step="1" bind:value={cfg.bounties.tax_percent} /><small>Kept when it is placed; the rest is the reward.</small></label>
          <label>Expires after (days)<input type="number" min="0" max="365" bind:value={cfg.bounties.expire_days} /><small>0 = never. Uncollected bounties go back to whoever placed them.</small></label>
          <label>Bounties out at once, per player<input type="number" min="1" max="100" bind:value={cfg.bounties.max_active_per_player} /></label>
          <label>Same-pair cooldown (minutes)<input type="number" min="0" max="10080" bind:value={cfg.bounties.claim_cooldown_minutes} /><small>Stops two friends farming a bounty by killing each other.</small></label>
        </div>
        <Toggle bind:checked={cfg.bounties.allow_anonymous} label="Allow anonymous bounties" help="Placers can hide their name from the target and the board." />
        <Toggle bind:checked={cfg.bounties.allow_cancel} label="Let players withdraw a bounty" help="They get the reward back; the house keeps its cut." />
        <button type="button" class="ghost reset" onclick={() => resetGame('bounties')}><RotateCcw size={14} /> Standard settings</button>
      </section>
      <section class="card">
        <h2>Latest bounties</h2>
        <table>
          <thead><tr><th>Target</th><th>Placed by</th><th class="n">Reward</th><th>Status</th><th></th></tr></thead>
          <tbody>
            {#each bounties as b}
              <tr><td><b>{b.target}</b></td><td>{b.placer}{b.anonymous ? ' (hidden)' : ''}</td><td class="n">{money(b.reward)}</td>
                <td><span class="pill {b.status}">{b.status}</span>{#if b.claimer} <small class="muted">by {b.claimer}</small>{/if}</td>
                <td class="n">{#if b.status === 'active'}<button type="button" class="ghost" onclick={() => cancelBounty(b.id)}><Ban size={14} /> Remove</button>{/if}</td></tr>
            {:else}<tr><td colspan="5" class="muted">No bounties yet</td></tr>{/each}
          </tbody>
        </table>
      </section>

    {:else if tab === 'betting'}
      <section class="card">
        <header class="sec"><h2><Swords size={18} /> Player betting</h2><Toggle bind:checked={cfg.betting.enabled} label="Open" /></header>
        <p class="muted">A bet asks "will this player reach N of something before time runs out?". Everyone puts money on Yes or No; when time is up the winning side splits the whole pool, minus the house's rake. If nobody backed the other side, every stake is returned. Results come from the stats the game servers already report.</p>
        <div class="chips">
          <span class="lbl">Who can open a bet</span>
          <div class="seg"><button type="button" class:on={cfg.betting.creators === 'players'} onclick={() => cfg && (cfg.betting.creators = 'players')}>Any player</button><button type="button" class:on={cfg.betting.creators === 'admins'} onclick={() => cfg && (cfg.betting.creators = 'admins')}>Admins only</button></div>
        </div>
        <div class="chips">
          <span class="lbl">What players can bet on</span>
          {#each metrics as [id, label]}<button type="button" class="chip" class:on={cfg.betting.metrics.includes(id)} onclick={() => cfg && toggleIn(cfg.betting.metrics, id)}>{label}</button>{/each}
        </div>
        <div class="chips">
          <span class="lbl">Time limits on offer</span>
          {#each WINDOWS as w}<button type="button" class="chip" class:on={cfg.betting.windows_minutes.includes(w)} onclick={() => cfg && toggleWindow(cfg.betting.windows_minutes, w)}>{windowText(w)}</button>{/each}
        </div>
        <div class="fields">
          <label>Smallest stake ($)<input type="number" min="0.01" bind:value={cfg.betting.min_stake} /></label>
          <label>Biggest stake ($)<input type="number" min="0.01" bind:value={cfg.betting.max_stake} /></label>
          <label>House rake (%)<input type="number" min="0" max="50" step="0.5" bind:value={cfg.betting.rake_percent} /></label>
          <label>Betting closes before the end (min)<input type="number" min="0" bind:value={cfg.betting.lock_minutes} /></label>
          <label>Open bets per player<input type="number" min="1" max="50" bind:value={cfg.betting.max_open_per_player} /></label>
          <label>Highest target a bet can set<input type="number" min="1" bind:value={cfg.betting.max_threshold} /></label>
        </div>
        <button type="button" class="ghost reset" onclick={() => resetGame('betting')}><RotateCcw size={14} /> Standard settings</button>
      </section>
      <section class="card">
        <h2>Latest bets</h2>
        <table>
          <thead><tr><th>Bet</th><th>Opened by</th><th class="n">Pool</th><th>Status</th><th></th></tr></thead>
          <tbody>
            {#each markets as m}
              <tr><td><b>{m.subject}</b> reaches {m.threshold} {metricName(m.metric)}</td><td>{m.creator}</td><td class="n">{money(m.volume)} <small class="muted">({m.bets})</small></td>
                <td><span class="pill {m.status}">{m.status}</span>{#if m.outcome} <small class="muted">{m.outcome}</small>{/if}</td>
                <td class="n">{#if m.status === 'open'}<button type="button" class="ghost" onclick={() => voidMarket(m.id)}><X size={14} /> Cancel</button>{/if}</td></tr>
            {:else}<tr><td colspan="5" class="muted">No bets yet</td></tr>{/each}
          </tbody>
        </table>
      </section>
    {/if}
  </div>

  {#if dirty}
    <div class="savebar">
      <span><Sparkles size={14} /> Unsaved changes</span>
      <button type="button" class="ghost" onclick={discard}>Discard</button>
      <button type="button" onclick={save} disabled={busy}><Save size={14} /> Save casino settings</button>
    </div>
  {/if}
{/if}
</div>

<style>
  header.head { display: flex; justify-content: space-between; align-items: flex-start; gap: 24px; margin-bottom: 18px; }
  header.head h1 { display: flex; align-items: center; gap: 10px; margin: 0 0 4px; }
  header.head p { margin: 0; color: var(--muted); max-width: 70ch; }
  .tabs { margin-bottom: 18px; }
  .tabs button { display: inline-flex; align-items: center; gap: 6px; }
  .stage { display: flex; flex-direction: column; gap: 16px; padding-bottom: 80px; }
  .card { padding: 18px 20px; display: flex; flex-direction: column; gap: 14px; }
  .card h2 { margin: 0; font-size: 1rem; display: flex; align-items: center; gap: 8px; }
  .sec { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
  .sec :global(.toggle) { flex: none; }
  .fields { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 12px 16px; }
  .fields label { display: flex; flex-direction: column; gap: 4px; font-size: 0.82rem; color: var(--text-2); }
  .fields small, .slider small { color: var(--muted); font-size: 0.74rem; }
  .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 12px; }
  .tile { background: var(--surface); border: 1px solid var(--line); border-radius: 14px; padding: 14px 16px; display: flex; flex-direction: column; gap: 3px; }
  .tile span { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 0.78rem; }
  .tile b { font-size: 1.5rem; font-variant-numeric: tabular-nums; }
  .tile small { color: var(--muted); font-size: 0.75rem; }
  .tile.pos b { color: var(--ok, #22c55e); }
  .tile.neg b { color: var(--danger, #ef4444); }
  .two { display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 16px; }
  table { width: 100%; border-collapse: collapse; font-size: 0.86rem; }
  th { text-align: left; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); padding: 6px 8px; }
  td { padding: 8px; border-top: 1px solid var(--line); }
  .n { text-align: right; font-variant-numeric: tabular-nums; }
  .goodtxt { color: var(--ok, #22c55e); } .badtxt { color: var(--danger, #ef4444); }
  .mini { max-width: 420px; }
  dl { margin: 0; display: grid; gap: 8px; } dl div { display: flex; justify-content: space-between; gap: 12px; border-bottom: 1px dashed var(--line); padding-bottom: 6px; }
  dt { color: var(--muted); } dd { margin: 0; font-variant-numeric: tabular-nums; font-weight: 600; }
  .rtp { padding: 10px 14px; border-radius: 12px; font-size: 0.86rem; border: 1px solid var(--line); background: var(--surface-2); }
  .rtp b { font-size: 1.1rem; }
  .rtp.good { border-color: color-mix(in srgb, #22c55e 50%, transparent); background: color-mix(in srgb, #22c55e 10%, var(--surface-2)); }
  .rtp.warn { border-color: color-mix(in srgb, #eab308 50%, transparent); background: color-mix(in srgb, #eab308 10%, var(--surface-2)); }
  .rtp.bad { border-color: color-mix(in srgb, #ef4444 55%, transparent); background: color-mix(in srgb, #ef4444 12%, var(--surface-2)); }
  .rtpval { font-variant-numeric: tabular-nums; margin-left: 6px; }
  .rtpval.good { color: #22c55e; } .rtpval.warn { color: #eab308; } .rtpval.bad { color: #ef4444; }
  .symbols { display: flex; flex-direction: column; gap: 6px; }
  .symbols .head, .symbols .row { display: grid; grid-template-columns: 34px 1fr 150px 90px 34px; gap: 8px; align-items: center; }
  .symbols .head { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); }
  .symbols input[type='color'] { width: 34px; height: 32px; padding: 2px; border-radius: 8px; }
  .symbols > button { align-self: flex-start; display: inline-flex; gap: 6px; align-items: center; }
  .wheelrow { display: grid; grid-template-columns: 220px 1fr; gap: 24px; align-items: start; }
  @media (max-width: 800px) { .wheelrow { grid-template-columns: 1fr; justify-items: center; } }
  .slider { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; color: var(--text-2); max-width: 520px; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .chips .lbl { font-size: 0.78rem; color: var(--muted); min-width: 170px; }
  .chip { padding: 5px 13px; border-radius: 99px; border: 1px solid var(--line); background: var(--surface-2); color: var(--text-2); font-size: 0.8rem; cursor: pointer; text-transform: capitalize; transition: all 0.12s; }
  .chip:hover { border-color: var(--accent); color: var(--text); }
  .chip.on { background: var(--accent); border-color: var(--accent); color: #fff; box-shadow: 0 2px 10px -3px var(--accent); }
  .plinko-preview { display: flex; flex-direction: column; gap: 10px; background: var(--surface-2); border: 1px solid var(--line); border-radius: 14px; padding: 14px; }
  .plinko-preview .controls { display: flex; flex-wrap: wrap; gap: 18px; align-items: center; }
  .inline { display: flex; align-items: center; gap: 10px; font-size: 0.85rem; }
  .bars { display: flex; align-items: flex-end; gap: 4px; height: 130px; }
  .bar { flex: 1; min-width: 0; height: 100%; display: flex; flex-direction: column; justify-content: flex-end; align-items: center; gap: 4px; }
  .bar i { display: block; width: 100%; border-radius: 6px 6px 2px 2px; background: linear-gradient(180deg, var(--c), color-mix(in srgb, var(--c) 55%, #000)); transition: height 0.2s; }
  .bar span { font-size: 0.6rem; color: var(--text-2); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .reset { align-self: flex-start; display: inline-flex; gap: 6px; align-items: center; }
  .pill { font-size: 0.7rem; padding: 2px 9px; border-radius: 99px; background: var(--surface-3); text-transform: capitalize; }
  .pill.active, .pill.open { background: color-mix(in srgb, #22c55e 25%, transparent); color: #4ade80; }
  .pill.claimed, .pill.settled { background: color-mix(in srgb, var(--accent) 25%, transparent); }
  .pill.cancelled, .pill.void, .pill.expired { background: var(--surface-3); color: var(--muted); }
  .savebar { position: sticky; bottom: 16px; margin-top: 8px; display: flex; align-items: center; gap: 10px; justify-content: flex-end; padding: 10px 14px; border-radius: 16px; background: color-mix(in srgb, var(--surface) 88%, transparent); backdrop-filter: blur(14px); border: 1px solid var(--line); box-shadow: 0 10px 40px -10px #000c; }
  .savebar span { margin-right: auto; display: inline-flex; gap: 6px; align-items: center; color: var(--text-2); font-size: 0.85rem; }
  .savebar button { display: inline-flex; gap: 6px; align-items: center; }
</style>
