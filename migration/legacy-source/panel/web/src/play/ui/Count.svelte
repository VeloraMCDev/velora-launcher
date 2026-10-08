<script lang="ts">
  import { untrack } from 'svelte';
  // A number that counts to its new value instead of jumping. `format` turns the in-between number into text.
  let { value, format = (v: number) => v.toLocaleString(undefined, { maximumFractionDigits: 2 }) }: { value: number | null; format?: (v: number) => string } = $props();
  let shown = $state<number | null>(untrack(() => value));
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
<span style="font-variant-numeric: tabular-nums">{shown == null ? '—' : format(shown)}</span>
