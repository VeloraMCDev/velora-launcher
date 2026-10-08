<script lang="ts">
  import { avatarUrl } from './host';
  // A player's head, or a coloured initial when there is no skin (or the host gave no URL).
  let { name = null, uuid = null, size = 2.2 }: { name?: string | null; uuid?: string | null; size?: number } = $props();
  let failed = $state(false);
  const label = $derived(name ?? '?');
  const src = $derived(avatarUrl(uuid, 64));
  const hue = $derived([...label].reduce((a, c) => a + c.charCodeAt(0), 0) % 360);
  $effect(() => { void src; failed = false; });
</script>

{#if src && !failed}
  <img {src} alt="" style:width="{size}rem" style:height="{size}rem" onerror={() => (failed = true)} />
{:else}
  <span class="fb" style:width="{size}rem" style:height="{size}rem" style:background="hsl({hue} 45% 40%)" style:font-size="{size * 0.45}rem">{label.slice(0, 1).toUpperCase()}</span>
{/if}

<style>
  img, .fb { border-radius: 22%; flex-shrink: 0; image-rendering: pixelated; object-fit: cover; }
  .fb { display: inline-grid; place-items: center; color: #fff; font-weight: 700; image-rendering: auto; }
</style>
