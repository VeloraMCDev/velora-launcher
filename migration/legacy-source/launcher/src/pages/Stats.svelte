<script lang="ts">
  import PlayerLink from '../components/PlayerLink.svelte';
  import {
    Trophy, Swords, Skull, Pickaxe, Clock, Box, MessageSquare, LogIn,
    RefreshCw, LoaderCircle, Users, Server, Flame, Sparkles, ChevronDown, Check,
    Coins, DollarSign, Wallet, ArrowUpRight, ArrowDownLeft, TrendingUp, History,
  } from '@lucide/svelte';
  import Avatar from '../components/Avatar.svelte';
  import { activeAccount, app, toast } from '../lib/store.svelte';
  import { invoke } from '../lib/tauri';
  import type { ServerEconomyBalance, BaltopEntry, EconomyTransaction } from '../lib/types';

  let activeTab = $state<'personal' | 'economy' | 'leaderboard'>('personal');
  let loadingPersonal = $state(false);
  let loadingLeaderboard = $state(false);
  let loadingEconomy = $state(false);

  // Economy state
  let economyBalances = $state<ServerEconomyBalance[]>([]);
  let selectedEconomyServerId = $state<number | null>(null);
  let baltopData = $state<BaltopEntry[]>([]);
  let transactions = $state<EconomyTransaction[]>([]);

  // Personal stats data
  let personalData = $state<{
    username: string;
    uuid: string;
    total: {
      playtime_secs: number;
      joins: number;
      deaths: number;
      player_kills: number;
      mob_kills: number;
      blocks_broken: number;
      blocks_placed: number;
      messages: number;
      first_seen: string;
      last_seen: string;
    } | null;
    servers: Array<{
      server_id: number;
      server_name: string;
      name: string;
      playtime_secs: number;
      joins: number;
      deaths: number;
      player_kills: number;
      mob_kills: number;
      blocks_broken: number;
      blocks_placed: number;
      messages: number;
      first_seen: string;
      last_seen: string;
    }>;
    events: Array<{
      id: number;
      server_id: number;
      kind: string;
      detail: string | null;
      created_at: string;
    }>;
  } | null>(null);

  // Leaderboard data
  let servers = $state<Array<{ id: number; name: string; online: boolean; players_online: number; players_max: number }>>([]);
  let selectedServerId = $state<number | null>(null);
  let sortField = $state<string>('playtime_secs');
  let leaderboardData = $state<{
    leaderboard: Array<{
      uuid: string;
      name: string;
      playtime_secs: number;
      joins: number;
      deaths: number;
      player_kills: number;
      mob_kills: number;
      blocks_broken: number;
      blocks_placed: number;
      messages: number;
      first_seen: string;
      last_seen: string;
    }>;
    totals: { players: number; playtime_secs: number };
    sort: string;
  } | null>(null);

  const sortOptions = [
    { id: 'playtime_secs', label: 'Playtime', icon: Clock },
    { id: 'player_kills', label: 'Player Kills', icon: Swords },
    { id: 'mob_kills', label: 'Mob Kills', icon: Flame },
    { id: 'blocks_broken', label: 'Blocks Mined', icon: Pickaxe },
    { id: 'blocks_placed', label: 'Blocks Placed', icon: Box },
    { id: 'joins', label: 'Server Joins', icon: LogIn },
  ];

  function formatPlaytime(secs: number): string {
    if (!secs || secs < 60) return `${secs || 0}s`;
    const hours = Math.floor(secs / 3600);
    const mins = Math.floor((secs % 3600) / 60);
    if (hours > 0) return `${hours}h ${mins}m`;
    return `${mins}m`;
  }

  function formatNumber(num: number): string {
    return (num ?? 0).toLocaleString();
  }

  function formatDate(d: string): string {
    if (!d) return 'Never';
    try {
      return new Date(d).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
    } catch {
      return d;
    }
  }

  async function loadPersonalStats() {
    if (!activeAccount() || activeAccount()?.kind === 'offline') {
      personalData = null;
      return;
    }
    loadingPersonal = true;
    try {
      personalData = await invoke<typeof personalData>('get_player_stats');
    } catch {
      personalData = null;
    } finally {
      loadingPersonal = false;
    }
  }

  async function loadServers() {
    try {
      const res = await invoke<{ servers: typeof servers }>('get_public_servers');
      servers = res.servers || [];
    } catch {
      servers = [];
    }
  }

  async function loadLeaderboard() {
    loadingLeaderboard = true;
    try {
      leaderboardData = await invoke<typeof leaderboardData>('get_leaderboard', {
        serverId: selectedServerId,
        sort: sortField,
      });
    } catch {
      leaderboardData = null;
    } finally {
      loadingLeaderboard = false;
    }
  }

  async function loadEconomy() {
    loadingEconomy = true;
    try {
      const [balances, txs] = await Promise.all([
        invoke<ServerEconomyBalance[]>('get_economy_balances').catch(() => []),
        invoke<EconomyTransaction[]>('get_transactions').catch(() => []),
      ]);
      economyBalances = balances;
      transactions = txs;
      if (!selectedEconomyServerId && balances.length > 0) {
        selectedEconomyServerId = balances[0].server_id;
      }
      if (selectedEconomyServerId) {
        await loadBaltop(selectedEconomyServerId);
      }
    } catch (e: any) {
      toast(e?.message ?? 'Failed to load economy data', 'error');
    } finally {
      loadingEconomy = false;
    }
  }

  async function loadBaltop(serverId: number) {
    try {
      baltopData = await invoke<BaltopEntry[]>('get_server_baltop', { serverId });
    } catch (e) {
      baltopData = [];
    }
  }

  function handleEconomyServerChange(serverId: number) {
    selectedEconomyServerId = serverId;
    loadBaltop(serverId);
  }

  $effect(() => {
    if (app.view === 'stats') {
      loadPersonalStats();
      loadServers();
      loadLeaderboard();
      loadEconomy();
    }
  });

  $effect(() => {
    void selectedServerId;
    void sortField;
    if (app.view === 'stats') {
      loadLeaderboard();
    }
  });

  const me = $derived(activeAccount());
  const top3 = $derived(leaderboardData?.leaderboard.slice(0, 3) ?? []);
  const rest = $derived(leaderboardData?.leaderboard.slice(3) ?? []);

  function statValue(row: any, field: string): string {
    if (field === 'playtime_secs') return formatPlaytime(row.playtime_secs);
    return formatNumber(row[field] ?? 0);
  }
