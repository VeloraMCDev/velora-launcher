<script lang="ts">
  import { onMount } from 'svelte';
  import { ClipboardList, ScrollText, Store, Gavel, Tag, RefreshCw, Search, Clock, Crown, Flame, History, Package, Wallet, Shield, X, TrendingUp, ShoppingCart, ArrowDownLeft, ArrowUpRight, Archive } from '@lucide/svelte';
  import ItemTile from '../components/ItemTile.svelte';
  import Modal from '../components/Modal.svelte';
  import Orders from '@scopenet/board/Orders.svelte';
  import Contracts from '@scopenet/board/Contracts.svelte';
  import { setBoardHost } from '@scopenet/board';
  import { app, instances, selectedInstance, toast } from '../lib/store.svelte';
  import { errorText, invoke } from '../lib/tauri';

  type Listing = {
    id: number; seller_name: string; seller_guild: string | null; item_id: string; item_name: string; amount: number; price: number; created_at: string;
    kind: 'buy_now' | 'auction'; ends_at: string | null; current_bid: number | null; bidder_name: string | null; bid_count: number; min_next_bid: number; mine: boolean; leading: boolean;
  };
  type Activity = { kind: 'sold' | 'bought' | 'bid'; amount: number; description: string; at: string };
  type Server = { id: number; name: string; instance_id: string };

  let servers = $state<Server[]>([]);
  let serverId = $state<number | null>(null);
  let listings = $state<Listing[]>([]);
  let balance = $state<number | null>(null);
  let history = $state<Activity[]>([]);
  let waiting = $state(0);
  let waitingItems = $state<{ item_name: string; amount: number; note: string }[]>([]);
  let tab = $state<'browse' | 'mine' | 'orders' | 'contracts'>('browse');
  // Buy orders and contracts are shared with the web player panel; this plugs them into the launcher's account session.
  setBoardHost({
    get: (id, path) => invoke('board_get', { serverId: id, path }),
    post: (id, path, body) => invoke('board_post', { serverId: id, path, body }),
    toast: (text, kind) => toast(text, kind === 'error' ? 'error' : 'ok'),
  });
  let filter = $state<'all' | 'buy_now' | 'auction'>('all');
  let sort = $state<'new' | 'low' | 'high' | 'ending'>('new');
  let query = $state('');
  let hideMine = $state(true);
  let loading = $state(false);
  let error = $state('');
  let now = $state(Date.now());
  let target = $state<Listing | null>(null);
  let bidAmount = $state(0);
  let busy = $state(false);

  const money = (v: number | null | undefined) => (v == null ? '—' : '$' + v.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }));
  function left(iso: string | null) {
    if (!iso) return '';
    const s = Math.floor((new Date(iso).getTime() - now) / 1000);
    if (s <= 0) return 'ending…';
    const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
    return d ? `${d}d ${h}h` : h ? `${h}h ${m}m` : m ? `${m}m ${s % 60}s` : `${s}s`;
  }
  const soon = (l: Listing) => !!l.ends_at && new Date(l.ends_at).getTime() - now < 10 * 60e3;
  const ago = (iso: string) => {
    const s = Math.max(1, (now - new Date(iso).getTime()) / 1000);
    return s < 3600 ? `${Math.round(s / 60)}m ago` : s < 86400 ? `${Math.round(s / 3600)}h ago` : `${Math.round(s / 86400)}d ago`;
  };

  async function loadServers() {
    try {
      const r = await invoke<{ servers: Server[] }>('get_public_servers');
      servers = r.servers;
      const inst = selectedInstance();
      servers = servers.filter(s => s.instance_id === inst?.id);
      serverId = servers[0]?.id ?? null;
    } catch (e) { error = errorText(e); }
  }
  async function load(quiet = false) {
    if (serverId == null) return;
    if (!quiet) loading = true;
    try {
      const [l, m] = await Promise.all([
        invoke<{ balance: number | null; listings: Listing[] }>('market_listings', { serverId }),
        invoke<{ history: Activity[]; waiting: number; waiting_items: typeof waitingItems }>('market_mine', { serverId }),
      ]);
      listings = l.listings; balance = l.balance; history = m.history; waiting = m.waiting; waitingItems = m.waiting_items; error = '';
    } catch (e) { error = errorText(e); } finally { loading = false; }
  }
  onMount(() => {
    void loadServers();
    const tick = setInterval(() => (now = Date.now()), 1000);
    const poll = setInterval(() => void load(true), 15000);
    return () => { clearInterval(tick); clearInterval(poll); };
  });
  $effect(() => { void serverId; listings = []; void load(); });

  const mineListings = $derived(listings.filter((l) => l.mine));
  const leadingBids = $derived(listings.filter((l) => l.leading && !l.mine));
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let list = listings.filter((l) => (filter === 'all' || l.kind === filter) && (!hideMine || !l.mine)
      && (!q || l.item_name.toLowerCase().includes(q) || l.item_id.toLowerCase().includes(q) || l.seller_name.toLowerCase().includes(q)));
    const cost = (l: Listing) => (l.kind === 'auction' ? (l.current_bid ?? l.price) : l.price);
    if (sort === 'low') list = [...list].sort((a, b) => cost(a) - cost(b));
    else if (sort === 'high') list = [...list].sort((a, b) => cost(b) - cost(a));
    else if (sort === 'ending') list = [...list].sort((a, b) => (a.ends_at ? new Date(a.ends_at).getTime() : Infinity) - (b.ends_at ? new Date(b.ends_at).getTime() : Infinity));
    return list;
  });
  const auctionsEnding = $derived(listings.filter((l) => l.kind === 'auction' && soon(l)).length);

  function open(l: Listing) {
    target = l;
    bidAmount = l.min_next_bid;
  }
  async function act(action: 'buy' | 'bid' | 'cancel', l: Listing) {
    if (serverId == null) return;
    busy = true;
    try {
      const r = await invoke<{ message?: string }>('market_act', { serverId, action, listingId: l.id, amount: action === 'bid' ? bidAmount : null });
      toast(r.message ?? 'Done', 'ok');
      target = null;
      await load(true);
    } catch (e) { toast(errorText(e), 'error'); await load(true); } finally { busy = false; }
  }
  const afford = (price: number) => balance == null || balance >= price;
