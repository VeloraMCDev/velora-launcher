<script lang="ts">
  // Full-window backdrop: instance banner → admin image/video → plain colour.
  import { experienceBranding, abs, app, selectedInstance } from '../lib/store.svelte';

  const b = $derived(experienceBranding());
  const inst = $derived(selectedInstance());
  const image = $derived(abs(inst?.banner_url) ?? (b?.background.kind === 'image' ? abs(b.background.url) : null));
  const video = $derived(!inst?.banner_url && b?.background.kind === 'video' && app.settings?.background_video !== false && app.running.length === 0 ? abs(b.background.url) : null);
  const blur = $derived(b?.background.blur ?? 0);
  const dim = $derived((b?.background.dim ?? 55) / 100);
  let hidden = $state(false);
</script>

<svelte:document onvisibilitychange={() => (hidden = document.hidden)} />

<div class="bg" aria-hidden="true">
  {#if video && !hidden}
    {#key video}<video src={video} autoplay muted loop playsinline style:filter="blur({blur}px)"></video>{/key}
    <div class="shade" style:--dim={dim}></div>
  {:else if image}
    {#key image}<div class="img" style:background-image="url(&quot;{image}&quot;)" style:filter="blur({blur}px)"></div>{/key}
    <div class="shade" style:--dim={dim}></div>
  {/if}
</div>

<style>
  .bg { position: fixed; inset: 0; z-index: 0; overflow: hidden; background: var(--bg); }
  video, .img { position: absolute; inset: -1.5rem; width: calc(100% + 3rem); height: calc(100% + 3rem); object-fit: cover; background-size: cover; background-position: center; animation: fade 0.4s ease; }
  /* Keep text readable: darken, strongest on the left where the title sits. */
  .shade {
    position: absolute; inset: 0;
    background:
      linear-gradient(90deg, var(--bg) 0%, color-mix(in srgb, var(--bg) calc(var(--dim) * 100%), transparent) 55%, color-mix(in srgb, var(--bg) calc(var(--dim) * 70%), transparent) 100%),
      linear-gradient(0deg, var(--bg) 0%, transparent 40%);
  }
</style>
