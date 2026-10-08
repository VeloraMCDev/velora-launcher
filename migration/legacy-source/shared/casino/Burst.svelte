<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { reducedMotion } from './anim';
  // Coins and confetti that fly out of the middle and fall. Remount (via {#key}) to replay. Uses the Web Animations API, which
  // keeps working when the system turns CSS animations off.
  let { count = 32, colors = ['#f5d97a', '#fff3b0', '#f59e0b', '#fde68a', '#f472b6', '#60a5fa'] }: { count?: number; colors?: string[] } = $props();
  const n = untrack(() => count);
  const bits = Array.from({ length: n }, (_, i) => {
    const a = -Math.PI / 2 + (Math.random() - 0.5) * Math.PI * 1.35;
    const power = 110 + Math.random() * 190;
    return { vx: Math.cos(a) * power, vy: Math.sin(a) * power, r: Math.random() * 720 - 360, s: 6 + Math.random() * 8, c: colors[i % colors.length], t: 900 + Math.random() * 700, coin: i % 3 === 0 };
  });
  let box: HTMLDivElement;
  onMount(() => {
    if (reducedMotion()) { box.style.display = 'none'; return; }
    const anims: Animation[] = [];
    [...box.children].forEach((el, i) => {
      const b = bits[i];
      const peak = { x: b.vx * 0.55, y: b.vy * 0.55 };
      anims.push((el as HTMLElement).animate(
        [
          { transform: 'translate(0,0) rotate(0deg) scale(0.3)', opacity: 1 },
          { transform: `translate(${peak.x}px,${peak.y}px) rotate(${b.r * 0.4}deg) scale(1)`, opacity: 1, offset: 0.35 },
          { transform: `translate(${b.vx}px,${b.vy + 260}px) rotate(${b.r}deg) scale(0.9)`, opacity: 0 },
        ],
        { duration: b.t, easing: 'cubic-bezier(0.2, 0.6, 0.4, 1)', fill: 'forwards' }
      ));
    });
    return () => anims.forEach((a) => a.cancel());
  });
</script>
<div class="burst" aria-hidden="true" bind:this={box}>
  {#each bits as b}<i class:coin={b.coin} style="width:{b.s}px;height:{b.coin ? b.s : b.s * 0.55}px;background:{b.c}"></i>{/each}
</div>
<style>
  .burst { position: absolute; inset: 0; pointer-events: none; display: grid; place-items: center; z-index: 5; }
  i { position: absolute; border-radius: 20%; opacity: 0; box-shadow: 0 0 8px #fff5; }
  i.coin { border-radius: 50%; box-shadow: inset -2px -2px 0 #0003, 0 0 8px #f5d97a99; }
</style>
