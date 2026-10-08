<script lang="ts">
  import { onMount } from 'svelte';
  import { PackagePlus, Search, Hand, Undo2, Trash2, Clock, Inbox, Info, CheckCircle2, ArrowRight } from '@lucide/svelte';
  import { ago, bget, bpost, COMMON_ITEMS, errorText, money, toast, untilText, type Order, type OrdersBoard } from './host';

  let { serverId, onbalance }: { serverId: number; onbalance?: (v: number | null) => void } = $props();
  let board = $state<OrdersBoard | null>(null);
  let error = $state('');
  let busy = $state(false);
  let query = $state('');
  let showMine = $state(true);
  let now = $state(Date.now());
  let item = $state('COBBLESTONE');
  let amount = $state(64);
  let total = $state(100);

  async function load(quiet = true) {
    try {
      board = await bget<OrdersBoard>(serverId, '/orders');
      onbalance?.(board.balance);
      error = '';
    } catch (e) { if (!quiet) error = errorText(e); else error = errorText(e); }
  }
  onMount(() => { void load(); const t = setInterval(() => (now = Date.now()), 1000); const p = setInterval(() => void load(), 15000); return () => { clearInterval(t); clearInterval(p); }; });
  $effect(() => { void serverId; board = null; void load(); });

  async function act(path: string, body: Record<string, unknown> = {}, ok?: string) {
    busy = true;
    try {
      const r = await bpost<{ message?: string; balance?: number }>(serverId, path, body);
      toast(ok ?? r.message ?? 'Done', 'ok');
      await load();
    } catch (e) { toast(errorText(e), 'error'); await load(); } finally { busy = false; }
  }
  const post = () => act('/orders', { item_id: item.trim(), amount, total }, `Order posted. ${money(total)} is held until it's filled.`);

  const each = $derived(amount > 0 ? total / amount : 0);
  const afford = $derived(board?.balance == null || board.balance >= total);
  const valid = $derived(!!item.trim() && amount >= 1 && total >= (board?.rules.min_total ?? 1) && afford && (board?.my_open ?? 0) < (board?.rules.max_open ?? 5));
  const list = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return (board?.orders ?? []).filter((o) => (showMine || !o.mine) && (!q || o.item_name.toLowerCase().includes(q) || o.item_id.toLowerCase().includes(q) || o.buyer_name.toLowerCase().includes(q)));
  });
  const mine = $derived((board?.orders ?? []).filter((o) => o.mine));
  const picked = $derived((board?.orders ?? []).filter((o) => o.claimed_by_me));
</script>

