<script lang="ts">
  import { ImageUp, Box } from '@lucide/svelte';
  import { spans } from '../lib/mccolor';
  import AssetPicker from './AssetPicker.svelte';
  import BaseItemPicker from './BaseItemPicker.svelte';
  import EnchantPicker from './EnchantPicker.svelte';
  import AttributePicker from './AttributePicker.svelte';
  import McTextInput from './McTextInput.svelte';
  import McLoreEditor from './McLoreEditor.svelte';
  import Toggle from './Toggle.svelte';
  import { get, post } from '../lib/api';
  import { toastError } from '../lib/toast.svelte';
  import { enchantName, roman, attrInfo, itemLabel } from '../lib/mcData';

  export type Attr = { attribute: string; amount: number; operation: string; slot: string };
  export type ItemSpec = {
    item: string; amount: number; name?: string; lore?: string[]; enchants?: Record<string, number>; unbreakable?: boolean;
    glow?: boolean; hide_flags?: boolean; custom_model_data?: number; attributes?: Attr[];
    texture?: string; model?: string; data?: { format: string; value: string };
    category?: string; source?: string; food?: { nutrition: number; saturation: number };
  };
  let { spec = $bindable() }: { spec: ItemSpec } = $props();

  // What the item looks like: the picked texture, or the first texture of the picked model.
  let look = $state<string | null>(null);
  $effect(() => {
    const t = spec.texture, m = spec.model;
    if (!t && !m) { look = null; return; }
    let live = true;
    get<{ preview: string | null }>(`/api/admin/resource-assets-preview?texture=${encodeURIComponent(t ?? '')}&model=${encodeURIComponent(m ?? '')}`)
      .then((r) => { if (live) look = r.preview; }).catch(() => { if (live) look = null; });
    return () => { live = false; };
  });
  // Textured items need a vanilla base item and a model data number nobody else uses on that base item.
  async function pick(field: 'texture' | 'model', ref: string) {
    if (field === 'texture') { spec.texture = ref; spec.model = undefined; } else { spec.model = ref; spec.texture = undefined; }
    if (!spec.item.startsWith('minecraft:')) spec.item = 'minecraft:' + spec.item.replace(/^.*:/, '');
    if (!spec.custom_model_data) {
      try {
        const used = new Set((await get<{ items: { id: string; spec: ItemSpec }[] }>('/api/admin/custom-items')).items
          .filter((i) => i.spec.item === spec.item && (i.spec.texture || i.spec.model)).map((i) => i.spec.custom_model_data));
        let n = 100000; while (used.has(n)) n++;
        spec.custom_model_data = n;
      } catch (e) { toastError(e); }
    }
  }
  // Upload a PNG straight from the creator: it joins the server assets and becomes this item's texture.
  let uploading = $state(false);
  let drop = $state(false);
  let fileInput: HTMLInputElement | undefined = $state();
  async function upload(file?: File) {
    if (!file) return;
    uploading = true;
    try {
      if (!/\.png$/i.test(file.name)) throw new Error('Textures must be PNG images');
      if (file.size > 2 * 1024 * 1024) throw new Error('Textures must be at most 2 MiB');
      const name = file.name.toLowerCase().replace(/\.png$/, '').replace(/[^a-z0-9_.-]+/g, '_').replace(/^[_.-]+|[_.-]+$/g, '') || 'texture';
      const data = await new Promise<string>((resolve, reject) => { const r = new FileReader(); r.onload = () => resolve(String(r.result).split(',')[1]); r.onerror = reject; r.readAsDataURL(file); });
      await post('/api/admin/resource-assets', { path: `assets/scopenet/textures/item/${name}.png`, data });
      await pick('texture', `scopenet:item/${name}`);
    } catch (e) { toastError(e); } finally { uploading = false; if (fileInput) fileInput.value = ''; }
  }
  const ench = $derived(Object.entries(spec.enchants ?? {}));
  const title = $derived(spec.name?.trim() || itemLabel(spec.item));
  const opText = (a: Attr) => {
    const info = attrInfo(a.attribute);
    const n = a.operation === 'add' ? `${a.amount >= 0 ? '+' : ''}${+a.amount.toFixed(3)}` : `${a.amount >= 0 ? '+' : ''}${Math.round(a.amount * 100)}%`;
    return `${n} ${(info?.name ?? a.attribute).toLowerCase()}`;
  };
  const slotText = (s: string) => (s === 'any' ? 'in any slot' : s === 'mainhand' ? 'in Main Hand' : s === 'offhand' ? 'in Off Hand' : `on ${s}`);
  const lore = $derived({ get: () => spec.lore ?? [], set: (v: string[]) => (spec.lore = v) });
  let amountPct = $derived(((spec.amount - 1) / 63) * 100);
