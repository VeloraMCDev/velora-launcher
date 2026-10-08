<script lang="ts">
  import { ImageUp, Trash2, Save, Plus } from '@lucide/svelte';
  import AssetPicker from './AssetPicker.svelte';
  import BaseItemPicker from './BaseItemPicker.svelte';
  import McTextInput from './McTextInput.svelte';
  import McLoreEditor from './McLoreEditor.svelte';
  import ContentFields from './ContentFields.svelte';
  import ModelViewer from './ModelViewer.svelte';
  import Slide from './Slide.svelte';
  import { get, post, put, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import { kindMeta, type ViewBundle } from '../lib/content';

  type Look = { item?: string; texture?: string; model?: string; custom_model_data?: number; scale?: number };
  type Doc = { id: string; kind: string; title: string; source?: string; spec: Record<string, any> };
  let { doc = $bindable(), isNew = false, onsaved, ondeleted }: { doc: Doc; isNew?: boolean; onsaved?: (id: string) => void; ondeleted?: () => void } = $props();

  const meta = $derived(kindMeta(doc.kind));
  const display = $derived((doc.spec.display ?? {}) as Look);
  let view = $state<ViewBundle | null>(null);
  let busy = $state(false);
  let drop = $state(false);
  let fileInput: HTMLInputElement | undefined = $state();
  const slug = (t: string) => t.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '').slice(0, 32);

  const setLook = (p: Partial<Look>) => (doc.spec = { ...doc.spec, display: { item: 'minecraft:paper', ...display, ...p } });
  function chooseLook(field: 'texture' | 'model', ref: string) {
    setLook(field === 'texture' ? { texture: ref, model: undefined } : { model: ref, texture: undefined });
  }
  const stages = $derived((doc.spec.stages ?? []) as Look[]);
  const setStages = (l: Look[]) => (doc.spec = { ...doc.spec, stages: l });

  // A 3D bundle for what is picked right now.
  $effect(() => {
    const t = display.texture ?? stages[0]?.texture ?? '', m = display.model ?? stages[0]?.model ?? '';
    if (!t && !m) { view = null; return; }
    let live = true;
    get<{ view: ViewBundle | null }>(`/api/admin/content/view?texture=${encodeURIComponent(t)}&model=${encodeURIComponent(m)}`).then((r) => { if (live) view = r.view; }).catch(() => { if (live) view = null; });
    return () => { live = false; };
  });

  async function upload(file?: File, forStage?: number) {
    if (!file) return;
    busy = true;
    try {
      if (!/\.png$/i.test(file.name)) throw new Error('Textures must be PNG images');
      const name = file.name.toLowerCase().replace(/\.png$/, '').replace(/[^a-z0-9_.-]+/g, '_').replace(/^[_.-]+|[_.-]+$/g, '') || 'texture';
      const data = await new Promise<string>((resolve, reject) => { const r = new FileReader(); r.onload = () => resolve(String(r.result).split(',')[1]); r.onerror = reject; r.readAsDataURL(file); });
      await post('/api/admin/resource-assets', { path: `assets/scopenet/textures/item/${name}.png`, data });
      if (forStage === undefined) chooseLook('texture', `scopenet:item/${name}`);
      else setStages(stages.map((s, i) => (i === forStage ? { ...s, texture: `scopenet:item/${name}`, model: undefined } : s)));
    } catch (e) { toastError(e); } finally { busy = false; if (fileInput) fileInput.value = ''; }
  }

  async function save() {
    busy = true;
    try {
      const id = isNew ? slug(doc.id || doc.title) : doc.id;
      const r = await put<{ id: string }>(`/api/admin/content/${id}`, { kind: doc.kind, title: doc.title, spec: doc.spec, source: doc.source ?? 'manual' });
      toast('Saved — servers have it within seconds');
      onsaved?.(r.id);
    } catch (e) { toastError(e); } finally { busy = false; }
  }
  async function remove() {
    if (!confirm(`Delete "${doc.title}"?`)) return;
    try { await del(`/api/admin/content/${doc.id}`); ondeleted?.(); } catch (e) { toastError(e); }
  }
  const addStage = () => setStages([...stages, { ...(stages[stages.length - 1] ?? {}), custom_model_data: undefined }]);
</script>

