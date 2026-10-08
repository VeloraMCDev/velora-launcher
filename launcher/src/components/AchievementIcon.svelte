<script lang="ts">
  import { abs } from '../lib/store.svelte';
  import { resolveIcon } from '../lib/achievementIcons';

  let { ach, size = 2.8, locked = false }: {
    ach: { icon_item?: string | null; icon_bg?: string | null; icon_border?: string | null; frame_type?: string | null };
    size?: number;
    locked?: boolean;
  } = $props();

  // Achievements store ids ("diamond_sword", "obsidian", "gold"); resolve them into something drawable.
  const icon = $derived(resolveIcon(ach));
  const frame = $derived(ach.frame_type === 'goal' || ach.frame_type === 'challenge' ? ach.frame_type : 'task');
</script>

<span
  class="frame {frame}"
  class:locked
  style:--size="{size}rem"
  style:--rim={icon.borderColor}
  style:--glow={icon.glow}
  aria-hidden="true"
>
  <span class="inner" style:background={icon.background}>{#if icon.image}<img src={abs(icon.image)} alt="" />{:else}<span class="glyph">{icon.emoji}</span>{/if}</span>
</span>

<style>
  .frame { width: var(--size); height: var(--size); flex-shrink: 0; display: grid; place-items: center; border: 2px solid var(--rim); background: #18181b; box-shadow: var(--glow); transition: filter 0.2s, box-shadow 0.2s; }
  .frame.task { border-radius: 0.4rem; }
  .frame.goal { border-radius: 50%; }
  .frame.challenge { border-radius: 0.4rem; clip-path: polygon(15% 0%, 85% 0%, 100% 15%, 100% 85%, 85% 100%, 15% 100%, 0% 85%, 0% 15%); box-shadow: none; }
  .inner { width: 78%; height: 78%; display: grid; place-items: center; border-radius: inherit; box-shadow: inset 0 0 0.4rem rgba(0, 0, 0, 0.5); }
  .inner img { width: 100%; height: 100%; object-fit: contain; image-rendering: pixelated; }
  .glyph { font-size: calc(var(--size) * 0.5); line-height: 1; filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.6)); }
  .frame.locked { filter: grayscale(0.85) brightness(0.75); box-shadow: none; }
</style>
