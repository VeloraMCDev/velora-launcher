<script lang="ts">
  import { Library, Search } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { get } from '../lib/api';
  import { iconifyValue } from '../lib/achievementIcons';
  import { toastError } from '../lib/toast.svelte';

  /** Called with a value such as `iconify:game-icons:broadsword@ffd700`. */
  let { onpick, label = 'Browse icon library' }: { onpick: (value: string) => void; label?: string } = $props();

  type Pack = { id: string; label: string; palette: boolean; license: string; total: number };
  type Hit = { pack: string; name: string };
  let open = $state(false);
  let packs = $state<Pack[]>([]);
  let total = $state(0);
  let q = $state('');
  let pack = $state('');
  let tint = $state('#ffffff');
  let hits = $state<Hit[]>([]);
  let found = $state(0);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  const PAGE = 120;
  const palettePacks = $derived(new Set(packs.filter((p) => p.palette).map((p) => p.id)));
  const url = (h: Hit) => `/api/v1/icons/${h.pack}/${h.name}.svg${palettePacks.has(h.pack) ? '' : '?color=' + tint.replace('#', '')}`;

  async function run(more = false) {
    const mine = ++seq;
    busy = true;
    try {
      const r = await get<{ total: number; icons: Hit[] }>(`/api/admin/icons/search?q=${encodeURIComponent(q)}&pack=${encodeURIComponent(pack)}&offset=${more ? hits.length : 0}&limit=${PAGE}`);
      if (mine !== seq) return;
      hits = more ? [...hits, ...r.icons] : r.icons;
      found = r.total;
    } catch (e) { toastError(e); } finally { if (mine === seq) busy = false; }
  }
  function later() { clearTimeout(timer); timer = setTimeout(() => run(), 220); }
  async function show() {
    open = true;
    if (!packs.length) {
      try { const r = await get<{ packs: Pack[]; total: number }>('/api/admin/icons/packs'); packs = r.packs; total = r.total; } catch (e) { toastError(e); }
    }
    if (!hits.length) run();
  }
  function choose(h: Hit) {
    onpick(iconifyValue(h.pack, h.name, palettePacks.has(h.pack) ? null : tint));
    open = false;
  }
</script>

<button type="button" class="ghost" onclick={show}><Library size={15} /> {label}</button>

<Modal bind:open title="Icon library" width={820}>
  {#if packs.length === 0 && !busy}
    <p class="muted">The icon library isn't installed on this panel. Build it with <code>cd panel/icons &amp;&amp; npm ci &amp;&amp; node build.mjs</code> (the Docker image does this for you) and set <code>SCOPENET_ICONS_DIR</code>.</p>
  {/if}
  <div class="bar">
    <label class="search"><Search size={15} /><input bind:value={q} oninput={later} placeholder={total ? `Search ${total.toLocaleString()} icons — sword, heart, crown…` : 'Search icons'} /></label>
    <select bind:value={pack} onchange={() => run()} aria-label="Icon pack">
      <option value="">All packs</option>
      {#each packs as p}<option value={p.id}>{p.label} ({p.total.toLocaleString()})</option>{/each}
    </select>
    <label class="tint" title="Colour for single-colour icons"><input type="color" bind:value={tint} /></label>
  </div>
  <div class="grid">
    {#each hits as h (h.pack + h.name)}
      <button type="button" class="cell" title="{h.pack}:{h.name}" onclick={() => choose(h)}>
        <img src={url(h)} alt={h.name} loading="lazy" />
        <span>{h.name}</span><em>{h.pack}</em>
      </button>
    {/each}
  </div>
  {#if hits.length === 0 && !busy && packs.length}<p class="muted">Nothing matches “{q}”.</p>{/if}
  {#if hits.length < found}
    <div class="more"><button type="button" class="ghost" disabled={busy} onclick={() => run(true)}>Show more ({(found - hits.length).toLocaleString()} left)</button></div>
  {/if}
  <p class="credit muted">Icons from open-source Iconify sets — check each pack's license (shown in the pack list) before commercial use.</p>
</Modal>

<style>
  .bar { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
  .search { flex: 1 1 14rem; display: flex; align-items: center; gap: 8px; padding: 0 10px; border: 1px solid var(--line); border-radius: 10px; background: var(--bg-2); }
  .search input { border: 0; background: none; flex: 1; padding: 9px 0; outline: none; color: inherit; }
  select { width: auto; max-width: 15rem; }
  .tint input { width: 42px; height: 38px; padding: 2px; border-radius: 8px; border: 1px solid var(--line); background: var(--bg-2); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(86px, 1fr)); gap: 6px; max-height: 52vh; overflow: auto; grid-auto-rows: min-content; align-content: start; padding: 2px; }
  .cell { display: grid; justify-items: center; gap: 4px; padding: 10px 4px 6px; border-radius: 10px; border: 1px solid transparent; background: #ffffff08; cursor: pointer; color: inherit; transition: background 0.12s, border-color 0.12s, transform 0.12s; }
  .cell:hover, .cell:focus-visible { background: color-mix(in srgb, var(--accent, #7c5cff) 18%, transparent); border-color: color-mix(in srgb, var(--accent, #7c5cff) 60%, transparent); transform: translateY(-1px); }
  .cell img { width: 34px; height: 34px; object-fit: contain; }
  .cell span { font-size: 0.64rem; opacity: 0.65; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cell em { font-size: 0.58rem; opacity: 0.4; font-style: normal; margin-top: -3px; }
  .more { display: grid; place-items: center; margin-top: 10px; }
  .credit { font-size: 0.72rem; margin: 10px 0 0; }
</style>
