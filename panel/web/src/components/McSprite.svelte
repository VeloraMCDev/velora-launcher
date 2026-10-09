<script lang="ts">
  import { guiUrl } from '../lib/mc.svelte';

  // A Minecraft GUI sprite (hearts, hunger, hotbar…). Renders nothing until textures are installed, so it is safe for decoration only.
  let { path, width, height, scale = 2, dim = false }: { path: string; width: number; height: number; scale?: number; dim?: boolean } = $props();
  const src = $derived(guiUrl(path));
</script>

{#if src}<img {src} alt="" draggable="false" class:dim style:width="{width * scale}px" style:height="{height * scale}px" />{/if}

<style>
  img { image-rendering: pixelated; flex-shrink: 0; }
  .dim { opacity: 0.35; }
</style>