</script>

<div class="editor">
  {#if spec.data}<p>This item was captured in game ({spec.data.format}). Its original metadata is preserved when given; recreate the kit in game to change the item.</p>{/if}
  <div class="form">
    <div class="block look" class:drop role="group" aria-label="Item appearance" ondragover={(e) => { e.preventDefault(); drop = true; }} ondragleave={() => (drop = false)}
      ondrop={(e) => { e.preventDefault(); drop = false; upload(e.dataTransfer?.files?.[0]); }}>
      <span class="slot big">{#if look}<img src={look} alt="" />{:else}<small>no<br />model</small>{/if}</span>
      <div class="lookbody">
        <b>Appearance</b>
        <small>{spec.model ? `Model ${spec.model}` : spec.texture ? `Texture ${spec.texture}` : 'Looks like the plain base item. Drop a PNG here, upload one, or pick a texture / 3D model from your server assets (or import a pack from the Content Studio).'}</small>
        <div class="row tight">
          <button type="button" class="ghost" disabled={uploading} onclick={() => fileInput?.click()}><ImageUp size={14} /> {uploading ? 'Uploading…' : 'Upload PNG'}</button>
          <input bind:this={fileInput} type="file" accept="image/png" hidden onchange={(e) => upload(e.currentTarget.files?.[0])} />
          <AssetPicker kind="texture" onpick={(r) => pick('texture', r)} />
          <AssetPicker kind="model" onpick={(r) => pick('model', r)} />
          {#if spec.texture || spec.model}<button type="button" class="ghost" onclick={() => { spec.texture = undefined; spec.model = undefined; }}>Use plain item</button>{/if}
        </div>
      </div>
    </div>

    <div class="block basics">
      <div class="two">
        <BaseItemPicker bind:value={spec.item} />
        <div class="amt">
          <span class="lbl">Stack size <b>{spec.amount}</b></span>
          <input class="fancy" type="range" min="1" max="64" step="1" value={spec.amount} style:--p="{amountPct}%" aria-label="Amount" oninput={(e) => (spec.amount = +e.currentTarget.value)} />
        </div>
      </div>
      <McTextInput label="Display name" placeholder="Stormbreaker" bind:value={() => spec.name ?? '', (v) => (spec.name = v || undefined)} />
      <McLoreEditor name={title} bind:lines={() => lore.get(), (v) => lore.set(v)} />
    </div>

    <EnchantPicker bind:value={() => spec.enchants ?? {}, (v) => (spec.enchants = v)} />
    <AttributePicker bind:value={() => spec.attributes ?? [], (v) => (spec.attributes = v)} />

    <div class="block flags">
      <Toggle bind:checked={() => !!spec.unbreakable, (v) => (spec.unbreakable = v)} label="Unbreakable" help="Never loses durability" />
      <Toggle bind:checked={() => !!spec.glow, (v) => (spec.glow = v)} label="Enchant glint" help="The purple shimmer, even without enchantments" />
      <Toggle bind:checked={() => !!spec.hide_flags, (v) => (spec.hide_flags = v)} label="Hide vanilla details" help="Hides the enchantment and attribute lines from the tooltip" />
      {#if spec.category === 'food' || spec.food}
        <div class="food">
          <span class="lbl">Hunger restored <b>{spec.food?.nutrition ?? 4}</b></span>
          <input class="fancy" type="range" min="0" max="20" step="1" value={spec.food?.nutrition ?? 4} style:--p="{((spec.food?.nutrition ?? 4) / 20) * 100}%" style:--c="#f0a35e"
            aria-label="Hunger restored" oninput={(e) => (spec.food = { nutrition: +e.currentTarget.value, saturation: spec.food?.saturation ?? 2 })} />
          <span class="lbl">Saturation <b>{spec.food?.saturation ?? 2}</b></span>
          <input class="fancy" type="range" min="0" max="20" step="0.5" value={spec.food?.saturation ?? 2} style:--p="{((spec.food?.saturation ?? 2) / 20) * 100}%" style:--c="#e6c35a"
            aria-label="Saturation" oninput={(e) => (spec.food = { nutrition: spec.food?.nutrition ?? 4, saturation: +e.currentTarget.value })} />
        </div>
      {/if}
      <details class="adv"><summary>Advanced</summary>
        <label class="md">Model data number <small>assigned automatically when you pick a texture or model</small>
          <input type="number" min="0" bind:value={spec.custom_model_data} placeholder="none" />
        </label>
      </details>
    </div>
  </div>

  <aside class="tip" aria-label="Preview">
    <div class="tiphead"><span class="slot">{#if look}<img src={look} alt="" />{/if}</span></div>
    <div class="line name">{#each spans(title, '#ffffff') as s}<span style:color={s.color} class:b={s.bold} class:i={s.italic} class:u={s.underline} class:st={s.strike}>{s.text}</span>{/each}</div>
    {#if !spec.hide_flags}{#each ench as [k, v]}<div class="line" style:color="#a8a8a8">{enchantName(k)} {roman(v)}</div>{/each}{/if}
    {#each (spec.lore ?? []).filter((l) => l.trim() !== '') as l}
      <div class="line">{#each spans(l, '#aa00aa') as s}<span style:color={s.color} class:b={s.bold} class:i={s.italic} class:u={s.underline} class:st={s.strike}>{s.text}</span>{/each}</div>
    {/each}
    {#if spec.unbreakable}<div class="line" style:color="#5555ff">Unbreakable</div>{/if}
    {#if !spec.hide_flags}{#each spec.attributes ?? [] as a}
      <div class="line attr"><span style:color="#a8a8a8">When {slotText(a.slot)}:</span><br /><span style:color={a.amount >= 0 ? '#5555ff' : '#ff5555'}>{opText(a)}</span></div>
    {/each}{/if}
    <div class="line dim">{spec.item}</div>
  </aside>
</div>

<style>
  .editor { display: grid; grid-template-columns: minmax(0, 1.7fr) minmax(15rem, 1fr); gap: 16px; align-items: start; }
  .form { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
  .lbl { font-size: 0.8rem; color: var(--muted); display: flex; gap: 6px; align-items: baseline; }
  .lbl b { color: var(--text); font-family: var(--mono); }
  .row { display: flex; gap: 10px; align-items: flex-end; flex-wrap: wrap; }
  .row.tight { align-items: center; gap: 6px; }
  .block { display: flex; flex-direction: column; gap: 14px; padding: 14px; border: 1px solid var(--line); border-radius: 14px; background: var(--surface); }
  .two { display: grid; grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr); gap: 16px; align-items: end; }
  .amt { display: grid; gap: 10px; padding-bottom: 8px; }
  .flags { gap: 8px; }
  .food { display: grid; gap: 8px; padding: 10px 0 4px; }
  .adv summary { cursor: pointer; color: var(--muted); font-size: 0.8rem; }
  .md { display: grid; gap: 4px; margin-top: 8px; font-size: 0.8rem; color: var(--muted); max-width: 18rem; }
  .look.drop { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 14%, transparent); }
  .look { flex-direction: row; gap: 12px; align-items: center; }
  .lookbody { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .lookbody small { color: var(--muted); }
  .slot { width: 44px; height: 44px; flex-shrink: 0; display: grid; place-items: center; border-radius: 6px; background: #8b8b8b; box-shadow: inset 2px 2px 0 #373737, inset -2px -2px 0 #fff8; }
  .slot.big { width: 72px; height: 72px; }
  .slot img { width: 75%; height: 75%; object-fit: contain; image-rendering: pixelated; }
  .slot small { color: #3a3a3a; font-size: 0.6rem; text-align: center; }
  .tiphead { margin-bottom: 8px; }
  .tip { position: sticky; top: 16px; background: #100010f0; border: 2px solid #2a0a5e; outline: 2px solid #1a0338; border-radius: 4px; padding: 10px 12px; font: 14px/1.45 ui-monospace, 'Cascadia Mono', Consolas, monospace; color: #fff; text-shadow: 1px 1px 0 #0007; min-height: 4rem; }
  .line { white-space: pre-wrap; word-break: break-word; }
  .line.dim { color: #555; margin-top: 4px; }
  .line.attr { margin-top: 6px; }
  .name { font-size: 15px; }
  .b { font-weight: 700; } .i { font-style: italic; } .u { text-decoration: underline; } .st { text-decoration: line-through; }
  @media (max-width: 1000px) { .editor { grid-template-columns: 1fr; } .tip { position: static; } .two { grid-template-columns: 1fr; } }
</style>