</script>

<div class="market">
  <header class="top">
    <div>
      <h1><Store size={26} /> Market &amp; Auctions</h1>
      <p class="lead">Browse what players are selling, bid on auctions and buy — all from the launcher. Anything you get is delivered to your <b>/vault</b> in game, even when you are offline.</p>
    </div>
    <div class="right">
      {#if servers.length > 1}
        <select aria-label="Server" bind:value={serverId}>{#each servers as s}<option value={s.id}>{s.name}</option>{/each}</select>
      {/if}
      <span class="wallet" title="Your balance on this server"><Wallet size={15} /> {money(balance)}</span>
      <button class="ghost icon" aria-label="Refresh" title="Refresh" onclick={() => load()} disabled={loading}><RefreshCw size={16} class={loading ? 'spin' : ''} /></button>
    </div>
  </header>

  {#if waiting}
    <div class="banner"><Archive size={16} /> <span><b>{waiting} item{waiting === 1 ? '' : 's'}</b> on the way to your vault{#if waitingItems[0]} &mdash; {waitingItems.slice(0, 3).map((i) => `${i.amount}× ${i.item_name}`).join(', ')}{/if}. They arrive within a minute, as soon as the server is online.</span></div>
  {/if}
  {#if error}<div class="banner err" role="alert">{error}</div>{/if}

  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === 'browse'} class:active={tab === 'browse'} onclick={() => (tab = 'browse')}><Tag size={15} /> Browse <span class="n">{listings.length}</span></button>
    <button role="tab" aria-selected={tab === 'mine'} class:active={tab === 'mine'} onclick={() => (tab = 'mine')}><History size={15} /> My activity {#if mineListings.length + leadingBids.length}<span class="n">{mineListings.length + leadingBids.length}</span>{/if}</button>
    <button role="tab" aria-selected={tab === 'orders'} class:active={tab === 'orders'} onclick={() => (tab = 'orders')}><ClipboardList size={15} /> Buy orders</button>
    <button role="tab" aria-selected={tab === 'contracts'} class:active={tab === 'contracts'} onclick={() => (tab = 'contracts')}><ScrollText size={15} /> Contracts</button>
    {#if auctionsEnding}<span class="ending"><Flame size={14} /> {auctionsEnding} auction{auctionsEnding === 1 ? '' : 's'} ending soon</span>{/if}
  </div>

  {#snippet card(l: Listing)}
    <article class="item" class:auction={l.kind === 'auction'} class:leading={l.leading} class:hot={soon(l)}>
      <div class="head">
        <ItemTile id={l.item_id} amount={l.amount} />
        <div class="title">
          <strong title={l.item_name}>{l.item_name.replace(/§./g, '')}</strong>
          <small>{#if l.seller_guild}<Shield size={11} /> [{l.seller_guild}] guild{:else}by {l.seller_name}{/if}</small>
        </div>
        <span class="kind" class:a={l.kind === 'auction'}>{#if l.kind === 'auction'}<Gavel size={11} /> Auction{:else}<Tag size={11} /> Buy now{/if}</span>
      </div>
      <div class="price">
        {#if l.kind === 'auction'}
          <div><small>{l.current_bid != null ? 'Top bid' : 'Starting bid'}</small><b>{money(l.current_bid ?? l.price)}</b></div>
          <div class="r"><small><Clock size={11} /> ends in</small><b class="t" class:warn={soon(l)}>{left(l.ends_at)}</b></div>
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
          <span class="chip you">Your listing</span>
          <button class="ghost sm" onclick={() => open(l)}>Manage</button>
        {:else if l.kind === 'auction'}
          <button class="primary" onclick={() => open(l)}><Gavel size={14} /> {l.leading ? 'Raise bid' : 'Bid ' + money(l.min_next_bid)}</button>
        {:else}
          <button class="primary" disabled={!afford(l.price)} onclick={() => open(l)}><ShoppingCart size={14} /> Buy</button>
        {/if}
      </div>
    </article>
  {/snippet}

  {#if tab === 'orders' && serverId != null}
    {#key serverId}<Orders {serverId} onbalance={(v) => (balance = v)} />{/key}
  {:else if tab === 'contracts' && serverId != null}
    {#key serverId}<Contracts {serverId} onbalance={(v) => (balance = v)} />{/key}
  {:else if tab === 'browse'}
    <div class="bar">
      <label class="search"><Search size={15} /><input placeholder="Search items or sellers…" bind:value={query} aria-label="Search the market" /></label>
      <div class="seg" role="group" aria-label="Type">
        {#each [['all', 'All'], ['buy_now', 'Buy now'], ['auction', 'Auctions']] as [v, label]}
          <button class:on={filter === v} onclick={() => (filter = v as typeof filter)}>{label}</button>
        {/each}
      </div>
      <select bind:value={sort} aria-label="Sort"><option value="new">Newest</option><option value="ending">Ending soon</option><option value="low">Price: low to high</option><option value="high">Price: high to low</option></select>
      <label class="check"><input type="checkbox" bind:checked={hideMine} /> Hide mine</label>
    </div>
    {#if shown.length}
      <div class="grid">{#each shown as l (l.id)}{@render card(l)}{/each}</div>
    {:else}
      <div class="empty"><Store size={34} /><h3>{loading ? 'Loading the market…' : listings.length ? 'Nothing matches' : 'Nobody is selling yet'}</h3>
        <p>{listings.length ? 'Try another search or filter.' : 'In game, hold an item and type /market sell to list it — or /market auction to start an auction.'}</p></div>
    {/if}
  {:else}
    <div class="mine">
      <section>
        <h2><Package size={16} /> Your listings <span class="n">{mineListings.length}</span></h2>
        {#if mineListings.length}<div class="grid">{#each mineListings as l (l.id)}{@render card(l)}{/each}</div>{:else}<p class="muted">You aren't selling anything. List items in game with <code>/market sell</code>.</p>{/if}
      </section>
      {#if leadingBids.length}
        <section>
          <h2><Crown size={16} /> Auctions you're winning <span class="n">{leadingBids.length}</span></h2>
          <div class="grid">{#each leadingBids as l (l.id)}{@render card(l)}{/each}</div>
        </section>
      {/if}
      <section>
        <h2><TrendingUp size={16} /> Recent activity</h2>
        {#if history.length}
          <ul class="feed">
            {#each history as h, i (i)}
              <li class={h.kind}>
                <span class="ic">{#if h.kind === 'sold'}<ArrowDownLeft size={15} />{:else if h.kind === 'bought'}<ShoppingCart size={15} />{:else}<Gavel size={15} />{/if}</span>
                <span class="what"><b>{h.kind === 'sold' ? 'Sold' : h.kind === 'bought' ? 'Bought' : 'Bid'}</b> {h.description.replace(/^(Market purchase|Auction sale|Auction bid):\s*/, '')}</span>
                <span class="amt">{h.kind === 'sold' ? '+' : '−'}{money(h.amount)}</span>
                <span class="when">{ago(h.at)}</span>
              </li>
            {/each}
          </ul>
        {:else}<p class="muted">Sales, purchases and bids will show up here.</p>{/if}
      </section>
    </div>
  {/if}
</div>

<Modal open={!!target} onclose={() => (target = null)} title={target?.mine ? 'Manage listing' : target?.kind === 'auction' ? 'Place a bid' : 'Confirm purchase'} width={27}>
  {#if target}
    {@const t = target}
    <div class="dlg">
      <div class="who"><ItemTile id={t.item_id} amount={t.amount} size={3.6} /><div><strong>{t.item_name.replace(/§./g, '')}</strong><small>{t.amount}× · {t.seller_guild ? `[${t.seller_guild}] guild` : t.seller_name}</small></div></div>
      {#if t.mine}
        {#if t.kind === 'auction' && t.bid_count}
          <p class="muted">Bidding has started ({t.bid_count} bid{t.bid_count === 1 ? '' : 's'}, top {money(t.current_bid)}), so this auction can't be cancelled. It ends in {left(t.ends_at)}.</p>
        {:else}
          <p class="muted">Cancelling returns the item to your vault.</p>
          <button class="danger" disabled={busy} onclick={() => act('cancel', t)}><X size={14} /> Cancel listing</button>
        {/if}
      {:else if t.kind === 'auction'}
        <div class="rows"><span>Top bid</span><b>{money(t.current_bid ?? t.price)}</b><span>Minimum bid</span><b>{money(t.min_next_bid)}</b><span>Ends in</span><b>{left(t.ends_at)}</b><span>Your balance</span><b>{money(balance)}</b></div>
        <label>Your bid<input type="number" min={t.min_next_bid} step="1" bind:value={bidAmount} /></label>
        <div class="quick">
          {#each [1, 1.1, 1.25, 1.5] as m}<button class="ghost sm" onclick={() => (bidAmount = Math.round(t.min_next_bid * m * 100) / 100)}>{m === 1 ? 'Minimum' : '+' + Math.round((m - 1) * 100) + '%'}</button>{/each}
        </div>
        <p class="muted">Your bid is held while you lead and refunded if someone outbids you. Win it and the item goes to your vault.</p>
        <button class="primary" disabled={busy || bidAmount < t.min_next_bid || !afford(bidAmount)} onclick={() => act('bid', t)}><Gavel size={14} /> Bid {money(bidAmount)}</button>
        {#if !afford(bidAmount)}<p class="err">You can't afford that.</p>{/if}
      {:else}
        <div class="rows"><span>Price</span><b>{money(t.price)}</b><span>Your balance</span><b>{money(balance)}</b><span>After buying</span><b class:warn={!afford(t.price)}>{money((balance ?? 0) - t.price)}</b></div>
        <p class="muted">The item is delivered to your <b>/vault</b> in game — you don't need to be online.</p>
        <button class="primary" disabled={busy || !afford(t.price)} onclick={() => act('buy', t)}><ShoppingCart size={14} /> Buy for {money(t.price)}</button>
      {/if}
    </div>
  {/if}
</Modal>

<style>
  .market { display: flex; flex-direction: column; gap: 1rem; padding: 1.5rem 1.75rem 2rem; overflow-y: auto; height: 100%; }
  .top { display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start; flex-wrap: wrap; }
  h1 { display: flex; align-items: center; gap: 0.6rem; font-size: 1.6rem; }
  .lead { color: var(--muted); max-width: 40rem; margin-top: 0.3rem; font-size: 0.88rem; }
  .right { display: flex; align-items: center; gap: 0.6rem; }
  .wallet { display: inline-flex; align-items: center; gap: 0.4rem; padding: 0.45rem 0.8rem; border-radius: 99rem; background: color-mix(in srgb, var(--success) 14%, transparent); border: 1px solid color-mix(in srgb, var(--success) 35%, transparent); color: var(--success); font-weight: 650; font-size: 0.85rem; }
  .banner { display: flex; align-items: center; gap: 0.6rem; padding: 0.65rem 0.9rem; border-radius: var(--radius-sm); font-size: 0.84rem; background: color-mix(in srgb, var(--accent) 12%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); }
  .banner.err { background: color-mix(in srgb, var(--danger) 12%, transparent); border-color: color-mix(in srgb, var(--danger) 40%, transparent); color: var(--danger); }
  .tabs { display: flex; align-items: center; gap: 0.4rem; border-bottom: 1px solid var(--line); }
  .tabs button { display: inline-flex; align-items: center; gap: 0.45rem; border: 0; border-bottom: 2px solid transparent; border-radius: 0; background: transparent; padding: 0.6rem 0.9rem; color: var(--muted); }
  .tabs button.active { color: var(--text); border-bottom-color: var(--accent); }
  .n { font-size: 0.7rem; padding: 0.05rem 0.45rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 10%, transparent); }
  .ending { margin-left: auto; display: inline-flex; align-items: center; gap: 0.35rem; color: var(--warn); font-size: 0.8rem; }
  .bar select, .right select { width: auto; }
  .bar { display: flex; gap: 0.6rem; align-items: center; flex-wrap: wrap; }
  .search { flex: 1 1 14rem; display: flex; align-items: center; gap: 0.5rem; padding: 0 0.7rem; border: 1px solid var(--line); border-radius: var(--radius-sm); color: var(--muted); }
  .search:focus-within { border-color: var(--accent); }
  .search input { border: 0; background: transparent; box-shadow: none; outline: none; padding: 0.55rem 0; width: 100%; }
  .seg { display: inline-flex; border: 1px solid var(--line); border-radius: var(--radius-sm); overflow: hidden; }
  .seg button { border: 0; border-radius: 0; background: transparent; padding: 0.5rem 0.8rem; font-size: 0.82rem; }
  .seg button.on { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--text); }
  .check { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.82rem; color: var(--muted); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(15.5rem, 1fr)); gap: 0.85rem; }
  .item { display: flex; flex-direction: column; gap: 0.7rem; padding: 0.9rem; border: 1px solid var(--line); border-radius: var(--radius); background: var(--surface); transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s; }
  .item:hover { transform: translateY(-2px); border-color: var(--line-strong); box-shadow: 0 0.8rem 1.6rem -1rem #000a; }
  .item.auction { background: linear-gradient(160deg, color-mix(in srgb, var(--accent) 9%, var(--surface)), var(--surface) 60%); }
  .item.leading { border-color: color-mix(in srgb, var(--success) 55%, transparent); }
  .item.hot { border-color: color-mix(in srgb, var(--warn) 55%, transparent); }
  .head { display: flex; gap: 0.7rem; align-items: center; }
  .title { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .title strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.92rem; }
  .title small { display: inline-flex; align-items: center; gap: 0.25rem; color: var(--muted); font-size: 0.74rem; }
  .kind { align-self: flex-start; display: inline-flex; align-items: center; gap: 0.25rem; font-size: 0.64rem; text-transform: uppercase; letter-spacing: 0.05em; padding: 0.15rem 0.45rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 9%, transparent); color: var(--muted); }
  .kind.a { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--accent); }
  .price { display: flex; justify-content: space-between; gap: 0.5rem; }
  .price small { display: flex; align-items: center; gap: 0.25rem; font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .price b { font-size: 1.15rem; }
  .price .r { text-align: right; } .price .r small { justify-content: flex-end; }
  .price .t { font-size: 0.95rem; font-variant-numeric: tabular-nums; }
  .t.warn, .warn { color: var(--warn); }
  .sub { font-size: 0.76rem; color: var(--muted); margin: -0.2rem 0 0; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; margin-top: auto; }
  .foot .primary { flex: 1; display: inline-flex; align-items: center; justify-content: center; gap: 0.4rem; }
  .chip.you { font-size: 0.74rem; color: var(--accent); }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 0.4rem; padding: 3rem 1rem; color: var(--muted); text-align: center; }
  .empty h3 { color: var(--text); } .empty p { max-width: 28rem; font-size: 0.86rem; }
  .mine { display: flex; flex-direction: column; gap: 1.6rem; }
  .mine h2 { display: flex; align-items: center; gap: 0.5rem; font-size: 1rem; margin-bottom: 0.7rem; }
  .muted { color: var(--muted); font-size: 0.85rem; } code { font-size: 0.8rem; background: color-mix(in srgb, var(--text) 8%, transparent); padding: 0.05rem 0.35rem; border-radius: 0.3rem; }
  .feed { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.35rem; }
  .feed li { display: grid; grid-template-columns: 2rem 1fr auto 5rem; gap: 0.7rem; align-items: center; padding: 0.55rem 0.8rem; border-radius: var(--radius-sm); background: var(--surface); border: 1px solid var(--line); font-size: 0.84rem; }
  .feed .ic { width: 1.8rem; height: 1.8rem; border-radius: 0.5rem; display: grid; place-items: center; background: color-mix(in srgb, var(--text) 8%, transparent); }
  .feed .sold .ic, .feed .sold .amt { color: var(--success); } .feed .bought .ic, .feed .bought .amt { color: var(--danger); } .feed .bid .ic { color: var(--accent); }
  .feed .what { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .feed .amt { font-weight: 650; font-variant-numeric: tabular-nums; } .feed .when { color: var(--muted); font-size: 0.74rem; text-align: right; }
  .dlg { display: flex; flex-direction: column; gap: 0.8rem; }
  .who { display: flex; gap: 0.8rem; align-items: center; } .who small { display: block; color: var(--muted); }
  .rows { display: grid; grid-template-columns: 1fr auto; gap: 0.35rem 1rem; font-size: 0.86rem; padding: 0.7rem 0.8rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 5%, transparent); }
  .rows span { color: var(--muted); } .rows b { text-align: right; font-variant-numeric: tabular-nums; }
  .quick { display: flex; gap: 0.4rem; flex-wrap: wrap; }
  .dlg .primary, .dlg .danger { display: inline-flex; align-items: center; justify-content: center; gap: 0.4rem; }
  .err { color: var(--danger); font-size: 0.8rem; margin: 0; }
  :global(.spin) { animation: spin 0.8s linear infinite; } @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 760px) { .market { padding: 1rem; } .feed li { grid-template-columns: 2rem 1fr auto; } .feed .when { display: none; } }
</style>
