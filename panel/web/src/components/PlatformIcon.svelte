<script lang="ts">
  import InterfaceIcon from '@velora/experience/InterfaceIcon.svelte';
  import { Apple, Bot, Smartphone, Terminal, Download } from '@lucide/svelte';
  import { iconUrl, interfaceIcons } from "@velora/experience/icons.svelte";
  let { platform, size = 24 }: { platform: string; size?: number } = $props();
  let failed = $state(false);
  const custom = $derived(iconUrl(interfaceIcons.overrides["ui:Platform" + platform[0]?.toUpperCase() + platform.slice(1)]));
  $effect(() => { void custom; failed = false; });
</script>

{#if custom && !failed}<img src={custom} alt="" width={size} height={size} onerror={() => failed = true} />
{:else if platform === 'windows'}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
    <path d="M3 4.6 10.4 3.6v7.2H3zM11.6 3.4 21 2v8.8h-9.4zM3 11.9h7.4v7.2L3 18.1zM11.6 11.9H21V20.7l-9.4-1.3z" />
  </svg>
{:else if platform === 'mac'}<InterfaceIcon name="Apple" fallback={Apple} {size} />
{:else if platform === 'linux'}<InterfaceIcon name="Terminal" fallback={Terminal} {size} />
{:else if platform === 'android'}<InterfaceIcon name="Bot" fallback={Bot} {size} />
{:else if platform === 'ios'}<InterfaceIcon name="Smartphone" fallback={Smartphone} {size} />
{:else}<InterfaceIcon name="Download" fallback={Download} {size} />{/if}
