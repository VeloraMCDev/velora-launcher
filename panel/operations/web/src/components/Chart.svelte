<script lang="ts">
  // Area chart for a single metric over time, sized to its container.
  interface Props { points: { t: string; v: number | null }[]; max?: number; unit?: string; color?: string; height?: number; format?: (v: number) => string }
  let { points, max, unit = '', color = '#9b7bff', height = 120, format = (v: number) => `${v.toFixed(0)}${unit}` }: Props = $props();
  let width = $state(400);
  let hover = $state<number | null>(null);
  const id = `g${Math.random().toString(36).slice(2, 8)}`;

  const values = $derived(points.map(p => p.v).filter((v): v is number => v !== null));
  const top = $derived(max ?? Math.max(1, ...values) * 1.15);
  const x = (i: number) => (points.length < 2 ? 0 : (i / (points.length - 1)) * width);
  const y = (v: number) => height - 6 - (Math.min(v, top) / top) * (height - 14);
  const line = $derived.by(() => {
    let d = '', open = false;
    points.forEach((p, i) => {
      if (p.v === null) { open = false; return; }
      d += `${open ? 'L' : 'M'}${x(i).toFixed(1)},${y(p.v).toFixed(1)}`;
      open = true;
    });
    return d;
  });
  const area = $derived.by(() => {
    const valid = points.map((p, i) => [i, p.v] as const).filter(([, v]) => v !== null) as [number, number][];
    if (valid.length < 2) return '';
    return `M${x(valid[0][0])},${height}` + valid.map(([i, v]) => `L${x(i).toFixed(1)},${y(v).toFixed(1)}`).join('') + `L${x(valid.at(-1)![0])},${height}Z`;
  });
  function move(e: PointerEvent) {
    const rect = (e.currentTarget as SVGElement).getBoundingClientRect();
    hover = Math.round(((e.clientX - rect.left) / rect.width) * (points.length - 1));
  }
</script>

<div class="chart" bind:clientWidth={width}>
  {#if values.length < 2}
    <div class="none" style:height="{height}px">Collecting data…</div>
  {:else}
    <svg {width} {height} role="img" aria-label="Metric history" onpointermove={move} onpointerleave={() => (hover = null)}>
      <defs><linearGradient {id} x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color={color} stop-opacity="0.35" /><stop offset="1" stop-color={color} stop-opacity="0" /></linearGradient></defs>
      {#each [0.25, 0.5, 0.75] as f}<line x1="0" x2={width} y1={height * f} y2={height * f} class="grid" />{/each}
      <path d={area} fill="url(#{id})" />
      <path d={line} fill="none" stroke={color} stroke-width="2" stroke-linejoin="round" />
      {#if hover !== null && points[hover]?.v !== null && points[hover]}
        <line x1={x(hover)} x2={x(hover)} y1="0" y2={height} class="cursor" />
        <circle cx={x(hover)} cy={y(points[hover].v!)} r="4" fill={color} />
      {/if}
    </svg>
    {#if hover !== null && points[hover]}
      <div class="tip" style:left="{Math.min(Math.max(x(hover), 60), width - 60)}px">
        <b>{points[hover].v === null ? '—' : format(points[hover].v!)}</b>
        <span>{new Date(points[hover].t).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}</span>
      </div>
    {/if}
  {/if}
</div>

<style>
  .chart { position: relative; width: 100%; }
  svg { display: block; touch-action: none; }
  .grid { stroke: var(--line); stroke-dasharray: 3 5; }
  .cursor { stroke: var(--line-2); }
  .none { display: grid; place-items: center; color: var(--faint); font-size: 13px; }
  .tip { position: absolute; top: -6px; transform: translateX(-50%); background: var(--panel-2); border: 1px solid var(--line-2); border-radius: 8px; padding: 3px 9px; font-size: 12px; pointer-events: none; display: flex; gap: 8px; white-space: nowrap; }
  .tip b { color: var(--head); }
  .tip span { color: var(--muted); }
</style>
