<script lang="ts">
  import { onMount } from 'svelte';
  import { Plus, X } from '@lucide/svelte';
  import BaseItemPicker from './BaseItemPicker.svelte';
  import Slide from './Slide.svelte';
  import { get } from '../lib/api';
  import type { LibraryItem } from '../lib/content';

  type Drop = { item: string; min: number; max: number; chance: number };
  let { value = $bindable([]), title = 'Drops', empty = 'Drops itself' }: { value: Drop[]; title?: string; empty?: string } = $props();

  let custom = $state<LibraryItem[]>([]);
  onMount(async () => { try { custom = (await get<{ items: LibraryItem[] }>('/api/admin/content')).items; } catch { /* the library is optional here */ } });

  const isCustom = (d: Drop) => !!d.item && !d.item.includes(':') && custom.some((c) => c.id === d.item);
  const vanilla = (d: Drop) => 'minecraft:' + (d.item || 'stone');
  const patch = (i: number, p: Partial<Drop>) => (value = value.map((d, j) => (j === i ? { ...d, ...p } : d)));
  const add = () => { if (value.length < 16) value = [...value, { item: 'diamond', min: 1, max: 1, chance: 1 }]; };
  const remove = (i: number) => (value = value.filter((_, j) => j !== i));
  const modeOf = (d: Drop) => (isCustom(d) ? 'custom' : 'vanilla');
</script>

<div class="de">
  <div class="head"><b>{title}</b>{#if !value.length}<small>{empty}</small>{/if}</div>
  {#each value as d, i (i)}
    <div class="row">
      <div class="top">
        <div class="seg" role="radiogroup" aria-label="Item kind">
          <button role="radio" aria-checked={modeOf(d) === 'vanilla'} class:on={modeOf(d) === 'vanilla'} onclick={() => patch(i, { item: 'diamond' })}>Vanilla item</button>
          <button role="radio" aria-checked={modeOf(d) === 'custom'} class:on={modeOf(d) === 'custom'} disabled={!custom.length} onclick={() => patch(i, { item: custom[0]?.id ?? d.item })}>My custom item</button>
        </div>
        <button class="x" aria-label="Remove drop" onclick={() => remove(i)}><X size={14} /></button>
      </div>
      {#if modeOf(d) === 'vanilla'}
        <BaseItemPicker label="Item" bind:value={() => vanilla(d), (v) => patch(i, { item: v.replace('minecraft:', '') })} />
      {:else}
        <div class="chips">
          {#each custom as c (c.id)}<button class="chip" class:on={d.item === c.id} onclick={() => patch(i, { item: c.id })}>{#if c.preview}<img src={c.preview} alt="" />{/if}{c.title}</button>{/each}
        </div>
      {/if}
      <div class="two">
        <Slide label="At least" min={0} max={64} bind:value={() => d.min, (v) => patch(i, { min: v, max: Math.max(v, d.max) })} />
        <Slide label="At most" min={0} max={64} bind:value={() => d.max, (v) => patch(i, { max: v, min: Math.min(v, d.min) })} />
      </div>
      <Slide label="Chance" min={0} max={1} step={0.01} color="#e2a336" bind:value={() => d.chance, (v) => patch(i, { chance: v })} format={(n) => `${Math.round(n * 100)}%`} />
    </div>
  {/each}
  <button class="add" onclick={add} disabled={value.length >= 16}><Plus size={14} /> Add a drop</button>
</div>

<style>
  .de { display: grid; gap: 10px; }
  .head { display: flex; gap: 8px; align-items: baseline; }
  .head small { color: var(--muted); }
  .row { display: grid; gap: 12px; padding: 12px; border-radius: 12px; background: var(--surface-2); border: 1px solid var(--line); }
  .top { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  .x { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; border: 0; background: transparent; color: var(--muted); cursor: pointer; }
  .x:hover { background: var(--surface-3); color: var(--bad); }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; max-height: 150px; overflow: auto; }
  .chip { display: inline-flex; align-items: center; gap: 6px; padding: 5px 11px; border-radius: 999px; border: 1px solid var(--line-strong); background: transparent; color: var(--text-2); font-size: 0.78rem; cursor: pointer; }
  .chip img { width: 18px; height: 18px; image-rendering: pixelated; }
  .chip.on { background: var(--accent-soft); border-color: var(--accent); color: var(--text); }
  .add { justify-self: start; display: inline-flex; align-items: center; gap: 6px; padding: 7px 13px; border-radius: 9px; border: 1px dashed var(--line-strong); background: transparent; color: var(--text-2); cursor: pointer; }
  .add:hover:not(:disabled) { border-color: var(--accent); color: var(--accent-2); }
</style>
