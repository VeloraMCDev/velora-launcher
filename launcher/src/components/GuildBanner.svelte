<script lang="ts">
  import { abs } from '../lib/store.svelte';
  import { guildHue } from '../lib/guildColor';
  let { guild, height = 6, children }: { guild: { id: string; banner_url?: string | null }; height?: number; children?: import('svelte').Snippet } = $props();
  let failed = $state(false);
  $effect(() => { void guild.banner_url; failed = false; });
  const hue = $derived(guildHue(guild.id));
</script>

<div class="banner" style:--h={hue} style:height="{height}rem">
  {#if guild.banner_url && !failed}
    <img src={abs(guild.banner_url)} alt="" onerror={() => (failed = true)} />
  {/if}
  <div class="shade"></div>
  {@render children?.()}
</div>

<style>
  .banner { position: relative; overflow: hidden; background: radial-gradient(120% 140% at 0% 0%, hsl(var(--h) 65% 38%), transparent 60%), radial-gradient(100% 120% at 100% 100%, hsl(calc(var(--h) + 50) 60% 28%), transparent 55%), hsl(var(--h) 40% 12%); }
  .banner::before { content: ''; position: absolute; inset: 0; background: repeating-linear-gradient(45deg, hsl(var(--h) 60% 70% / 0.05) 0 12px, transparent 12px 24px); }
  img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .shade { position: absolute; inset: 0; background: linear-gradient(180deg, transparent 30%, color-mix(in srgb, var(--surface) 92%, transparent)); }
</style>