<div class="orders">
  {#if error && !board}<div class="banner err" role="alert">{error}</div>{/if}
  {#if board && !board.enabled}<div class="banner">Buy orders are switched off on this server.</div>{/if}

  {#if board?.enabled}
    <section class="panel how">
      <h2><Info size={16} /> How buy orders work</h2>
      <ol>
        <li><b>Ask</b> for the items you need and set the price. The money is held safely (escrow) until someone delivers.</li>
        <li><b>Someone picks it up</b> from this board and gathers the items. It's reserved for them for {board.rules.claim_minutes} minutes.</li>
        <li><b>They hand the items in</b> with <code>/orders fill &lt;id&gt;</code> in game. The items appear in <b>your vault</b> and they get paid.</li>
      </ol>
    </section>

    <section class="panel form">
      <h2><PackagePlus size={16} /> Post a request</h2>
      <div class="fields">
        <label>Item<input list="board-items" bind:value={item} placeholder="DIAMOND" autocapitalize="characters" spellcheck="false" /><datalist id="board-items">{#each COMMON_ITEMS as i}<option value={i}></option>{/each}</datalist></label>
        <label>How many<input type="number" min="1" max={board.rules.max_amount} bind:value={amount} /></label>
        <label>Total you'll pay ($)<input type="number" min={board.rules.min_total} max={board.rules.max_total} step="1" bind:value={total} /></label>
      </div>
      <div class="sum"><span>{money(each)} each</span><span>Held in escrow: <b>{money(total)}</b></span>{#if board.balance != null}<span class:bad={!afford}>Balance {money(board.balance)}</span>{/if}</div>
      <button class="primary" onclick={post} disabled={busy || !valid}><PackagePlus size={16} /> Post order</button>
      <small class="muted">You can have {board.rules.max_open} open at once ({board.my_open} now). Unfilled orders come down after {Math.round(board.rules.expire_hours / 24 * 10) / 10 || 1} days and refund you. Items must be plain (no enchants or custom names).{board.rules.fee_percent > 0 ? ` The house keeps ${board.rules.fee_percent}% of the price.` : ''}</small>
    </section>

    {#if picked.length}
      <section class="panel">
        <h2><Hand size={16} /> You picked up</h2>
        <div class="list">{#each picked as o (o.id)}{@render card(o)}{/each}</div>
      </section>
    {/if}

    <section class="panel">
      <div class="bar">
        <h2><Inbox size={16} /> The board <span class="n">{list.length}</span></h2>
        <label class="search"><Search size={14} /><input placeholder="Search items or players" bind:value={query} /></label>
        <label class="chk"><input type="checkbox" bind:checked={showMine} /> Show mine</label>
      </div>
      {#if list.length}<div class="list">{#each list as o (o.id)}{@render card(o)}{/each}</div>
      {:else}<p class="muted">Nothing is wanted right now. Post a request above and someone might have it.</p>{/if}
    </section>

    {#if board.history.length}
      <section class="panel">
        <h2><CheckCircle2 size={16} /> Recent</h2>
        <ul class="hist">{#each board.history as o (o.id)}
          <li><b>{o.amount}× {o.item_name}</b> <span class="chip {o.resolved}">{o.resolved}</span> <span class="muted">{o.mine ? (o.resolved === 'filled' ? `filled by ${o.filler_name}` : '') : `for ${o.buyer_name}`}</span><small>{ago(o.created_at, now)}</small><span class="amt">{money(o.total)}</span></li>{/each}</ul>
      </section>
    {/if}
  {/if}
</div>

{#snippet card(o: Order)}
  <article class="card" class:mine={o.mine} class:taken={o.status === 'claimed'}>
    <div class="what"><span class="qty">{o.amount}×</span><div><b>{o.item_name}</b><small>{o.mine ? 'Your order' : `from ${o.buyer_name}`} · {ago(o.created_at, now)}</small></div></div>
    <div class="price"><b>{money(o.total)}</b><small>{money(o.each)} each</small></div>
    <div class="state">
      {#if o.status === 'claimed'}<span class="chip taken"><Clock size={11} /> {o.claimed_by_me ? 'yours' : `${o.claimer_name}`} · {untilText(o.claim_until, now)}</span>{:else}<span class="chip open">open</span>{/if}
    </div>
    <div class="act">
      {#if o.mine}
        <button class="ghost" onclick={() => act(`/orders/${o.id}/cancel`, {}, 'Order cancelled, your money is back.')} disabled={busy}><Trash2 size={14} /> Cancel</button>
      {:else if o.claimed_by_me}
        <span class="how">In game: <code>/orders fill {o.id}</code></span>
        <button class="ghost" onclick={() => act(`/orders/${o.id}/release`, {}, 'Order released.')} disabled={busy}><Undo2 size={14} /> Give up</button>
      {:else if o.status === 'open'}
        <button class="primary sm" onclick={() => act(`/orders/${o.id}/claim`, {}, `Order #${o.id} is yours. Bring the items and use /orders fill ${o.id} in game.`)} disabled={busy}><Hand size={14} /> Pick up <ArrowRight size={13} /></button>
      {/if}
    </div>
  </article>
{/snippet}

<style>
  .orders { display: flex; flex-direction: column; gap: 1rem; }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.8rem; min-width: 0; }
  h2 { display: flex; gap: 0.45rem; align-items: center; font-size: 1rem; }
  .n { font-size: 0.72rem; padding: 0.05rem 0.5rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 10%, transparent); }
  .how ol { margin: 0; padding-left: 1.2rem; display: flex; flex-direction: column; gap: 0.3rem; color: color-mix(in srgb, var(--text) 82%, transparent); font-size: 0.9rem; }
  code { font-family: var(--mono, monospace); background: color-mix(in srgb, var(--text) 10%, transparent); padding: 0.05rem 0.4rem; border-radius: 0.35rem; font-size: 0.85em; }
  .fields { display: grid; grid-template-columns: 1.4fr 1fr 1fr; gap: 0.7rem; }
  .fields label { display: flex; flex-direction: column; gap: 0.25rem; font-size: 0.8rem; font-weight: 600; min-width: 0; }
  .sum { display: flex; gap: 1rem; flex-wrap: wrap; font-size: 0.86rem; color: var(--muted); } .sum b { color: var(--text); } .bad { color: #f87171; }
  .muted { color: var(--muted); font-size: 0.82rem; }
  .primary { display: inline-flex; gap: 0.45rem; align-items: center; justify-content: center; padding: 0.75rem 1.2rem; border: none; border-radius: var(--radius); background: linear-gradient(135deg, #22c55e, #0d9488); color: #fff; font-weight: 800; box-shadow: 0 6px 22px -8px #22c55e; touch-action: manipulation; }
  .primary.sm { padding: 0.5rem 0.9rem; font-size: 0.85rem; } .primary:disabled { opacity: 0.5; }
  .ghost { display: inline-flex; gap: 0.35rem; align-items: center; padding: 0.45rem 0.8rem; font-size: 0.82rem; }
  .bar { display: flex; gap: 0.8rem; align-items: center; flex-wrap: wrap; } .bar h2 { margin-right: auto; }
  .search { display: flex; align-items: center; gap: 0.4rem; padding: 0 0.7rem; border: 1px solid var(--line-strong); border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 4%, transparent); } .search input { border: none; background: transparent; padding: 0.5rem 0; min-width: 9rem; }
  .chk { display: inline-flex; gap: 0.35rem; align-items: center; font-size: 0.82rem; color: var(--muted); }
  .list { display: flex; flex-direction: column; gap: 0.5rem; }
  .card { display: grid; grid-template-columns: minmax(0, 1.6fr) auto auto auto; gap: 0.9rem; align-items: center; padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 4%, transparent); border: 1px solid var(--line); }
  .card.mine { border-color: color-mix(in srgb, var(--accent) 50%, transparent); } .card.taken { opacity: 0.85; }
  .what { display: flex; gap: 0.7rem; align-items: center; min-width: 0; } .what b { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .what small, .price small { color: var(--muted); font-size: 0.75rem; display: block; }
  .qty { font-size: 1.15rem; font-weight: 900; color: #4ade80; font-variant-numeric: tabular-nums; min-width: 3.4rem; text-align: right; }
  .price { text-align: right; } .price b { font-size: 1.05rem; font-variant-numeric: tabular-nums; }
  .chip { display: inline-flex; gap: 0.25rem; align-items: center; font-size: 0.7rem; font-weight: 700; padding: 0.15rem 0.55rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 10%, transparent); text-transform: capitalize; }
  .chip.open { background: #22c55e22; color: #4ade80; } .chip.taken { background: #f59e0b22; color: #fbbf24; } .chip.filled { background: #22c55e22; color: #4ade80; } .chip.cancelled, .chip.expired { background: #64748b33; color: #94a3b8; }
  .act { display: flex; gap: 0.5rem; align-items: center; justify-content: flex-end; flex-wrap: wrap; } .how { font-size: 0.78rem; color: var(--muted); }
  .hist { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; } .hist li { display: flex; gap: 0.6rem; align-items: center; flex-wrap: wrap; padding: 0.4rem 0; border-bottom: 1px solid var(--line); font-size: 0.86rem; } .hist small { margin-left: auto; color: var(--muted); font-size: 0.72rem; } .amt { font-variant-numeric: tabular-nums; font-weight: 700; }
  .banner { padding: 0.7rem 1rem; border-radius: var(--radius-sm); font-size: 0.88rem; background: color-mix(in srgb, var(--text) 8%, transparent); } .banner.err { background: color-mix(in srgb, var(--danger, #e5484d) 16%, transparent); border: 1px solid var(--danger, #e5484d); }
  @media (max-width: 720px) {
    .fields { grid-template-columns: 1fr 1fr; } .fields label:first-child { grid-column: span 2; }
    .card { grid-template-columns: minmax(0, 1fr) auto; } .state { grid-column: 1; } .act { grid-column: 1 / -1; justify-content: stretch; } .act button { flex: 1; justify-content: center; }
    .search input { min-width: 6rem; }
  }
</style>
