<script module lang="ts">
  export type CasinoServer = { id: number; name: string };
</script>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { Dice5, Dices, Gem, CircleDollarSign, Triangle, Bomb, Crosshair, Swords, Wallet, RefreshCw, Sparkles, Lock, Home, History, Trophy, Spade, Rocket, Coins, Zap, Keyboard, Volume2, VolumeX, X, Play, TrendingUp, TrendingDown } from '@lucide/svelte';
  import Money from './Money.svelte';
  import Pane from './Pane.svelte';
  import { cprefs, setChaos, setKeys, setLast } from './prefs.svelte';
  import { hotkeys } from './hotkeys';
  import { setSound, soundOn } from './sfx';
  import DailySpin from './DailySpin.svelte';
  import Slots from './Slots.svelte';
  import Wheel from './Wheel.svelte';
  import Plinko from './Plinko.svelte';
  import Mines from './Mines.svelte';
  import Blackjack from './Blackjack.svelte';
  import Crash from './Crash.svelte';
  import Dice from './Dice.svelte';
  import CoinFlip from './CoinFlip.svelte';
  import DoubleOrNothing from './DoubleOrNothing.svelte';
  import Bounties from './Bounties.svelte';
  import Betting from './Betting.svelte';
  import { cget, money, mult, ago, type CasinoState } from './casino';
  import { errorText } from './host';

  type Tab = 'lobby' | 'slots' | 'wheel' | 'plinko' | 'mines' | 'blackjack' | 'crash' | 'dice' | 'coinflip' | 'bounties' | 'betting';
  type Hist = { rounds: { game: string; bet: number; payout: number; at: string }[]; wagered: number; won: number; rounds_played: number };

  let { servers, initialServerId = null, compact = false, onbalance }: { servers: CasinoServer[]; initialServerId?: number | null; compact?: boolean; onbalance?: (v: number) => void } = $props();
  let serverId = $state<number | null>(untrack(() => initialServerId ?? servers[0]?.id ?? null));
  let st = $state<CasinoState | null>(null);
  let balance = $state<number | null>(null);
  let hist = $state<Hist | null>(null);
  let tab = $state<Tab>('lobby');
  let tabsEl = $state<HTMLElement | null>(null);
  $effect(() => { void tab; tabsEl?.querySelector('button.active')?.scrollIntoView({ inline: 'center', block: 'nearest', behavior: 'smooth' }); });
  let visited = $state<Partial<Record<Tab, boolean>>>({});
  $effect(() => { visited[tab] = true; if (tab !== 'lobby' && tab !== 'bounties' && tab !== 'betting') setLast(tab); });
  // Switching servers is a different table: drop the old ones.
  $effect(() => { void serverId; untrack(() => { visited = { [tab]: true }; }); });
  let error = $state('');
  let loading = $state(false);
  let now = $state(Date.now());
  let sound = $state(soundOn());
  let help = $state(false);
  const toggleSound = () => { sound = !sound; setSound(sound); };
  const chaosOn = $derived(!!st?.config.chaos.enabled && cprefs.chaos);

  // Scale the table with the window: a roomy stage grows (up to 1.55x) instead of floating in empty space, and never outgrows the height.
  let root = $state<HTMLElement | null>(null);
  let fit = $state(1);
  $effect(() => {
    if (!root) return;
    const measure = () => {
      const w = root!.clientWidth, h = window.innerHeight;
      const stage = w > 760 ? w - Math.min(400, Math.max(304, w * 0.24)) - 60 : w - 24;
      fit = Math.round(Math.min(1.55, Math.max(1, Math.min(stage / 620, (h - 230) / 520))) * 100) / 100;
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(root);
    window.addEventListener('resize', measure);
    return () => { ro.disconnect(); window.removeEventListener('resize', measure); };
  });

  // How the session is going: baseline taken when the casino first loads.
  let base = $state<{ won: number; wagered: number; rounds: number } | null>(null);
  $effect(() => { if (hist && !base) base = { won: hist.won, wagered: hist.wagered, rounds: hist.rounds_played }; });
  const sessionNet = $derived(hist && base ? (hist.won - hist.wagered) - (base.won - base.wagered) : 0);
  const sessionRounds = $derived(hist && base ? hist.rounds_played - base.rounds : 0);
  const recent = $derived((hist?.rounds ?? []).slice(0, 8));
  function setBalance(v: number) { balance = v; onbalance?.(v); }

  $effect(() => { if (serverId == null && servers.length) serverId = initialServerId ?? servers[0].id; });
  $effect(() => { if (!servers.length) error = 'No servers to play on yet.'; else if (error === 'No servers to play on yet.') error = ''; });
  async function refresh(quiet = true) {
    if (serverId == null) return;
    if (!quiet) loading = true;
    try {
      const [s, h] = await Promise.all([cget<CasinoState>(serverId, ''), cget<Hist>(serverId, '/history')]);
      st = s; hist = h; balance = s.balance; if (s.balance != null) onbalance?.(s.balance); error = '';
    } catch (e) { error = errorText(e); } finally { loading = false; }
  }
  onMount(() => { const t = setInterval(() => (now = Date.now()), 1000); const p = setInterval(() => void refresh(), 30000); return () => { clearInterval(t); clearInterval(p); }; });
  $effect(() => { void serverId; st = null; void refresh(false); });
  // A table left mid-game (a Crash round, a Blackjack hand, Mines) opens on that game so nothing is forgotten.
  let resumed = false;
  $effect(() => { if (st && !resumed) { resumed = true; if (st.crash) tab = 'crash'; else if (st.blackjack) tab = 'blackjack'; else if (st.mines) tab = 'mines'; } });

  const GAMES = [
    { id: 'slots', label: 'Slots', icon: Gem, blurb: 'Three reels, one pay line. Match them for up to', hue: ['#f59e0b', '#b45309'], key: 'slots' },
    { id: 'plinko', label: 'Plinko', icon: Triangle, blurb: 'Drop the ball and watch it bounce down to a multiplier of up to', hue: ['#38bdf8', '#4f46e5'], key: 'plinko' },
    { id: 'mines', label: 'Mines', icon: Bomb, blurb: 'Turn over gems, dodge the mines, cash out whenever you dare.', hue: ['#22c55e', '#166534'], key: 'mines' },
    { id: 'wheel', label: 'Wheel', icon: CircleDollarSign, blurb: 'One spin, one pointer. Land on multipliers up to', hue: ['#ec4899', '#7c3aed'], key: 'wheel' },
    { id: 'blackjack', label: 'Blackjack', icon: Spade, blurb: 'Hit, stand or double down. Beat the dealer to 21, blackjack pays 3 to 2.', hue: ['#10b981', '#064e3b'], key: 'blackjack' },
    { id: 'crash', label: 'Crash', icon: Rocket, blurb: 'Ride the multiplier and cash out before it crashes. Nobody knows when.', hue: ['#f43f5e', '#6d28d9'], key: 'crash' },
    { id: 'dice', label: 'Dice', icon: Dices, blurb: 'Pick your own odds. The riskier the roll, the bigger the payout, up to', hue: ['#38bdf8', '#1d4ed8'], key: 'dice' },
    { id: 'coinflip', label: 'Coin Flip', icon: Coins, blurb: 'Heads or tails. One toss, nearly double your bet.', hue: ['#f5b942', '#92400e'], key: 'coinflip' },
  ] as const;
  const open = (id: string) => st?.config[id as 'slots']?.enabled !== false;
  const top = (id: string) => {
    if (!st) return '';
    if (id === 'slots') return mult(Math.max(...st.config.slots.symbols.map((s) => s.pay)));
    if (id === 'wheel') return mult(Math.max(...st.config.wheel.segments.map((s) => s.value)));
    if (id === 'plinko') return mult(Math.max(...(st.plinko_tables.high?.['16'] ?? st.plinko_tables[Object.keys(st.plinko_tables)[0]]?.[String(st.config.plinko.max_rows)] ?? [1])));
    if (id === 'dice') return mult(Math.floor((100 * (1 - st.config.dice.house_edge) / st.config.dice.min_chance) * 100) / 100);
    return '';
  };
  const NAV: { id: Tab; label: string; icon: typeof Home }[] = [
    { id: 'lobby', label: 'Lobby', icon: Home }, { id: 'slots', label: 'Slots', icon: Gem }, { id: 'plinko', label: 'Plinko', icon: Triangle }, { id: 'mines', label: 'Mines', icon: Bomb },
    { id: 'wheel', label: 'Wheel', icon: CircleDollarSign }, { id: 'blackjack', label: 'Blackjack', icon: Spade }, { id: 'crash', label: 'Crash', icon: Rocket }, { id: 'dice', label: 'Dice', icon: Dices }, { id: 'coinflip', label: 'Coin Flip', icon: Coins }, { id: 'bounties', label: 'Bounties', icon: Crosshair }, { id: 'betting', label: 'Betting', icon: Swords },
  ];
  const net = $derived(hist ? hist.won - hist.wagered : 0);
  const lastGame = $derived(GAMES.find((g) => g.id === cprefs.last && open(g.id)) ?? null);

  // Keys that work everywhere (each table adds its own: Space plays, arrows change the bet, and so on).
  hotkeys(() => ({
    '?': () => (help = !help), '/': () => (help = !help),
    escape: () => { if (help) help = false; else if (tab !== 'lobby') tab = 'lobby'; },
    g: () => (tab = 'lobby'),
    x: () => { if (st?.config.chaos.enabled) setChaos(!cprefs.chaos); },
    v: toggleSound,
    ...(tab === 'lobby' ? Object.fromEntries(GAMES.slice(0, 9).map((g, i) => [String(i + 1), () => { if (open(g.id)) tab = g.id; }])) : {}),
  }), true);

  const KEYS: { group: string; rows: [string, string][] }[] = [
    { group: 'At every table', rows: [['Space / Enter', 'Play the main button'], ['↑ ↓  or  + −', 'Raise or lower the bet'], ['1 – 5', 'Pick a bet chip'], ['[  ]', 'Halve or double the bet'], ['M  /  N', 'Maximum or minimum bet']] },
    { group: 'Casino', rows: [['X', 'Chaos mode on / off'], ['V', 'Sound on / off'], ['G  /  Esc', 'Back to the lobby'], ['1 – 8 in the lobby', 'Open a game'], ['?', 'This list']] },
    { group: 'Game keys', rows: [['Blackjack', 'H hit · S stand · D double down'], ['Crash', 'Space launches, then cashes out'], ['Mines', 'C cash out'], ['Dice', 'U under · O over'], ['Coin Flip', 'H heads · T tails'], ['Plinko', 'R changes the risk']] },
  ];
</script>

<div class="casino casino-motion" class:compact class:chaosmode={chaosOn} bind:this={root} style:--fit={fit}>
  {#if !compact}
    <header class="top">
      <div>
        <h1><Dice5 size={26} /> Casino</h1>
        <p class="lead">Play with your Velora balance. Winnings land in your account straight away, in the launcher and in game.</p>
      </div>
      <div class="right">
        {#if servers.length > 1}<select aria-label="Server" bind:value={serverId}>{#each servers as s}<option value={s.id}>{s.name}</option>{/each}</select>{/if}
        <button class="ghost icon" aria-label="Refresh" title="Refresh" onclick={() => refresh(false)} disabled={loading}><RefreshCw size={16} class={loading ? 'spin' : ''} /></button>
      </div>
    </header>
  {:else if servers.length > 1}
    <div class="right solo"><select aria-label="Server" bind:value={serverId}>{#each servers as s}<option value={s.id}>{s.name}</option>{/each}</select></div>
  {/if}

  {#if error}<div class="banner err" role="alert">{error}</div>{/if}

  {#if st}
    {#if !st.enabled}
      <div class="closed"><Lock size={34} /><h2>The casino is closed</h2><p>The server's admins have switched it off for now. Check back later.</p></div>
    {:else}
      <!-- The bar that stays with you: balance, how the session is going, the last rounds, and the switches. -->
      <div class="bar">
        <div class="wallet" title="Your balance on this server"><Wallet size={18} /><div><small>Balance</small><b><Money value={balance} /></b></div></div>
        <div class="session" class:up={sessionNet > 0} class:down={sessionNet < 0} title="Won or lost since you opened the casino">
          {#if sessionNet >= 0}<TrendingUp size={16} />{:else}<TrendingDown size={16} />{/if}
          <div><small>This session</small><b>{sessionNet > 0 ? '+' : sessionNet < 0 ? '−' : ''}{money(Math.abs(sessionNet))}</b></div>
          <em>{sessionRounds} round{sessionRounds === 1 ? '' : 's'}</em>
        </div>
        <div class="recent" aria-label="Your last rounds">
          {#each recent as r, i (r.at + i)}
            <span class="pip" class:w={r.payout > r.bet} class:l={r.payout < r.bet} title="{r.game}: {r.payout > r.bet ? 'won' : r.payout < r.bet ? 'lost' : 'push'} {money(Math.abs(r.payout - r.bet))}">{r.payout > r.bet ? '+' : r.payout < r.bet ? '−' : '='}{money(Math.abs(r.payout - r.bet), 0)}</span>
          {/each}
          {#if !recent.length}<span class="muted">Your results appear here.</span>{/if}
        </div>
        <div class="switches">
          {#if st.config.chaos.enabled}
            <button class="chaos" class:on={cprefs.chaos} role="switch" aria-checked={cprefs.chaos} onclick={() => setChaos(!cprefs.chaos)} title="Chaos mode: wins can surge ×{(1.5).toFixed(1)}+ or be cursed to half. Press X.">
              <Zap size={15} /><span>Chaos</span><i class="track"><i class="knob"></i></i>
            </button>
          {/if}
          <button class="ghost icon" onclick={toggleSound} aria-pressed={sound} aria-label={sound ? 'Mute sound effects' : 'Turn on sound effects'} title="Sound effects (V)">{#if sound}<Volume2 size={17} />{:else}<VolumeX size={17} />{/if}</button>
          <button class="ghost icon" onclick={() => (help = true)} aria-label="Keyboard shortcuts" title="Keyboard shortcuts (?)"><Keyboard size={17} /></button>
          {#if compact}<button class="ghost icon" aria-label="Refresh" title="Refresh" onclick={() => refresh(false)} disabled={loading}><RefreshCw size={16} class={loading ? 'spin' : ''} /></button>{/if}
        </div>
      </div>
      {#if chaosOn}<p class="chaos-note"><Zap size={14} /> Chaos is on: every win at Slots, Wheel, Plinko, Dice and Coin Flip can be surged or cursed.</p>{/if}

      <div class="tabs" role="tablist" bind:this={tabsEl}>
        {#each NAV as n}
          <button role="tab" aria-selected={tab === n.id} class:active={tab === n.id} onclick={() => (tab = n.id)}>
            <n.icon size={15} /> {n.label}{#if n.id === 'bounties' && st.bounty_on_me > 0}<i class="dot" title="There's a bounty on you"></i>{/if}{#if n.id === 'lobby' && st.free.left > 0}<i class="dot gift" title="Free spin ready"></i>{/if}
          </button>
        {/each}
      </div>

      {#if st.lost_today > 0 && st.config.daily_loss_limit > 0}
        <div class="banner warn">You've lost {money(st.lost_today)} today. The limit is {money(st.config.daily_loss_limit)}; after that the tables close until tomorrow.</div>
      {/if}

      {#if tab === 'lobby'}
        <div class="lobby">
          {#if lastGame}
            <button class="resume" style="--a:{lastGame.hue[0]};--b:{lastGame.hue[1]}" onclick={() => (tab = lastGame.id)}>
              <span class="ico"><lastGame.icon size={26} /></span>
              <span class="txt"><small>Jump back in</small><b>{lastGame.label}</b></span>
              <span class="go"><Play size={16} /> Play</span>
            </button>
          {/if}
          {#if st.config.daily.enabled && st.config.daily.spins_per_day > 0}<DailySpin {st} onbalance={setBalance} onplayed={() => refresh()} />{/if}
          {#if st.double}{#key st.double.id}<DoubleOrNothing {st} offer={st.double} onbalance={setBalance} onplayed={() => refresh()} />{/key}{/if}

          <div class="tiles">
            {#each GAMES as g, i}
              <button class="tile" style="--a:{g.hue[0]};--b:{g.hue[1]}" disabled={!open(g.id)} onclick={() => (tab = g.id)}>
                <span class="art"><g.icon size={46} strokeWidth={1.6} /></span>
                <kbd class="hk" aria-hidden="true">{i + 1}</kbd>
                <b>{g.label}</b>
                <span class="blurb">{g.blurb}{#if top(g.id)}{' '}<em>{top(g.id)}</em>{/if}</span>
                <span class="rtp">{open(g.id) ? 'Pays back ' + (st.rtp[g.id as 'slots'] * 100).toFixed(0) + '%' : 'Closed'}</span>
              </button>
            {/each}
            <button class="tile wide" style="--a:#ef4444;--b:#7f1d1d" disabled={!st.config.bounties.enabled} onclick={() => (tab = 'bounties')}>
              <span class="art"><Crosshair size={40} strokeWidth={1.6} /></span><b>Bounties</b>
              <span class="blurb">Put a price on a rival's head, or hunt the most wanted for the reward.</span>
              {#if st.bounty_on_me > 0}<span class="rtp hot">{money(st.bounty_on_me)} on you</span>{/if}
            </button>
            <button class="tile wide" style="--a:#3b82f6;--b:#1e3a8a" disabled={!st.config.betting.enabled} onclick={() => (tab = 'betting')}>
              <span class="art"><Swords size={40} strokeWidth={1.6} /></span><b>Player betting</b>
              <span class="blurb">Will they get the kills? Back Yes or No and share the pool.</span>
            </button>
          </div>

          <div class="cols">
            <section class="panel">
              <h2><Trophy size={16} /> Big wins</h2>
              {#if st.feed.length}
                <ul class="feed">{#each st.feed as f}<li><b>{f.name}</b> won <span class="win">+{money(f.payout - f.bet)}</span> on {f.game}<small>{ago(f.at, now)}</small></li>{/each}</ul>
              {:else}<p class="muted"><Sparkles size={14} /> Nobody's hit it big yet. Be the first.</p>{/if}
            </section>
            <section class="panel">
              <h2><History size={16} /> Your play</h2>
              {#if hist && hist.rounds_played}
                <div class="stats"><div><span>Rounds</span><b>{hist.rounds_played}</b></div><div><span>Wagered</span><b>{money(hist.wagered)}</b></div><div><span>Net</span><b class:pos={net >= 0} class:neg={net < 0}>{net >= 0 ? '+' : ''}{money(net)}</b></div></div>
                <ul class="feed">{#each hist.rounds.slice(0, 6) as r}<li class="r"><span class="g">{r.game}</span>{money(r.bet)} <span class:win={r.payout > r.bet} class:lose={r.payout < r.bet}>{r.payout > r.bet ? '+' : ''}{money(r.payout - r.bet)}</span><small>{ago(r.at, now)}</small></li>{/each}</ul>
              {:else}<p class="muted">No rounds yet. Your first spin is free.</p>{/if}
            </section>
          </div>
        </div>
      {/if}
      <!-- Game tables stay alive (just hidden) once opened, so switching tabs never loses a round, an animation or a bet in progress. -->
      {#if visited.slots}<div class="pane" data-game="slots" hidden={tab !== 'slots'}><Pane active={tab === 'slots'}><Slots {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if visited.wheel}<div class="pane" data-game="wheel" hidden={tab !== 'wheel'}><Pane active={tab === 'wheel'}><Wheel {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if visited.plinko}<div class="pane" data-game="plinko" hidden={tab !== 'plinko'}><Pane active={tab === 'plinko'}><Plinko {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if visited.mines}<div class="pane" data-game="mines" hidden={tab !== 'mines'}><Pane active={tab === 'mines'}><Mines {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if visited.blackjack}<div class="pane" data-game="blackjack" hidden={tab !== 'blackjack'}><Pane active={tab === 'blackjack'}><Blackjack {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if visited.crash}<div class="pane" data-game="crash" hidden={tab !== 'crash'}><Pane active={tab === 'crash'}><Crash {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if visited.dice}<div class="pane" data-game="dice" hidden={tab !== 'dice'}><Pane active={tab === 'dice'}><Dice {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if visited.coinflip}<div class="pane" data-game="coinflip" hidden={tab !== 'coinflip'}><Pane active={tab === 'coinflip'}><CoinFlip {st} {balance} onbalance={setBalance} onplayed={() => refresh()} /></Pane></div>{/if}
      {#if tab === 'bounties'}<Bounties {st} {balance} onbalance={setBalance} onplayed={() => refresh()} />
      {:else if tab === 'betting'}<Betting {st} {balance} onbalance={setBalance} onplayed={() => refresh()} />
      {/if}
    {/if}
  {:else if !error}
    <p class="muted">Opening the casino…</p>
  {/if}

  {#if help}
    <div class="scrim" role="presentation" onclick={() => (help = false)}></div>
    <div class="sheet" data-casino-modal role="dialog" aria-modal="true" aria-label="Keyboard shortcuts">
      <header><h2><Keyboard size={18} /> Keyboard shortcuts</h2><button class="ghost icon" onclick={() => (help = false)} aria-label="Close"><X size={16} /></button></header>
      <div class="keys">
        {#each KEYS as g}
          <section><h3>{g.group}</h3>{#each g.rows as [k, what]}<div class="krow"><kbd>{k}</kbd><span>{what}</span></div>{/each}</section>
        {/each}
      </div>
      <label class="toggle"><input type="checkbox" checked={cprefs.keys} onchange={(e) => setKeys(e.currentTarget.checked)} /> Keyboard shortcuts on</label>
    </div>
  {/if}
</div>

<style>
  .casino { --gold: #f5c451; --felt: #0b1a14; position: relative; padding: 1.6rem 1.8rem 2.5rem; display: flex; flex-direction: column; gap: 1.1rem; height: 100%; overflow-y: auto; container-type: inline-size; container-name: casino; isolation: isolate; }
  .casino::before { content: ''; position: fixed; inset: 0; z-index: -1; pointer-events: none; background: radial-gradient(900px 480px at 85% -10%, color-mix(in srgb, var(--gold) 9%, transparent), transparent 70%), radial-gradient(700px 420px at 0% 30%, color-mix(in srgb, var(--accent) 10%, transparent), transparent 70%); }
  .casino.chaosmode { --accent: #a855f7; --gold: #e879f9; }
  .casino.chaosmode .bar { border-color: #e879f9aa; box-shadow: 0 0 40px -10px #c026d3, 0 18px 40px -26px #000, inset 0 1px 0 #ffffff12; animation: chaosPulse 3.2s ease-in-out infinite; }
  @keyframes chaosPulse { 50% { box-shadow: 0 0 18px -10px #c026d3, 0 18px 40px -26px #000, inset 0 1px 0 #ffffff12; } }
  .casino.compact { padding: 0.2rem 0 1.5rem; height: auto; overflow: visible; }
  .casino.compact::before { position: absolute; }
  .casino.compact .tabs { overflow-x: auto; flex-wrap: nowrap; scrollbar-width: none; }
  .casino.compact .tabs button { flex-shrink: 0; }
  .top { display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start; flex-wrap: wrap; }
  h1 { display: flex; gap: 0.6rem; align-items: center; } .lead { color: var(--muted); max-width: 40rem; margin-top: 0.2rem; font-size: 0.92rem; }
  .right { display: flex; gap: 0.6rem; align-items: center; } .right.solo { justify-content: flex-end; }

  /* the session bar */
  .bar { position: sticky; top: 0; z-index: 5; display: grid; grid-template-columns: auto auto minmax(0, 1fr) auto; gap: 0.8rem; align-items: center; padding: 0.65rem 0.8rem; border-radius: 1.1rem; background: linear-gradient(180deg, color-mix(in srgb, #fff 6%, var(--panel-solid, var(--surface))), var(--panel-solid, var(--surface))); border: 1px solid color-mix(in srgb, var(--gold) 22%, var(--line)); box-shadow: 0 18px 40px -26px #000, inset 0 1px 0 #ffffff12; backdrop-filter: blur(14px); }
  .wallet, .session { display: flex; align-items: center; gap: 0.6rem; min-width: 0; }
  .wallet { padding: 0.35rem 0.9rem 0.35rem 0.7rem; border-radius: 0.9rem; background: linear-gradient(135deg, color-mix(in srgb, var(--gold) 24%, #0000), #0000 70%), #00000030; border: 1px solid color-mix(in srgb, var(--gold) 40%, transparent); color: var(--gold); }
  .wallet small, .session small { display: block; font-size: 0.64rem; font-weight: 700; letter-spacing: 0.09em; text-transform: uppercase; color: var(--muted); line-height: 1.1; }
  .wallet b { display: block; font-size: 1.35rem; font-weight: 800; line-height: 1.15; color: #fff; letter-spacing: -0.01em; }
  .session b { display: block; font-size: 1.05rem; font-weight: 800; font-variant-numeric: tabular-nums; line-height: 1.15; }
  .session em { font-style: normal; font-size: 0.72rem; color: var(--muted); }
  .session.up { color: #4ade80; } .session.down { color: #f87171; } .session b { color: inherit; }
  .recent { display: flex; gap: 0.3rem; flex-wrap: nowrap; overflow: hidden; mask-image: linear-gradient(90deg, #000 85%, transparent); -webkit-mask-image: linear-gradient(90deg, #000 85%, transparent); }
  .pip { flex: none; font-size: 0.72rem; font-weight: 800; font-variant-numeric: tabular-nums; padding: 0.22rem 0.55rem; border-radius: 99rem; background: #ffffff10; color: var(--muted); }
  .pip.w { background: #22c55e22; color: #4ade80; } .pip.l { background: #ef444422; color: #f87171; }
  .switches { display: flex; gap: 0.35rem; align-items: center; }
  .chaos { display: inline-flex; align-items: center; gap: 0.45rem; padding: 0.4rem 0.55rem 0.4rem 0.75rem; border-radius: 99rem; font-weight: 800; font-size: 0.82rem; color: var(--muted); }
  .chaos .track { position: relative; width: 2.1rem; height: 1.2rem; border-radius: 99rem; background: #ffffff1c; transition: background 0.2s; display: block; }
  .chaos .knob { position: absolute; top: 0.15rem; left: 0.15rem; width: 0.9rem; height: 0.9rem; border-radius: 50%; background: #fff; box-shadow: 0 1px 4px #0006; transition: transform 0.22s cubic-bezier(0.3, 1.5, 0.5, 1); display: block; }
  .chaos.on { color: #fff; background: linear-gradient(135deg, #7c3aed, #db2777); border-color: #e879f9; box-shadow: 0 0 22px -4px #c026d3; }
  .chaos.on .track { background: #ffffff40; } .chaos.on .knob { transform: translateX(0.9rem); }
  .chaos.on :global(svg) { animation: zap 1.1s ease-in-out infinite; } @keyframes zap { 50% { transform: scale(1.25) rotate(-12deg); filter: drop-shadow(0 0 6px #fde68a); } }
  .chaos-note { margin: -0.4rem 0 0; display: flex; gap: 0.4rem; align-items: center; font-size: 0.8rem; color: #e9b8ff; }
  @container casino (max-width: 860px) {
    .bar { grid-template-columns: 1fr auto; } .recent { grid-column: 1 / -1; grid-row: 2; } .session { justify-self: end; } .switches { grid-column: 1 / -1; grid-row: 3; justify-content: flex-end; }
    .recent { order: 5; }
  }

  .banner { padding: 0.7rem 1rem; border-radius: var(--radius-sm); font-size: 0.88rem; } .banner.err { background: color-mix(in srgb, var(--danger) 16%, transparent); border: 1px solid var(--danger); } .banner.warn { background: color-mix(in srgb, var(--warn) 14%, transparent); border: 1px solid color-mix(in srgb, var(--warn) 50%, transparent); }
  .tabs { display: flex; gap: 0.3rem; flex-wrap: wrap; padding: 0.25rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); width: fit-content; max-width: 100%; }
  .tabs button { position: relative; display: inline-flex; gap: 0.4rem; align-items: center; padding: 0.5rem 0.95rem; border: none; background: transparent; color: var(--muted); border-radius: var(--radius-sm); font-size: 0.88rem; box-shadow: none; }
  .tabs button:hover { color: var(--text); background: color-mix(in srgb, var(--text) 6%, transparent); }
  .pane[hidden] { display: none !important; }
  .tabs button.active { color: #fff; background: var(--accent); box-shadow: 0 4px 16px -4px var(--accent); }
  .dot { position: absolute; top: 0.3rem; right: 0.35rem; width: 0.5rem; height: 0.5rem; border-radius: 50%; background: #ef4444; box-shadow: 0 0 8px #ef4444; } .dot.gift { background: #ec4899; box-shadow: 0 0 8px #ec4899; }

  /* tables fill the window instead of floating in it */
  /* Tables drawn at a fixed size grow with the window; the ones that already fill their space (Plinko, Crash, Dice, Blackjack) do not need to. */
  .pane:is([data-game='slots'], [data-game='wheel'], [data-game='mines'], [data-game='coinflip']) :global(.game > .stage) { zoom: var(--fit, 1); }
  @container casino (min-width: 761px) { .casino :global(.game) { grid-template-columns: clamp(19rem, 24cqw, 25rem) minmax(0, 1fr) !important; } }
  @container casino (max-width: 760px) {
    .casino :global(.game) { grid-template-columns: minmax(0, 1fr) !important; } .casino :global(.game > .stage) { order: -1; zoom: 1 !important; }
    .casino :global(.act) { position: sticky; bottom: calc(var(--pl-tab, 0px) + env(safe-area-inset-bottom, 0px) + 10px); z-index: 4; }
  }
  .casino :global(.game .panel.controls) { background: linear-gradient(180deg, color-mix(in srgb, #fff 4%, var(--panel-solid, var(--surface))), var(--panel-solid, var(--surface))); border-color: color-mix(in srgb, var(--gold) 18%, var(--line)); box-shadow: 0 24px 50px -34px #000; }
  /* the main button stays in reach while the controls scroll */
  @container casino (min-width: 761px) { .casino :global(.game .panel.controls .act) { position: sticky; bottom: 0.9rem; z-index: 4; padding-top: 0.5rem; background: linear-gradient(180deg, transparent, var(--panel-solid, var(--surface)) 28%); } }
  /* one main button everywhere: big, glowing, with its key on it */
  .casino :global(.spinbtn), .casino :global(.drop), .casino :global(.go), .casino :global(.cash), .casino :global(.act .big.go) {
    position: relative; min-height: 3.2rem; font-size: 1.05rem; font-weight: 800; letter-spacing: 0.01em; border-radius: 0.95rem; color: #1b1203; border: 1px solid color-mix(in srgb, var(--gold) 70%, #fff);
    background: linear-gradient(180deg, color-mix(in srgb, var(--gold) 80%, #fff), var(--gold) 55%, color-mix(in srgb, var(--gold) 80%, #7a4a00)); text-shadow: 0 1px 0 #fff6;
    box-shadow: inset 0 1px 0 #fff8, 0 14px 30px -12px var(--gold);
  }
  .casino :global(.spinbtn:hover:not(:disabled)), .casino :global(.drop:hover:not(:disabled)), .casino :global(.go:hover:not(:disabled)), .casino :global(.cash:hover:not(:disabled)) { filter: brightness(1.07); transform: translateY(-2px); }
  .casino :global(.spinbtn:not(:disabled)), .casino :global(.drop:not(:disabled)), .casino :global(.act .big.go:not(:disabled)) { animation: ready 2.6s ease-in-out infinite; }
  @keyframes ready { 50% { box-shadow: inset 0 1px 0 #fff8, 0 14px 36px -8px var(--gold); } }
  .casino :global(.k) { margin-left: 0.5rem; font: 700 0.62rem/1 var(--mono, monospace); padding: 0.22rem 0.4rem; border-radius: 0.35rem; background: #0003; border: 1px solid #0004; color: inherit; opacity: 0.8; vertical-align: middle; }

  /* lobby */
  .lobby { display: flex; flex-direction: column; gap: 1.1rem; }
  .resume { display: flex; align-items: center; gap: 1rem; padding: 0.8rem 1.1rem; border-radius: 1.1rem; text-align: left; color: var(--text); border: 1px solid color-mix(in srgb, var(--a) 50%, transparent); background: linear-gradient(100deg, color-mix(in srgb, var(--a) 28%, transparent), transparent 70%), color-mix(in srgb, var(--b) 25%, var(--surface)); }
  .resume .ico { width: 2.8rem; height: 2.8rem; border-radius: 0.8rem; display: grid; place-items: center; background: color-mix(in srgb, var(--a) 40%, #0000); color: #fff; }
  .resume .txt { flex: 1; } .resume small { display: block; font-size: 0.68rem; letter-spacing: 0.1em; text-transform: uppercase; color: var(--muted); font-weight: 700; } .resume b { font-size: 1.2rem; }
  .resume .go { display: inline-flex; gap: 0.35rem; align-items: center; font-weight: 800; padding: 0.5rem 1rem; border-radius: 99rem; background: var(--a); color: #fff; }
  .resume:hover { transform: translateY(-2px); box-shadow: 0 16px 36px -18px var(--a); }
  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr)); gap: 0.9rem; }
  .tile { position: relative; overflow: hidden; display: flex; flex-direction: column; gap: 0.4rem; align-items: flex-start; text-align: left; padding: 1.2rem 1.2rem 1rem; min-height: 11.5rem; border-radius: calc(var(--radius) + 2px); border: 1px solid color-mix(in srgb, var(--a) 40%, transparent); background: radial-gradient(130% 120% at 100% 0%, color-mix(in srgb, var(--a) 38%, transparent), transparent 60%), linear-gradient(160deg, color-mix(in srgb, var(--b) 40%, var(--surface)), var(--surface)); color: var(--text); transition: transform 0.18s var(--ease), box-shadow 0.18s, border-color 0.18s; }
  .tile:hover:not(:disabled) { transform: translateY(-4px); box-shadow: 0 18px 40px -16px var(--a); border-color: var(--a); }
  .tile:disabled { opacity: 0.5; filter: grayscale(0.7); cursor: not-allowed; }
  .tile .hk { position: absolute; top: 0.7rem; left: 0.8rem; z-index: 2; font: 700 0.66rem/1 var(--mono, monospace); padding: 0.2rem 0.38rem; border-radius: 0.35rem; background: #0005; color: #fffb; border: 1px solid #ffffff22; }
  .tile .art { position: absolute; right: -0.6rem; top: -0.4rem; opacity: 0.22; transform: rotate(14deg) scale(2.2); transform-origin: top right; color: var(--a); transition: transform 0.3s var(--ease), opacity 0.3s; }
  .tile:hover:not(:disabled) .art { transform: rotate(4deg) scale(2.5); opacity: 0.4; }
  .tile b { font-size: 1.25rem; z-index: 1; margin-top: 1rem; } .tile.wide b { margin-top: 0; } .tile .blurb { color: color-mix(in srgb, var(--text) 78%, transparent); font-size: 0.85rem; z-index: 1; flex: 1; } .blurb em { font-style: normal; font-weight: 800; color: var(--text); }
  .tile .rtp { z-index: 1; font-size: 0.72rem; font-weight: 700; padding: 0.15rem 0.6rem; border-radius: 99rem; background: #0006; color: color-mix(in srgb, var(--a) 60%, #fff); } .rtp.hot { background: #ef4444; color: #fff; }
  .tile.wide { min-height: 8rem; }
  .cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr)); gap: 1rem; align-items: start; }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.7rem; } .panel h2 { display: flex; gap: 0.45rem; align-items: center; font-size: 1rem; }
  .feed { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; } .feed li { display: flex; gap: 0.4rem; align-items: baseline; flex-wrap: wrap; padding: 0.4rem 0; border-bottom: 1px solid var(--line); font-size: 0.88rem; } .feed small { margin-left: auto; color: var(--muted); font-size: 0.72rem; }
  .feed .g { text-transform: capitalize; color: var(--muted); min-width: 4.2rem; } .win { color: #4ade80; font-weight: 700; } .lose { color: #f87171; }
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.4rem; } .stats div { background: color-mix(in srgb, var(--text) 5%, transparent); border-radius: var(--radius-sm); padding: 0.45rem 0.7rem; display: flex; flex-direction: column; } .stats span { font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; } .stats b { font-variant-numeric: tabular-nums; } .pos { color: #4ade80; } .neg { color: #f87171; }
  .muted { color: var(--muted); font-size: 0.88rem; display: flex; gap: 0.4rem; align-items: center; }
  .closed { display: grid; place-items: center; text-align: center; gap: 0.5rem; padding: 4rem 1rem; color: var(--muted); } .closed h2 { color: var(--text); }
  :global(.spin) { animation: spin 1s linear infinite; } @keyframes spin { to { transform: rotate(360deg); } }

  /* keyboard help */
  .scrim { position: fixed; inset: 0; z-index: 60; background: #000a; backdrop-filter: blur(4px); }
  .sheet { position: fixed; z-index: 61; left: 50%; top: 50%; transform: translate(-50%, -50%); width: min(40rem, calc(100vw - 1.6rem)); max-height: calc(100vh - 2rem); overflow: auto; padding: 1.2rem 1.4rem 1.3rem; border-radius: 1.3rem; background: var(--panel-solid, var(--surface)); border: 1px solid color-mix(in srgb, var(--gold) 30%, var(--line)); box-shadow: 0 40px 90px -20px #000; display: flex; flex-direction: column; gap: 1rem; animation: pop 0.2s var(--ease); }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, -46%) scale(0.96); } }
  .sheet header { display: flex; justify-content: space-between; align-items: center; } .sheet h2 { display: flex; gap: 0.5rem; align-items: center; font-size: 1.1rem; }
  .keys { display: grid; grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr)); gap: 1.1rem 1.6rem; } .keys h3 { font-size: 0.68rem; letter-spacing: 0.1em; text-transform: uppercase; color: var(--gold); margin-bottom: 0.4rem; }
  .krow { display: flex; gap: 0.7rem; align-items: baseline; padding: 0.28rem 0; font-size: 0.85rem; } .krow kbd { flex: none; min-width: 5.2rem; font: 700 0.72rem/1.2 var(--mono, monospace); padding: 0.2rem 0.45rem; border-radius: 0.4rem; background: var(--surface-3, #25272f); border: 1px solid var(--line-strong); color: var(--text); text-align: center; } .krow span { color: var(--muted); }
  .toggle { display: flex; gap: 0.5rem; align-items: center; font-size: 0.85rem; color: var(--muted); }
  @media (max-width: 640px) { .casino { padding: 1rem 0.9rem 2rem; } .lobby .tiles { grid-template-columns: 1fr 1fr; } .tile { min-height: 9.5rem; padding: 0.9rem; } .tile.wide { grid-column: span 2; } .tabs { flex-wrap: nowrap; overflow-x: auto; scrollbar-width: none; width: 100%; scroll-snap-type: x proximity; mask-image: linear-gradient(90deg, transparent 0, #000 14px, #000 calc(100% - 22px), transparent 100%); -webkit-mask-image: linear-gradient(90deg, transparent 0, #000 14px, #000 calc(100% - 22px), transparent 100%); } .tabs::-webkit-scrollbar { display: none; } .tabs button { flex-shrink: 0; scroll-snap-align: center; padding: 0.6rem 0.9rem; } .cols { grid-template-columns: minmax(0, 1fr); } .tile .hk { display: none; } }
  @media (hover: none) { .tile .hk, .casino :global(.k), .casino :global(.chip kbd) { display: none; } }
</style>
