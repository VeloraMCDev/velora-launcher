<script lang="ts">
  import { spriteUrl } from '@velora/experience/icons.svelte';

  // Shared ccVaults source with administrator overrides and a graceful image fallback.
  let { name, width, height, scale = 2 }: { name: string; width: number; height: number; scale?: number } = $props();
  const src = $derived(spriteUrl(name));
  let failed = $state(false);
  $effect(() => { void src; failed = false; });
</script>

{#if src && !failed}<img {src} alt="" draggable="false" onerror={() => (failed = true)} style:width="{width * scale}px" style:height="{height * scale}px" />{/if}

<style>
  img { image-rendering: pixelated; flex-shrink: 0; }
</style>
