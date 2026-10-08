<script lang="ts">
  import { onMount } from 'svelte';
  import { Plus, Trash2, Save, Wand2, Terminal } from '@lucide/svelte';
  import ItemEditor, { type ItemSpec } from '../components/ItemEditor.svelte';
  import { get, put, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  let { focusId = null }: { focusId?: string | null } = $props();
  type Item = { id: string; title: string; spec: ItemSpec };
  let items = $state<Item[]>([]);
  let current = $state<string | null>(null);
  let draft = $state<Item | null>(null);
  let isNew = $state(false);

  async function load() { try { items = (await get<{ items: Item[] }>('/api/admin/custom-items')).items; } catch (e) { toastError(e); } }
  onMount(async () => { await load(); if (focusId) { const it = items.find((i) => i.id === focusId); if (it) pick(it); } });

  function pick(i: Item) { current = i.id; draft = structuredClone($state.snapshot(i)) as Item; isNew = false; }
  function start() {
    current = null; isNew = true;
    draft = { id: '', title: '', spec: { item: 'minecraft:netherite_sword', amount: 1, name: '&b&lNew Item', lore: ['&7Describe it here'], enchants: { sharpness: 10 }, unbreakable: true, glow: true } };
  }
  const slug = (t: string) => t.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '').slice(0, 32);

  async function save() {
    if (!draft) return;
    const id = isNew ? slug(draft.id || draft.title) : draft.id;
    try {
      await put(`/api/admin/custom-items/${id}`, { title: draft.title, spec: draft.spec });
      toast('Saved — servers have it within seconds'); await load();
      const saved = items.find((i) => i.id === id); if (saved) pick(saved);
    } catch (e) { toastError(e); }
  }
  async function remove() {
    if (!draft || !confirm(`Delete "${draft.title}"?`)) return;
    try { await del(`/api/admin/custom-items/${draft.id}`); draft = null; current = null; await load(); } catch (e) { toastError(e); }
  }
</script>

<div class="page wide">
  <div class="bar"><button onclick={start}><Plus size={15} /> New item</button></div>

  <div class="layout">
    <aside class="card list">
      {#each items as i (i.id)}
        <button class="item" class:on={current === i.id} onclick={() => pick(i)}>
          <Wand2 size={15} /><span>{i.title}<small>{i.id}</small></span>
        </button>
      {:else}
        <p class="muted">Nothing yet. Make your first item!</p>
      {/each}
    </aside>

    {#if draft}
      <section class="card">
        <div class="row">
          <label class="grow">Title <small>for this list</small><input bind:value={draft.title} placeholder="Stormbreaker" /></label>
          {#if isNew}<label>Id <small>used in commands</small><input bind:value={draft.id} placeholder={slug(draft.title) || 'stormbreaker'} /></label>{/if}
        </div>
        <ItemEditor bind:spec={draft.spec} />
        <details class="adv">
          <summary>Edit asset references by hand</summary>
          <div class="row">
            <label class="grow">Texture <small>namespace:path, e.g. scopenet:item/blade</small><input bind:value={draft.spec.texture} placeholder="scopenet:item/blade" /></label>
            <label class="grow">Model <small>optional, replaces generated flat texture model</small><input bind:value={draft.spec.model} placeholder="scopenet:item/blade_3d" /></label>
          </div>
        </details>
        {#if !isNew}<p class="cmd"><Terminal size={14} /> <code>/customitem give &lt;player&gt; {draft.id}</code></p>{/if}
        <div class="row">
          <button onclick={save} disabled={!draft.title.trim()}><Save size={14} /> Save</button>
          {#if !isNew}<button class="danger" onclick={remove}><Trash2 size={14} /> Delete</button>{/if}
        </div>
      </section>
    {:else}
      <section class="card empty"><Wand2 size={28} /><p>Pick an item or create a new one.</p></section>
    {/if}
  </div>
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  .bar { display: flex; justify-content: flex-end; }
  .bar button, .row button { display: inline-flex; align-items: center; gap: 6px; }
  code { font: 0.8rem ui-monospace, monospace; background: var(--bg-2); padding: 1px 5px; border-radius: 5px; }
  .layout { display: grid; grid-template-columns: minmax(0, 1fr); gap: 14px; align-items: start; }
  .list { display: flex; flex-direction: row; flex-wrap: wrap; gap: 6px; padding: 10px; }
  .item { display: flex; align-items: center; gap: 9px; text-align: left; padding: 8px 14px 8px 10px; border-radius: 10px; border: 1px solid var(--line); background: var(--surface-2); }
  .item.on { background: color-mix(in srgb, var(--accent) 14%, transparent); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
  .item small { display: block; color: var(--muted); font-weight: 400; font-size: 0.72rem; }
  section.card { display: flex; flex-direction: column; gap: 14px; }
  .row { display: flex; gap: 10px; align-items: flex-end; flex-wrap: wrap; }
  .grow { flex: 1; min-width: 12rem; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8rem; color: var(--muted); }
  label small { font-weight: 400; }
  .adv summary { cursor: pointer; color: var(--muted); font-size: 0.8rem; margin-bottom: 8px; }
  .cmd { display: flex; align-items: center; gap: 6px; margin: 0; color: var(--muted); font-size: 0.82rem; }
  .empty { align-items: center; justify-content: center; min-height: 14rem; color: var(--muted); }
  .muted { color: var(--muted); font-size: 0.85rem; }
  
</style>
