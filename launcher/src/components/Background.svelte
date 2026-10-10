<script lang="ts">
  // Full-window backdrop: instance banner → admin image/video → ambient glow on plain colour.
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
  {:else if !hidden}
    <div class="orb a"></div>
    <div class="orb b"></div>
    <div class="orb c"></div>
  {/if}
  <div class="vignette"></div>
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
  /* Slow drifting colour behind everything when the admin has not set an image. */
  .orb { position: absolute; border-radius: 50%; filter: blur(7rem); opacity: 0.34; will-change: transform; }
  .a { width: 46rem; height: 46rem; left: -12rem; top: -14rem; background: var(--accent); animation: drift-a 26s ease-in-out infinite alternate; }
  .b { width: 38rem; height: 38rem; right: -10rem; bottom: -12rem; background: color-mix(in srgb, var(--accent-2) 60%, #2dd4bf); opacity: 0.2; animation: drift-b 32s ease-in-out infinite alternate; }
  .c { width: 28rem; height: 28rem; left: 42%; top: 38%; background: color-mix(in srgb, var(--accent) 50%, #f472b6); opacity: 0.12; animation: drift-c 38s ease-in-out infinite alternate; }
  @keyframes drift-a { to { transform: translate(14rem, 9rem) scale(1.15); } }
  @keyframes drift-b { to { transform: translate(-12rem, -8rem) scale(1.1); } }
  @keyframes drift-c { to { transform: translate(-9rem, 7rem) scale(0.85); } }
  .vignette { position: absolute; inset: 0; background: radial-gradient(ellipse at 50% 40%, transparent 45%, color-mix(in srgb, var(--bg) 70%, transparent) 100%); pointer-events: none; }
</style>
