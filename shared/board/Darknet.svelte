<script lang="ts">
  import { bget, bpost, errorText, money, toast } from './host';
  import type { Snippet } from 'svelte';
  type Product = { id: string; item_id: string; item_name: string; amount: number; price_cents: number; vault_number?: number };
  let { serverId, onbalance, item }: { serverId: number; onbalance?: (value: number | null) => void; item?: Snippet<[string, number]> } = $props();
  let search = $state('');
  let sort = $state('featured');
  const shown = $derived.by(() => {
    const matches = products.filter(p => `${p.item_name} ${p.item_id}`.toLowerCase().includes(search.trim().toLowerCase()));
    return sort === 'featured' ? matches : matches.sort((a, b) => sort === 'price' ? a.price_cents - b.price_cents : a.item_name.localeCompare(b.item_name));
  });
  let products = $state<Product[]>([]);
  let balance = $state<number | null>(null);
  let error = $state('');
  let loading = $state(true);
  let busy = $state(false);
  let pending = $state<{ product: Product; operation: string } | null>(null);
  let generation = 0;
  let review = $state<HTMLDivElement>();
  $effect(() => {
    if (pending && review) { review.scrollIntoView({ block: 'nearest', behavior: 'smooth' }); review.focus({ preventScroll: true }); }
  });
  $effect(() => {
    const server = serverId, current = ++generation;
    products = []; balance = null; pending = null; error = ''; loading = true;
    bget<{ products: Product[]; balance: number | null }>(server, '/darknet').then(data => {
      if (current !== generation) return;
      products = data.products; balance = data.balance; onbalance?.(balance);
    }).catch(e => { if (current === generation) error = errorText(e); }).finally(() => { if (current === generation) loading = false; });
  });
  async function buy() {
    if (!pending || busy) return;
    const purchase = pending, server = serverId, current = generation;
    busy = true;
    try {
      const result = await bpost<{ balance: number }>(server, '/darknet/buy', { product_id: purchase.product.id, operation_id: purchase.operation, expected_price_cents: purchase.product.price_cents });
      if (current !== generation) return;
      balance = result.balance; onbalance?.(balance); pending = null; error = '';
      toast(purchase.product.vault_number ? 'Vault unlocked. Open it from Vaults or in game.' : 'Purchased. Your items are queued for your in-game vault.');
      if (purchase.product.vault_number) {
        products = products.filter(p => p.id !== purchase.product.id);
        try {
          const catalog = await bget<{ products: Product[] }>(server, '/darknet');
          if (current === generation) products = catalog.products;
        } catch { /* The purchase succeeded; a catalog refresh must not offer a retry of it. */ }
      }
    } catch (e) { if (current === generation) error = errorText(e); }
    finally { busy = false; }
  }
</script>

