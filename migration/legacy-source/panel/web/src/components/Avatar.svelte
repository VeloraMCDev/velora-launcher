<script lang="ts">
  // Player head rendered by the panel from the stored skin; falls back to
  // a coloured initial for players without one.
  let { name, uuid = null, size = 32, v = 0 }: { name: string; uuid?: string | null; size?: number; v?: number } = $props();
  let failed = $state(false);
  const hue = $derived([...name].reduce((a, c) => a + c.charCodeAt(0), 0) % 360);
  const src = $derived(`/api/v1/avatar/${encodeURIComponent(uuid ?? name)}?size=${Math.max(16, Math.min(256, size * 2))}${v ? `&v=${v}` : ''}`);
  $effect(() => {
    void src;
    failed = false;
  });
</script>

{#if failed}
  <span class="fallback" style:width="{size}px" style:height="{size}px" style:font-size="{Math.max(10, size * 0.4)}px" style:background="hsl({hue} 45% 40%)">{name.slice(0, 1).toUpperCase()}</span>
{:else}
  <img {src} alt="" width={size} height={size} onerror={() => (failed = true)} />
{/if}

<style>
  img, .fallback { border-radius: 25%; image-rendering: pixelated; flex-shrink: 0; }
  .fallback { display: inline-grid; place-items: center; font-weight: 650; color: white; }
</style>
