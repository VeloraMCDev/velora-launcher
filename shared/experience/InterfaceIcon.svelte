<script lang="ts">
  import type { Component } from 'svelte';
  import { iconUrl } from './icons.svelte';
  import { UI_ITEMS } from './ui-icons';
  let { name, value, fallback: Fallback, size = 24, ...rest }: { name: string; value?: string; fallback: Component<any>; size?: number | string; [key: string]: any } = $props();
  const src = $derived(iconUrl(value ?? UI_ITEMS[name], value ? undefined : `ui:${name}`));
  let failed = $state(false);
  $effect(() => { void src; failed = false; });
</script>
{#if src && !failed}<img {...rest} {src} width={size} height={size} alt="" draggable="false" onerror={() => (failed = true)} />{:else}<Fallback {size} {...rest} />{/if}
<style>img { object-fit: contain; image-rendering: pixelated; flex-shrink: 0; vertical-align: middle; }</style>
