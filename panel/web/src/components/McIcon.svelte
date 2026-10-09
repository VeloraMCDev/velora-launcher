<script lang="ts">
  import type { Component } from 'svelte';
  import { itemUrl } from '../lib/mc.svelte';

  // Navigation icon: a Minecraft item when textures are installed, otherwise the usual line icon.
  let { item, fallback: Fallback, size = 18 }: { item?: string; fallback: Component<any>; size?: number } = $props();
  const src = $derived(item ? itemUrl(item) : null);
  let failed = $state(false);
  $effect(() => { void src; failed = false; });
</script>

{#if src && !failed}<img {src} alt="" draggable="false" width={size} height={size} onerror={() => (failed = true)} />{:else}<Fallback {size} />{/if}

<style>
  img { image-rendering: pixelated; object-fit: contain; flex-shrink: 0; filter: drop-shadow(0 1px 0 rgb(0 0 0 / 0.35)); }
</style>
