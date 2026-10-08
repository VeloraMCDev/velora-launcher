<script lang="ts">
  import { Image, Box, Search } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { get } from '../lib/api';
  import { toastError } from '../lib/toast.svelte';

  /** Pick a texture or model that is already in the server assets; `onpick` gets its `namespace:path`. */
  let { kind, onpick, label }: { kind: 'texture' | 'model'; onpick: (ref: string) => void; label?: string } = $props();

  type Hit = { ref: string; preview: string | null };
  let open = $state(false);
  let q = $state('');
  let hits = $state<Hit[]>([]);
  let total = $state(0);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  async function run(more = false) {
    const mine = ++seq;
    busy = true;
    try {
      const r = await get<{ total: number; items: Hit[] }>(`/api/admin/resource-assets-browse?kind=${kind}&q=${encodeURIComponent(q)}&offset=${more ? hits.length : 0}&limit=96`);
      if (mine !== seq) return;
      hits = more ? [...hits, ...r.items] : r.items;
      total = r.total;
    } catch (e) { toastError(e); } finally { if (mine === seq) busy = false; }
  }
  function later() { clearTimeout(timer); timer = setTimeout(() => run(), 200); }
  function show() { open = true; run(); }
</script>

<button type="button" class="ghost" onclick={show}>
  {#if kind === 'texture'}<Image size={14} />{:else}<Box size={14} />{/if} {label ?? (kind === 'texture' ? 'Choose texture' : 'Choose model')}
</button>

<Modal bind:open title={kind === 'texture' ? 'Choose a texture' : 'Choose a model'} width={760}>
  <label class="search"><Search size={15} /><input bind:value={q} oninput={later} placeholder="Search {total.toLocaleString()} {kind}s…" /></label>
  {#if hits.length === 0 && !busy}
    <p class="empty">Nothing here yet. Import an Oraxen / ItemsAdder pack or upload {kind}s under <b>Server assets</b> first.</p>
  {/if}
  <div class="grid">
    {#each hits as h (h.ref)}
      <button type="button" class="cell" title={h.ref} onclick={() => { onpick(h.ref); open = false; }}>
        <span class="slot">{#if h.preview}<img src={h.preview} alt="" />{:else}<Box size={22} />{/if}</span>
        <span class="name">{h.ref.split(':')[1].split('/').pop()}</span>
        <em>{h.ref.split(':')[0]}</em>
      </button>
    {/each}
  </div>
  {#if hits.length < total}<div class="more"><button type="button" class="ghost" disabled={busy} onclick={() => run(true)}>Show more ({(total - hits.length).toLocaleString()} left)</button></div>{/if}
</Modal>

<style>
  .search { display: flex; align-items: center; gap: 8px; padding: 0 10px; border: 1px solid var(--line); border-radius: 10px; background: var(--bg-2); margin-bottom: 12px; }
  .search input { border: 0; background: none; flex: 1; padding: 9px 0; outline: none; color: inherit; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(92px, 1fr)); gap: 8px; max-height: 50vh; overflow: auto; grid-auto-rows: min-content; align-content: start; padding: 2px; }
  .cell { display: grid; justify-items: center; gap: 4px; padding: 10px 4px 6px; border-radius: 10px; border: 1px solid transparent; background: #ffffff08; cursor: pointer; color: inherit; transition: background 0.12s, border-color 0.12s, transform 0.12s; }
  .cell:hover, .cell:focus-visible { background: color-mix(in srgb, var(--accent, #7c5cff) 18%, transparent); border-color: color-mix(in srgb, var(--accent, #7c5cff) 60%, transparent); transform: translateY(-1px); }
  .slot { width: 52px; height: 52px; display: grid; place-items: center; border-radius: 8px; background: #8b8b8b22; border: 1px solid #ffffff14; }
  .slot img { width: 40px; height: 40px; object-fit: contain; image-rendering: pixelated; }
  .name { font-size: 0.72rem; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  em { font-size: 0.6rem; opacity: 0.45; font-style: normal; margin-top: -3px; }
  .more { display: grid; place-items: center; margin-top: 10px; }
  .empty { color: var(--muted); font-size: 0.85rem; }
</style>
