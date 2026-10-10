<script lang="ts">
  import { onMount } from 'svelte';
  import { UserPlus, Link2, LoaderCircle, Coins, Users, Landmark, Store, ScrollText, LayoutDashboard, Search, Plus, Minus, Equal, Trash2, Clock, ChevronLeft, ChevronRight, ArrowUpDown, TrendingUp, Gavel, Tag, Dices } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import PlayerLink from '../components/PlayerLink.svelte';
  import { get, post, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Top = { name: string; uuid: string; amount: number };
  type Overview = {
    server: { id: number; name: string };
    totals: { circulation: number; accounts: number; average: number; median: number; richest: { name: string; uuid: string; balance: number } | null; guild_wallets_total: number; guild_wallets_count: number };
    volume: { today: number; last_7d: number; last_30d: number; tx_today: number; tx_7d: number };
    daily: { day: string; volume: number; count: number }[];
    top_earners: Top[]; top_spenders: Top[];
    market: { fixed_listings: number; auctions: number; total_value: number; ending_soon: number; sales_7d_volume: number };
    casino?: { wagered_7d: number; paid_7d: number; house_net_7d: number };
  };
  type PlayerRow = { uuid: string; username: string; balance: number; updated_at: string };
  type GuildRow = { id: string; name: string; tag: string; members: number; balance: number };
  type Listing = {
    id: number; seller_name: string; seller_uuid: string; seller_guild: string | null; item_id: string; item_name: string; amount: number; price: number;
    kind: string; ends_at: string | null; current_bid: number | null; bidder_name: string | null; bid_count: number; created_at: string;
  };
  type Tx = { id: number; from_uuid: string; from_name: string; to_uuid: string; to_name: string; amount: number; description: string; created_at: string };
  type Target = { kind: 'player'; uuid: string; name: string; balance: number } | { kind: 'guild'; id: string; name: string; balance: number };

  const TABS = [
    { id: 'overview', label: 'Overview', icon: LayoutDashboard },
    { id: 'players', label: 'Players', icon: Users },
    { id: 'guilds', label: 'Factions', icon: Landmark },
    { id: 'market', label: 'Market', icon: Store },
    { id: 'ledger', label: 'Ledger', icon: ScrollText },
  ] as const;
  type Tab = (typeof TABS)[number]['id'];

  const money = (n: number | null | undefined) => '$' + (n ?? 0).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const KEY = 'scopenet_economy_server';

  let servers = $state<{ id: number; name: string }[]>([]);
  let serverId = $state(0);
  let tab = $state<Tab>('overview');
  let loading = $state(true);

  let ov = $state<Overview | null>(null);

  let pq = $state('');
  let psort = $state<'balance' | 'name' | 'updated'>('balance');
  let pdir = $state<'asc' | 'desc'>('desc');
  let ppage = $state(0);
  const PAGE = 50;
  let players = $state<{ total: number; rows: PlayerRow[] } | null>(null);

  // Players with no bank account, and accounts that belong to nobody (so one can be handed to a player).
  type Missing = { uuid: string; name: string; suggested_from: string | null };
  type Orphan = { uuid: string; username: string; balance: number };
  let acc = $state<{ total: number; missing: Missing[]; orphans: Orphan[] } | null>(null);
  let accBusy = $state('');
  let takeOver = $state<Record<string, string>>({});
  async function loadAccounts() {
    try { acc = await get(`/api/admin/economy/accounts/missing?server_id=${serverId}`); } catch (e) { toastError(e); }
  }
  async function repairAccounts() {
    accBusy = 'all';
    try {
      const r = await post<{ created: number; linked: number; remaining: number }>('/api/admin/economy/accounts/repair', { server_id: serverId, link_by_name: true });
      toast(`${r.created} account${r.created === 1 ? '' : 's'} created${r.linked ? `, ${r.linked} re-linked to the right player` : ''}`);
      await Promise.all([loadAccounts(), loadPlayers()]);
    } catch (e) { toastError(e); } finally { accBusy = ''; }
  }
  async function assignAccount(m: Missing) {
    accBusy = m.uuid;
    try {
      await post('/api/admin/economy/accounts/assign', { server_id: serverId, uuid: m.uuid, from_uuid: takeOver[m.uuid] || undefined });
      toast(takeOver[m.uuid] ? `${m.name} took over the account` : `Account opened for ${m.name}`);
      await Promise.all([loadAccounts(), loadPlayers()]);
    } catch (e) { toastError(e); } finally { accBusy = ''; }
  }

  let guilds = $state<GuildRow[] | null>(null);

  let mq = $state('');
  let mkind = $state<'all' | 'buy_now' | 'auction'>('all');
  let msort = $state<'newest' | 'price' | 'ending'>('newest');
  let market = $state<{ total: number; rows: Listing[] } | null>(null);

  let lq = $state('');
  let ledger = $state<Tx[] | null>(null);
  let ledgerMore = $state<number | null>(null);

  // Adjust modal
  let adjOpen = $state(false);
  let target = $state<Target | null>(null);
  let mode = $state<'add' | 'remove' | 'set'>('add');
  let amount = $state<string | number>('');
  let reason = $state('');
  let busy = $state(false);

  // Remove / extend modals
  let rmOpen = $state(false);
  let rmListing = $state<Listing | null>(null);
  let rmReason = $state('');
  let extOpen = $state(false);
  let extListing = $state<Listing | null>(null);
  let extHours = $state(24);
  let prOpen = $state(false);
  let prListing = $state<Listing | null>(null);
  let prValue = $state('');

  const amt = $derived(Number(amount));
  const amtValid = $derived(String(amount ?? '').trim() !== '' && Number.isFinite(amt) && amt >= 0 && amt <= 1e12 && (mode === 'set' || amt > 0));
  const preview = $derived.by(() => {
    if (!target || !amtValid) return null;
    const a = Math.round(amt * 100) / 100;
    return mode === 'add' ? target.balance + a : mode === 'remove' ? target.balance - a : a;
  });
  const previewBad = $derived(preview !== null && preview < -1e-9);

  async function loadServers() {
    try {
      servers = await get('/api/admin/servers');
      let saved = 0;
      try { saved = Number(localStorage.getItem(KEY)) || 0; } catch { /* storage blocked */ }
      serverId = servers.find((s) => s.id === saved)?.id ?? [...servers].sort((a, b) => a.id - b.id)[0]?.id ?? 0;
    } catch (e) { toastError(e); }
    loading = false;
  }

  function pickServer(id: number) {
    serverId = id;
    try { localStorage.setItem(KEY, String(id)); } catch { /* storage blocked */ }
  }

  async function loadOverview() {
    try { ov = await get(`/api/admin/economy/overview?server_id=${serverId}`); } catch (e) { toastError(e); }
  }
  async function loadPlayers() {
    try {
      const qs = new URLSearchParams({ server_id: String(serverId), q: pq, sort: psort, dir: pdir, limit: String(PAGE), offset: String(ppage * PAGE) });
      players = await get(`/api/admin/economy/players?${qs}`);
    } catch (e) { toastError(e); }
  }
  async function loadGuilds() {
    try { guilds = await get(`/api/admin/economy/guilds?server_id=${serverId}`); } catch (e) { toastError(e); }
  }
  async function loadMarket() {
    try {
      const qs = new URLSearchParams({ server_id: String(serverId), q: mq, kind: mkind, sort: msort, limit: '100' });
      market = await get(`/api/admin/economy/market?${qs}`);
    } catch (e) { toastError(e); }
  }
  async function loadLedger(more = false) {
    try {
      const qs = new URLSearchParams({ server_id: String(serverId), q: lq, limit: '100' });
      if (more && ledgerMore) qs.set('before_id', String(ledgerMore));
      const r = await get<{ rows: Tx[]; next_before_id: number | null }>(`/api/admin/economy/transactions?${qs}`);
      ledger = more && ledger ? [...ledger, ...r.rows] : r.rows;
      ledgerMore = r.rows.length >= 100 ? r.next_before_id : null;
    } catch (e) { toastError(e); }
  }

  function refresh() {
    if (!serverId) return;
    if (tab === 'overview') loadOverview();
    else if (tab === 'players') { loadPlayers(); loadAccounts(); }
    else if (tab === 'guilds') loadGuilds();
    else if (tab === 'market') loadMarket();
    else loadLedger();
  }

  // Reload when the server, tab or any filter changes (search boxes are debounced).
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void [serverId, tab, pq, psort, pdir, ppage, mq, mkind, msort, lq];
    clearTimeout(timer);
    timer = setTimeout(refresh, 200);
    return () => clearTimeout(timer);
  });
  let firstServer = true;
  $effect(() => {
    void serverId;
    if (firstServer) { firstServer = false; return; }
    ov = null; players = null; acc = null; guilds = null; market = null; ledger = null; ppage = 0;
  });

  onMount(loadServers);

  function sortBy(col: 'balance' | 'name' | 'updated') {
    if (psort === col) pdir = pdir === 'asc' ? 'desc' : 'asc';
    else { psort = col; pdir = col === 'name' ? 'asc' : 'desc'; }
    ppage = 0;
  }

  function openAdjust(t: Target) {
    target = t; mode = 'add'; amount = ''; reason = ''; adjOpen = true;
  }
  async function submitAdjust() {
    if (!target || !amtValid || previewBad) return;
    busy = true;
    try {
      const base = { server_id: serverId, mode, amount: amt, reason: reason.trim() || undefined };
      const r = target.kind === 'player'
        ? await post('/api/admin/economy/players/adjust', { ...base, uuid: target.uuid, username: target.name })
        : await post('/api/admin/economy/guilds/adjust', { ...base, guild_id: target.id });
      toast(r.delta === 0 ? 'Balance unchanged' : `${target.name} is now at ${money(r.balance)}`);
      adjOpen = false;
      refresh();
    } catch (e) { toastError(e); }
    busy = false;
  }

  async function submitRemove() {
    if (!rmListing) return;
    busy = true;
    try {
      const r = await post(`/api/admin/economy/market/${rmListing.id}/remove`, { server_id: serverId, reason: rmReason.trim() || undefined });
      toast(r.refunded ? `Listing removed, ${money(r.refunded)} refunded` : 'Listing removed, item returned to the seller');
      rmOpen = false;
      loadMarket();
    } catch (e) { toastError(e); }
    busy = false;
  }
  async function submitExtend() {
    if (!extListing) return;
    busy = true;
    try {
      await post(`/api/admin/economy/market/${extListing.id}/extend`, { server_id: serverId, hours: extHours });
      toast(`Auction extended by ${extHours}h`);
      extOpen = false;
      loadMarket();
    } catch (e) { toastError(e); }
    busy = false;
  }

  async function submitPrice() {
    if (!prListing) return;
    busy = true;
    try {
      const r = await post(`/api/admin/economy/market/${prListing.id}/price`, { server_id: serverId, price: Number(prValue) });
      toast(`Price set to ${money(r.price)}`);
      prOpen = false;
      loadMarket();
    } catch (e) { toastError(e); }
    busy = false;
  }

  function left(iso: string | null): { text: string; soon: boolean; over: boolean } {
    if (!iso) return { text: '', soon: false, over: false };
    const s = (new Date(iso).getTime() - Date.now()) / 1000;
    if (s <= 0) return { text: 'ended', soon: false, over: true };
    const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60);
    return { text: h >= 24 ? `${Math.floor(h / 24)}d ${h % 24}h` : h > 0 ? `${h}h ${m}m` : `${m}m`, soon: s < 3600 * 6, over: false };
  }

  const maxDay = $derived(Math.max(1, ...(ov?.daily.map((d) => d.volume) ?? [1])));
  const fmtDay = (d: string) => new Date(d + 'T00:00:00').toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
