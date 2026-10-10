<script lang="ts">
  import { untrack } from 'svelte';
  import { errorText, money } from './host';
  type Stack = { slot: number; item: string; count: number; name: string; enchanted?: boolean };
  type Vault = { key: string; owner: string; number: number; label: string; rows: number; unlocked: boolean; price_cents: number | null; revision: number; in_game: boolean; contents: Stack[] };
  type Data = { vaults: Vault[]; incoming: { id: number; item_name: string; amount: number }[]; balance: number | null };
  type Place = { key: string; number: number; slot: number; revision: number };
  let { serverId, get, post, onbalance }: {
    serverId: number; get: (server: number) => Promise<Data>;
    post: (server: number, action: 'move' | 'buy', body: Record<string, unknown>) => Promise<unknown>;
    onbalance?: (balance: number | null) => void;
  } = $props();
  let data = $state<Data | null>(null), error = $state(''), busy = $state(false), loading = $state(true);
  let selected = $state<Place | null>(null);
  let purchase = $state<{ number: number; price: number; operation: string } | null>(null);
  let generation = 0;
  async function load(server = serverId, current = generation) {
    loading = true;
    try {
      const result = await get(server);
      if (current !== generation) return;
      data = result; selected = null; error = ''; onbalance?.(result.balance);
    } catch (e) { if (current === generation) error = errorText(e); }
    finally { if (current === generation) loading = false; }
  }
  $effect(() => {
    const server = serverId, current = ++generation;
    data = null; selected = null; purchase = null;
    untrack(() => void load(server, current));
    return () => { ++generation; };
  });
  async function choose(vault: Vault, slot: number) {
    if (busy || !vault.unlocked || vault.in_game || purchase) return;
    const place = { key: vault.key, number: vault.number, slot, revision: vault.revision };
    if (!selected) {
      if (vault.contents.some(s => s.slot === slot)) selected = place;
      return;
    }
    if (selected.key === place.key && selected.number === place.number && selected.slot === slot) { selected = null; return; }
    const from = selected, server = serverId, current = generation;
    busy = true;
    try {
      await post(server, 'move', { from, to: place });
      if (current === generation) await load(server, current);
    } catch (e) { if (current === generation) { selected = null; error = errorText(e); } }
    finally { busy = false; }
  }
  async function buy() {
    if (!purchase || busy) return;
    const pending = purchase, server = serverId, current = generation;
    busy = true;
    try {
      await post(server, 'buy', { number: pending.number, expected_price_cents: pending.price, operation_id: pending.operation });
      if (current === generation) { purchase = null; await load(server, current); }
    } catch (e) { if (current === generation) error = errorText(e); }
    finally { busy = false; }
  }
</script>

<section class="vaults">
  <header><div><span class="eyebrow">VELORA SMP / CLOUD STORAGE</span><h2>Your vaults</h2><p>Select a stack, then its destination to move or swap it. Close vaults in game before rearranging them here.</p></div><button disabled={busy || loading || !!purchase} onclick={() => load()}>Refresh</button></header>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if loading && !data}<p>Loading vaults…</p>{/if}
  {#if data}
    {#if data.incoming.length}<aside><strong>Deliveries in transit</strong><p>{data.incoming.map(item => `${item.amount}× ${item.item_name}`).join(', ')}. Items arrive when the game server processes delivery and an unlocked vault has room.</p></aside>{/if}
    {#if selected}<aside>Choose a destination slot. <button disabled={busy} onclick={() => selected = null}>Cancel move</button></aside>{/if}
    {#if purchase}<aside class="review"><strong>Unlock Vault {purchase.number} for {money(purchase.price / 100)}?</strong><p>Uses your in-game dollars. Balance: {money(data.balance)}.</p><button disabled={busy} onclick={buy}>{busy ? 'Purchasing…' : 'Confirm unlock'}</button> <button disabled={busy} onclick={() => { purchase = null; error = ''; }}>Cancel</button></aside>{/if}
    <div class="pages">{#each data.vaults as vault (`${vault.key}:${vault.number}`)}
      <article><div class="heading"><h3>{vault.label}</h3><span>{vault.owner === 'faction' ? 'Faction storage' : 'Personal storage'}</span></div>
        {#if !vault.unlocked}<p>Locked</p>{#if vault.price_cents != null}<button disabled={busy || !!purchase} onclick={() => { selected = null; purchase = { number: vault.number, price: vault.price_cents!, operation: crypto.randomUUID() }; }}>Review unlock · {money(vault.price_cents / 100)}</button>{/if}
        {:else}
          {#if vault.in_game}<p class="locked">Open in game · viewing only</p>{/if}
          <div class="inventory" aria-label={vault.label}>{#each Array.from({ length: Math.min(63, vault.rows * 9) }, (_, slot) => slot) as slot}
            {@const item = vault.contents.find(item => item.slot === slot)}
            <button class="slot" class:selected={selected?.key === vault.key && selected.number === vault.number && selected.slot === slot} class:enchanted={item?.enchanted}
              disabled={busy || vault.in_game || !!purchase} aria-pressed={selected?.key === vault.key && selected.number === vault.number && selected.slot === slot}
              aria-label={`${vault.label}, slot ${slot + 1}: ${item ? `${item.count} ${item.name || item.item}` : 'empty'}`} title={item?.name || item?.item || 'Empty slot'} onclick={() => choose(vault, slot)}>
              {#if item}<span class="item">{item.name || item.item.split(':').pop()?.replaceAll('_', ' ')}</span><b>{item.count}</b>{/if}
            </button>
          {/each}</div>
        {/if}
      </article>
    {/each}</div>
  {/if}
</section>

<style>
  .vaults { color: var(--text, #f0edf8); display: grid; gap: 1rem; }
  header, .heading { display: flex; justify-content: space-between; align-items: center; gap: 1rem; } header { flex-wrap: wrap; }
  h2 { margin: .5rem 0; font-size: 1.8rem; } h3 { margin: 0; } p { color: var(--muted, #a99bbf); line-height: 1.5; }
  .eyebrow { color: #be9eea; font-size: .7rem; letter-spacing: .15em; }
  .pages { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 420px), 1fr)); gap: 1rem; }
  article, aside { padding: 1rem; border: 1px solid #a386cd33; background: var(--surface, #171421); border-radius: 16px; }
  .heading span { color: var(--muted, #a99bbf); font-size: .75rem; }
  button { font: inherit; color: inherit; border: 1px solid #a386cd55; background: #a386cd18; border-radius: 8px; padding: .7rem; cursor: pointer; } button:disabled { opacity: .55; cursor: default; }
  .inventory { margin-top: 1rem; display: grid; grid-template-columns: repeat(9, minmax(0, 1fr)); gap: 4px; }
  .slot { position: relative; min-width: 0; aspect-ratio: 1; padding: 3px; display: grid; place-items: center; background: #ffffff05; }
  .item { font-size: clamp(.45rem, 1.5vw, .65rem); overflow-wrap: anywhere; line-height: 1.1; text-align: center; } b { position: absolute; right: 3px; bottom: 1px; font-size: .65rem; background: #171421dd; }
  .selected { outline: 2px solid #c6a3fa; background: #7954ac55; } .enchanted { box-shadow: inset 0 0 14px #b183ec33; }
  .locked { color: #e0c389; } .review { border-color: #be9eea; } .error { color: #f2a9b3; }
</style>
