<script lang="ts">
  import { guiUrl } from '../lib/mc.svelte';

  // Shared ccVaults source with administrator overrides and a graceful image fallback.
  let { path, width, height, scale = 2, dim = false }: { path: string; width: number; height: number; scale?: number; dim?: boolean } = $props();
  const src = $derived(guiUrl(path));
  let failed = $state(false);
  $effect(() => { void src; failed = false; });
</script>

{#if src && !failed}<img {src} alt="" draggable="false" onerror={() => (failed = true)} class:dim style:width="{width * scale}px" style:height="{height * scale}px" />{/if}

<style>
  img { image-rendering: pixelated; flex-shrink: 0; }
  .dim { opacity: 0.35; }
</style>
