<script lang="ts">
  // Single-series column chart: ≤24px columns with 4px rounded caps,
  // hairline grid, per-column hover tooltip and a screen-reader table.
  let { data, label }: { data: { day: string; launches: number }[]; label: string } = $props();

  const W = 640, H = 200, PAD_L = 34, PAD_B = 26, PAD_T = 12;
  let hover = $state<number | null>(null);

  const max = $derived(Math.max(4, ...data.map((d) => d.launches)));
  const step = $derived(niceStep(max));
  const top = $derived(Math.ceil(max / step) * step);
  const ticks = $derived(Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step));
  const band = $derived((W - PAD_L) / Math.max(1, data.length));
  const barW = $derived(Math.min(24, band - 6));
  const y = (v: number) => PAD_T + (H - PAD_T - PAD_B) * (1 - v / top);

  function niceStep(m: number) {
    const raw = m / 4;
    const pow = 10 ** Math.floor(Math.log10(raw));
    return [1, 2, 5, 10].map((s) => s * pow).find((s) => s >= raw) ?? raw;
  }

  function colPath(x: number, v: number) {
    const yb = H - PAD_B, yt = y(v), h = yb - yt;
    if (h <= 0) return '';
    const r = Math.min(4, h, barW / 2);
    return `M${x},${yb} V${yt + r} Q${x},${yt} ${x + r},${yt} H${x + barW - r} Q${x + barW},${yt} ${x + barW},${yt + r} V${yb} Z`;
  }

  const fmtDay = (d: string) => new Date(d + 'T00:00:00').toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
</script>

<div class="chart">
  <svg viewBox="0 0 {W} {H}" role="img" aria-label={label}>
    {#each ticks as t}
      <line x1={PAD_L} x2={W} y1={y(t)} y2={y(t)} class="grid" />
      <text x={PAD_L - 8} y={y(t) + 4} class="tick" text-anchor="end">{t.toLocaleString()}</text>
    {/each}
    {#each data as d, i}
      {@const x = PAD_L + i * band + (band - barW) / 2}
      <path d={colPath(x, d.launches)} class="bar" class:dim={hover !== null && hover !== i} />
      {#if i % 2 === data.length % 2 || data.length < 8}
        <text x={x + barW / 2} y={H - 8} class="tick" text-anchor="middle">{fmtDay(d.day)}</text>
      {/if}
      <rect
        x={PAD_L + i * band} y={PAD_T} width={band} height={H - PAD_T - PAD_B} fill="transparent"
        role="presentation" onmouseenter={() => (hover = i)} onmouseleave={() => (hover = null)}
      />
    {/each}
  </svg>
  {#if hover !== null}
    {@const d = data[hover]}
    <div class="tip" style:left="{((PAD_L + hover * band + band / 2) / W) * 100}%" style:top="{(y(d.launches) / H) * 100}%">
      <strong>{d.launches.toLocaleString()}</strong> launch{d.launches === 1 ? '' : 'es'}
      <span>{fmtDay(d.day)}</span>
    </div>
  {/if}
  <table class="sr-only">
    <caption>{label}</caption>
    <tbody>{#each data as d}<tr><th>{d.day}</th><td>{d.launches}</td></tr>{/each}</tbody>
  </table>
</div>

<style>
  .chart { position: relative; }
  svg { width: 100%; height: auto; display: block; overflow: visible; }
  .grid { stroke: rgba(255, 255, 255, 0.06); stroke-width: 1; }
  .tick { fill: var(--muted); font-size: 11px; font-family: var(--font); }
  .bar { fill: #6d6af5; transition: opacity 0.15s; }
  .bar.dim { opacity: 0.4; }
  .tip {
    position: absolute; transform: translate(-50%, calc(-100% - 10px)); pointer-events: none; background: var(--surface-3);
    border: 1px solid var(--line-strong); border-radius: 8px; padding: 6px 10px; font-size: 0.8rem; white-space: nowrap;
    display: flex; gap: 6px; align-items: baseline; box-shadow: var(--shadow);
  }
  .tip span { color: var(--muted); }
</style>
