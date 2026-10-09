<script lang="ts">
  import { sprite, texVersion } from '../lib/textures.svelte';

  // A Minecraft GUI sprite (`gui/hud/heart/full.png`, `effect/speed.png`…). Draws nothing until the textures are unpacked.
  let { name, width, height, scale = 2 }: { name: string; width: number; height: number; scale?: number } = $props();
  let src = $state<string | null>(null);
  $effect(() => {
    void texVersion.n;
    const want = name;
    src = null;
    sprite(want).then((u) => { if (want === name) src = u; });
  });
</script>

{#if src}<img {src} alt="" draggable="false" style:width="{width * scale}px" style:height="{height * scale}px" />{/if}

<style>
  img { image-rendering: pixelated; flex-shrink: 0; }
</style>
