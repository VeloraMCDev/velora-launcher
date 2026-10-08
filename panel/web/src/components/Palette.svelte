<script lang="ts" module>
  import type { Component } from 'svelte';
  export type PaletteItem = {
    id: string;
    label: string;
    group: string;
    hint?: string;
    icon?: Component<any>;
    keywords?: string;
    run: () => void;
  };
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import { Search, CornerDownLeft } from '@lucide/svelte';

  // A Ctrl/Cmd+K quick switcher shared by the admin panel and the player panel.
  let { open = $bindable(false), items, placeholder = 'Jump to a page or command…' }: { open: boolean; items: PaletteItem[]; placeholder?: string } = $props();

  let query = $state('');
  let cursor = $state(0);
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLDivElement>();

  const score = (it: PaletteItem, q: string) => {
    const label = it.label.toLowerCase();
    if (label.startsWith(q)) return 3;
    if (label.includes(q)) return 2;
    return `${it.hint ?? ''} ${it.keywords ?? ''} ${it.group}`.toLowerCase().includes(q) ? 1 : 0;
  };
  const results = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items.slice(0, 60);
    return items.map((it) => ({ it, s: score(it, q) })).filter((x) => x.s > 0).sort((a, b) => b.s - a.s).map((x) => x.it).slice(0, 60);
  });
  const grouped = $derived.by(() => {
    const out: { group: string; rows: { it: PaletteItem; index: number }[] }[] = [];
    results.forEach((it, index) => {
      let g = out.find((x) => x.group === it.group);
      if (!g) out.push((g = { group: it.group, rows: [] }));
      g.rows.push({ it, index });
    });
    return out;
  });

  $effect(() => {
    if (open) {
      query = '';
      cursor = 0;
      void tick().then(() => input?.focus());
    }
  });
  $effect(() => { void query; cursor = 0; });

  function choose(it: PaletteItem | undefined) {
    if (!it) return;
    open = false;
    it.run();
  }
  async function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); cursor = Math.min(results.length - 1, cursor + 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); cursor = Math.max(0, cursor - 1); }
    else if (e.key === 'Enter') { e.preventDefault(); choose(results[cursor]); }
    else if (e.key === 'Escape') open = false;
    else return;
    await tick();
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  }
  function global(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); open = !open; }
  }
</script>

<svelte:window onkeydown={global} />

{#if open}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && (open = false)}>
    <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <label class="search">
        <Search size={18} />
        <input bind:this={input} bind:value={query} {placeholder} onkeydown={key} autocomplete="off" spellcheck="false" aria-label="Search" />
        <kbd>esc</kbd>
      </label>
      <div class="list" bind:this={list}>
        {#each grouped as g (g.group)}
          <div class="group">{g.group}</div>
          {#each g.rows as { it, index } (it.id)}
            <button class="item" class:on={index === cursor} onmousemove={() => (cursor = index)} onclick={() => choose(it)}>
              <span class="ico">{#if it.icon}<it.icon size={16} />{/if}</span>
              <span class="lbl">{it.label}</span>
              {#if it.hint}<span class="hint">{it.hint}</span>{/if}
              {#if index === cursor}<CornerDownLeft size={13} class="enter" />{/if}
            </button>
          {/each}
        {:else}
          <p class="none">Nothing matches “{query}”.</p>
        {/each}
      </div>
      <footer><span><kbd>↑</kbd><kbd>↓</kbd> move</span><span><kbd>↵</kbd> open</span><span><kbd>ctrl</kbd><kbd>K</kbd> toggle</span></footer>
    </div>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 200; background: rgba(4, 5, 9, 0.66); backdrop-filter: blur(6px); display: grid; place-items: start center; padding: 12vh 16px 16px; animation: fade 0.14s ease; }
  .palette { width: min(640px, 100%); max-height: min(560px, 76vh); display: flex; flex-direction: column; overflow: hidden; border-radius: 16px; background: var(--surface); border: 1px solid var(--line-strong); box-shadow: 0 30px 80px -20px #000, 0 0 0 1px rgba(255, 255, 255, 0.03); animation: pop 0.18s var(--ease, ease); }
  .search { display: flex; align-items: center; gap: 12px; padding: 14px 16px; border-bottom: 1px solid var(--line); color: var(--muted); }
  .search input { flex: 1; background: transparent; border: none; padding: 0; font-size: 1.02rem; color: var(--text); box-shadow: none !important; }
  .search input:focus { background: transparent; }
  .list { overflow-y: auto; padding: 6px 8px 10px; }
  .group { padding: 10px 10px 4px; font-size: 0.68rem; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); font-weight: 600; }
  .item { width: 100%; display: flex; align-items: center; justify-content: flex-start; gap: 12px; padding: 9px 10px; border: none; background: transparent; border-radius: 10px; text-align: left; color: var(--text); font-weight: 500; }
  .item.on { background: color-mix(in srgb, var(--accent) 20%, transparent); }
  .ico { width: 28px; height: 28px; display: grid; place-items: center; border-radius: 8px; background: var(--surface-2); color: var(--text-2); flex-shrink: 0; }
  .item.on .ico { background: var(--accent); color: #fff; }
  .lbl { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .hint { margin-left: auto; color: var(--muted); font-size: 0.78rem; font-weight: 400; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 55%; }
  .item :global(.enter) { color: var(--muted); flex-shrink: 0; }
  .none { padding: 28px; text-align: center; color: var(--muted); }
  footer { display: flex; gap: 16px; padding: 9px 16px; border-top: 1px solid var(--line); font-size: 0.72rem; color: var(--muted); }
  footer kbd { margin-right: 3px; }
  @keyframes fade { from { opacity: 0; } }
  @keyframes pop { from { opacity: 0; transform: translateY(-8px) scale(0.98); } }
  @media (max-width: 640px) { .backdrop { padding-top: 8vh; } footer { display: none; } }
</style>
