<script lang="ts">
  import { Bold, Italic, Underline, Strikethrough, Palette, Sparkles, Eraser } from '@lucide/svelte';
  import { ampToMarkdown, markdownToAmp, gradientMarkdown, PALETTE, plainText } from '../lib/mcText';
  import { spans } from '../lib/mccolor';

  let { value = $bindable(''), base = '#ffffff', placeholder = 'Type here…', label = '', compact = false, showPreview = true }: {
    value: string; base?: string; placeholder?: string; label?: string; compact?: boolean; showPreview?: boolean;
  } = $props();

  let md = $state('');
  let input: HTMLInputElement | undefined = $state();
  let pop = $state<'' | 'color' | 'gradient'>('');
  let custom = $state('#ff8800');
  let stops = $state(['#ff8800', '#ff2d95']);
  let recent = $state<string[]>([]);
  let root: HTMLDivElement | undefined = $state();

  try { recent = JSON.parse(localStorage.getItem('mc-recent-colors') ?? '[]'); } catch { /* none yet */ }

  // Keep the markdown in step with outside changes (loading an item, undo elsewhere), without fighting the user's typing.
  $effect(() => {
    const v = value;
    if (markdownToAmp(md) !== v) md = ampToMarkdown(v, base);
  });

  function commit(next: string, from?: number, to?: number) {
    md = next;
    value = markdownToAmp(next);
    if (from !== undefined) queueMicrotask(() => { input?.focus(); input?.setSelectionRange(from, to ?? from); });
  }
  const sel = () => [input?.selectionStart ?? md.length, input?.selectionEnd ?? md.length] as const;

  function wrap(tok: string) {
    const [s, e] = sel();
    if (s === e) {
      commit(md.slice(0, s) + tok + tok + md.slice(e), s + tok.length);
    } else if (md.slice(s - tok.length, s) === tok && md.slice(e, e + tok.length) === tok) {
      commit(md.slice(0, s - tok.length) + md.slice(s, e) + md.slice(e + tok.length), s - tok.length, e - tok.length);
    } else {
      commit(md.slice(0, s) + tok + md.slice(s, e) + tok + md.slice(e), s + tok.length, e + tok.length);
    }
  }
  function colour(c: string) {
    const [s, e] = sel();
    const token = `{${c}}`;
    if (s === e) commit(md.slice(0, s) + token + md.slice(e), s + token.length);
    else commit(md.slice(0, s) + token + md.slice(s, e) + '{/}' + md.slice(e), s + token.length, e + token.length);
    if (c.startsWith('#')) {
      recent = [c, ...recent.filter((x) => x !== c)].slice(0, 8);
      try { localStorage.setItem('mc-recent-colors', JSON.stringify(recent)); } catch { /* private mode */ }
    }
    pop = '';
  }
  function applyGradient() {
    let [s, e] = sel();
    if (s === e) { s = 0; e = md.length; }
    const piece = md.slice(s, e).replace(/\{[^}]*\}|\*\*|__|~~|\\/g, '');
    commit(md.slice(0, s) + gradientMarkdown(piece, stops) + md.slice(e));
    pop = '';
  }
  function clear() {
    commit(plainText(value));
  }
  function key(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    const map: Record<string, string> = { b: '**', i: '*', u: '__' };
    if (map[k]) { e.preventDefault(); wrap(map[k]); }
  }
  function outside(e: MouseEvent) {
    if (pop && root && !root.contains(e.target as Node)) pop = '';
  }
  const GRADIENTS: [string, string[]][] = [
    ['Fire', ['#ff9a00', '#ff2d2d']], ['Ocean', ['#00c6ff', '#0072ff']], ['Royal', ['#8e2de2', '#ff5fa2']],
    ['Forest', ['#11998e', '#8ee06d']], ['Gold', ['#fff200', '#ff9800']], ['Sunset', ['#ff5f6d', '#ffc371']],
    ['Frost', ['#e0f7ff', '#55a8ff']], ['Rainbow', ['#ff3b30', '#ffd60a', '#34c759', '#0a84ff', '#bf5af2']],
  ];
  const preview = $derived(spans(value || '', base));
