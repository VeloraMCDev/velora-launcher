<script lang="ts">
  import { onMount } from 'svelte';
  import { Plus, Search, Copy, Pencil, Trash2, Wand2, Boxes, Hammer, Package, Cuboid } from '@lucide/svelte';
  import ContentWizard from '../components/ContentWizard.svelte';
  import ContentEditor from '../components/ContentEditor.svelte';
  import ResourceAssets from '../components/ResourceAssets.svelte';
  import CustomItems from './CustomItems.svelte';
  import { get, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import { KINDS, kindMeta, SOURCE_LABEL, frameworkMeta, type KindId, type LibraryItem } from '../lib/content';

  type Tab = 'library' | 'advanced' | 'pack';
  let tab = $state<Tab>('library');
  let wizard = $state(false);
  let library = $state<LibraryItem[]>([]);
  let loaded = $state(false);
  let q = $state('');
  let kind = $state<'all' | KindId>('all');
  let advanced = $state<'items' | 'world'>('items');
  let focusItem = $state<string | null>(null);
  let doc = $state<{ id: string; kind: string; title: string; source?: string; spec: Record<string, any> } | null>(null);
  let isNew = $state(false);

  async function load() {
    try { library = (await get<{ items: LibraryItem[] }>('/api/admin/content')).items; } catch (e) { toastError(e); } finally { loaded = true; }
  }
  onMount(load);

  const counts = $derived(Object.fromEntries(KINDS.map((k) => [k.id, library.filter((i) => i.kind === k.id).length])) as Record<KindId, number>);
  const shown = $derived(library.filter((i) => (kind === 'all' || i.kind === kind) && (!q || (i.title + i.id).toLowerCase().includes(q.toLowerCase()))));

  async function openEntry(it: LibraryItem) {
    tab = 'advanced';
    if (it.store === 'item') { advanced = 'items'; focusItem = it.id; return; }
    advanced = 'world';
    try { doc = await get(`/api/admin/content/${it.id}`); isNew = false; } catch (e) { toastError(e); }
  }
  function startNew(k: KindId) {
    tab = 'advanced';
    if (kindMeta(k).store === 'item') { advanced = 'items'; focusItem = null; return; }
    advanced = 'world';
    doc = { id: '', kind: k, title: '', spec: { display: { item: 'minecraft:paper', scale: 1 }, lore: [] } };
    isNew = true;
  }
  async function remove(it: LibraryItem) {
    if (!confirm(`Delete "${it.title}"?`)) return;
    try { await del(it.store === 'item' ? `/api/admin/custom-items/${it.id}` : `/api/admin/content/${it.id}`); await load(); toast('Deleted'); } catch (e) { toastError(e); }
  }
  async function copy(id: string) { try { await navigator.clipboard.writeText(`/customitem give @p ${id}`); toast('Command copied'); } catch { toast(`/customitem give @p ${id}`, 'info'); } }
  function wizardOpen(id: string, store: 'item' | 'content') {
    const it = library.find((i) => i.id === id) ?? ({ id, store, kind: 'item', title: id, source: '', name: null, item: null, preview: null, has_look: true } as LibraryItem);
    openEntry(it);
  }
  const worldKinds = KINDS.filter((k) => k.store === 'content');
</script>

<div class="page wide studio">
  <header class="hero">
    <div>
      <h1>Content Studio</h1>
      <p>Bring in custom items, blocks, furniture, NPCs, vehicles, crops and mobs from ItemsAdder, Nexo, Oraxen, CraftEngine, ModelEngine and MythicMobs — or make them by hand.</p>
    </div>
    <button class="cta" onclick={() => (wizard = true)}><Plus size={17} /> Add content</button>
  </header>

  <div class="seg tabs" role="tablist">
    <button role="tab" aria-selected={tab === 'library'} class:on={tab === 'library'} onclick={() => (tab = 'library')}><Boxes size={14} /> Library <span class="n">{library.length}</span></button>
    <button role="tab" aria-selected={tab === 'advanced'} class:on={tab === 'advanced'} onclick={() => (tab = 'advanced')}><Hammer size={14} /> Advanced editor</button>
    <button role="tab" aria-selected={tab === 'pack'} class:on={tab === 'pack'} onclick={() => (tab = 'pack')}><Package size={14} /> Resource pack</button>
  </div>

  {#if tab === 'library'}
    {#if loaded && library.length === 0}
      <section class="empty">
        <div class="orb"><Wand2 size={34} /></div>
        <h2>Nothing here yet</h2>
        <p>Drop in the files from your old setup and we'll sort them out — or build your first item by hand.</p>
        <div class="row"><button class="cta" onclick={() => (wizard = true)}><Plus size={16} /> Import from a plugin</button><button class="ghost" onclick={() => startNew('weapon')}>Create by hand</button></div>
      </section>
    {:else}
      <div class="filters">
        <label class="search"><Search size={15} /><input bind:value={q} placeholder="Search {library.length} things…" /></label>
        <div class="seg">
          <button class:on={kind === 'all'} onclick={() => (kind = 'all')}>All {library.length}</button>
          {#each KINDS.filter((k) => counts[k.id] > 0) as k}<button class:on={kind === k.id} onclick={() => (kind = k.id)}>{k.plural} {counts[k.id]}</button>{/each}
        </div>
      </div>
      <div class="grid">
        {#each shown as it (it.id)}
          {@const m = kindMeta(it.kind)}
          {@const Icon = m.icon}
          <article class="card" style:--c={m.color}>
            <button class="body" onclick={() => openEntry(it)} aria-label="Edit {it.title}">
              <span class="slot">{#if it.preview}<img src={it.preview} alt="" />{:else}<Cuboid size={24} />{/if}</span>
              <span class="meta">
                <b>{it.title}</b>
                <span class="tags"><span class="kind"><Icon size={12} /> {m.label}</span><span class="src" style:--s={frameworkMeta(it.source)?.color ?? '#8b8f9a'}>{SOURCE_LABEL[it.source] ?? it.source}</span></span>
                <code>{it.id}</code>
              </span>
            </button>
            <div class="acts">
              <button title="Copy give command" aria-label="Copy give command" onclick={() => copy(it.id)}><Copy size={14} /></button>
              <button title="Edit" aria-label="Edit" onclick={() => openEntry(it)}><Pencil size={14} /></button>
              <button class="del" title="Delete" aria-label="Delete" onclick={() => remove(it)}><Trash2 size={14} /></button>
            </div>
          </article>
        {/each}
        {#if shown.length === 0}<p class="none">Nothing matches.</p>{/if}
      </div>
    {/if}

  {:else if tab === 'advanced'}
    <div class="seg sub" role="tablist">
      <button role="tab" aria-selected={advanced === 'items'} class:on={advanced === 'items'} onclick={() => (advanced = 'items')}>Items · weapons, tools, armor, food</button>
      <button role="tab" aria-selected={advanced === 'world'} class:on={advanced === 'world'} onclick={() => (advanced = 'world')}>World · blocks, chests, decorations, NPCs, vehicles, crops, mobs</button>
    </div>
    {#if advanced === 'items'}
      {#key focusItem}<CustomItems focusId={focusItem} />{/key}
    {:else}
      <div class="world">
        <aside class="list">
          <div class="mk">
            <small>Create a new…</small>
            <div class="mkgrid">
              {#each worldKinds as k}{@const I = k.icon}<button style:--c={k.color} onclick={() => startNew(k.id)}><I size={15} /> {k.label}</button>{/each}
            </div>
          </div>
          {#each library.filter((i) => i.store === 'content') as it (it.id)}
            <button class="li" class:on={doc?.id === it.id && !isNew} onclick={() => openEntry(it)}>{#if it.preview}<img src={it.preview} alt="" />{/if}<span>{it.title}<small>{kindMeta(it.kind).label} · {it.id}</small></span></button>
          {/each}
        </aside>
        <div class="edit">
          {#if doc}
            {#key doc.id + (isNew ? 'new' : '')}
              <ContentEditor bind:doc {isNew} onsaved={async (id) => { await load(); const full = await get<typeof doc>(`/api/admin/content/${id}`); doc = full; isNew = false; }} ondeleted={async () => { doc = null; await load(); }} />
            {/key}
          {:else}
            <div class="blank"><Wand2 size={28} /><p>Pick something on the left, or create a new block, chest, decoration, NPC, vehicle, crop or mob.</p></div>
          {/if}
        </div>
      </div>
    {/if}

  {:else}
    <ResourceAssets onchange={load} />
  {/if}
</div>

<ContentWizard bind:open={wizard} onchange={load} onmanual={(k) => startNew(k === 'auto' ? 'weapon' : k)} onopen={wizardOpen} />

<style>
  .studio { display: grid; gap: 16px; }
  .hero { display: flex; justify-content: space-between; align-items: flex-end; gap: 20px; flex-wrap: wrap; padding: 22px 24px; border-radius: 18px; background: radial-gradient(120% 140% at 0% 0%, color-mix(in srgb, var(--accent) 24%, transparent), transparent 60%), var(--surface); border: 1px solid var(--line); }
  .hero p { color: var(--text-2); max-width: 44rem; margin-top: 4px; }
  .cta { display: inline-flex; align-items: center; gap: 9px; padding: 12px 22px; border-radius: 13px; border: 0; background: var(--accent); color: #fff; font-weight: 700; font-size: 0.95rem; cursor: pointer; box-shadow: 0 10px 28px -10px var(--accent); transition: transform 0.12s, filter 0.12s; }
  .cta:hover { transform: translateY(-2px); filter: brightness(1.1); }
  .ghost { padding: 11px 18px; border-radius: 12px; border: 1px solid var(--line-strong); background: transparent; color: var(--text-2); cursor: pointer; }
  .ghost:hover { color: var(--text); background: var(--surface-2); }
  .tabs { justify-self: start; }
  .tabs button { display: inline-flex; align-items: center; gap: 7px; }
  .n { font-size: 0.7rem; padding: 1px 7px; border-radius: 999px; background: #ffffff1c; }
  .sub { justify-self: start; }
  .filters { display: flex; gap: 12px; flex-wrap: wrap; align-items: center; }
  .search { flex: 1 1 14rem; max-width: 22rem; display: flex; align-items: center; gap: 8px; padding: 0 12px; border-radius: 11px; background: var(--surface); border: 1px solid var(--line); }
  .search input { border: 0; background: none; padding: 10px 0; outline: none; flex: 1; color: inherit; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: 12px; }
  .card { --c: var(--accent); position: relative; display: flex; align-items: stretch; border-radius: 16px; background: var(--surface); border: 1px solid var(--line); overflow: hidden; transition: transform 0.14s, border-color 0.14s, box-shadow 0.14s; }
  .card:hover { transform: translateY(-3px); border-color: color-mix(in srgb, var(--c) 60%, transparent); box-shadow: 0 16px 30px -18px var(--c); }
  .card .body { display: flex; align-items: center; gap: 13px; flex: 1; min-width: 0; padding: 13px; border: 0; background: none; color: inherit; text-align: left; cursor: pointer; }
  .slot { display: grid; place-items: center; width: 62px; height: 62px; flex-shrink: 0; border-radius: 8px; background: #8b8b8b; box-shadow: inset 2px 2px 0 #373737, inset -2px -2px 0 #fff8; color: #3a3a3a; }
  .slot img { width: 78%; height: 78%; object-fit: contain; image-rendering: pixelated; }
  .meta { display: grid; gap: 3px; min-width: 0; }
  .meta b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta code { color: var(--muted); font-size: 0.72rem; }
  .tags { display: flex; gap: 6px; flex-wrap: wrap; }
  .kind { display: inline-flex; align-items: center; gap: 4px; font-size: 0.72rem; color: var(--c); }
  .src { font-size: 0.66rem; padding: 1px 8px; border-radius: 999px; background: color-mix(in srgb, var(--s) 20%, transparent); color: var(--s); font-weight: 600; }
  .acts { display: flex; flex-direction: column; justify-content: center; gap: 2px; padding: 6px; opacity: 0; transition: opacity 0.12s; }
  .card:hover .acts, .card:focus-within .acts { opacity: 1; }
  .acts button { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 8px; border: 0; background: transparent; color: var(--text-2); cursor: pointer; }
  .acts button:hover { background: var(--surface-3); color: var(--text); }
  .acts .del:hover { color: var(--bad); }
  .none { color: var(--muted); grid-column: 1 / -1; }
  .empty { display: grid; justify-items: center; text-align: center; gap: 10px; padding: 60px 20px; border-radius: 20px; border: 2px dashed var(--line-strong); background: var(--surface); }
  .empty p { color: var(--text-2); max-width: 30rem; }
  .orb { display: grid; place-items: center; width: 76px; height: 76px; border-radius: 24px; background: var(--accent-soft); color: var(--accent-2); }
  .row { display: flex; gap: 10px; flex-wrap: wrap; justify-content: center; margin-top: 6px; }
  .world { display: grid; grid-template-columns: 250px minmax(0, 1fr); gap: 16px; align-items: start; }
  .list { display: grid; gap: 6px; padding: 10px; border-radius: 14px; background: var(--surface); border: 1px solid var(--line); position: sticky; top: 16px; }
  .mk small { color: var(--muted); }
  .mkgrid { display: grid; grid-template-columns: 1fr 1fr; gap: 5px; margin: 6px 0 10px; }
  .mkgrid button { --c: var(--accent); display: inline-flex; align-items: center; gap: 6px; padding: 7px 9px; border-radius: 9px; border: 1px solid var(--line); background: var(--surface-2); color: var(--text-2); font-size: 0.78rem; cursor: pointer; }
  .mkgrid button:hover { border-color: var(--c); color: var(--text); }
  .li { display: flex; align-items: center; gap: 9px; text-align: left; padding: 8px; border-radius: 10px; border: 1px solid transparent; background: transparent; color: inherit; cursor: pointer; }
  .li img { width: 28px; height: 28px; image-rendering: pixelated; }
  .li small { display: block; color: var(--muted); font-size: 0.7rem; }
  .li.on { background: var(--accent-soft); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
  .blank { display: grid; place-items: center; gap: 8px; min-height: 16rem; color: var(--muted); border: 1px dashed var(--line-strong); border-radius: 16px; text-align: center; padding: 20px; }
  @media (max-width: 900px) { .world { grid-template-columns: 1fr; } .list { position: static; } }
</style>
