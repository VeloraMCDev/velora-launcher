<script lang="ts">
  import { onMount } from 'svelte';
  import { Archive, Clock, Crown, Flame, Gavel, History, Package, RefreshCw, Search, ShoppingCart, Store, Tag, ClipboardList, ScrollText, TrendingUp, ArrowDownLeft, X, Shield, TriangleAlert } from '@lucide/svelte';
  import { get, post } from '../../lib/api';
  import { toast, toastError } from '../../lib/toast.svelte';
  import { balanceChanged, currentServer, money, play } from '../store.svelte';
  import Orders from '@velora/board/Orders.svelte';
  import Contracts from '@velora/board/Contracts.svelte';
  import Darknet from '@velora/board/Darknet.svelte';
  import Vaults from '@velora/board/Vaults.svelte';
  import { setBoardHost } from '@velora/board';
  import Sheet from '../ui/Sheet.svelte';
  import Empty from '../ui/Empty.svelte';
  import Count from '../ui/Count.svelte';
  import MoneyItem from '../lib/MoneyItem.svelte';
  import { ago, parseTime, stripColors, timeLeft } from '../lib/money-pages';

  type Listing = {
    id: number; seller_name: string; seller_guild: string | null; item_id: string; item_name: string; amount: number; price: number; created_at: string;
    kind: 'buy_now' | 'auction'; ends_at: string | null; current_bid: number | null; bidder_name: string | null; bid_count: number; min_next_bid: number; mine: boolean; leading: boolean;
  };
  type Activity = { kind: 'sold' | 'bought' | 'bid'; amount: number; description: string; at: string };
  type Waiting = { item_name: string; amount: number; note: string };
  type Filter = 'all' | 'buy_now' | 'auction' | 'mine' | 'ending';

  let listings = $state<Listing[]>([]);
  let history = $state<Activity[]>([]);
  let waiting = $state(0);
  let waitingItems = $state<Waiting[]>([]);
  let tab = $state<'browse' | 'mine' | 'orders' | 'contracts' | 'darknet' | 'vaults'>('browse');
  const coreExperience = $derived(play.manifest?.instances.find(instance => instance.id === currentServer()?.instance_id)?.experience?.kind === 'velora-smp');
  // Buy orders and contracts are shared with the launcher; this plugs them into the website's API.
  setBoardHost({
    get: (serverId, path) => get(`/api/v1/board/${serverId}${path}`),
    post: (serverId, path, body) => post(`/api/v1/board/${serverId}${path}`, body),
    toast,
  });
  let filter = $state<Filter>('all');
  let sort = $state<'new' | 'low' | 'high' | 'ending'>('new');
  let query = $state('');
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let now = $state(Date.now());
  let target = $state<Listing | null>(null);
  let sheetOpen = $state(false);
  let bidAmount = $state<number | string>(0);
  let busy = $state(false);
  let loadedFor: number | null = null;

  const balance = $derived(play.balance);
  const soon = (l: Listing) => !!l.ends_at && parseTime(l.ends_at) - now < 10 * 60e3;
  const cost = (l: Listing) => (l.kind === 'auction' ? (l.current_bid ?? l.price) : l.price);
  const afford = (price: number) => balance == null || balance >= price;
  const clean = (l: Listing) => stripColors(l.item_name);

  async function load(quiet = false) {
    const sid = play.serverId;
    if (sid == null) { loading = false; return; }
    if (!quiet) loading = true; else refreshing = true;
    try {
      const [l, m] = await Promise.all([
        get<{ balance: number | null; listings: Listing[] }>(`/api/v1/market/${sid}`),
        get<{ history: Activity[]; waiting: number; waiting_items: Waiting[] }>(`/api/v1/market/${sid}/mine`),
      ]);
      if (sid !== play.serverId) return;
      listings = l.listings; history = m.history; waiting = m.waiting; waitingItems = m.waiting_items; error = '';
      if (l.balance != null && l.balance !== play.balance) balanceChanged(l.balance);
    } catch (e) { if (sid === play.serverId) error = e instanceof Error ? e.message : 'Could not load the market'; }
    finally { loading = false; refreshing = false; }
  }

  $effect(() => {
    const sid = play.serverId;
    if (sid !== loadedFor) { loadedFor = sid; listings = []; history = []; waiting = 0; target = null; sheetOpen = false; }
    void load();
  });

  onMount(() => {
    const poll = setInterval(() => { if (!document.hidden) void load(true); }, 15000);
    return () => clearInterval(poll);
  });

  const mineListings = $derived(listings.filter((l) => l.mine));
  const leadingBids = $derived(listings.filter((l) => l.leading && !l.mine));
  const outbid = $derived(listings.filter((l) => l.kind === 'auction' && !l.mine && !l.leading && l.bid_count > 0 && history.some((h) => h.kind === 'bid' && h.description.includes(l.item_name))));
  const endingCount = $derived(listings.filter((l) => l.kind === 'auction' && soon(l)).length);

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let list = listings.filter((l) => {
      if (filter === 'buy_now' && l.kind !== 'buy_now') return false;
      if (filter === 'auction' && l.kind !== 'auction') return false;
      if (filter === 'mine' && !l.mine) return false;
      if (filter === 'ending' && !(l.kind === 'auction' && soon(l))) return false;
      return !q || clean(l).toLowerCase().includes(q) || l.item_id.toLowerCase().includes(q) || l.seller_name.toLowerCase().includes(q);
    });
    const endsAt = (l: Listing) => (l.ends_at ? parseTime(l.ends_at) : Infinity);
    if (sort === 'low') list = [...list].sort((a, b) => cost(a) - cost(b));
    else if (sort === 'high') list = [...list].sort((a, b) => cost(b) - cost(a));
    else if (sort === 'ending') list = [...list].sort((a, b) => endsAt(a) - endsAt(b));
    return list;
  });

  // Tick once a second, but only while an auction countdown is actually on screen.
  const hasCountdown = $derived(tab === 'browse' ? shown.some((l) => l.kind === 'auction') : mineListings.concat(leadingBids).some((l) => l.kind === 'auction'));
  $effect(() => {
    if (!hasCountdown && !sheetOpen) return;
    now = Date.now();
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  function open(l: Listing) { target = l; bidAmount = l.min_next_bid; sheetOpen = true; }
  const bid = $derived(Number(bidAmount) || 0);
  const quick = (m: number) => { if (target) bidAmount = Math.ceil(target.min_next_bid * m); };

  async function act(action: 'buy' | 'bid' | 'cancel') {
    const l = target, sid = play.serverId;
    if (!l || sid == null || busy) return;
    busy = true;
    try {
      const r = await post<{ message?: string; balance?: number }>(`/api/v1/market/${sid}/${action}`, action === 'bid' ? { listing_id: l.id, amount: bid } : { listing_id: l.id });
      toast(r.message ?? (action === 'bid' ? 'Bid placed' : action === 'buy' ? 'Purchased' : 'Listing cancelled'), 'ok');
      sheetOpen = false;
      balanceChanged(typeof r.balance === 'number' ? r.balance : undefined);
      await load(true);
    } catch (e) { toastError(e); await load(true); }
    finally { busy = false; }
  }

  const tabs = $derived([
    ['all', 'All', listings.length], ['buy_now', 'Buy now', listings.filter((l) => l.kind === 'buy_now').length],
    ['auction', 'Auctions', listings.filter((l) => l.kind === 'auction').length], ['mine', 'Mine', mineListings.length], ['ending', '🔥 Ending soon', endingCount],
  ] as [Filter, string, number][]);
</script>

<div class="pl-page market">
  <div class="pl-head">
    <div>
      <h1><Store size={26} /> Market</h1>
      <p>{currentServer()?.name ?? 'Pick a server'} · buy, bid and track your listings</p>
    </div>
    <div class="pl-actions">
      <span class="pl-pill gold pl-hide-sm" title="Your balance on this server">{#if balance == null}<span>No balance yet</span>{:else}<Count value={balance} format={(v) => money(v)} />{/if}</span>
      <button class="pl-iconbtn" aria-label="Refresh" aria-busy={refreshing} onclick={() => load(true)}><RefreshCw size={16} class={refreshing ? 'spin' : ''} /></button>
    </div>
  </div>

  {#if play.serverId == null}
    <div class="pl-card"><Empty icon={Store} title="No servers yet" text="Once a game server is linked to the panel, its market shows up here." /></div>
  {:else}
    <div class="pl-stack">
      <div class="note"><Archive size={16} /><span>Everything you buy or win is delivered to your <b>/vault</b> in game — you don't need to be online.</span></div>
      {#if waiting}
        <div class="pl-alert info wait"><Package size={16} /> <span><b>{waiting} item{waiting === 1 ? '' : 's'}</b> waiting for you{waitingItems[0] ? ': ' + waitingItems.slice(0, 3).map((i) => `${i.amount}× ${stripColors(i.item_name)}`).join(', ') : ''}. Type <code>/market claim</code> in game to collect {waiting === 1 ? 'it' : 'them'}.</span></div>
      {/if}
      {#if endingCount && tab === 'browse' && !loading && filter !== 'ending'}
        <button class="note hot" onclick={() => { filter = 'ending'; sort = 'ending'; }}><Flame size={16} /><span><b>{endingCount} auction{endingCount === 1 ? '' : 's'}</b> ending in the next 10 minutes — tap to see</span></button>
      {/if}
      {#if error}
        <div class="pl-alert err row" role="alert"><TriangleAlert size={16} /><span class="grow">{error}</span><button class="pl-btn sm" onclick={() => load()}>Retry</button></div>
      {/if}

      <div class="pl-tabs" role="tablist" style="align-self: flex-start">
        <button role="tab" aria-selected={tab === 'browse'} class:on={tab === 'browse'} onclick={() => (tab = 'browse')}><Tag size={15} /> Browse <span class="count">{listings.length}</span></button>
        <button role="tab" aria-selected={tab === 'mine'} class:on={tab === 'mine'} onclick={() => (tab = 'mine')}><History size={15} /> My activity {#if mineListings.length + leadingBids.length}<span class="count">{mineListings.length + leadingBids.length}</span>{/if}</button>
        <button role="tab" aria-selected={tab === 'orders'} class:on={tab === 'orders'} onclick={() => (tab = 'orders')}><ClipboardList size={15} /> {coreExperience ? 'Buy contracts' : 'Buy orders'}</button>
        <button role="tab" aria-selected={tab === 'darknet'} class:on={tab === 'darknet'} onclick={() => (tab = 'darknet')}><Store size={15} /> Darknet</button>
        {#if coreExperience}<button role="tab" aria-selected={tab === 'vaults'} class:on={tab === 'vaults'} onclick={() => (tab = 'vaults')}><Archive size={15} /> Vaults</button>{/if}
        {#if !coreExperience}<button role="tab" aria-selected={tab === 'contracts'} class:on={tab === 'contracts'} onclick={() => (tab = 'contracts')}><ScrollText size={15} /> Contracts</button>{/if}
      </div>

      {#snippet card(l: Listing, i: number)}
        <article class="item" class:auction={l.kind === 'auction'} class:leading={l.leading} class:hot={soon(l)} style="--i: {Math.min(i, 12)}">
          <div class="head">
            <MoneyItem id={l.item_id} amount={l.amount} size={52} />
            <div class="title">
              <strong title={clean(l)}>{clean(l)}</strong>
              <small>{#if l.seller_guild}<Shield size={11} /> [{l.seller_guild}]{:else}by {l.seller_name}{/if}</small>
            </div>
            <div class="badges">
              <span class="pl-chip" class:accent={l.kind === 'auction'}>{#if l.kind === 'auction'}<Gavel size={11} /> Auction{:else}<Tag size={11} /> Buy now{/if}</span>
              {#if l.leading}<span class="pl-chip good"><Crown size={11} /> You lead</span>{/if}
            </div>
          </div>
          <div class="price">
            {#if l.kind === 'auction'}
              <div><small>{l.current_bid != null ? 'Top bid' : 'Starting bid'}</small><b>{money(l.current_bid ?? l.price)}</b></div>
              <div class="r"><small><Clock size={11} /> ends in</small><b class="t" class:warn={soon(l)}>{timeLeft(l.ends_at, now)}</b></div>
            {:else}
              <div><small>Price</small><b>{money(l.price)}</b></div>
              {#if l.amount > 1}<div class="r"><small>per item</small><b class="t">{money(l.price / l.amount)}</b></div>{/if}
            {/if}
          </div>
          {#if l.kind === 'auction'}
            <p class="sub">{l.bid_count ? `${l.bid_count} bid${l.bid_count === 1 ? '' : 's'} · ${l.leading ? 'you are leading' : 'led by ' + l.bidder_name}` : 'No bids yet'}</p>
          {/if}
          <div class="foot">
            {#if l.mine}
              <span class="pl-chip accent">Your listing</span>
              <button class="pl-btn sm" onclick={() => open(l)}>Manage</button>
            {:else if l.kind === 'auction'}
              <button class="pl-btn primary block" disabled={!afford(l.min_next_bid)} onclick={() => open(l)}><Gavel size={15} /> {l.leading ? 'Raise bid' : 'Bid ' + money(l.min_next_bid)}</button>
            {:else}
              <button class="pl-btn primary block" disabled={!afford(l.price)} onclick={() => open(l)}><ShoppingCart size={15} /> {afford(l.price) ? 'Buy' : 'Not enough funds'}</button>
            {/if}
          </div>
        </article>
      {/snippet}

      {#if tab === 'vaults' && play.serverId != null}
        <Vaults serverId={play.serverId} get={(id) => get(`/api/v1/vaults/${id}`)} post={(id, action, body) => post(`/api/v1/vaults/${id}/${action}`, body)} onbalance={(value) => value != null && balanceChanged(value)} />
      {:else if tab === 'darknet' && play.serverId != null}
        <Darknet serverId={play.serverId} onbalance={(value) => value != null && balanceChanged(value)}>
          {#snippet item(id: string, amount: number)}<MoneyItem {id} {amount} size={72} />{/snippet}
        </Darknet>
      {:else if tab === 'orders' && play.serverId != null}
        {#key play.serverId}<Orders serverId={play.serverId} onbalance={(v) => v != null && balanceChanged(v)} />{/key}
      {:else if tab === 'contracts' && play.serverId != null}
        {#key play.serverId}<Contracts serverId={play.serverId} onbalance={(v) => v != null && balanceChanged(v)} />{/key}
      {:else if tab === 'browse'}
        <div class="bar">
          <label class="sbox"><Search size={16} /><input placeholder="Search the market…" bind:value={query} aria-label="Search the market" /></label>
          <select class="pl-input sortsel" bind:value={sort} aria-label="Sort">
            <option value="new">Newest</option><option value="ending">Ending soon</option><option value="low">Price: low to high</option><option value="high">Price: high to low</option>
          </select>
        </div>
        <div class="pl-tabs wide" role="group" aria-label="Filter">
          {#each tabs as [id, label, n] (id)}
            <button class:on={filter === id} onclick={() => (filter = id)}>{label}{#if n}<span class="count">{n}</span>{/if}</button>
          {/each}
        </div>

        {#if loading}
          <div class="pl-grid" style="--min: 270px">{#each Array(6) as _, i (i)}<div class="pl-skel" style="height: 168px; border-radius: 18px"></div>{/each}</div>
        {:else if shown.length}
          <div class="pl-grid" style="--min: 270px">{#each shown as l, i (l.id)}{@render card(l, i)}{/each}</div>
        {:else}
          <div class="pl-card"><Empty icon={Store} title={listings.length ? 'Nothing matches' : 'Nobody is selling yet'} text={listings.length ? 'Try another search or filter.' : 'In game, hold an item and type /market sell to list it, or /market auction to start an auction.'}>
            {#if listings.length && (query || filter !== 'all')}<button class="pl-btn sm" onclick={() => { query = ''; filter = 'all'; }}>Clear filters</button>{/if}
          </Empty></div>
        {/if}
      {:else}
        <div class="pl-stack mine">
          <section>
            <h2><Package size={16} /> Your listings <span class="pl-chip">{mineListings.length}</span></h2>
            {#if mineListings.length}<div class="pl-grid" style="--min: 270px">{#each mineListings as l, i (l.id)}{@render card(l, i)}{/each}</div>
            {:else}<div class="pl-card"><Empty icon={Package} title="You aren't selling anything" text="List an item in game with /market sell (or /market auction) and it appears here." /></div>{/if}
          </section>
          {#if leadingBids.length}
            <section>
              <h2><Crown size={16} /> Auctions you're winning <span class="pl-chip good">{leadingBids.length}</span></h2>
              <div class="pl-grid" style="--min: 270px">{#each leadingBids as l, i (l.id)}{@render card(l, i)}{/each}</div>
            </section>
          {/if}
          {#if outbid.length}
            <section>
              <h2><TriangleAlert size={16} /> Outbid <span class="pl-chip warn">{outbid.length}</span></h2>
              <div class="pl-grid" style="--min: 270px">{#each outbid as l, i (l.id)}{@render card(l, i)}{/each}</div>
            </section>
          {/if}
          <section>
            <h2><TrendingUp size={16} /> Recent activity</h2>
            <div class="pl-card tight">
              {#if history.length}
                <div class="pl-list">
                  {#each history as h, i (i)}
                    <div class="pl-item">
                      <span class="ic {h.kind}">{#if h.kind === 'sold'}<ArrowDownLeft size={16} />{:else if h.kind === 'bought'}<ShoppingCart size={16} />{:else}<Gavel size={16} />{/if}</span>
                      <div class="grow"><b>{h.kind === 'sold' ? 'Sold' : h.kind === 'bought' ? 'Bought' : 'Bid on'} {h.description.replace(/^(Market purchase|Auction sale|Auction bid):\s*/, '')}</b><span class="sub">{ago(h.at)}</span></div>
                      <div class="end"><b class={h.kind === 'sold' ? 'pos' : 'neg'}>{h.kind === 'sold' ? '+' : '−'}{money(h.amount)}</b></div>
                    </div>
                  {/each}
                </div>
              {:else}<Empty icon={History} title="No activity yet" text="Sales, purchases and bids will show up here." />{/if}
            </div>
          </section>
        </div>
      {/if}
    </div>
  {/if}
</div>

<Sheet bind:open={sheetOpen} title={target?.mine ? 'Manage listing' : target?.kind === 'auction' ? 'Place a bid' : 'Confirm purchase'}>
  {#if target}
    {@const t = target}
    <div class="who"><MoneyItem id={t.item_id} amount={t.amount} size={58} /><div><strong>{clean(t)}</strong><small>{t.amount}× · {t.seller_guild ? `[${t.seller_guild}] faction` : t.seller_name}</small></div></div>
    {#if t.mine}
      {#if t.kind === 'auction' && t.bid_count}
        <p class="muted">Bidding has started ({t.bid_count} bid{t.bid_count === 1 ? '' : 's'}, top {money(t.current_bid)}), so this auction can't be cancelled. It ends in {timeLeft(t.ends_at, now)}.</p>
      {:else}
        <div class="rows"><span>Listed for</span><b>{money(t.price)}</b>{#if t.kind === 'auction'}<span>Ends in</span><b>{timeLeft(t.ends_at, now)}</b>{/if}</div>
        <p class="muted">Cancelling returns the item to your vault.</p>
      {/if}
    {:else if t.kind === 'auction'}
      <div class="rows"><span>Top bid</span><b>{money(t.current_bid ?? t.price)}</b><span>Minimum next bid</span><b>{money(t.min_next_bid)}</b><span>Ends in</span><b class:warn={soon(t)}>{timeLeft(t.ends_at, now)}</b><span>Your balance</span><b>{money(balance)}</b></div>
      <label class="fld">Your bid
        <input class="pl-input" type="number" inputmode="decimal" min={t.min_next_bid} step="1" bind:value={bidAmount} />
      </label>
      <div class="quick">
        <button class="pl-btn sm" onclick={() => quick(1)}>Minimum</button><button class="pl-btn sm" onclick={() => quick(1.05)}>+5%</button><button class="pl-btn sm" onclick={() => quick(1.1)}>+10%</button><button class="pl-btn sm" onclick={() => quick(1.25)}>+25%</button>
      </div>
      <p class="muted">Your bid is held while you lead and refunded if someone outbids you. Win and the item goes to your <b>vault</b>.</p>
      {#if bid < t.min_next_bid}<p class="err">The minimum bid is {money(t.min_next_bid)}.</p>{:else if !afford(bid)}<p class="err">You can't afford that — you have {money(balance)}.</p>{/if}
    {:else}
      <div class="rows"><span>Price</span><b>{money(t.price)}</b><span>Your balance</span><b>{money(balance)}</b><span>After buying</span><b class:warn={!afford(t.price)}>{money((balance ?? 0) - t.price)}</b></div>
      <p class="muted">The item is delivered to your <b>/vault</b> in game — you don't need to be online.</p>
    {/if}
  {/if}
  {#snippet footer()}
    {#if target}
      {@const t = target}
      {#if t.mine}
        {#if !(t.kind === 'auction' && t.bid_count)}<button class="pl-btn lg danger" aria-busy={busy} disabled={busy} onclick={() => act('cancel')}><X size={15} /> Cancel listing</button>{/if}
      {:else if t.kind === 'auction'}
        <button class="pl-btn lg primary" aria-busy={busy} disabled={busy || bid < t.min_next_bid || !afford(bid)} onclick={() => act('bid')}><Gavel size={15} /> Bid {money(bid)}</button>
      {:else}
        <button class="pl-btn lg primary" aria-busy={busy} disabled={busy || !afford(t.price)} onclick={() => act('buy')}><ShoppingCart size={15} /> Buy for {money(t.price)}</button>
      {/if}
    {/if}
  {/snippet}
</Sheet>

<style>
  h1 :global(svg) { color: var(--accent-2); }
  .note { display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-radius: 14px; font-size: 0.85rem; color: var(--text-2); background: var(--pl-glass); border: 1px solid var(--pl-line); }
  .note :global(svg) { color: var(--accent-2); flex-shrink: 0; }
  .wait { display: flex; gap: 10px; align-items: flex-start; }
  .wait :global(svg) { margin-top: 2px; flex-shrink: 0; }
  code { font-size: 0.8rem; background: rgba(255, 255, 255, 0.08); padding: 1px 6px; border-radius: 6px; }
  .row { display: flex; align-items: center; gap: 10px; }
  .grow { flex: 1; min-width: 0; }
  .bar { display: flex; gap: 10px; }
  .bar .sbox { flex: 1; min-width: 0; }
  .sortsel { width: auto; max-width: 46%; }
  .wide { width: 100%; }
  .item { display: flex; flex-direction: column; gap: 12px; padding: 14px; border-radius: 18px; background: var(--pl-glass); border: 1px solid var(--pl-line); backdrop-filter: blur(14px); animation: list-in 0.4s var(--ease, ease) both; animation-delay: calc(var(--i) * 35ms); transition: transform 0.18s, border-color 0.18s, box-shadow 0.18s; min-width: 0; }
  .item:hover { transform: translateY(-3px); border-color: color-mix(in srgb, var(--accent) 45%, transparent); box-shadow: 0 18px 36px -22px var(--accent); }
  .item.auction { background: linear-gradient(160deg, color-mix(in srgb, var(--accent) 12%, var(--surface)), color-mix(in srgb, var(--surface) 80%, transparent) 65%); }
  .item.leading { border-color: color-mix(in srgb, var(--good) 55%, transparent); }
  .item.hot { border-color: color-mix(in srgb, var(--warn) 60%, transparent); box-shadow: 0 0 0 1px color-mix(in srgb, var(--warn) 25%, transparent), 0 14px 30px -22px var(--warn); }
  @keyframes list-in { from { opacity: 0; transform: translateY(10px); } }
  .head { display: flex; gap: 12px; align-items: center; }
  .title { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .title strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .title small { display: inline-flex; align-items: center; gap: 4px; color: var(--muted); font-size: 0.78rem; }
  .badges { display: flex; flex-direction: column; gap: 4px; align-items: flex-end; align-self: flex-start; }
  .price { display: flex; justify-content: space-between; gap: 8px; align-items: flex-end; }
  .price small { display: flex; align-items: center; gap: 4px; font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; font-weight: 600; }
  .price b { font-size: 1.4rem; font-variant-numeric: tabular-nums; letter-spacing: -0.02em; }
  .price .r { text-align: right; } .price .r small { justify-content: flex-end; }
  .price .t { font-size: 1.02rem; } .warn { color: var(--warn); }
  .sub { font-size: 0.8rem; color: var(--muted); margin: -4px 0 0; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  .foot .pl-btn { display: inline-flex; align-items: center; justify-content: center; gap: 7px; }
  .mine h2 { display: flex; align-items: center; gap: 8px; font-size: 1rem; margin-bottom: 10px; }
  .ic { width: 36px; height: 36px; border-radius: 11px; display: grid; place-items: center; background: rgba(255, 255, 255, 0.07); flex-shrink: 0; }
  .ic.sold { color: var(--good); background: color-mix(in srgb, var(--good) 15%, transparent); } .ic.bought { color: var(--bad); background: color-mix(in srgb, var(--bad) 13%, transparent); } .ic.bid { color: var(--accent-2); background: color-mix(in srgb, var(--accent) 18%, transparent); }
  .who { display: flex; gap: 14px; align-items: center; } .who small { display: block; color: var(--muted); margin-top: 2px; }
  .rows { display: grid; grid-template-columns: 1fr auto; gap: 8px 16px; font-size: 0.92rem; padding: 12px 14px; border-radius: 14px; background: rgba(255, 255, 255, 0.05); }
  .rows span { color: var(--muted); } .rows b { text-align: right; font-variant-numeric: tabular-nums; }
  .fld { display: flex; flex-direction: column; gap: 6px; font-size: 0.82rem; color: var(--muted); }
  .fld input { font-size: 1.3rem; font-weight: 700; color: var(--text); }
  .quick { display: flex; gap: 8px; flex-wrap: wrap; }
  .muted { color: var(--muted); font-size: 0.85rem; margin: 0; } .err { color: var(--bad); font-size: 0.84rem; margin: 0; }
  .danger { background: color-mix(in srgb, var(--bad) 20%, transparent); border-color: color-mix(in srgb, var(--bad) 50%, transparent); color: #fbb; }
  .pl-btn[aria-busy='true'] { opacity: 0.65; pointer-events: none; }
  .note.hot { text-align: left; justify-content: flex-start; width: 100%; color: #f6d58e; background: color-mix(in srgb, var(--warn) 16%, var(--surface)); border-color: color-mix(in srgb, var(--warn) 45%, transparent); }
  .note.hot :global(svg) { color: var(--warn); }
  .sbox { position: relative; display: flex; align-items: center; }
  .sbox :global(svg) { position: absolute; left: 14px; color: var(--muted); pointer-events: none; }
  .sbox input { padding-left: 40px; border-radius: 12px; width: 100%; }
  @media (prefers-reduced-motion: reduce) { .item { animation: none; } }
</style>
