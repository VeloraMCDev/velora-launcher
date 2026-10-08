<script lang="ts">
  import { playPath } from '../../lib/router.svelte';
  import { onMount } from 'svelte';
  import { ArrowDownLeft, ArrowUpRight, Banknote, Crown, Dices, Landmark, RefreshCw, Shield, Store, TriangleAlert, Wallet as WalletIcon, Repeat, Trophy } from '@lucide/svelte';
  import { get } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import Avatar from '../../components/Avatar.svelte';
  import { compact, currentServer, money, play } from '../store.svelte';
  import Count from '../ui/Count.svelte';
  import Empty from '../ui/Empty.svelte';
  import { clock, dayLabel, parseTime } from '../lib/money-pages';

  type Bal = { server_id: number; server_name: string; balance: number };
  type Tx = { id: number; server_id: number; server_name: string | null; from_uuid: string; from_name: string; to_uuid: string; to_name: string; amount: number; description: string; created_at: string };
  type Bt = { rank: number; uuid: string; username: string; balance: number };
  type GuildWallet = { balance: number; role: string; my_balance: number; transactions: { id: number; actor_uuid: string; kind: string; amount: number; created_at: string; note: string }[] };
  type Filter = 'all' | 'in' | 'out' | 'market' | 'casino';

  let balances = $state<Bal[]>([]);
  let txs = $state<Tx[]>([]);
  let baltop = $state<Bt[]>([]);
  let guild = $state<{ id: string; name: string; tag: string } | null>(null);
  let gwallet = $state<GuildWallet | null>(null);
  let loading = $state(true);
  let topLoading = $state(false);
  let error = $state('');
  let filter = $state<Filter>('all');
  let refreshing = $state(false);

  const me = $derived(session.user?.uuid ?? '');
  const mine = (t: Tx) => t.to_uuid === me;
  const isMarket = (t: Tx) => /^(market|auction)/i.test(t.description);
  const isCasino = (t: Tx) => /casino|slots|wheel|plinko|mines|blackjack|crash|dice|coin flip|double or nothing|daily spin|bet/i.test(t.description);
  const net = $derived(balances.reduce((s, b) => s + b.balance, 0));
  const selected = $derived(balances.find((b) => b.server_id === play.serverId) ?? null);

  async function loadAll(quiet = false) {
    if (!quiet) loading = true; else refreshing = true;
    try {
      const [b, t] = await Promise.all([get<Bal[]>('/api/v1/economy/me'), get<Tx[]>('/api/v1/economy/transactions')]);
      balances = b; txs = t; error = '';
    } catch (e) { error = e instanceof Error ? e.message : 'Could not load your wallet'; }
    finally { loading = false; refreshing = false; }
  }
  async function loadTop() {
    const sid = play.serverId;
    baltop = [];
    if (sid == null) return;
    topLoading = true;
    try { const r = await get<Bt[]>(`/api/v1/servers/${sid}/economy/baltop`); if (sid === play.serverId) baltop = r; } catch { /* leaderboard is optional */ }
    finally { if (sid === play.serverId) topLoading = false; }
  }
  async function loadGuild() {
    const sid = play.serverId;
    guild = null; gwallet = null;
    if (sid == null) return;
    const inst = currentServer()?.instance_id ?? '';
    try {
      const g = await get<{ id: string; name: string; tag: string } | null>(`/api/v1/guilds/my${inst ? `?instance_id=${encodeURIComponent(inst)}` : ''}`);
      if (sid !== play.serverId || !g) return;
      const w = await get<GuildWallet>(`/api/v1/guilds/${g.id}/wallet?server_id=${sid}`);
      if (sid === play.serverId) { guild = g; gwallet = w; }
    } catch { /* not in a guild, or no access */ }
  }

  onMount(() => {
    void loadAll();
    const t = setInterval(() => { if (!document.hidden) void loadAll(true); }, 30000);
    return () => clearInterval(t);
  });
  $effect(() => { void play.serverId; void loadTop(); void loadGuild(); });
  $effect(() => { void play.tick; if (!loading) void loadAll(true); });

  const shown = $derived(txs.filter((t) => filter === 'all' || (filter === 'in' && mine(t)) || (filter === 'out' && !mine(t)) || (filter === 'market' && isMarket(t)) || (filter === 'casino' && isCasino(t))));
  const groups = $derived.by(() => {
    const out: { day: string; items: Tx[]; sum: number }[] = [];
    for (const t of shown) {
      const d = dayLabel(t.created_at);
      let g = out[out.length - 1];
      if (!g || g.day !== d) { g = { day: d, items: [], sum: 0 }; out.push(g); }
      g.items.push(t); g.sum += mine(t) ? t.amount : -t.amount;
    }
    return out;
  });
  const flow = $derived.by(() => {
    const week = Date.now() - 7 * 86400e3;
    let inn = 0, out = 0;
    for (const t of txs) if (parseTime(t.created_at) >= week) { if (mine(t)) inn += t.amount; else out += t.amount; }
    return { inn, out };
  });
  const myRank = $derived(baltop.find((b) => b.uuid === me));
  const maxBal = $derived(Math.max(1, ...balances.map((b) => b.balance)));
  const counter = (t: Tx) => (mine(t) ? t.from_name : t.to_name);
  const kindIcon = (t: Tx) => (isMarket(t) ? Store : isCasino(t) ? Dices : mine(t) ? ArrowDownLeft : ArrowUpRight);
  const chips: [Filter, string][] = [['all', 'All'], ['in', 'Money in'], ['out', 'Money out'], ['market', 'Market'], ['casino', 'Casino']];
