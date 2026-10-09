<script lang="ts">
  import McSprite from './McSprite.svelte';

  // Ten hearts or drumsticks showing a fraction (health, hunger, a percentage). Draws nothing without textures.
  let { value, max, kind = 'heart', scale = 1.5 }: { value: number; max: number; kind?: 'heart' | 'food'; scale?: number } = $props();
  const units = $derived(max > 0 ? Math.max(0, Math.min(20, Math.round((value / max) * 20))) : 0);
  const sprite = (i: number) => {
    const half = units - i * 2 === 1;
    const full = units - i * 2 >= 2;
    if (kind === 'heart') return `gui/${full ? 'hud/heart/full' : half ? 'hud/heart/half' : 'hud/heart/container'}.png`;
    return `gui/${full ? 'hud/food_full' : half ? 'hud/food_half' : 'hud/food_empty'}.png`;
  };
</script>

<span class="meter" role="img" aria-label="{value} of {max}">
  {#each Array(10) as _, i}<McSprite name={sprite(i)} width={9} height={9} {scale} />{/each}
</span>

<style>
  .meter { display: inline-flex; gap: 1px; align-items: center; }
</style>