</script>

<svelte:window onmousedown={outside} />

<div class="mt" class:compact bind:this={root}>
  {#if label}<span class="lbl">{label}</span>{/if}
  <div class="bar" role="toolbar" aria-label="Text formatting">
    <button type="button" title="Bold (Ctrl+B)" onmousedown={(e) => e.preventDefault()} onclick={() => wrap('**')}><Bold size={14} /></button>
    <button type="button" title="Italic (Ctrl+I)" onmousedown={(e) => e.preventDefault()} onclick={() => wrap('*')}><Italic size={14} /></button>
    <button type="button" title="Underline (Ctrl+U)" onmousedown={(e) => e.preventDefault()} onclick={() => wrap('__')}><Underline size={14} /></button>
    <button type="button" title="Strikethrough" onmousedown={(e) => e.preventDefault()} onclick={() => wrap('~~')}><Strikethrough size={14} /></button>
    <span class="sep"></span>
    <button type="button" class:on={pop === 'color'} title="Colour" onmousedown={(e) => e.preventDefault()} onclick={() => (pop = pop === 'color' ? '' : 'color')}><Palette size={14} /> <span>Colour</span></button>
    <button type="button" class:on={pop === 'gradient'} title="Gradient" onmousedown={(e) => e.preventDefault()} onclick={() => (pop = pop === 'gradient' ? '' : 'gradient')}><Sparkles size={14} /> <span>Gradient</span></button>
    <button type="button" title="Clear all formatting" onmousedown={(e) => e.preventDefault()} onclick={clear}><Eraser size={14} /></button>

    {#if pop === 'color'}
      <div class="pop" role="dialog" aria-label="Pick a colour">
        <div class="swatches">
          {#each PALETTE as [name, , hex]}
            <button type="button" class="sw" style:background={hex} title={name.replace(/_/g, ' ')} aria-label={name.replace(/_/g, ' ')} onclick={() => colour(name)}></button>
          {/each}
        </div>
        <div class="custom">
          <input type="color" bind:value={custom} aria-label="Custom colour" />
          <code>{custom}</code>
          <button type="button" class="use" onclick={() => colour(custom)}>Use</button>
        </div>
        {#if recent.length}
          <div class="recent"><small>Recent</small>{#each recent as c}<button type="button" class="sw small" style:background={c} aria-label={c} onclick={() => colour(c)}></button>{/each}</div>
        {/if}
      </div>
    {:else if pop === 'gradient'}
      <div class="pop wide" role="dialog" aria-label="Make a gradient">
        <div class="gprev" style:background="linear-gradient(90deg, {stops.join(', ')})"></div>
        <div class="gstops">
          {#each stops as _, i}<input type="color" bind:value={stops[i]} aria-label="Gradient colour {i + 1}" />{/each}
          {#if stops.length < 5}<button type="button" class="chip" onclick={() => (stops = [...stops, '#ffffff'])}>+ colour</button>{/if}
          {#if stops.length > 2}<button type="button" class="chip" onclick={() => (stops = stops.slice(0, -1))}>− colour</button>{/if}
        </div>
        <div class="presets">
          {#each GRADIENTS as [n, g]}<button type="button" class="preset" style:background="linear-gradient(90deg, {g.join(', ')})" onclick={() => (stops = [...g])}>{n}</button>{/each}
        </div>
        <button type="button" class="use full" onclick={applyGradient}>Apply to {sel()[0] === sel()[1] ? 'all text' : 'selection'}</button>
      </div>
    {/if}
  </div>
  <input bind:this={input} class="field" value={md} oninput={(e) => commit(e.currentTarget.value)} onkeydown={key} {placeholder} spellcheck="false" autocomplete="off" />
  {#if showPreview}
    <div class="out" aria-label="Preview">{#if preview.length}{#each preview as s}<span style:color={s.color} class:b={s.bold} class:i={s.italic} class:u={s.underline} class:st={s.strike}>{s.text}</span>{/each}{:else}<em>Preview</em>{/if}</div>
  {/if}
</div>

<style>
  .mt { position: relative; display: grid; gap: 6px; min-width: 0; }
  .lbl { font-size: 0.8rem; color: var(--muted); }
  .bar { display: flex; align-items: center; gap: 3px; flex-wrap: wrap; padding: 3px; border-radius: 9px; background: var(--surface-2); border: 1px solid var(--line); position: relative; }
  .bar > button { display: inline-flex; align-items: center; gap: 5px; height: 28px; padding: 0 8px; border-radius: 7px; border: 0; background: transparent; color: var(--text-2); font-size: 0.78rem; cursor: pointer; }
  .bar > button:hover { background: var(--surface-3); color: var(--text); }
  .bar > button.on { background: var(--accent-soft); color: var(--accent-2); }
  .compact .bar > button span { display: none; }
  .sep { width: 1px; height: 16px; background: var(--line-strong); margin: 0 3px; }
  .field { font-family: var(--mono); font-size: 0.86rem; }
  .out { padding: 7px 10px; border-radius: 4px; background: #100010f2; border: 2px solid #2a0a5e; font: 14px/1.4 ui-monospace, 'Cascadia Mono', Consolas, monospace; color: #fff; text-shadow: 1px 1px 0 #0007; min-height: 2.1em; white-space: pre-wrap; word-break: break-word; }
  .out em { color: #555; font-style: normal; }
  .b { font-weight: 700; } .i { font-style: italic; } .u { text-decoration: underline; } .st { text-decoration: line-through; }
  .u.st { text-decoration: underline line-through; }
  .pop { position: absolute; z-index: 30; top: calc(100% + 6px); left: 0; width: 250px; padding: 12px; border-radius: 12px; background: var(--surface); border: 1px solid var(--line-strong); box-shadow: var(--shadow); display: grid; gap: 10px; }
  .pop.wide { width: 290px; }
  .swatches { display: grid; grid-template-columns: repeat(8, 1fr); gap: 6px; }
  .sw { aspect-ratio: 1; border-radius: 7px; border: 2px solid transparent; cursor: pointer; padding: 0; box-shadow: inset 0 0 0 1px #0006; transition: transform 0.1s, border-color 0.1s; }
  .sw:hover { transform: scale(1.15); border-color: #fff8; }
  .sw.small { width: 20px; aspect-ratio: auto; height: 20px; }
  .custom { display: flex; align-items: center; gap: 8px; }
  .custom input[type='color'], .gstops input[type='color'] { width: 34px; height: 30px; padding: 2px; border-radius: 8px; border: 1px solid var(--line-strong); background: var(--surface-2); cursor: pointer; }
  .recent { display: flex; align-items: center; gap: 5px; flex-wrap: wrap; }
  .recent small { color: var(--muted); margin-right: 3px; }
  .use { margin-left: auto; padding: 5px 12px; border-radius: 8px; border: 0; background: var(--accent); color: #fff; font-weight: 600; cursor: pointer; }
  .use.full { width: 100%; margin: 0; padding: 8px; }
  .gprev { height: 16px; border-radius: 8px; }
  .gstops { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .chip { padding: 3px 9px; border-radius: 999px; border: 1px solid var(--line-strong); background: transparent; color: var(--text-2); font-size: 0.74rem; cursor: pointer; }
  .presets { display: grid; grid-template-columns: repeat(4, 1fr); gap: 5px; }
  .preset { height: 26px; border-radius: 7px; border: 0; font-size: 0.68rem; font-weight: 700; color: #fff; text-shadow: 0 1px 2px #000a; cursor: pointer; transition: transform 0.1s; }
  .preset:hover { transform: translateY(-1px); }
</style>