<section class="darknet">
  <div class="intro"><span class="eyebrow">VELORA SMP / DARKNET</span><h2>Rare finds. Yours to keep.</h2><p>Spend in-game dollars on the server's exclusive catalog. Items wait in your vault, even when you're offline.</p><span class="balance">Your balance <strong>{money(balance)}</strong></span></div>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if loading}<p>Opening the catalog…</p>
  {:else if !products.length}<div class="empty"><h3>The catalog is quiet.</h3><p>New stock will appear when the server opens its next collection.</p></div>
  {:else}
  <div class="toolbar"><label>Find an item<input type="search" bind:value={search} placeholder="Search the catalog…" /></label><label>Sort by<select bind:value={sort}><option value="featured">Featured</option><option value="price">Price: low to high</option><option value="name">Item name</option></select></label><span>{shown.length} listings</span></div>
  {#if !shown.length}<p>No matching listings. Try another search.</p>{/if}
  <div class="products">{#each shown as product (product.id)}
    <article><span class="quantity">{product.amount}×</span><div class="art">{#if item}{@render item(product.item_id, product.amount)}{:else}<span aria-hidden="true">◆</span>{/if}</div><span class="category">{product.vault_number ? 'Permanent unlock' : 'Vault delivery'}</span><h3>{product.item_name}</h3><p class="id">{product.item_id}</p><div class="purchase"><strong>{money(product.price_cents / 100)}</strong><button disabled={busy} onclick={() => { pending = { product, operation: crypto.randomUUID() }; error = ''; }}>Review purchase</button></div>{#if balance != null && balance * 100 < product.price_cents}<small class="shortfall">You need {money(product.price_cents / 100 - balance)} more</small>{/if}</article>
  {/each}</div>{/if}
  {#if pending}<div class="confirmation" bind:this={review} tabindex="-1" role="region" aria-label="Review Darknet purchase"><div><h3>{pending.product.amount}× {pending.product.item_name}</h3><p>{money(pending.product.price_cents / 100)} in-game dollars. {pending.product.vault_number ? 'Unlocks a permanent 7 × 9 vault.' : 'Delivery to your vault.'}</p></div><div class="actions"><button disabled={busy} onclick={() => { pending = null; error = ''; }}>Cancel</button><button class="confirm" disabled={busy} onclick={buy}>{busy ? 'Purchasing…' : 'Confirm purchase'}</button></div></div>{/if}
</section>

<style>
  .darknet { color: var(--text, #f0edf8); display: grid; gap: 1.2rem; }
  .intro { padding: 2rem; background: radial-gradient(ellipse at 95% 0%, #7c4ec533, transparent 65%), var(--surface, #171421); border: 1px solid #a386cd33; border-radius: 18px; }
  .eyebrow { font-size: .7rem; letter-spacing: .18em; color: #be9eea; } h2 { font-size: clamp(1.6rem, 3vw, 2.4rem); margin: .7rem 0; letter-spacing: -.04em; }
  p { color: var(--muted, #a99bbf); line-height: 1.6; margin: .5rem 0; } .balance { display: inline-flex; gap: 1rem; margin-top: 1rem; font-size: .85rem; } .balance strong { color: #d6bfff; }
  .products { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 240px), 1fr)); gap: 1rem; }
  article { position: relative; padding: 1.4rem; border-radius: 16px; background: linear-gradient(155deg, #a386cd12, transparent 65%), var(--surface, #171421); border: 1px solid #a386cd33; transition: border-color .15s; }
  article:hover { border-color: #be9eea88; }
  .quantity { position: absolute; right: 1rem; top: 1rem; color: #d6bfff; font-size: .8rem; background: #a386cd18; padding: .3rem .5rem; border-radius: 6px; }
  .art { min-height: 100px; display: grid; place-items: center; margin-bottom: 1rem; background: radial-gradient(ellipse, #a386cd22, transparent 70%); font-size: 3rem; color: #d6bfff; }
  .category { color: #be9eea; text-transform: uppercase; font-size: .65rem; letter-spacing: .12em; }
  .toolbar { display: flex; align-items: end; flex-wrap: wrap; gap: 1rem; } .toolbar label { display: grid; gap: .4rem; font-size: .8rem; } .toolbar label:first-child { flex: 1; min-width: 180px; } .toolbar input, .toolbar select { font: inherit; padding: .7rem; color: inherit; background: var(--surface, #171421); border: 1px solid #a386cd55; border-radius: 9px; } .toolbar span, .shortfall { color: var(--muted, #a99bbf); font-size: .75rem; } .shortfall { display: block; margin-top: .8rem; }
  h3 { margin: .3rem 0; font-size: 1.05rem; } .id { font-size: .72rem; overflow-wrap: anywhere; } .purchase { display: flex; align-items: center; justify-content: space-between; gap: .7rem; margin-top: 1.5rem; }
  button { font: inherit; font-size: .8rem; padding: .7rem .9rem; border-radius: 9px; border: 1px solid #a386cd55; color: inherit; background: #a386cd18; cursor: pointer; } button:disabled { opacity: .5; cursor: wait; }
  .confirmation { padding: 1.5rem; border: 1px solid #be9eea; border-radius: 16px; display: flex; flex-wrap: wrap; justify-content: space-between; gap: 1rem; background: var(--surface, #171421); } .actions { display: flex; align-items: center; gap: .7rem; } .confirm { background: #7954ac; color: #fff; }
  .empty { padding: 2rem; border: 1px dashed #a386cd55; border-radius: 16px; } .error { color: #f2a9b3; }
</style>
