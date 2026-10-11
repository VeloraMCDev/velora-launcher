<script lang="ts">
  import { iconUrl } from '@velora/experience/icons.svelte';
  import { abs } from '../lib/store.svelte';
  import { resolveIcon } from '../lib/achievementIcons';

  // Shared ccVaults source with administrator overrides and a graceful image fallback.
  let { id, size = 2, emoji }: { id: string; size?: number; emoji?: string } = $props();
  const tex = $derived(iconUrl(id));
  let failed = $state(false);
  $effect(() => { void tex; failed = false; });
  const glyph = $derived(emoji ?? resolveIcon({ icon_item: id.replace(/^minecraft:/i, '').toLowerCase() }).emoji);
</script>

<span class="mc" style:--s="{size}rem">{#if tex && !failed}<img src={abs(tex)} alt="" draggable="false" onerror={() => (failed = true)} />{:else}<span class="g">{glyph}</span>{/if}</span>

<style>
  .mc { display: inline-grid; place-items: center; width: var(--s); height: var(--s); flex-shrink: 0; }
  img { width: 100%; height: 100%; object-fit: contain; image-rendering: pixelated; filter: drop-shadow(0 2px 0 rgb(0 0 0 / 0.28)); }
  .g { font-size: calc(var(--s) * 0.6); line-height: 1; }
</style>
