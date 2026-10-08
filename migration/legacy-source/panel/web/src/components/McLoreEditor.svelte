<script lang="ts">
  import { Plus, X, ArrowUp, ArrowDown, Sparkles } from '@lucide/svelte';
  import McTextInput from './McTextInput.svelte';
  import { spans } from '../lib/mccolor';
  import { markdownToAmp } from '../lib/mcText';

  let { lines = $bindable([]), name = '', max = 12 }: { lines: string[]; name?: string; max?: number } = $props();

  function add() { if (lines.length < max) lines = [...lines, '']; }
  function remove(i: number) { lines = lines.filter((_, j) => j !== i); }
  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= lines.length) return;
    const next = [...lines];
    [next[i], next[j]] = [next[j], next[i]];
    lines = next;
  }
  function set(i: number, v: string) { lines = lines.map((l, j) => (j === i ? v : l)); }
  const SNIPPETS: [string, string][] = [
    ['Rarity', '{gold}**LEGENDARY**'], ['Divider', '{dark_gray}━━━━━━━━━━'], ['Stat', '{gray}Damage: {red}+12'], ['Flavour', '{dark_purple}*“Forged in lightning.”*'],
  ];
</script>

<div class="lore">
  <div class="head"><b>Lore</b><small>the lines under the name</small></div>
  <div class="split">
    <div class="rows">
      {#each lines as line, i (i)}
        <div class="row">
          <div class="ctl">
            <button type="button" aria-label="Move up" disabled={i === 0} onclick={() => move(i, -1)}><ArrowUp size={13} /></button>
            <button type="button" aria-label="Move down" disabled={i === lines.length - 1} onclick={() => move(i, 1)}><ArrowDown size={13} /></button>
          </div>
          <div class="in"><McTextInput compact showPreview={false} base="#aa00aa" placeholder="Line {i + 1}" bind:value={() => lines[i], (v) => set(i, v)} /></div>
          <button type="button" class="del" aria-label="Remove line" onclick={() => remove(i)}><X size={14} /></button>
        </div>
      {/each}
      <div class="add">
        <button type="button" class="addbtn" onclick={add} disabled={lines.length >= max}><Plus size={14} /> Add a line</button>
        <span class="snips"><Sparkles size={12} />{#each SNIPPETS as [n, md]}<button type="button" class="snip" onclick={() => { if (lines.length < max) lines = [...lines, markdownToAmp(md)]; }}>{n}</button>{/each}</span>
      </div>
    </div>
    <aside class="tip" aria-label="Item tooltip preview">
      <div class="n">{#if name}{#each spans(name, '#ffffff') as s}<span style:color={s.color} class:b={s.bold} class:i={s.italic} class:u={s.underline} class:st={s.strike}>{s.text}</span>{/each}{:else}Item name{/if}</div>
      {#each lines.filter((l) => l.trim()) as l}<div class="l">{#each spans(l, '#aa00aa') as s}<span style:color={s.color} class:b={s.bold} class:i={s.italic} class:u={s.underline} class:st={s.strike}>{s.text}</span>{/each}</div>{/each}
      {#if !lines.some((l) => l.trim())}<div class="l dim">No lore yet</div>{/if}
    </aside>
  </div>
</div>

<style>
  .lore { display: grid; gap: 8px; }
  .head { display: flex; align-items: baseline; gap: 8px; }
  .head small { color: var(--muted); }
  .split { display: grid; grid-template-columns: minmax(0, 1fr) 220px; gap: 12px; align-items: start; }
  .rows { display: grid; gap: 8px; min-width: 0; }
  .row { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 6px; align-items: start; padding: 8px; border-radius: 12px; background: var(--surface); border: 1px solid var(--line); }
  .ctl { display: grid; gap: 2px; padding-top: 34px; }
  .ctl button, .del { display: grid; place-items: center; width: 24px; height: 24px; border-radius: 6px; border: 0; background: transparent; color: var(--muted); cursor: pointer; }
  .ctl button:hover:not(:disabled), .del:hover { background: var(--surface-3); color: var(--text); }
  .ctl button:disabled { opacity: 0.25; cursor: default; }
  .del { margin-top: 34px; }
  .add { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .addbtn { display: inline-flex; align-items: center; gap: 6px; padding: 7px 13px; border-radius: 9px; border: 1px dashed var(--line-strong); background: transparent; color: var(--text-2); cursor: pointer; }
  .addbtn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent-2); }
  .snips { display: inline-flex; align-items: center; gap: 5px; color: var(--muted); }
  .snip { padding: 3px 9px; border-radius: 999px; border: 1px solid var(--line-strong); background: transparent; color: var(--text-2); font-size: 0.74rem; cursor: pointer; }
  .snip:hover { border-color: var(--accent); color: var(--accent-2); }
  .tip { position: sticky; top: 12px; padding: 9px 11px; border-radius: 4px; background: #100010f2; border: 2px solid #2a0a5e; font: 13.5px/1.45 ui-monospace, 'Cascadia Mono', Consolas, monospace; color: #fff; text-shadow: 1px 1px 0 #0007; }
  .tip .n { color: #fff; margin-bottom: 2px; }
  .tip .l { white-space: pre-wrap; word-break: break-word; color: #aa00aa; font-style: italic; }
  .tip .dim { color: #555; }
  .b { font-weight: 700; } .i { font-style: italic; } .u { text-decoration: underline; } .st { text-decoration: line-through; }
  @media (max-width: 760px) { .split { grid-template-columns: 1fr; } }
</style>
