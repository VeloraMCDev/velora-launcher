<script lang="ts">
  let { label, value = $bindable(0), min = 0, max = 100, step = 1, unit = '', hint = '', color = 'var(--accent)', format }: {
    label: string; value: number; min?: number; max?: number; step?: number; unit?: string; hint?: string; color?: string; format?: (n: number) => string;
  } = $props();
  const pct = $derived(max === min ? 0 : ((value - min) / (max - min)) * 100);
  const text = $derived(format ? format(value) : `${+value.toFixed(3)}${unit ? ' ' + unit : ''}`);
</script>

<div class="sl">
  <div class="top"><span class="l">{label}</span><span class="v" style:color>{text}</span></div>
  <input class="fancy" type="range" {min} {max} {step} {value} style:--p="{pct}%" style:--c={color} aria-label={label} oninput={(e) => (value = +e.currentTarget.value)} />
  {#if hint}<small>{hint}</small>{/if}
</div>

<style>
  .sl { display: grid; gap: 7px; }
  .top { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .l { font-size: 0.82rem; color: var(--text-2); }
  .v { font-family: var(--mono); font-weight: 700; font-size: 0.9rem; }
  small { color: var(--muted); font-size: 0.74rem; }
</style>
