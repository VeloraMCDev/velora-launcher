<script lang="ts">
  import { textures } from '../lib/textures.svelte';
  import { Sparkles, X } from '@lucide/svelte';
  let { belowUpdate = false }: { belowUpdate?: boolean } = $props();
  const show = $derived(textures.checked && !textures.ready && !textures.bannerDismissed);
</script>

{#if show}
  <div class="banner" class:belowUpdate role="status">
    <Sparkles size={18} />
    <div>
      <b>Showing emoji for items — for now</b>
      <span>Launch the game once and SCOPENET will unpack the real Minecraft textures from your own copy of the game. They're kept between launcher updates.</span>
    </div>
    <button class="x" aria-label="Dismiss" onclick={() => (textures.bannerDismissed = true)}><X size={16} /></button>
  </div>
{/if}

<style>
  .banner { position: fixed; left: 50%; top: 3.4rem; transform: translateX(-50%); z-index: 60; max-width: min(34rem, calc(100vw - 2rem)); display: flex; gap: 0.8rem; align-items: center;
    padding: 0.75rem 0.9rem; border-radius: 0.9rem; color: var(--text, #fff); background: color-mix(in srgb, var(--accent, #7c5cff) 22%, rgb(18 18 28 / 0.92)); border: 1px solid color-mix(in srgb, var(--accent, #7c5cff) 55%, transparent);
    box-shadow: 0 10px 40px #0009, 0 0 24px color-mix(in srgb, var(--accent, #7c5cff) 30%, transparent); backdrop-filter: blur(14px); animation: rise 0.4s cubic-bezier(0.2, 0.9, 0.3, 1.2); }
  div { display: grid; gap: 0.15rem; font-size: 0.82rem; line-height: 1.35; }
  .banner.belowUpdate { top: 6.4rem; }
  b { font-size: 0.88rem; }
  span { opacity: 0.8; }
  .x { background: none; border: 0; color: inherit; opacity: 0.6; cursor: pointer; padding: 0.25rem; border-radius: 0.4rem; }
  .x:hover { opacity: 1; background: #fff2; }
  @keyframes rise { from { opacity: 0; transform: translate(-50%, -1rem); } }
  @media (prefers-reduced-motion: reduce) { .banner { animation: none; } }
</style>
