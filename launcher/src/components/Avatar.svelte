<script lang="ts">
  import { app } from '../lib/store.svelte';
  import type { Account } from '../lib/types';

  let { account = null, name = null, uuid = null, size = 2.2 }: {
    account?: Account | null;
    name?: string | null;
    uuid?: string | null;
    size?: number;
  } = $props();

  let failed = $state(false);
  const username = $derived(account?.username ?? name ?? '?');
  const playerUuid = $derived(account?.uuid ?? uuid ?? null);
  const panel = $derived(account?.panel_url ?? app.panelUrl);
  // Head rendered by the panel from the player's skin.
  const src = $derived(panel && playerUuid ? `${panel}/api/v1/avatar/${playerUuid}?size=64&v=${app.skinVersion}` : null);
  const hue = $derived([...username].reduce((a, c) => a + c.charCodeAt(0), 0) % 360);
  $effect(() => {
    void src;
    failed = false;
  });
</script>

{#if src && !failed}
  <img {src} alt="" style:width="{size}rem" style:height="{size}rem" onerror={() => (failed = true)} />
{:else}
  <span class="fb" style:width="{size}rem" style:height="{size}rem" style:background="hsl({hue} 45% 40%)">{username.slice(0, 1).toUpperCase()}</span>
{/if}

<style>
  img, .fb { border-radius: 25%; image-rendering: pixelated; flex-shrink: 0; display: inline-grid; place-items: center; font-weight: 650; color: white; font-size: 0.9rem; }
</style>
