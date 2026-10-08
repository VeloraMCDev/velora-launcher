<script lang="ts">
  import { Search, ChevronRight } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { BASE_GROUPS, itemLabel } from '../lib/mcData';
  import { resolveIcon } from '../lib/achievementIcons';

  let { value = $bindable('minecraft:paper'), label = 'Base item' }: { value: string; label?: string } = $props();

  let open = $state(false);
  let q = $state('');
  let group = $state('weapons');
  let custom = $state('');
  const emoji = (id: string) => resolveIcon({ icon_item: id.replace('minecraft:', '') }).emoji;
  const all = BASE_GROUPS.flatMap((g) => g.items);
  const shown = $derived(q ? all.filter((i) => itemLabel(i).toLowerCase().includes(q.toLowerCase())) : (BASE_GROUPS.find((g) => g.id === group)?.items ?? []));
  function pick(id: string) { value = id; open = false; }
  function useCustom() {
    const id = custom.trim().toLowerCase();
    if (/^[a-z0-9_]+$/.test(id)) pick('minecraft:' + id);
    else if (/^minecraft:[a-z0-9_]+$/.test(id)) pick(id);
  }
</script>

<div class="bip">
  <span class="lbl">{label}</span>
  <button type="button" class="cur" onclick={() => (open = true)}>
    <span class="tile">{emoji(value)}</span>
    <span class="txt"><b>{itemLabel(value)}</b><small>{value}</small></span>
    <span class="chg">Change <ChevronRight size={14} /></span>
  </button>
</div>

<Modal bind:open title="Choose the base item" width={760}>
  <label class="search"><Search size={15} /><input bind:value={q} placeholder="Search items…" /></label>
  {#if !q}
    <div class="seg">
      {#each BASE_GROUPS as g}<button class:on={group === g.id} onclick={() => (group = g.id)}>{g.label}</button>{/each}
    </div>
  {/if}
  <div class="grid">
    {#each shown as id (id)}
      <button class="t" class:on={value === id} onclick={() => pick(id)}><span class="tile big">{emoji(id)}</span><span>{itemLabel(id)}</span></button>
    {/each}
    {#if !shown.length}<p class="none">No match. Type the item id below.</p>{/if}
  </div>
  <div class="custom">
    <small>Something else?</small>
    <input bind:value={custom} placeholder="e.g. cooked_salmon" onkeydown={(e) => e.key === 'Enter' && useCustom()} />
    <button type="button" onclick={useCustom} disabled={!custom.trim()}>Use this id</button>
  </div>
  <p class="note">Textured items are shown as this item, then re-skinned by the resource pack — pick something that behaves how you want (a sword swings, a bread can be eaten).</p>
</Modal>

<style>
  .bip { display: grid; gap: 5px; }
  .lbl { font-size: 0.8rem; color: var(--muted); }
  .cur { display: flex; align-items: center; gap: 12px; padding: 8px 12px 8px 8px; border-radius: 12px; border: 1px solid var(--line-strong); background: var(--surface-2); color: inherit; cursor: pointer; text-align: left; transition: border-color 0.12s; }
  .cur:hover { border-color: var(--accent); }
  .txt { display: grid; flex: 1; min-width: 0; }
  .txt small { color: var(--muted); font-family: var(--mono); font-size: 0.7rem; }
  .chg { display: inline-flex; align-items: center; color: var(--accent-2); font-size: 0.8rem; }
  .tile { display: grid; place-items: center; width: 42px; height: 42px; border-radius: 9px; background: #8b8b8b; box-shadow: inset 2px 2px 0 #373737, inset -2px -2px 0 #fff8; font-size: 22px; }
  .tile.big { width: 52px; height: 52px; font-size: 26px; }
  .search { display: flex; align-items: center; gap: 8px; padding: 0 11px; border-radius: 10px; background: var(--surface-2); border: 1px solid var(--line); margin-bottom: 10px; }
  .search input { border: 0; background: none; padding: 9px 0; outline: none; flex: 1; color: inherit; }
  .seg { margin-bottom: 10px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(104px, 1fr)); gap: 8px; max-height: 46vh; overflow: auto; grid-auto-rows: min-content; align-content: start; padding: 2px; }
  .t { display: grid; justify-items: center; gap: 6px; padding: 10px 6px; border-radius: 11px; border: 1px solid var(--line); background: var(--surface-2); color: inherit; font-size: 0.74rem; text-align: center; cursor: pointer; transition: transform 0.1s, border-color 0.1s; }
  .t:hover { transform: translateY(-2px); border-color: var(--accent); }
  .t.on { border-color: var(--accent); background: var(--accent-soft); }
  .none { color: var(--muted); grid-column: 1 / -1; }
  .custom { display: flex; align-items: center; gap: 8px; margin-top: 12px; flex-wrap: wrap; }
  .custom small { color: var(--muted); }
  .custom input { flex: 1; min-width: 10rem; }
  .note { margin-top: 10px; color: var(--muted); font-size: 0.78rem; }
</style>
