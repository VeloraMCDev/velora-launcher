<script lang="ts">
  import { Wand2, Sparkles } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { ICONS, PRESETS, renderPrefix, renderToFile, unsupported, type PrefixStyle } from '../lib/prefixPng';
  import { uploadMedia } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  /** Receives the uploaded PNG's URL. */
  let { onmade, initialText = '' }: { onmade: (url: string) => void; initialText?: string } = $props();

  let open = $state(false);
  let busy = $state(false);
  let style = $state<PrefixStyle>({ ...PRESETS.Member });
  let canvas: HTMLCanvasElement | undefined = $state();
  const bad = $derived(unsupported(style.text));

  $effect(() => {
    // Re-draw whenever any control changes.
    const snapshot = { ...style };
    if (canvas) renderPrefix(canvas, snapshot);
  });

  function show() {
    style = { ...style, text: initialText.trim() ? initialText.trim().slice(0, 24) : style.text };
    open = true;
  }
  function preset(name: string) { style = { ...PRESETS[name] }; }
  async function make() {
    busy = true;
    try {
      const url = await uploadMedia(await renderToFile($state.snapshot(style)));
      onmade(url);
      toast('Badge created and attached');
      open = false;
    } catch (e) { toastError(e); } finally { busy = false; }
  }
</script>

<button type="button" class="ghost" onclick={show}><Wand2 size={15} /> Design a badge</button>

<Modal bind:open title="Rank badge designer" width={640}>
  <div class="stage">
    <div class="checker"><canvas bind:this={canvas} style:--w="{style.scale * 0.35}"></canvas></div>
    <div class="presets">
      {#each Object.keys(PRESETS) as p}<button type="button" class="ghost sm" onclick={() => preset(p)}><Sparkles size={13} /> {p}</button>{/each}
    </div>
  </div>

  <div class="grid">
    <label class="wide">Text<input bind:value={style.text} maxlength="24" placeholder="MEMBER" />
      {#if bad.length}<small class="warn">No pixel glyph for {bad.join(' ')} — shown as “?”</small>{/if}
    </label>
    <label>Icon
      <select bind:value={style.icon}>{#each Object.entries(ICONS) as [k, v]}<option value={k}>{v.label}</option>{/each}</select>
    </label>
    <label>Size
      <select bind:value={style.scale}><option value={2}>Small</option><option value={3}>Medium</option><option value={4}>Large</option></select>
    </label>
    <label>Gradient from<input type="color" bind:value={style.from} /></label>
    <label>Gradient to<input type="color" bind:value={style.to} /></label>
    <label class="wide">Angle <b>{style.angle}°</b><input type="range" min="0" max="360" step="5" bind:value={style.angle} /></label>
    <label>Text<input type="color" bind:value={style.textColor} /></label>
    <label>Icon colour<input type="color" bind:value={style.iconColor} /></label>
    <label>Outline<input type="color" bind:value={style.outline} /></label>
    <label>Shadow<input type="color" bind:value={style.shadow} /></label>
    <label class="check"><input type="checkbox" bind:checked={style.border} /> Light border</label>
  </div>

  {#snippet footer()}
    <button type="button" class="ghost" onclick={() => (open = false)}>Cancel</button>
    <button type="button" onclick={make} disabled={busy || !style.text.trim()}>{busy ? 'Uploading…' : 'Create & use'}</button>
  {/snippet}
</Modal>

<style>
  .stage { display: grid; gap: 10px; justify-items: center; margin-bottom: 14px; }
  .checker { padding: 22px 28px; border-radius: 14px; border: 1px solid var(--line); background: conic-gradient(#ffffff0d 25%, #0000 0 50%, #ffffff0d 0 75%, #0000 0) 0 0 / 16px 16px, var(--bg-2); }
  canvas { image-rendering: pixelated; display: block; height: auto; transform: scale(calc(1 + var(--w))); transform-origin: center; margin: 4px; }
  .presets { display: flex; gap: 6px; flex-wrap: wrap; justify-content: center; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 10px 14px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 0.82rem; color: var(--muted); }
  label.wide { grid-column: 1 / -1; }
  label.check { flex-direction: row; align-items: center; gap: 8px; }
  input[type='color'] { width: 100%; height: 34px; padding: 2px; border-radius: 8px; border: 1px solid var(--line); background: var(--bg-2); }
  .warn { color: #ffb86b; }
  .sm { padding: 4px 10px; font-size: 0.8rem; display: inline-flex; gap: 5px; align-items: center; }
</style>
