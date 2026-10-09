<script lang="ts">
  import McSprite from './McSprite.svelte';

  // Ten hearts or drumsticks that show a fraction (players online / capacity, a percentage…). Renders nothing without textures.
  let { value, max, kind = 'heart', scale = 1.5 }: { value: number; max: number; kind?: 'heart' | 'food'; scale?: number } = $props();
  const units = $derived(max > 0 ? Math.max(0, Math.min(20, Math.round((value / max) * 20))) : 0);
  const sprite = (i: number) => {
    const half = units - i * 2 === 1;
    const full = units - i * 2 >= 2;
    if (kind === 'heart') return full ? 'hud/heart/full' : half ? 'hud/heart/half' : 'hud/heart/container';
    return full ? 'hud/food_full' : half ? 'hud/food_half' : 'hud/food_empty';
  };
</script>

<span class="meter" role="img" aria-label="{value} of {max}">
  {#each Array(10) as _, i}<McSprite path={sprite(i)} width={9} height={9} {scale} />{/each}
</span>

<style>
  .meter { display: inline-flex; gap: 1px; align-items: center; }
</style>