</script>

<div class="pl-page wallet">
  <div class="pl-head">
    <div><h1><Banknote size={26} /> Wallet</h1><p>Your money across every server</p></div>
    <button class="pl-iconbtn" aria-label="Refresh" aria-busy={refreshing} onclick={() => { void loadAll(true); void loadTop(); void loadGuild(); }}><RefreshCw size={16} class={refreshing ? 'spin' : ''} /></button>
  </div>

  {#if error}
    <div class="pl-alert err row" role="alert" style="margin-bottom: 14px"><TriangleAlert size={16} /><span class="grow">{error}</span><button class="pl-btn sm" onclick={() => loadAll()}>Retry</button></div>
  {/if}

  <div class="pl-stack">
    <section class="pl-hero hero">
      {#if loading}
        <div class="pl-skel" style="height: 18px; width: 120px"></div><div class="pl-skel" style="height: 56px; width: 220px; margin: 12px 0"></div><div class="pl-skel" style="height: 40px"></div>
      {:else}
        <span class="kick"><WalletIcon size={14} /> Net worth{balances.length > 1 ? ` across ${balances.length} servers` : ''}</span>
        <div class="big"><Count value={net} format={(v) => money(v)} /></div>
        {#if selected && balances.length > 1}<p class="onsel">{currentServer()?.name}: <b>{money(selected.balance)}</b></p>{/if}
        <div class="flow">
          <div><small>In · last 7 days</small><b class="pos"><ArrowDownLeft size={14} /> {money(flow.inn, 0)}</b></div>
          <div><small>Out · last 7 days</small><b class="neg"><ArrowUpRight size={14} /> {money(flow.out, 0)}</b></div>
          {#if myRank}<div><small>Rank here</small><b><Trophy size={14} /> #{myRank.rank}</b></div>{/if}
        </div>
      {/if}
    </section>

    <div class="cols">
      <div class="pl-stack left">
        <section class="pl-card o1">
          <div class="pl-card-head"><h2><Landmark size={17} /> Balances</h2></div>
          {#if loading}
            {#each Array(2) as _, i (i)}<div class="pl-skel" style="height: 54px; margin-bottom: 8px"></div>{/each}
          {:else if balances.length}
            <div class="bals">
              {#each balances as b, i (b.server_id)}
                <div class="bal" class:on={b.server_id === play.serverId} style="--i: {i}">
                  <div class="bt"><b>{b.server_name}</b><span class="pl-chip" class:accent={b.server_id === play.serverId}>{b.server_id === play.serverId ? 'Selected' : 'Server'}</span></div>
                  <div class="amt">{money(b.balance)}</div>
                  <div class="pl-progress"><i style="width: {Math.max(4, (b.balance / maxBal) * 100)}%"></i></div>
                </div>
              {/each}
            </div>
          {:else}
            <Empty icon={WalletIcon} title="No balance yet" text="Join a server once and your starting balance appears here." />
          {/if}
        </section>

        {#if guild && gwallet}
          <section class="pl-card guild o3">
            <div class="pl-card-head"><h2><Shield size={17} /> {guild.name} <span class="pl-chip accent">[{guild.tag}]</span></h2><a class="more" href={playPath('guilds')}>Open guild</a></div>
            <div class="pl-stats">
              <div class="pl-stat"><span>Guild bank</span><b><Count value={gwallet.balance} format={(v) => money(v)} /></b><small>your role: {gwallet.role}</small></div>
              <div class="pl-stat"><span>Your balance</span><b>{money(gwallet.my_balance)}</b><small>available to deposit</small></div>
            </div>
            {#if gwallet.transactions.length}
              <div class="pl-list" style="margin-top: 10px">
                {#each gwallet.transactions.slice(0, 4) as g (g.id)}
                  <div class="pl-item"><span class="ic" class:in={g.kind === 'deposit'} class:out={g.kind !== 'deposit'}>{#if g.kind === 'deposit'}<ArrowDownLeft size={15} />{:else}<ArrowUpRight size={15} />{/if}</span>
                    <div class="grow"><b style="text-transform: capitalize">{g.kind}</b><span class="sub">{g.note || dayLabel(g.created_at)}</span></div>
                    <div class="end"><b class={g.kind === 'deposit' ? 'pos' : 'neg'}>{g.kind === 'deposit' ? '+' : '−'}{money(g.amount)}</b></div></div>
                {/each}
              </div>
            {/if}
          </section>
        {/if}

        <section class="pl-card o4">
          <div class="pl-card-head"><h2><Crown size={17} /> Richest on {currentServer()?.name ?? 'this server'}</h2></div>
          {#if topLoading}
            {#each Array(5) as _, i (i)}<div class="pl-skel" style="height: 46px; margin-bottom: 8px"></div>{/each}
          {:else if baltop.length}
            <div class="pl-list">
              {#each baltop.slice(0, 10) as r, i (r.uuid)}
                <div class="pl-item" class:me={r.uuid === me} style="--i: {i}">
                  <span class="rank" class:g={r.rank === 1} class:s={r.rank === 2} class:b={r.rank === 3}>{r.rank}</span>
                  <Avatar name={r.username} uuid={r.uuid} size={34} />
                  <div class="grow"><b>{r.username}{#if r.uuid === me} <span class="pl-chip accent">You</span>{/if}</b></div>
                  <div class="end"><b>{money(r.balance, 0)}</b></div>
                </div>
              {/each}
              {#if myRank && myRank.rank > 10}
                <div class="pl-item me"><span class="rank">{myRank.rank}</span><Avatar name={myRank.username} uuid={myRank.uuid} size={34} /><div class="grow"><b>{myRank.username} <span class="pl-chip accent">You</span></b></div><div class="end"><b>{money(myRank.balance, 0)}</b></div></div>
              {/if}
            </div>
          {:else}
            <Empty icon={Crown} title="No ranking yet" text="Nobody has a balance on this server so far." />
          {/if}
        </section>
      </div>

      <section class="pl-card feed o2">
        <div class="pl-card-head"><h2><Repeat size={17} /> Transactions</h2><span class="pl-chip">{shown.length}</span></div>
        <div class="pl-scroll-x chips">{#each chips as [id, label] (id)}<button class="fchip" class:on={filter === id} onclick={() => (filter = id)}>{label}</button>{/each}</div>
        {#if loading}
          {#each Array(6) as _, i (i)}<div class="pl-skel" style="height: 52px; margin-top: 8px"></div>{/each}
        {:else if groups.length}
          {#each groups as g (g.day)}
            <div class="day"><span>{g.day}</span><span class={g.sum >= 0 ? 'pos' : 'neg'}>{g.sum >= 0 ? '+' : '−'}{compact(Math.abs(g.sum))}</span></div>
            <div class="pl-list">
              {#each g.items as t (t.id)}
                {@const Ic = kindIcon(t)}
                <div class="pl-item">
                  <span class="ic" class:in={mine(t)} class:out={!mine(t)}><Ic size={16} /></span>
                  <div class="grow">
                    <b>{t.description || (mine(t) ? 'Received' : 'Sent')}</b>
                    <span class="sub">{mine(t) ? 'from' : 'to'} {counter(t)} · {clock(t.created_at)}{#if t.server_name && balances.length > 1} · <span class="srv">{t.server_name}</span>{/if}</span>
                  </div>
                  <div class="end"><b class={mine(t) ? 'pos' : 'neg'}>{mine(t) ? '+' : '−'}{money(t.amount)}</b></div>
                </div>
              {/each}
            </div>
          {/each}
        {:else}
          <Empty icon={Repeat} title={txs.length ? 'No transactions match' : 'No transactions yet'} text={txs.length ? 'Try another filter.' : 'Buy from the market or play the casino and it will show up here.'} />
        {/if}
      </section>
    </div>
  </div>
</div>

<style>
  h1 :global(svg) { color: var(--accent-2); }
  .row { display: flex; align-items: center; gap: 10px; } .grow { flex: 1; min-width: 0; }
  .hero { padding: 24px; }
  .kick { display: inline-flex; align-items: center; gap: 7px; font-size: 0.74rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.09em; color: var(--text-2); }
  .big { font-size: clamp(2.4rem, 11vw, 3.6rem); font-weight: 800; letter-spacing: -0.03em; line-height: 1.1; margin: 8px 0 4px; background: linear-gradient(180deg, #fff, color-mix(in srgb, var(--accent-2) 70%, #fff)); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .onsel { font-size: 0.9rem; margin: 0 0 6px; }
  .flow { display: flex; gap: 22px; flex-wrap: wrap; margin-top: 14px; padding-top: 14px; border-top: 1px solid var(--pl-line); }
  .flow small { display: block; font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.07em; font-weight: 600; margin-bottom: 2px; }
  .flow b { display: inline-flex; align-items: center; gap: 5px; font-size: 1.1rem; font-variant-numeric: tabular-nums; }
  .cols { display: grid; gap: 14px; grid-template-columns: minmax(0, 5fr) minmax(0, 6fr); align-items: start; }
  @media (max-width: 880px) { .cols { grid-template-columns: minmax(0, 1fr); } .left { display: contents; } .o1 { order: 1; } .o2 { order: 2; } .o3 { order: 3; } .o4 { order: 4; } }
  .bals { display: flex; flex-direction: column; gap: 10px; }
  .bal { padding: 12px 14px; border-radius: 14px; background: rgba(255, 255, 255, 0.04); border: 1px solid var(--pl-line); display: grid; gap: 6px; animation: rise 0.4s var(--ease, ease) both; animation-delay: calc(var(--i) * 50ms); }
  .bal.on { border-color: color-mix(in srgb, var(--accent) 55%, transparent); background: color-mix(in srgb, var(--accent) 10%, transparent); }
  .bt { display: flex; align-items: center; justify-content: space-between; gap: 8px; } .bt b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .amt { font-size: 1.5rem; font-weight: 750; font-variant-numeric: tabular-nums; letter-spacing: -0.02em; }
  @keyframes rise { from { opacity: 0; transform: translateY(8px); } }
  .ic { width: 36px; height: 36px; border-radius: 11px; display: grid; place-items: center; flex-shrink: 0; background: rgba(255, 255, 255, 0.07); }
  .ic.in { color: var(--good); background: color-mix(in srgb, var(--good) 15%, transparent); } .ic.out { color: var(--bad); background: color-mix(in srgb, var(--bad) 13%, transparent); }
  .chips { margin-bottom: 4px; }
  .fchip { padding: 8px 14px; border-radius: 99px; background: rgba(255, 255, 255, 0.06); border: 1px solid var(--pl-line); color: var(--text-2); font-weight: 600; font-size: 0.84rem; }
  .fchip.on { background: var(--accent); border-color: transparent; color: #fff; box-shadow: 0 6px 16px -8px var(--accent); }
  .day { display: flex; justify-content: space-between; margin: 14px 0 2px; font-size: 0.72rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
  .grow b :global(.pl-chip) { margin-left: 6px; }
  .srv { color: var(--accent-2); }
  .rank { width: 26px; text-align: center; font-weight: 800; color: var(--muted); font-variant-numeric: tabular-nums; flex-shrink: 0; }
  .rank.g { color: #f5d97a; } .rank.s { color: #cfd6e0; } .rank.b { color: #d9965b; }
  .pl-item.me { background: color-mix(in srgb, var(--accent) 14%, transparent); border-radius: 12px; padding-inline: 8px; margin-inline: -8px; border-bottom-color: transparent; }
  .guild { background: linear-gradient(160deg, color-mix(in srgb, var(--accent-2) 10%, var(--surface)), var(--pl-glass)); }
  @media (prefers-reduced-motion: reduce) { .bal { animation: none; } }
</style>
