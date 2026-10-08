<script lang="ts">
  import { onMount } from 'svelte';
  import { ClipboardList, ScrollText, Save, Trash2, RotateCcw } from '@lucide/svelte';
  import Toggle from '../components/Toggle.svelte';
  import { get, post, put, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type OrdersCfg = { enabled: boolean; max_open_per_player: number; min_total: number; max_total: number; max_amount: number; expire_hours: number; claim_minutes: number; max_claims_per_player: number; fee_percent: number };
  type ContractsCfg = { enabled: boolean; board_size: number; expire_hours: number; reward_multiplier: number; daily_limit: number; rerolls_per_day: number; kills: boolean; gather: boolean };
  type OrderRow = { id: number; server_id: number; buyer: string; item: string; amount: number; total: number; status: string; at: string };

  let orders = $state<OrdersCfg | null>(null), ordersDefaults = $state<OrdersCfg | null>(null);
  let contracts = $state<ContractsCfg | null>(null), contractsDefaults = $state<ContractsCfg | null>(null);
  let list = $state<OrderRow[]>([]);
  let stats = $state<{ completed: number; paid: number; active: number } | null>(null);
  let busy = $state(false);

  async function load() {
    try {
      const o = await get<{ config: OrdersCfg; defaults: OrdersCfg; orders: OrderRow[] }>('/api/admin/economy/orders');
      orders = o.config; ordersDefaults = o.defaults; list = o.orders;
      const c = await get<{ config: ContractsCfg; defaults: ContractsCfg; stats: typeof stats }>('/api/admin/economy/contracts');
      contracts = c.config; contractsDefaults = c.defaults; stats = c.stats;
    } catch (e) { toastError(e); }
  }
  onMount(load);

  async function save() {
    busy = true;
    try {
      const o = await put<{ config: OrdersCfg }>('/api/admin/economy/orders', orders);
      const c = await put<{ config: ContractsCfg }>('/api/admin/economy/contracts', contracts);
      orders = o.config; contracts = c.config;
      toast('Saved. Players see the new rules right away.');
    } catch (e) { toastError(e); } finally { busy = false; }
  }
  async function remove(o: OrderRow) {
    if (!confirm(`Take down ${o.buyer}'s order for ${o.amount}× ${o.item} and refund them?`)) return;
    try { await post(`/api/admin/economy/orders/${o.id}/cancel`, {}); toast('Order removed and refunded.'); await load(); } catch (e) { toastError(e); }
  }
  const money = (v: number) => '$' + v.toLocaleString(undefined, { maximumFractionDigits: 2 });
</script>

<div class="page">
  <header class="head">
    <div>
      <h1><ClipboardList size={26} /> Buy orders &amp; contracts</h1>
      <p>Buy orders let players ask for items with the money held in escrow until someone delivers them. Contracts are random tasks (kill mobs, hand in resources) that pay money. Both are played with <code>/orders</code> and <code>/contracts</code> in game and from the Market page.</p>
    </div>
    <button class="primary" onclick={save} disabled={busy || !orders || !contracts}><Save size={15} /> Save changes</button>
  </header>

  {#if orders}
    <section class="card">
      <header class="sec"><h2><ClipboardList size={18} /> Buy orders</h2><Toggle bind:checked={orders.enabled} label="On" /></header>
      <div class="fields">
        <label>Open orders per player<input type="number" min="1" max="100" bind:value={orders.max_open_per_player} /></label>
        <label>Most items per order<input type="number" min="1" bind:value={orders.max_amount} /></label>
        <label>Smallest price ($)<input type="number" min="0.01" bind:value={orders.min_total} /></label>
        <label>Biggest price ($)<input type="number" min="1" bind:value={orders.max_total} /></label>
        <label>Orders expire after (hours)<input type="number" min="1" bind:value={orders.expire_hours} /><small>Unfilled orders are refunded.</small></label>
        <label>Pick-up reservation (minutes)<input type="number" min="5" bind:value={orders.claim_minutes} /><small>How long a player holds an order they picked up.</small></label>
        <label>Orders one player can pick up<input type="number" min="1" bind:value={orders.max_claims_per_player} /></label>
        <label>House fee (%)<input type="number" min="0" max="50" step="0.5" bind:value={orders.fee_percent} /><small>Taken from the filler's payment. 0 keeps it free.</small></label>
      </div>
      <button type="button" class="ghost reset" onclick={() => ordersDefaults && (orders = structuredClone($state.snapshot(ordersDefaults)))}><RotateCcw size={14} /> Standard settings</button>
    </section>

    <section class="card">
      <h2>Orders on the board</h2>
      <table>
        <thead><tr><th>#</th><th>Buyer</th><th>Wants</th><th class="n">Held</th><th>Status</th><th></th></tr></thead>
        <tbody>
          {#each list as o (o.id)}
            <tr><td>{o.id}</td><td>{o.buyer}</td><td>{o.amount}× {o.item} <small class="muted">{timeAgo(o.at)}</small></td><td class="n">{money(o.total)}</td><td>{o.status}</td>
              <td class="n">{#if o.status === 'open' || o.status === 'claimed'}<button class="ghost sm" onclick={() => remove(o)}><Trash2 size={13} /> Remove</button>{/if}</td></tr>
          {:else}<tr><td colspan="6" class="muted">No orders yet.</td></tr>{/each}
        </tbody>
      </table>
    </section>
  {/if}

  {#if contracts}
    <section class="card">
      <header class="sec"><h2><ScrollText size={18} /> Contracts</h2><Toggle bind:checked={contracts.enabled} label="On" /></header>
      {#if stats}<p class="muted">{stats.completed.toLocaleString()} finished so far, {money(stats.paid)} paid out, {stats.active} active right now.</p>{/if}
      <div class="fields">
        <label>Contracts on each player's board<input type="number" min="1" max="12" bind:value={contracts.board_size} /></label>
        <label>Contracts expire after (hours)<input type="number" min="1" bind:value={contracts.expire_hours} /></label>
        <label>Payout multiplier<input type="number" min="0.1" max="20" step="0.1" bind:value={contracts.reward_multiplier} /><small>1 is the standard rate; 2 doubles every payout.</small></label>
        <label>Most finished per player per day<input type="number" min="0" bind:value={contracts.daily_limit} /><small>0 means no limit.</small></label>
        <label>Swaps per player per day<input type="number" min="0" bind:value={contracts.rerolls_per_day} /></label>
      </div>
      <div class="row">
        <Toggle bind:checked={contracts.kills} label="Kill contracts" help="Mob kills count by themselves" />
        <Toggle bind:checked={contracts.gather} label="Resource contracts" help="Items are handed in and used up" />
      </div>
      <button type="button" class="ghost reset" onclick={() => contractsDefaults && (contracts = structuredClone($state.snapshot(contractsDefaults)))}><RotateCcw size={14} /> Standard settings</button>
    </section>
  {/if}
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 16px; }
  .head { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; flex-wrap: wrap; }
  .head h1 { display: flex; gap: 10px; align-items: center; } .head p { color: var(--muted); max-width: 46rem; margin: 4px 0 0; }
  .card { padding: 18px 20px; display: flex; flex-direction: column; gap: 14px; }
  .card h2 { margin: 0; font-size: 1rem; display: flex; align-items: center; gap: 8px; }
  .sec { display: flex; justify-content: space-between; align-items: center; }
  .fields { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 12px 16px; }
  .fields label { display: flex; flex-direction: column; gap: 4px; font-size: 0.82rem; color: var(--text-2); }
  .fields small { color: var(--muted); font-size: 0.74rem; }
  .row { display: flex; gap: 24px; flex-wrap: wrap; } .muted { color: var(--muted); }
  .reset { align-self: flex-start; display: inline-flex; gap: 6px; align-items: center; }
  .n { text-align: right; } table { width: 100%; border-collapse: collapse; } th, td { padding: 8px 10px; border-bottom: 1px solid var(--line); text-align: left; font-size: 0.88rem; } th.n, td.n { text-align: right; }
  code { background: var(--surface-2); padding: 1px 6px; border-radius: 5px; }
</style>
