<script lang="ts">
  import InterfaceIcon from "@velora/experience/InterfaceIcon.svelte";
  import { iconName } from "@velora/experience/icon-components";
  import type { Component } from 'svelte';
  import { iconUrl } from '@velora/experience/icons.svelte';

  // Shared ccVaults source with administrator overrides and a graceful image fallback.
  let { item, iconKey, fallback: Fallback, size = 18 }: { item?: string; iconKey?: string; fallback: Component<any>; size?: number } = $props();
  const src = $derived(iconUrl(item, iconKey));
  let failed = $state(false);
  $effect(() => { void src; failed = false; });
</script>

{#if src && !failed}<img {src} alt="" draggable="false" width={size} height={size} onerror={() => (failed = true)} />{:else}<InterfaceIcon name={iconName(Fallback)} fallback={Fallback} {size} />{/if}

<style>
  img { image-rendering: pixelated; object-fit: contain; flex-shrink: 0; filter: drop-shadow(0 1px 0 rgb(0 0 0 / 0.35)); }
</style>
