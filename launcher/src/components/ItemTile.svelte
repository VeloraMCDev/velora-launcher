<script lang="ts">
  import { resolveIcon } from '../lib/achievementIcons';
  import { itemTexture, texVersion } from '../lib/textures.svelte';
  let { id, amount = 1, size = 3.2 }: { id: string; amount?: number; size?: number } = $props();
  const emoji = $derived(resolveIcon({ icon_item: id.toLowerCase() }).emoji);
  // A steady colour per item so the same thing always looks the same.
  let tex = $state<string | null>(null);
  $effect(() => {
    texVersion.n;
    const want = id;
    tex = null;
    itemTexture(want).then((u) => { if (want === id) tex = u; });
  });
  const hue = $derived([...id].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7));
</script>

<span class="tile" style:--s="{size}rem" style:--h={hue} title={id.replace(/_/g, ' ').toLowerCase()}>
  {#if tex}<img class="px" src={tex} alt="" draggable="false" />{:else}<span class="g">{emoji}</span>{/if}
  {#if amount > 1}<b>{amount}</b>{/if}
</span>

<style>
  .tile { position: relative; width: var(--s); height: var(--s); flex-shrink: 0; display: grid; place-items: center; border-radius: 0.6rem;
    background: linear-gradient(145deg, hsl(var(--h) 45% 26%), hsl(calc(var(--h) + 30) 50% 14%)); border: 1px solid hsl(var(--h) 45% 40% / 0.6); box-shadow: inset 0 0 0.8rem hsl(var(--h) 60% 50% / 0.18); }
  .px { width: 62%; height: 62%; image-rendering: pixelated; filter: drop-shadow(0 2px 3px #0008); }
  .g { font-size: calc(var(--s) * 0.5); filter: drop-shadow(0 2px 3px #0008); }
  b { position: absolute; right: 0.15rem; bottom: 0.05rem; font-size: 0.68rem; font-weight: 700; color: #fff; text-shadow: 1px 1px 0 #000, -1px -1px 0 #000, 1px -1px 0 #000, -1px 1px 0 #000; }
</style>