</script>

<div class="page">
  <header class="page-head">
    <div>
      <h1>Economy</h1>
      <p>Balances, faction banks and the market for each server.</p>
    </div>
    {#if servers.length}
      <label class="srv"><span class="muted small">Server</span>
        <select value={serverId} onchange={(e) => pickServer(Number(e.currentTarget.value))}>
          {#each servers as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
        </select>
      </label>
    {/if}
  </header>

  {#if loading}
    <div class="skeleton" style="height: 120px"></div>
  {:else if !servers.length}
    <div class="card empty"><Coins size={32} /><h3>No servers yet</h3><p>Connect a game server and its economy will show up here.</p></div>
  {:else}
    <div class="tabs">
      {#each TABS as t}
        <button type="button" class:active={tab === t.id} onclick={() => (tab = t.id)}><t.icon size={15} /> {t.label}</button>
      {/each}
    </div>

    {#if tab === 'overview'}
      {#if !ov}
        <div class="tiles">{#each Array(6) as _}<div class="skeleton" style="height: 92px"></div>{/each}</div>
      {:else}
        <div class="tiles">
          <div class="tile"><span><Coins size={14} /> In circulation</span><b>{money(ov.totals.circulation)}</b><small>{ov.totals.accounts.toLocaleString()} accounts</small></div>
          <div class="tile"><span><Users size={14} /> Average balance</span><b>{money(ov.totals.average)}</b><small>median {money(ov.totals.median)}</small></div>
          <div class="tile"><span><TrendingUp size={14} /> Volume (7d)</span><b>{money(ov.volume.last_7d)}</b><small>{ov.volume.tx_7d.toLocaleString()} transactions · {money(ov.volume.today)} today</small></div>
          <div class="tile"><span><Landmark size={14} /> Faction banks</span><b>{money(ov.totals.guild_wallets_total)}</b><small>{ov.totals.guild_wallets_count} wallets</small></div>
          <div class="tile"><span><Store size={14} /> Active listings</span><b>{ov.market.fixed_listings + ov.market.auctions}</b><small>{ov.market.fixed_listings} fixed · {ov.market.auctions} auctions</small></div>
          {#if ov.totals.richest}<div class="tile"><span>Richest player</span><b>{money(ov.totals.richest.balance)}</b><small><PlayerLink uuid={ov.totals.richest.uuid} name={ov.totals.richest.name} /></small></div>{/if}
        </div>

        <section class="card block">
          <div class="section-title"><h2>Volume, last 14 days</h2><span class="hint">30d total {money(ov.volume.last_30d)}</span></div>
          <div class="bars" role="img" aria-label="Daily transaction volume">
            {#each ov.daily as d}
              <div class="bar-col" title="{fmtDay(d.day)}: {money(d.volume)} in {d.count} transactions">
                <div class="bar" style:height="{Math.max(d.volume > 0 ? 3 : 0, (d.volume / maxDay) * 100)}%"></div>
                <small>{fmtDay(d.day)}</small>
              </div>
            {/each}
          </div>
        </section>

        <div class="two block">
          {#each [{ title: 'Top earners (7d)', rows: ov.top_earners }, { title: 'Top spenders (7d)', rows: ov.top_spenders }] as list}
            <section class="card">
              <h2>{list.title}</h2>
              <table class="table">
                <tbody>
                  {#each list.rows as r, i}<tr><td class="rank">{i + 1}</td><td><PlayerLink uuid={r.uuid} name={r.name} /></td><td class="n">{money(r.amount)}</td></tr>{:else}<tr><td class="muted">No activity yet</td></tr>{/each}
                </tbody>
              </table>
            </section>
          {/each}
        </div>

        <div class="two block">
          <section class="card">
            <h2>Market</h2>
            <dl>
              <div><dt>Fixed-price listings</dt><dd>{ov.market.fixed_listings}</dd></div>
              <div><dt>Auctions</dt><dd>{ov.market.auctions}</dd></div>
              <div><dt>Ending in 24h</dt><dd>{ov.market.ending_soon}</dd></div>
              <div><dt>Value on sale</dt><dd>{money(ov.market.total_value)}</dd></div>
              <div><dt>Sales volume (7d)</dt><dd>{money(ov.market.sales_7d_volume)}</dd></div>
            </dl>
          </section>
          {#if ov.casino}
            <section class="card">
              <h2><Dices size={16} /> Casino (7d)</h2>
              <dl>
                <div><dt>Wagered</dt><dd>{money(ov.casino.wagered_7d)}</dd></div>
                <div><dt>Paid out</dt><dd>{money(ov.casino.paid_7d)}</dd></div>
                <div><dt>House net</dt><dd class:good={ov.casino.house_net_7d >= 0} class:bad={ov.casino.house_net_7d < 0}>{money(ov.casino.house_net_7d)}</dd></div>
              </dl>
            </section>
          {/if}
        </div>
      {/if}

    {:else if tab === 'players'}
      {#if acc && acc.total > 0}
        <section class="card accounts">
          <div class="section-title">
            <UserPlus size={18} /><h2>{acc.total} player{acc.total === 1 ? '' : 's'} without a bank account</h2>
            <button class="primary sm" disabled={accBusy !== ''} onclick={repairAccounts}>{#if accBusy === 'all'}<LoaderCircle class="spin" size={14} /> Working…{:else}<UserPlus size={14} /> Create all missing accounts{/if}</button>
          </div>
          <p class="hint">New accounts start with the starting balance from Progression rules. An unclaimed account with the same name (for example money left under an old UUID) is handed to the player instead. The same fix runs daily as the scheduled job <b>Create missing bank accounts</b>.</p>
          <div class="table-wrap">
            <table class="table">
              <thead><tr><th>Player</th><th>Take over an existing account</th><th></th></tr></thead>
              <tbody>
                {#each acc.missing as m (m.uuid)}
                  <tr>
                    <td><PlayerLink uuid={m.uuid} name={m.name} /></td>
                    <td>
                      <select value={takeOver[m.uuid] ?? m.suggested_from ?? ''} onchange={(e) => (takeOver[m.uuid] = e.currentTarget.value)} aria-label={`Account for ${m.name}`}>
                        <option value="">Open a new account</option>
                        {#each acc.orphans as o (o.uuid)}<option value={o.uuid}>{o.username} · {money(o.balance)}{o.uuid === m.suggested_from ? ' (same name)' : ''}</option>{/each}
                      </select>
                    </td>
                    <td class="act"><button class="sm" disabled={accBusy !== ''} onclick={() => { if (takeOver[m.uuid] === undefined && m.suggested_from) takeOver[m.uuid] = m.suggested_from; assignAccount(m); }}>{#if accBusy === m.uuid}<LoaderCircle class="spin" size={13} />{:else}<Link2 size={13} />{/if} Assign</button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          {#if acc.total > acc.missing.length}<p class="hint">Showing the first {acc.missing.length}. Use “Create all missing accounts” for the rest.</p>{/if}
        </section>
      {/if}
      <section class="card">
        <div class="section-title">
          <div class="search"><Search size={16} /><input bind:value={pq} oninput={() => (ppage = 0)} placeholder="Search players" aria-label="Search players" /></div>
          <span class="hint">{players?.total ?? 0} accounts</span>
        </div>
        {#if !players}
          {#each Array(6) as _}<div class="skeleton line"></div>{/each}
        {:else if !players.rows.length}
          <div class="empty"><Users size={28} /><h3>No players found</h3><p>{pq ? 'Try a different search.' : 'Accounts appear once players join.'}</p></div>
        {:else}
          <div class="table-wrap">
            <table class="table">
              <thead><tr>
                <th><button class="th" onclick={() => sortBy('name')}>Player <ArrowUpDown size={12} /></button></th>
                <th class="n"><button class="th" onclick={() => sortBy('balance')}>Balance <ArrowUpDown size={12} /></button></th>
                <th><button class="th" onclick={() => sortBy('updated')}>Updated <ArrowUpDown size={12} /></button></th>
                <th></th>
              </tr></thead>
              <tbody>
                {#each players.rows as p (p.uuid)}
                  <tr>
                    <td><PlayerLink uuid={p.uuid} name={p.username} /></td>
                    <td class="n">{money(p.balance)}</td>
                    <td class="muted">{timeAgo(p.updated_at)}</td>
                    <td class="act"><button class="sm" onclick={() => openAdjust({ kind: 'player', uuid: p.uuid, name: p.username, balance: p.balance })}>Adjust</button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          <div class="pager">
            <button class="sm ghost" disabled={ppage === 0} onclick={() => (ppage -= 1)}><ChevronLeft size={14} /> Prev</button>
            <span class="muted small">Page {ppage + 1} of {Math.max(1, Math.ceil(players.total / PAGE))}</span>
            <button class="sm ghost" disabled={(ppage + 1) * PAGE >= players.total} onclick={() => (ppage += 1)}>Next <ChevronRight size={14} /></button>
          </div>
        {/if}
      </section>

    {:else if tab === 'guilds'}
      <section class="card">
        <div class="section-title"><h2>Faction banks</h2><span class="hint">{guilds?.length ?? 0} factions</span></div>
        {#if !guilds}
          {#each Array(4) as _}<div class="skeleton line"></div>{/each}
        {:else if !guilds.length}
          <div class="empty"><Landmark size={28} /><h3>No factions on this server</h3></div>
        {:else}
          <div class="table-wrap">
            <table class="table">
              <thead><tr><th>Faction</th><th class="n">Members</th><th class="n">Bank balance</th><th></th></tr></thead>
              <tbody>
                {#each guilds as g (g.id)}
                  <tr>
                    <td><span class="badge accent">{g.tag}</span> {g.name}</td>
                    <td class="n">{g.members}</td>
                    <td class="n">{money(g.balance)}</td>
                    <td class="act"><button class="sm" onclick={() => openAdjust({ kind: 'guild', id: g.id, name: g.name, balance: g.balance })}>Adjust</button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </section>

    {:else if tab === 'market'}
      <section class="card">
        <div class="section-title filters">
          <div class="search"><Search size={16} /><input bind:value={mq} placeholder="Search items or sellers" aria-label="Search listings" /></div>
          <div class="segmented">
            {#each [['all', 'All'], ['buy_now', 'Fixed'], ['auction', 'Auctions']] as [v, l]}
              <button class:active={mkind === v} onclick={() => (mkind = v as typeof mkind)}>{l}</button>
            {/each}
          </div>
          <select bind:value={msort} aria-label="Sort listings">
            <option value="newest">Newest</option><option value="price">Highest price</option><option value="ending">Ending soon</option>
          </select>
          <span class="hint">{market?.total ?? 0} listings</span>
        </div>
        {#if !market}
          {#each Array(5) as _}<div class="skeleton line"></div>{/each}
        {:else if !market.rows.length}
          <div class="empty"><Store size={28} /><h3>No listings</h3><p>Nothing matches right now.</p></div>
        {:else}
          <div class="table-wrap">
            <table class="table">
              <thead><tr><th>Item</th><th>Type</th><th>Seller</th><th class="n">Price</th><th>Time left</th><th></th></tr></thead>
              <tbody>
                {#each market.rows as m (m.id)}
                  {@const t = left(m.ends_at)}
                  <tr>
                    <td><b>{m.amount}× {m.item_name}</b><br /><small class="muted">#{m.id} · {timeAgo(m.created_at)}</small></td>
                    <td>{#if m.kind === 'auction'}<span class="badge warn"><Gavel size={12} /> Auction</span>{:else}<span class="badge"><Tag size={12} /> Fixed</span>{/if}</td>
                    <td><PlayerLink uuid={m.seller_uuid} name={m.seller_name} />{#if m.seller_guild} <span class="badge accent">{m.seller_guild}</span>{/if}</td>
                    <td class="n">
                      {#if m.kind === 'auction'}
                        {money(m.current_bid ?? m.price)}<br /><small class="muted">{m.bid_count ? `${m.bidder_name} · ${m.bid_count} bid${m.bid_count === 1 ? '' : 's'}` : 'no bids'}</small>
                      {:else}{money(m.price)}{/if}
                    </td>
                    <td>{#if m.ends_at}<span class:bad={t.soon || t.over} class="tl"><Clock size={12} /> {t.text}</span>{:else}<span class="muted">-</span>{/if}</td>
                    <td class="act">
                      {#if m.kind === 'auction'}<button class="sm ghost" onclick={() => { extListing = m; extHours = 24; extOpen = true; }}>Extend</button>
                      {:else}<button class="sm ghost" onclick={() => { prListing = m; prValue = String(m.price); prOpen = true; }}>Price</button>{/if}
                      <button class="sm danger" onclick={() => { rmListing = m; rmReason = ''; rmOpen = true; }}><Trash2 size={13} /> Remove</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </section>

    {:else}
      <section class="card">
        <div class="section-title">
          <div class="search"><Search size={16} /><input bind:value={lq} placeholder="Filter by player or description" aria-label="Filter ledger" /></div>
        </div>
        {#if !ledger}
          {#each Array(6) as _}<div class="skeleton line"></div>{/each}
        {:else if !ledger.length}
          <div class="empty"><ScrollText size={28} /><h3>No transactions</h3></div>
        {:else}
          <div class="table-wrap">
            <table class="table">
              <thead><tr><th>When</th><th>From</th><th>To</th><th class="n">Amount</th><th>Description</th></tr></thead>
              <tbody>
                {#each ledger as t (t.id)}
                  <tr>
                    <td class="muted nowrap" title={t.created_at}>{timeAgo(t.created_at)}</td>
                    <td>{t.from_name}</td><td>{t.to_name}</td>
                    <td class="n">{money(t.amount)}</td>
                    <td class="desc">{t.description}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
          {#if ledgerMore}<div class="pager"><button class="sm ghost" onclick={() => loadLedger(true)}>Load older</button></div>{/if}
        {/if}
      </section>
    {/if}
  {/if}
</div>

<Modal bind:open={adjOpen} title={target ? `Adjust ${target.kind === 'guild' ? 'faction bank' : 'balance'}: ${target.name}` : 'Adjust'} width={440}>
  {#if target}
    <div class="segmented wide">
      <button class:active={mode === 'add'} onclick={() => (mode = 'add')}><Plus size={13} /> Add</button>
      <button class:active={mode === 'remove'} onclick={() => (mode = 'remove')}><Minus size={13} /> Remove</button>
      <button class:active={mode === 'set'} onclick={() => (mode = 'set')}><Equal size={13} /> Set</button>
    </div>
    <label class="field">Amount
      <input type="number" min="0" step="0.01" inputmode="decimal" bind:value={amount} placeholder="0.00" />
    </label>
    <label class="field">Reason <span class="help">Shown to the player and kept in the ledger.</span>
      <input bind:value={reason} maxlength="100" placeholder="e.g. event prize" />
    </label>
    <div class="preview" class:bad={previewBad}>
      <span class="muted">Current</span><b>{money(target.balance)}</b>
      <span class="muted">New balance</span><b>{preview === null ? '-' : money(preview)}</b>
    </div>
    {#if previewBad}<p class="err">That is more than they have.</p>{/if}
    <div class="foot">
      <button class="ghost" onclick={() => (adjOpen = false)}>Cancel</button>
      <button class="primary" disabled={busy || !amtValid || previewBad} onclick={submitAdjust}>{busy ? 'Saving…' : 'Apply'}</button>
    </div>
  {/if}
</Modal>

<Modal bind:open={rmOpen} title="Remove listing" width={440}>
  {#if rmListing}
    <p>Remove <b>{rmListing.amount}× {rmListing.item_name}</b> from {rmListing.seller_name}? The item goes back to their vault.
      {#if rmListing.kind === 'auction' && rmListing.bid_count}{rmListing.bidder_name}'s {money(rmListing.current_bid)} bid is refunded.{/if}</p>
    <label class="field">Reason <span class="help">Sent to the seller.</span><input bind:value={rmReason} maxlength="100" placeholder="e.g. duped item" /></label>
    <div class="foot">
      <button class="ghost" onclick={() => (rmOpen = false)}>Cancel</button>
      <button class="danger" disabled={busy} onclick={submitRemove}>{busy ? 'Removing…' : 'Remove listing'}</button>
    </div>
  {/if}
</Modal>

<Modal bind:open={prOpen} title="Change price" width={400}>
  {#if prListing}
    <p>New price for <b>{prListing.amount}× {prListing.item_name}</b> (now {money(prListing.price)}).</p>
    <label class="field">Price<input type="number" min="0.01" step="0.01" bind:value={prValue} /></label>
    <div class="foot">
      <button class="ghost" onclick={() => (prOpen = false)}>Cancel</button>
      <button class="primary" disabled={busy || !(Number(prValue) > 0)} onclick={submitPrice}>Save price</button>
    </div>
  {/if}
</Modal>

<Modal bind:open={extOpen} title="Extend auction" width={400}>
  {#if extListing}
    <p>Add time to <b>{extListing.item_name}</b>.</p>
    <label class="field">Hours (1 to 168)<input type="number" min="1" max="168" step="1" bind:value={extHours} /></label>
    <div class="foot">
      <button class="ghost" onclick={() => (extOpen = false)}>Cancel</button>
      <button class="primary" disabled={busy || !(extHours >= 1 && extHours <= 168)} onclick={submitExtend}>Extend</button>
    </div>
  {/if}
</Modal>

<style>
  .srv { display: flex; align-items: center; gap: 10px; }
  .srv select { min-width: 180px; }
  .tabs button { display: inline-flex; align-items: center; gap: 7px; white-space: nowrap; }
  .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(158px, 1fr)); gap: 14px; margin-bottom: 20px; }
  @media (max-width: 700px) { .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; } .tile { padding: 13px 14px; } .tile b { font-size: 1.2rem; } }
  .tile { background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius); padding: 16px 18px; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .tile span { color: var(--muted); font-size: 0.8rem; display: inline-flex; align-items: center; gap: 6px; }
  .tile b { font-size: 1.5rem; font-weight: 620; letter-spacing: -0.01em; }
  .tile small { color: var(--muted); }
  .block { margin-bottom: 20px; }
  .two { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 16px; }
  h2 { font-size: 1rem; margin: 0 0 12px; display: flex; align-items: center; gap: 8px; }
  .section-title h2 { margin: 0; }
  .bars { display: flex; align-items: flex-end; gap: 6px; height: 180px; }
  .bar-col { flex: 1; min-width: 0; height: 100%; display: flex; flex-direction: column; justify-content: flex-end; align-items: center; gap: 6px; }
  .bar-col .bar { width: 100%; max-width: 28px; background: #6d6af5; border-radius: 4px 4px 0 0; transition: opacity 0.15s; flex: none; }
  .bar-col:hover .bar { opacity: 0.75; }
  .bar-col small { color: var(--muted); font-size: 0.68rem; white-space: nowrap; }
  @media (max-width: 700px) { .bar-col:nth-child(even) small { visibility: hidden; } }
  dl { margin: 0; display: flex; flex-direction: column; }
  dl > div { display: flex; justify-content: space-between; gap: 12px; padding: 9px 0; border-bottom: 1px solid var(--line); }
  dl > div:last-child { border-bottom: none; }
  dt { color: var(--muted); }
  dd { margin: 0; font-weight: 560; }
  .good { color: #6ee7b7; }
  .bad { color: #fda4af; }
  .rank { width: 28px; color: var(--muted); }
  .n { text-align: right; font-variant-numeric: tabular-nums; }
  .table-wrap { overflow-x: auto; }
  .table-wrap table { min-width: 560px; }
  .act { text-align: right; white-space: nowrap; }
  .act button + button { margin-left: 6px; }
  .nowrap { white-space: nowrap; }
  .desc { max-width: 360px; overflow-wrap: anywhere; }
  .th { background: none; border: none; padding: 0; color: inherit; font: inherit; text-transform: inherit; letter-spacing: inherit; display: inline-flex; align-items: center; gap: 4px; cursor: pointer; }
  .th:hover { color: var(--text); background: none; }
  .tl { display: inline-flex; align-items: center; gap: 5px; }
  .search { position: relative; flex: 1; min-width: 180px; max-width: 360px; }
  .search :global(svg) { position: absolute; left: 11px; top: 50%; transform: translateY(-50%); color: var(--muted); pointer-events: none; }
  .search input { width: 100%; padding-left: 34px; }
  .filters { flex-wrap: wrap; }
  .filters select { width: auto; min-width: 150px; }
  .pager { display: flex; align-items: center; justify-content: center; gap: 14px; margin-top: 14px; }
  .line { height: 40px; margin-bottom: 8px; }
  .segmented.wide { display: flex; }
  .segmented.wide button { flex: 1; display: inline-flex; align-items: center; justify-content: center; gap: 5px; }
  .preview { display: grid; grid-template-columns: 1fr auto; gap: 6px 12px; padding: 12px 14px; background: var(--bg-2); border: 1px solid var(--line); border-radius: var(--radius-sm); }
  .preview.bad b:last-child { color: #fda4af; }
  .preview b { text-align: right; font-variant-numeric: tabular-nums; }
  .err { color: #fda4af; margin: 0; font-size: 0.85rem; }
  .foot { display: flex; justify-content: flex-end; gap: 10px; }
  @media (max-width: 700px) { .srv { width: 100%; } .srv select { flex: 1; } }
</style>
