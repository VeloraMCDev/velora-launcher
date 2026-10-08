<script lang="ts">
  import { Apple, Bot, Smartphone, Terminal, Download } from '@lucide/svelte';
  let { platform, size = 24 }: { platform: string; size?: number } = $props();
</script>

{#if platform === 'windows'}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
    <path d="M3 4.6 10.4 3.6v7.2H3zM11.6 3.4 21 2v8.8h-9.4zM3 11.9h7.4v7.2L3 18.1zM11.6 11.9H21V20.7l-9.4-1.3z" />
  </svg>
{:else if platform === 'mac'}<Apple {size} />
{:else if platform === 'linux'}<Terminal {size} />
{:else if platform === 'android'}<Bot {size} />
{:else if platform === 'ios'}<Smartphone {size} />
{:else}<Download {size} />{/if}