</script>

<div class="stats-page">
  <header class="page-header">
    <div class="title-wrap">
      <h1><Trophy size={24} class="trophy-icon" /> Stats & Leaderboards</h1>
      <p class="lead">Track your Minecraft journey and see how you rank across servers.</p>
    </div>
    <div class="tabs-segmented">
      <button class:active={activeTab === 'personal'} onclick={() => (activeTab = 'personal')}>
        My Stats
      </button>
      <button class:active={activeTab === 'economy'} onclick={() => { activeTab = 'economy'; loadEconomy(); }}>
        <Coins size={14} /> Server Economy
      </button>
      <button class:active={activeTab === 'leaderboard'} onclick={() => (activeTab = 'leaderboard')}>
        Leaderboards
      </button>
    </div>
  </header>

  <div class="page-content">
    {#if activeTab === 'personal'}
      {#if loadingPersonal}
        <div class="center-state"><LoaderCircle size={28} class="spin" /></div>
      {:else if !me || me.kind === 'offline'}
        <div class="card glass empty-state">
          <Users size={36} class="muted" />
          <h3>Server account required</h3>
          <p class="muted small">Personal gameplay stats are synced when you play on servers with your SCOPENET server account.</p>
          <button class="sm primary" onclick={() => (app.addAccount = true)}>Sign in with Server Account</button>
        </div>
      {:else if !personalData || !personalData.total}
        <div class="card glass empty-state">
          <Sparkles size={36} class="muted" />
          <h3>No recorded stats yet</h3>
          <p class="muted small">Hop into a server through the launcher to begin logging kills, blocks mined, and playtime!</p>
        </div>
      {:else}
        {@const t = personalData.total}
        {@const kd = (t.player_kills / Math.max(1, t.deaths)).toFixed(2)}
        <div class="player-hero card glass">
          <Avatar account={me} size={4.5} />
          <div class="hero-details">
            <div class="row align-center gap-sm">
              <strong class="player-name">{personalData.username}</strong>
              <span class="badge online">Active</span>
            </div>
            <div class="hero-meta tiny muted">
              <span>UUID: {personalData.uuid.slice(0, 14)}…</span>
              <span>·</span>
              <span>First played: {formatDate(t.first_seen)}</span>
              <span>·</span>
              <span>Last active: {formatDate(t.last_seen)}</span>
            </div>
          </div>
          <div class="hero-playtime">
            <span class="tiny muted uppercase">Total Playtime</span>
            <strong class="big-stat">{formatPlaytime(t.playtime_secs)}</strong>
          </div>
        </div>

        <div class="stats-grid">
          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Playtime</span>
              <Clock size={18} class="icon-accent" />
            </div>
            <div class="stat-value">{formatPlaytime(t.playtime_secs)}</div>
            <div class="stat-sub tiny muted">Across all servers</div>
          </div>

          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Player Kills</span>
              <Swords size={18} class="icon-success" />
            </div>
            <div class="stat-value">{formatNumber(t.player_kills)}</div>
            <div class="stat-sub tiny muted">PvP eliminations</div>
          </div>

          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Deaths</span>
              <Skull size={18} class="icon-danger" />
            </div>
            <div class="stat-value">{formatNumber(t.deaths)}</div>
            <div class="stat-sub tiny muted">K/D: {kd}</div>
          </div>

          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Mob Kills</span>
              <Flame size={18} class="icon-warn" />
            </div>
            <div class="stat-value">{formatNumber(t.mob_kills)}</div>
            <div class="stat-sub tiny muted">Monsters & animals</div>
          </div>

          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Blocks Mined</span>
              <Pickaxe size={18} class="icon-accent" />
            </div>
            <div class="stat-value">{formatNumber(t.blocks_broken)}</div>
            <div class="stat-sub tiny muted">Broken blocks</div>
          </div>

          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Blocks Placed</span>
              <Box size={18} class="icon-accent" />
            </div>
            <div class="stat-value">{formatNumber(t.blocks_placed)}</div>
            <div class="stat-sub tiny muted">Structures built</div>
          </div>

          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Server Joins</span>
              <LogIn size={18} class="icon-muted" />
            </div>
            <div class="stat-value">{formatNumber(t.joins)}</div>
            <div class="stat-sub tiny muted">Sessions played</div>
          </div>

          <div class="stat-card glass">
            <div class="stat-header">
              <span class="stat-label">Chat Messages</span>
              <MessageSquare size={18} class="icon-muted" />
            </div>
            <div class="stat-value">{formatNumber(t.messages)}</div>
            <div class="stat-sub tiny muted">Messages sent</div>
          </div>
        </div>

        {#if personalData.servers.length}
          <div class="card glass section-card">
            <h3>Server Activity</h3>
            <div class="server-rows">
              {#each personalData.servers as s}
                <div class="server-stat-row">
                  <div class="server-info">
                    <strong>{s.server_name}</strong>
                    <span class="tiny muted">Last played {formatDate(s.last_seen)}</span>
                  </div>
                  <div class="server-metrics">
                    <div class="metric">
                      <span class="tiny muted">Playtime</span>
                      <strong>{formatPlaytime(s.playtime_secs)}</strong>
                    </div>
                    <div class="metric">
                      <span class="tiny muted">Kills</span>
                      <strong>{s.player_kills}</strong>
                    </div>
                    <div class="metric">
                      <span class="tiny muted">Deaths</span>
                      <strong>{s.deaths}</strong>
                    </div>
                    <div class="metric">
                      <span class="tiny muted">Mined</span>
                      <strong>{formatNumber(s.blocks_broken)}</strong>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/if}
    {:else if activeTab === 'economy'}
      <!-- Economy & Balances Tab -->
      <div class="economy-view">
        {#if loadingEconomy && !economyBalances.length}
          <div class="center-state"><LoaderCircle size={28} class="spin" /></div>
        {:else if !me || me.kind === 'offline'}
          <div class="card glass empty-state">
            <Coins size={36} class="muted" />
            <h3>Server account required</h3>
            <p class="muted small">In-game economy balances and transactions are synced to your SCOPENET server account.</p>
            <button class="sm primary" onclick={() => (app.addAccount = true)}>Sign in with Server Account</button>
          </div>
        {:else}
          <!-- Balances Section -->
          <div class="section-card card glass">
            <div class="row justify-between align-center">
              <div>
                <strong class="section-title">My Server Balances</strong>
                <p class="muted tiny">Synced in real-time from SCOPENET economy plugins and mods.</p>
              </div>
              <button class="ghost sm icon" onclick={loadEconomy} title="Refresh balances">
                <RefreshCw size={14} class={loadingEconomy ? 'spin' : ''} />
              </button>
            </div>

            {#if economyBalances.length === 0}
              <div class="empty-inline muted tiny">
                No active currency balances found yet. Join a SCOPENET server to start earning money!
              </div>
            {:else}
              <div class="econ-balances-grid">
                {#each economyBalances as bal (bal.server_id)}
                  <button
                    type="button"
                    class="econ-balance-card glass"
                    class:selected={selectedEconomyServerId === bal.server_id}
                    onclick={() => handleEconomyServerChange(bal.server_id)}
                  >
                    <div class="econ-card-top row justify-between align-center">
                      <span class="server-badge">{bal.server_name}</span>
                      <Wallet size={16} class="icon-accent" />
                    </div>
                    <div class="econ-balance-val">
                      <span class="currency-sym">{bal.currency_symbol || '$'}</span>
                      <span>{bal.balance.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
                    </div>
                    <div class="econ-card-footer tiny muted">Live server balance</div>
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Server Baltop Leaderboard Section -->
          <div class="section-card card glass">
            <div class="row justify-between align-center">
              <div>
                <strong class="section-title">Server Rich List (Baltop)</strong>
                <p class="muted tiny">Top richest players on the selected game server.</p>
              </div>

              {#if servers.length > 0}
                <div class="row align-center gap-xs">
                  <span class="tiny muted">Server:</span>
                  <select
                    class="filter-select"
                    value={selectedEconomyServerId}
                    onchange={(e) => handleEconomyServerChange(Number(e.currentTarget.value))}
                  >
                    {#each servers as s (s.id)}
                      <option value={s.id}>{s.name}</option>
                    {/each}
                  </select>
                </div>
              {/if}
            </div>

            {#if baltopData.length === 0}
              <div class="empty-inline muted tiny">
                No baltop entries recorded for this server yet.
              </div>
            {:else}
              <div class="baltop-list">
                {#each baltopData as row}
                  {@const isMe = row.uuid === me?.uuid}
                  <div class="baltop-row" class:is-me={isMe}>
                    <div class="row align-center gap-sm">
                      <span class="baltop-rank" class:gold={row.rank === 1} class:silver={row.rank === 2} class:bronze={row.rank === 3}>
                        #{row.rank}
                      </span>
                      <Avatar uuid={row.uuid} name={row.username} size={2.2} />
                      <strong class="baltop-name"><PlayerLink uuid={row.uuid} name={row.username} /></strong>
                      {#if isMe}<span class="badge you">You</span>{/if}
                    </div>
                    <span class="baltop-val highlight">
                      ${row.balance.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    </span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Transaction History Section -->
          <div class="section-card card glass">
            <div class="row justify-between align-center">
              <div>
                <strong class="section-title">Recent Transactions</strong>
                <p class="muted tiny">Payments, player trades, market listings, and server shop logs.</p>
              </div>
            </div>

            {#if transactions.length === 0}
              <div class="empty-inline muted tiny">
                No recorded economy transactions yet.
              </div>
            {:else}
              <div class="tx-table-wrap">
                <div class="tx-header">
                  <span>Type</span>
                  <span>Description</span>
                  <span>Date</span>
                  <span class="text-right">Amount</span>
                </div>
                {#each transactions.slice(0, 15) as tx (tx.id)}
                  {@const isSender = tx.from_uuid === me?.uuid}
                  <div class="tx-row">
                    <span class="tx-type-pill badge" class:send={isSender} class:receive={!isSender}>
                      {#if isSender}<ArrowUpRight size={12} /> Sent{:else}<ArrowDownLeft size={12} /> Received{/if}
                    </span>
                    <span class="tx-desc" title={tx.description}>{tx.description || `${tx.from_name} → ${tx.to_name}`}</span>
                    <span class="tx-date tiny muted">{new Date(tx.created_at).toLocaleString()}</span>
                    <span class="tx-amount text-right" class:negative={isSender} class:positive={!isSender}>
                      {isSender ? '-' : '+'}${Math.abs(tx.amount).toFixed(2)}
                    </span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {:else}
      <!-- Leaderboards Tab -->
      <div class="filters-bar card glass row justify-between align-center">
        <div class="row align-center gap-sm">
          <span class="filter-label tiny muted">Server:</span>
          <select bind:value={selectedServerId} class="filter-select">
            <option value={null}>All Servers</option>
            {#each servers as s (s.id)}
              <option value={s.id}>{s.name} {s.online ? `(${s.players_online} online)` : ''}</option>
            {/each}
          </select>
        </div>

        <div class="row align-center gap-xs">
          {#each sortOptions as opt}
            {@const Icon = opt.icon}
            <button
              class="sm sort-btn"
              class:active={sortField === opt.id}
              onclick={() => (sortField = opt.id)}
              title={opt.label}
            >
              <Icon size={14} /> <span>{opt.label}</span>
            </button>
          {/each}
        </div>
      </div>

      {#if loadingLeaderboard}
        <div class="center-state"><LoaderCircle size={28} class="spin" /></div>
      {:else if !leaderboardData || leaderboardData.leaderboard.length === 0}
        <div class="card glass empty-state">
          <Trophy size={36} class="muted" />
          <h3>No leaderboard entries yet</h3>
          <p class="muted small">As players explore, mine, and battle on servers, the rankings will appear here automatically.</p>
        </div>
      {:else}
        <!-- Top 3 Podium -->
        {#if top3.length > 0}
          <div class="podium-grid">
            {#if top3[1]}
              <div class="podium-card silver glass">
                <span class="podium-rank">2</span>
                <Avatar name={top3[1].name} uuid={top3[1].uuid} size={3.2} />
                <strong class="podium-name" title={top3[1].name}>{top3[1].name}</strong>
                <span class="podium-stat">{statValue(top3[1], sortField)}</span>
              </div>
            {/if}
            {#if top3[0]}
              <div class="podium-card gold glass">
                <span class="crown">👑</span>
                <span class="podium-rank">1</span>
                <Avatar name={top3[0].name} uuid={top3[0].uuid} size={4} />
                <strong class="podium-name" title={top3[0].name}>{top3[0].name}</strong>
                <span class="podium-stat">{statValue(top3[0], sortField)}</span>
              </div>
            {/if}
            {#if top3[2]}
              <div class="podium-card bronze glass">
                <span class="podium-rank">3</span>
                <Avatar name={top3[2].name} uuid={top3[2].uuid} size={3.2} />
                <strong class="podium-name" title={top3[2].name}>{top3[2].name}</strong>
                <span class="podium-stat">{statValue(top3[2], sortField)}</span>
              </div>
            {/if}
          </div>
        {/if}

        <!-- Leaderboard Table -->
        <div class="card glass list-card">
          <div class="table-header">
            <span class="col-rank">#</span>
            <span class="col-player">Player</span>
            <span class="col-main">{sortOptions.find(o => o.id === sortField)?.label ?? 'Stat'}</span>
            <span class="col-extra">Playtime</span>
            <span class="col-extra">PvP Kills</span>
            <span class="col-extra">Last Seen</span>
          </div>
          <div class="table-body">
            {#each leaderboardData.leaderboard as row, idx (row.uuid)}
              {@const isMe = me?.uuid === row.uuid}
              <div class="table-row" class:is-me={isMe}>
                <span class="col-rank font-bold">
                  {#if idx === 0}🥇{:else if idx === 1}🥈{:else if idx === 2}🥉{:else}{idx + 1}{/if}
                </span>
                <div class="col-player row align-center gap-sm">
                  <Avatar name={row.name} uuid={row.uuid} size={1.8} />
                  <strong class="row-player-name"><PlayerLink uuid={row.uuid} name={row.name} /></strong>
                  {#if isMe}<span class="badge you">You</span>{/if}
                </div>
                <strong class="col-main highlight">{statValue(row, sortField)}</strong>
                <span class="col-extra muted">{formatPlaytime(row.playtime_secs)}</span>
                <span class="col-extra muted">{row.player_kills}</span>
                <span class="col-extra tiny muted">{formatDate(row.last_seen)}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .stats-page { height: 100%; display: flex; flex-direction: column; overflow-y: auto; padding: 1.6rem 2.2rem 2.5rem; gap: 1.2rem; }
  .page-header { display: flex; justify-content: space-between; align-items: flex-end; flex-wrap: wrap; gap: 1rem; border-bottom: 1px solid var(--line); padding-bottom: 1rem; }
  .title-wrap h1 { font-size: 1.5rem; font-weight: 700; display: flex; align-items: center; gap: 0.6rem; }
  :global(.trophy-icon) { color: #f59e0b; }
  .lead { color: var(--muted); font-size: 0.88rem; margin-top: 0.2rem; }
  .tabs-segmented { display: flex; background: color-mix(in srgb, var(--surface) 80%, transparent); padding: 0.25rem; border-radius: var(--radius-sm); border: 1px solid var(--line); gap: 0.2rem; }
  .tabs-segmented button { background: transparent; border: none; padding: 0.45rem 1rem; font-size: 0.85rem; font-weight: 560; color: var(--muted); border-radius: calc(var(--radius-sm) - 2px); }
  .tabs-segmented button.active { background: color-mix(in srgb, var(--text) 10%, transparent); color: var(--text); }
  .page-content { display: flex; flex-direction: column; gap: 1.1rem; }
  .center-state { display: grid; place-items: center; min-height: 16rem; color: var(--accent); }
  .empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.8rem; padding: 3rem 1.5rem; text-align: center; }
  .card { padding: 1.2rem 1.4rem; background: var(--surface); border-radius: var(--radius); }
  .player-hero { display: flex; align-items: center; gap: 1.5rem; padding: 1.4rem 1.6rem; }
  .hero-details { flex: 1; display: flex; flex-direction: column; gap: 0.35rem; }
  .player-name { font-size: 1.35rem; font-weight: 700; }
  .badge { display: inline-flex; align-items: center; font-size: 0.72rem; font-weight: 600; padding: 0.15rem 0.5rem; border-radius: 99rem; }
  .badge.online { background: color-mix(in srgb, var(--success) 15%, transparent); color: var(--success); }
  .badge.you { background: color-mix(in srgb, var(--accent) 20%, transparent); color: var(--accent); }
  .hero-playtime { display: flex; flex-direction: column; align-items: flex-end; gap: 0.2rem; }
  .big-stat { font-size: 1.5rem; font-weight: 700; color: var(--accent); }
  .uppercase { text-transform: uppercase; letter-spacing: 0.06em; font-weight: 600; }
  .stats-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr)); gap: 0.9rem; }
  .stat-card { display: flex; flex-direction: column; gap: 0.4rem; padding: 1rem 1.1rem; }
  .stat-header { display: flex; justify-content: space-between; align-items: center; }
  .stat-label { font-size: 0.82rem; font-weight: 560; color: var(--muted); }
  .stat-value { font-size: 1.35rem; font-weight: 700; font-variant-numeric: tabular-nums; }
  :global(.icon-accent) { color: var(--accent); }
  :global(.icon-success) { color: var(--success); }
  :global(.icon-danger) { color: var(--danger); }
  :global(.icon-warn) { color: var(--warn); }
  :global(.icon-muted) { color: var(--muted); }
  .section-card { display: flex; flex-direction: column; gap: 0.9rem; }
  .server-rows { display: flex; flex-direction: column; gap: 0.6rem; }
  .server-stat-row { display: flex; justify-content: space-between; align-items: center; padding: 0.8rem 1rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--bg) 50%, transparent); border: 1px solid var(--line); }
  .server-info { display: flex; flex-direction: column; gap: 0.2rem; }
  .server-metrics { display: flex; gap: 1.4rem; }
  .metric { display: flex; flex-direction: column; align-items: flex-end; gap: 0.15rem; font-size: 0.85rem; }
  .filters-bar { padding: 0.7rem 1rem; gap: 0.8rem; }
  .filter-select { background: color-mix(in srgb, var(--bg) 60%, transparent); border: 1px solid var(--line); color: var(--text); padding: 0.35rem 0.7rem; border-radius: var(--radius-sm); font-size: 0.82rem; }
  .sort-btn { background: transparent; border-color: transparent; color: var(--muted); padding: 0.4rem 0.65rem; font-size: 0.8rem; }
  .sort-btn.active { background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--text); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
  .podium-grid { display: grid; grid-template-columns: 1fr 1.15fr 1fr; gap: 0.9rem; align-items: end; max-width: 38rem; margin: 0.5rem auto 1rem; }
  .podium-card { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 1.2rem 1rem; position: relative; border-radius: var(--radius); text-align: center; }
  .podium-card.gold { border-color: #f59e0b88; background: color-mix(in srgb, #f59e0b 8%, var(--surface)); order: 2; padding: 1.6rem 1rem; }
  .podium-card.silver { border-color: #94a3b866; order: 1; }
  .podium-card.bronze { border-color: #d9770666; order: 3; }
  .crown { font-size: 1.3rem; margin-top: -0.6rem; }
  .podium-rank { position: absolute; top: 0.5rem; left: 0.7rem; font-size: 0.85rem; font-weight: 700; color: var(--muted); }
  .podium-name { font-size: 0.95rem; font-weight: 650; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .podium-stat { font-size: 1.1rem; font-weight: 700; color: var(--accent); }
  .list-card { padding: 0.4rem 0; overflow: hidden; }
  .table-header, .table-row { display: grid; grid-template-columns: 3rem 1fr 8rem 7rem 6rem 7rem; align-items: center; padding: 0.65rem 1.2rem; font-size: 0.85rem; }
  .table-header { border-bottom: 1px solid var(--line); color: var(--muted); font-size: 0.78rem; text-transform: uppercase; font-weight: 600; letter-spacing: 0.05em; }
  .table-row { border-bottom: 1px solid color-mix(in srgb, var(--line) 40%, transparent); transition: background 0.12s; }
  .table-row:hover { background: color-mix(in srgb, var(--text) 4%, transparent); }
  .table-row.is-me { background: color-mix(in srgb, var(--accent) 10%, transparent); border-color: color-mix(in srgb, var(--accent) 30%, transparent); }
  .col-rank { font-variant-numeric: tabular-nums; }
  .row-player-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .highlight { color: var(--accent); font-weight: 650; }
  @media (max-width: 900px) {
    .table-header, .table-row { grid-template-columns: 2.5rem 1fr 7rem; }
    .col-extra { display: none; }
  }
  .economy-view { display: flex; flex-direction: column; gap: 1.2rem; }
  .section-title { font-size: 1.05rem; font-weight: 700; }
  .empty-inline { padding: 1.2rem 0; }
  .econ-balances-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr)); gap: 1rem; margin-top: 0.5rem; }
  .econ-balance-card { display: flex; flex-direction: column; gap: 0.6rem; padding: 1.1rem 1.2rem; border-radius: var(--radius-sm); cursor: pointer; transition: transform 0.15s, border-color 0.15s; text-align: left; font: inherit; color: inherit; width: 100%; }
  .econ-balance-card:hover { transform: translateY(-2px); }
  .econ-balance-card.selected { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 10%, transparent); }
  .server-badge { font-size: 0.82rem; font-weight: 700; color: var(--accent); }
  .econ-balance-val { font-size: 1.6rem; font-weight: 800; display: flex; align-items: baseline; gap: 0.2rem; font-variant-numeric: tabular-nums; }
  .currency-sym { font-size: 1.1rem; color: var(--muted); }
  .baltop-list { display: flex; flex-direction: column; gap: 0.5rem; margin-top: 0.5rem; }
  .baltop-row { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 1rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--bg) 50%, transparent); border: 1px solid var(--line); }
  .baltop-row.is-me { background: color-mix(in srgb, var(--accent) 10%, transparent); border-color: color-mix(in srgb, var(--accent) 30%, transparent); }
  .baltop-rank { font-weight: 800; font-size: 0.95rem; min-width: 2rem; font-variant-numeric: tabular-nums; }
  .baltop-rank.gold { color: #f59e0b; }
  .baltop-rank.silver { color: #94a3b8; }
  .baltop-rank.bronze { color: #d97706; }
  .baltop-name { font-size: 0.95rem; }
  .baltop-val { font-size: 1.05rem; }
  .tx-table-wrap { display: flex; flex-direction: column; margin-top: 0.5rem; border: 1px solid var(--line); border-radius: var(--radius-sm); overflow: hidden; }
  .tx-header { display: grid; grid-template-columns: 7rem 1fr 12rem 8rem; padding: 0.65rem 1rem; font-size: 0.78rem; text-transform: uppercase; font-weight: 600; color: var(--muted); border-bottom: 1px solid var(--line); }
  .tx-row { display: grid; grid-template-columns: 7rem 1fr 12rem 8rem; align-items: center; padding: 0.7rem 1rem; border-bottom: 1px solid color-mix(in srgb, var(--line) 40%, transparent); font-size: 0.85rem; }
  .tx-type-pill.send { background: color-mix(in srgb, var(--danger) 15%, transparent); color: var(--danger); }
  .tx-type-pill.receive { background: color-mix(in srgb, var(--success) 15%, transparent); color: var(--success); }
  .tx-desc { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tx-amount { font-weight: 700; font-variant-numeric: tabular-nums; }
  .tx-amount.negative { color: var(--danger); }
  .tx-amount.positive { color: var(--success); }
</style>
