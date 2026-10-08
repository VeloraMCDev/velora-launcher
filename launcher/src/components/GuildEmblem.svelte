<script lang="ts">
  import { abs } from '../lib/store.svelte';
  import { guildHue } from '../lib/guildColor';

  let { guild, size = 4, rounded = 'md' }: { guild: { id: string; tag: string; icon_url?: string | null }; size?: number; rounded?: 'md' | 'full' } = $props();
  let failed = $state(false);
  $effect(() => { void guild.icon_url; failed = false; });
  const hue = $derived(guildHue(guild.id));
</script>

<span class="emblem" class:full={rounded === 'full'} style:--s="{size}rem" style:--h={hue}>
  {#if guild.icon_url && !failed}
    <img src={abs(guild.icon_url)} alt="" onerror={() => (failed = true)} />
  {:else}
    <span class="tag">{guild.tag.slice(0, 4)}</span>
  {/if}
</span>

<style>
  .emblem { width: var(--s); height: var(--s); flex-shrink: 0; display: grid; place-items: center; overflow: hidden; border-radius: 22%; position: relative;
    background: linear-gradient(145deg, hsl(var(--h) 60% 42%), hsl(calc(var(--h) + 35) 65% 22%)); border: 2px solid hsl(var(--h) 70% 65% / 0.55);
    box-shadow: 0 0.5rem 1.4rem -0.5rem hsl(var(--h) 80% 50% / 0.55), inset 0 0 1rem hsl(var(--h) 80% 70% / 0.15); }
  .emblem.full { border-radius: 50%; }
  img { width: 100%; height: 100%; object-fit: cover; image-rendering: auto; }
  .tag { font-weight: 800; font-size: calc(var(--s) * 0.27); letter-spacing: 0.02em; color: #fff; text-shadow: 0 2px 6px #0007; }
</style>
