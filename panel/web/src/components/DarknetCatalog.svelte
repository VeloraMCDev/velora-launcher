<script lang="ts">
  import { onMount } from 'svelte';
  import { Plus, Copy, Trash2, Save } from '@lucide/svelte';
  import { get, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import BaseItemPicker from './BaseItemPicker.svelte';
  import McItem from './McItem.svelte';
  type Product = { id: string; item_id: string; item_name: string; amount: number; price_cents: number; enabled: boolean };
  type Draft = Product & { price: number };
  let rows = $state<Draft[]>([]);
  let loaded = $state(false);
  let busy = $state(false);
  let saved = $state('');
  let error = $state('');
  const payload = () => rows.map(({ price, ...p }) => ({ ...p, price_cents: Math.round(price * 100) }));
  const dirty = $derived(loaded && JSON.stringify(payload()) !== saved);
  async function load() {
    busy = true;
    try {
      const products = await get<Product[]>('/api/admin/economy/darknet');
      rows = products.map(p => ({ ...p, price: p.price_cents / 100 }));
      saved = JSON.stringify(products); loaded = true; error = '';
    } catch (e) { error = 'Could not load the catalog. Retry before editing.'; toastError(e); }
    finally { busy = false; }
  }
  onMount(load);
  function add(source?: Draft) {
    let id = source ? `${source.id.slice(0, 48)}-copy` : 'listing';
    let n = 2;
    const base = id;
    while (rows.some(p => p.id === id)) id = `${base}-${n++}`;
    rows.push(source ? { ...source, id, enabled: false } : { id, item_id: 'minecraft:diamond', item_name: 'Diamond', amount: 1, price_cents: 10000, price: 100, enabled: false });
  }
  async function save() {
    const products = payload();
    if (products.some(p => !/^[a-zA-Z0-9_-]{1,64}$/.test(p.id) || p.id.startsWith('velora-vault-')) || new Set(products.map(p => p.id)).size !== products.length) { error = 'Give each listing a unique ID (letters, numbers, dashes or underscores). velora-vault- is reserved.'; return; }
    if (products.some(p => !/^[a-z0-9_.-]+:[a-z0-9_./-]+$/.test(p.item_id) || !p.item_name.trim() || new TextEncoder().encode(p.item_name).length > 100 || !Number.isInteger(p.amount) || p.amount < 1 || p.amount > 64 || !Number.isSafeInteger(p.price_cents) || p.price_cents < 1 || p.price_cents > 100000000000)) { error = 'Check item IDs, names (up to 100 bytes), quantities (1–64) and prices ($0.01–$1,000,000,000).'; return; }
    busy = true; error = '';
    try {
      const result = await put<Product[]>('/api/admin/economy/darknet', products);
      rows = result.map(p => ({ ...p, price: p.price_cents / 100 })); saved = JSON.stringify(result);
      toast('Darknet catalog saved');
    } catch (e) { toastError(e); } finally { busy = false; }
  }
</script>

<section class="catalog">
  <div class="heading"><div><h2>Darknet catalog</h2><p>Pick an item, set its price in in-game dollars, then publish it. Items are delivered to player vaults.</p></div><button disabled={!loaded || busy || rows.length >= 200} onclick={() => add()}><Plus size={16} /> Add listing</button></div>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if !loaded}<button disabled={busy} onclick={load}>{busy ? 'Loading catalog…' : 'Retry loading'}</button>
  {:else}
    <fieldset disabled={busy}>
    {#each rows as row, i (row)}
      <article>
        <div class="preview"><McItem id={row.item_id} size={56} /><strong>{row.item_name || 'Untitled listing'}</strong><span>{row.enabled ? 'Published' : 'Hidden'}</span></div>
        <div class="fields">
          <BaseItemPicker bind:value={row.item_id} label="Choose item" />
          <label>Item ID (including modded items)<input bind:value={row.item_id} maxlength="128" placeholder="minecraft:diamond" /></label>
          <label>Display name<input bind:value={row.item_name} maxlength="100" /></label>
          <label>Listing ID<input bind:value={row.id} maxlength="64" /></label>
          <label>Quantity<input type="number" min="1" max="64" step="1" bind:value={row.amount} /></label>
          <label>Price ($ in-game)<input type="number" min="0.01" max="1000000000" step="0.01" bind:value={row.price} /></label>
        </div>
        <div class="actions"><label class="toggle"><input type="checkbox" bind:checked={row.enabled} /> Published</label><button onclick={() => add(row)} disabled={rows.length >= 200}><Copy size={14} /> Duplicate</button><button class="danger" onclick={() => rows.splice(i, 1)}><Trash2 size={14} /> Remove</button></div>
      </article>
    {/each}
    {#if !rows.length}<p>No listings yet. Add your first item to start the catalog.</p>{/if}
    </fieldset>
    <div class="footer"><span>{rows.length}/200 listings · {dirty ? 'Unsaved changes' : 'Saved'}</span><button class="primary" disabled={busy || !dirty} onclick={save}><Save size={16} /> {busy ? 'Saving…' : 'Save catalog'}</button></div>
    <p class="note">The catalog is shared across instances. Save publishes all edits together. Keep existing listing IDs stable. Verify modded items in staging before publishing.</p>
  {/if}
</section>

<style>
  .catalog { margin: 1.5rem 0; padding: 1.5rem; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  h2 { margin: 0; } p, .footer span, .note { color: var(--muted); line-height: 1.6; }
  .heading, .actions, .footer { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: .75rem; }
  fieldset { border: 0; padding: 0; margin: 0; min-width: 0; }
  article { padding: 1rem; margin: 1rem 0; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-2); }
  .preview { display: flex; align-items: center; gap: .8rem; margin-bottom: 1rem; } .preview span { margin-left: auto; font-size: .75rem; color: var(--accent); }
  .fields { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: .8rem; }
  label { display: grid; gap: .4rem; font-size: .8rem; } input { width: 100%; } .toggle { display: flex; align-items: center; margin-right: auto; } .toggle input { width: auto; }
  .actions { margin-top: 1rem; } .error { color: var(--danger, #ef8999); } .note { font-size: .8rem; }
</style>
