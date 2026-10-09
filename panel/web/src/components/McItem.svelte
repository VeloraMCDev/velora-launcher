<script lang="ts">
  import { itemUrl } from '../lib/mc.svelte';
  import { resolveIcon } from '../lib/achievementIcons';

  // A Minecraft item or block drawn with its real texture, falling back to an emoji until textures are installed.
  let { id, size = 32, slot = false, emoji, title }: { id: string; size?: number; slot?: boolean; emoji?: string; title?: string } = $props();

  let failed = $state(false);
  const src = $derived(itemUrl(id));
  $effect(() => { void src; failed = false; });
  const glyph = $derived(emoji ?? resolveIcon({ icon_item: id.replace(/^minecraft:/i, '').toLowerCase() }).emoji);
  const pad = $derived(slot ? Math.round(size * 0.14) : 0);
</script>

<span class="mc-item" class:slot style:--s="{size}px" style:--p="{pad}px" {title}>
  {#if src && !failed}<img {src} alt="" draggable="false" onerror={() => (failed = true)} />{:else}<span class="g" style:font-size="{size * 0.55}px">{glyph}</span>{/if}
</span>

<style>
  .mc-item { display: inline-grid; place-items: center; width: var(--s); height: var(--s); flex-shrink: 0; box-sizing: border-box; }
  .mc-item img { width: 100%; height: 100%; image-rendering: pixelated; object-fit: contain; filter: drop-shadow(0 2px 0 rgb(0 0 0 / 0.28)); }
  .g { line-height: 1; }
  /* The classic inventory slot. */
  .slot { padding: var(--p); background: #8b8b8b; box-shadow: inset 2px 2px 0 #373737, inset -2px -2px 0 rgb(255 255 255 / 0.55); border-radius: 2px; }
</style>
