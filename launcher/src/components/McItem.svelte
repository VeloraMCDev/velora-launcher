<script lang="ts">
  import { itemTexture, texVersion } from '../lib/textures.svelte';
  import { resolveIcon } from '../lib/achievementIcons';

  // A Minecraft item drawn with its real texture (unpacked from the player's own game), or an emoji until that has happened.
  let { id, size = 2, emoji }: { id: string; size?: number; emoji?: string } = $props();
  let tex = $state<string | null>(null);
  $effect(() => {
    void texVersion.n;
    const want = id;
    tex = null;
    if (/^[a-z0-9_:]+$/i.test(want)) itemTexture(want).then((u) => { if (want === id) tex = u; });
  });
  const glyph = $derived(emoji ?? resolveIcon({ icon_item: id.replace(/^minecraft:/i, '').toLowerCase() }).emoji);
</script>

<span class="mc" style:--s="{size}rem">{#if tex}<img src={tex} alt="" draggable="false" />{:else}<span class="g">{glyph}</span>{/if}</span>

<style>
  .mc { display: inline-grid; place-items: center; width: var(--s); height: var(--s); flex-shrink: 0; }
  img { width: 100%; height: 100%; object-fit: contain; image-rendering: pixelated; filter: drop-shadow(0 2px 0 rgb(0 0 0 / 0.28)); }
  .g { font-size: calc(var(--s) * 0.6); line-height: 1; }
</style>
