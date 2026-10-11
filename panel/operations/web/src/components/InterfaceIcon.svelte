<script lang="ts">
  import type { Component } from 'svelte';
  import { iconUrl } from '../lib/icons.svelte';
  let { name, iconKey, fallback: Fallback, size = 24, ...rest }: { name: string; iconKey?: string; fallback: Component<any>; size?: number | string; [key: string]: any } = $props();
  const src = $derived(iconUrl(name, iconKey));
  let failed = $state(false);
  $effect(() => { void src; failed = false; });
</script>
{#if src && !failed}<img {...rest} {src} width={size} height={size} alt="" draggable="false" onerror={() => (failed = true)} />{:else}<Fallback {size} {...rest} />{/if}
<style>img { object-fit: contain; image-rendering: pixelated; flex-shrink: 0; vertical-align: middle; }</style>
