<script lang="ts">
  // A number that counts up or down to its new value instead of jumping.
  import { money } from './casino';
  let { value, digits = 2 }: { value: number | null; digits?: number } = $props();
  let shown = $state<number | null>(null);
  let frame = 0;
  $effect(() => {
    const to = value;
    cancelAnimationFrame(frame);
    if (to == null || shown == null) { shown = to; return; }
    const from = shown, start = performance.now(), ms = Math.min(900, 250 + Math.abs(to - from) * 2);
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / ms);
      shown = from + (to - from) * (1 - Math.pow(1 - t, 3));
      if (t < 1) frame = requestAnimationFrame(step); else shown = to;
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });
</script>
<span class="money">{money(shown, digits)}</span>
<style>.money { font-variant-numeric: tabular-nums; }</style>
