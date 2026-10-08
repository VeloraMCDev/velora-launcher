<script lang="ts">
  import { X, Copy, ArrowDown, Search, Trash2 } from '@lucide/svelte';
  import { app, toast } from '../lib/store.svelte';

  let box = $state<HTMLDivElement>();
  let follow = $state(true);
  let filter = $state('');
  const shown = $derived(filter ? app.logs.filter((l) => l.toLowerCase().includes(filter.toLowerCase())) : app.logs.slice(-1500));

  $effect(() => {
    void shown.length;
    if (follow && box) requestAnimationFrame(() => box && (box.scrollTop = box.scrollHeight));
  });

  function level(line: string) {
    if (/\b(ERROR|FATAL|Exception)\b/.test(line)) return 'err';
    if (/\bWARN\b/.test(line)) return 'warn';
    return '';
  }
</script>

<div class="console glass">
  <header>
    <strong>Game output</strong>
    <span class="tiny muted">{app.logs.length.toLocaleString()} lines</span>
    <span class="spacer"></span>
    <div class="search"><Search size={13} /><input bind:value={filter} placeholder="Filter" /></div>
    <button class="ghost icon sm" title="Copy" aria-label="Copy log" onclick={() => { navigator.clipboard.writeText(app.logs.join('\n')); toast('Log copied'); }}><Copy size={14} /></button>
    <button class="ghost icon sm" title="Clear" aria-label="Clear log" onclick={() => (app.logs = [])}><Trash2 size={14} /></button>
    <button class="ghost icon sm" aria-label="Close console" onclick={() => (app.consoleOpen = false)}><X size={15} /></button>
  </header>
  <div class="lines selectable" bind:this={box} onscroll={() => box && (follow = box.scrollHeight - box.scrollTop - box.clientHeight < 40)}>
    {#each shown as line, i (i)}<div class="line {level(line)}">{line}</div>{:else}<div class="muted empty">Logs appear here while the game is running.</div>{/each}
  </div>
  {#if !follow}
    <button class="jump sm" onclick={() => { follow = true; if (box) box.scrollTop = box.scrollHeight; }}><ArrowDown size={13} /> Latest</button>
  {/if}
</div>

<style>
  .console { position: absolute; left: 1rem; right: 1rem; bottom: 1rem; height: 42%; display: flex; flex-direction: column; z-index: 25; background: var(--bg); border-color: var(--line-strong); animation: fade 0.15s ease; overflow: hidden; }
  header { display: flex; align-items: center; gap: 0.6rem; padding: 0.5rem 0.6rem 0.5rem 1rem; border-bottom: 1px solid var(--line); font-size: 0.88rem; }
  .search { display: flex; align-items: center; gap: 0.3rem; color: var(--muted); }
  .search input { width: 10rem; padding: 0.3rem 0.5rem; font-size: 0.8rem; }
  .lines { flex: 1; overflow: auto; padding: 0.6rem 1rem; font-family: var(--mono); font-size: 0.74rem; line-height: 1.55; }
  .line { white-space: pre-wrap; word-break: break-all; color: color-mix(in srgb, var(--text) 78%, transparent); }
  .line.warn { color: var(--warn); }
  .line.err { color: color-mix(in srgb, var(--danger) 75%, white); }
  .empty { font-family: var(--font); padding: 1rem 0; }
  .jump { position: absolute; right: 1.5rem; bottom: 1rem; background: var(--surface); }
</style>
