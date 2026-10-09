<script lang="ts">
  import type { Component } from 'svelte';
  import { itemTexture, texVersion } from '../lib/textures.svelte';

  // Navigation icon: a Minecraft item once the game's textures are unpacked, otherwise the usual line icon.
  let { item, fallback: Fallback, size = 19 }: { item?: string; fallback: Component<any>; size?: number } = $props();
  let tex = $state<string | null>(null);
  $effect(() => {
    void texVersion.n;
    const want = item;
    tex = null;
    if (want) itemTexture(want).then((u) => { if (want === item) tex = u; });
  });
</script>

{#if tex}<img src={tex} alt="" draggable="false" width={size} height={size} />{:else}<Fallback {size} />{/if}

<style>
  img { image-rendering: pixelated; object-fit: contain; flex-shrink: 0; filter: drop-shadow(0 1px 0 rgb(0 0 0 / 0.35)); }
</style>