<section class="ce">
  <header>
    <span class="badge" style:--c={meta.color}><meta.icon size={18} /></span>
    <div><b>{isNew ? `New ${meta.label.toLowerCase()}` : doc.title}</b><small>{meta.blurb}</small></div>
  </header>

  <div class="grid">
    <div class="col">
      <div class="card ids">
        <label>Title <small>for your list</small><input bind:value={doc.title} maxlength="60" placeholder="Garden bench" /></label>
        {#if isNew}<label>Id <small>used in commands</small><input bind:value={doc.id} placeholder={slug(doc.title) || 'garden_bench'} /></label>{/if}
      </div>

      <div class="card look" class:drop role="group" aria-label="Appearance" ondragover={(e) => { e.preventDefault(); drop = true; }} ondragleave={() => (drop = false)}
        ondrop={(e) => { e.preventDefault(); drop = false; upload(e.dataTransfer?.files?.[0]); }}>
        <b>Appearance</b>
        <small>{display.model ? `Model ${display.model}` : display.texture ? `Texture ${display.texture}` : 'Drop a PNG, upload one, or pick a texture or 3D model from your server assets.'}</small>
        <div class="row">
          <button type="button" class="ghost" disabled={busy} onclick={() => fileInput?.click()}><ImageUp size={14} /> Upload PNG</button>
          <input bind:this={fileInput} type="file" accept="image/png" hidden onchange={(e) => upload(e.currentTarget.files?.[0])} />
          <AssetPicker kind="texture" onpick={(r) => chooseLook('texture', r)} />
          <AssetPicker kind="model" onpick={(r) => chooseLook('model', r)} />
        </div>
        <BaseItemPicker label="Shown as (the item players hold)" bind:value={() => display.item ?? 'minecraft:paper', (v) => setLook({ item: v })} />
        <Slide label="Size in the world" min={0.25} max={4} step={0.05} bind:value={() => display.scale ?? 1, (n) => setLook({ scale: n })} format={(n) => `${Math.round(n * 100)}%`} />
      </div>

      {#if doc.kind === 'crop'}
        <div class="card">
          <div class="sh"><b>Growth stages</b><small>first = just planted, last = ready to harvest</small></div>
          {#each stages as st, i (i)}
            <div class="stage">
              <span class="n">{i + 1}</span>
              <span class="what">{st.model ?? st.texture ?? 'pick a look'}</span>
              <button class="ghost sm" disabled={busy} onclick={() => { const inp = document.createElement('input'); inp.type = 'file'; inp.accept = 'image/png'; inp.onchange = () => upload(inp.files?.[0], i); inp.click(); }}><ImageUp size={13} /></button>
              <AssetPicker kind="texture" label="Texture" onpick={(r) => setStages(stages.map((s, j) => (j === i ? { ...s, texture: r, model: undefined } : s)))} />
              <AssetPicker kind="model" label="Model" onpick={(r) => setStages(stages.map((s, j) => (j === i ? { ...s, model: r, texture: undefined } : s)))} />
              <button class="ghost sm" aria-label="Remove stage" disabled={stages.length <= 2} onclick={() => setStages(stages.filter((_, j) => j !== i))}><Trash2 size={13} /></button>
            </div>
          {/each}
          <button class="ghost" onclick={addStage} disabled={stages.length >= 8}><Plus size={14} /> Add a stage</button>
        </div>
      {/if}

      <div class="card">
        <McTextInput label="Item name" placeholder={doc.title || 'Name'} bind:value={() => doc.spec.name ?? '', (v) => (doc.spec = { ...doc.spec, name: v || undefined })} />
        <McLoreEditor name={doc.spec.name || doc.title} bind:lines={() => (doc.spec.lore ?? []) as string[], (l) => (doc.spec = { ...doc.spec, lore: l })} />
      </div>

      <div class="card"><ContentFields kind={doc.kind} title={doc.title} bind:v={doc.spec} /></div>

      <div class="actions">
        <button class="primary" onclick={save} disabled={busy || !doc.title.trim()}><Save size={15} /> {isNew ? 'Create' : 'Save'}</button>
        {#if !isNew}<button class="danger" onclick={remove}><Trash2 size={15} /> Delete</button>{/if}
        {#if !isNew}<span class="cmd"><code>/customitem give &lt;player&gt; {doc.id}</code></span>{/if}
      </div>
    </div>

    <aside class="side"><ModelViewer {view} height={340} /></aside>
  </div>
</section>

<style>
  .ce { display: grid; gap: 14px; }
  header { display: flex; align-items: center; gap: 12px; }
  header small { display: block; color: var(--muted); }
  .badge { display: grid; place-items: center; width: 40px; height: 40px; border-radius: 12px; background: color-mix(in srgb, var(--c) 22%, transparent); color: var(--c); }
  .grid { display: grid; grid-template-columns: minmax(0, 1.6fr) minmax(280px, 1fr); gap: 16px; align-items: start; }
  .col { display: grid; gap: 14px; min-width: 0; }
  .side { position: sticky; top: 16px; }
  .card { display: grid; gap: 12px; padding: 14px; border-radius: 14px; background: var(--surface); border: 1px solid var(--line); }
  .card.ids { grid-template-columns: 1fr 1fr; }
  .card.look.drop { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, var(--surface)); }
  label { display: grid; gap: 4px; font-size: 0.8rem; color: var(--muted); }
  label small, .card > small { color: var(--muted); font-weight: 400; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  .sh { display: flex; gap: 8px; align-items: baseline; }
  .sh small { color: var(--muted); }
  .stage { display: flex; gap: 8px; align-items: center; padding: 8px 10px; border-radius: 10px; background: var(--surface-2); border: 1px solid var(--line); flex-wrap: wrap; }
  .stage .n { display: grid; place-items: center; width: 24px; height: 24px; border-radius: 50%; background: var(--accent); color: #fff; font-size: 0.75rem; font-weight: 700; }
  .stage .what { flex: 1; min-width: 8rem; color: var(--muted); font-family: var(--mono); font-size: 0.75rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ghost { display: inline-flex; align-items: center; gap: 6px; padding: 7px 12px; border-radius: 9px; border: 1px solid var(--line-strong); background: transparent; color: var(--text-2); cursor: pointer; }
  .ghost:hover:not(:disabled) { color: var(--text); background: var(--surface-2); }
  .ghost.sm { padding: 5px 8px; }
  .actions { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .primary { display: inline-flex; align-items: center; gap: 7px; padding: 9px 18px; border-radius: 10px; border: 0; background: var(--accent); color: #fff; font-weight: 600; cursor: pointer; }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .danger { display: inline-flex; align-items: center; gap: 7px; padding: 9px 14px; border-radius: 10px; border: 1px solid color-mix(in srgb, var(--bad) 50%, transparent); background: transparent; color: var(--bad); cursor: pointer; }
  .cmd { color: var(--muted); font-size: 0.8rem; }
  @media (max-width: 1000px) { .grid { grid-template-columns: 1fr; } .side { position: static; } .card.ids { grid-template-columns: 1fr; } }
</style>
