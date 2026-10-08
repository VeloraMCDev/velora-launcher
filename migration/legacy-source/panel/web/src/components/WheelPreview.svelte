<script lang="ts">
  // A wheel drawn from the same segments the server uses; slice width follows each segment's chance.
  let { segments, size = 190, index = null }: { segments: { label: string; weight: number; color: string }[]; size?: number; index?: number | null } = $props();
  const r = 92;
  const total = $derived(segments.reduce((a, s) => a + Math.max(0.0001, s.weight), 0));
  const slices = $derived.by(() => {
    let a = -Math.PI / 2;
    return segments.map((s, i) => {
      const span = (Math.max(0.0001, s.weight) / total) * Math.PI * 2;
      const [x1, y1, x2, y2] = [Math.cos(a), Math.sin(a), Math.cos(a + span), Math.sin(a + span)];
      const mid = a + span / 2;
      const d = `M0 0 L${x1 * r} ${y1 * r} A${r} ${r} 0 ${span > Math.PI ? 1 : 0} 1 ${x2 * r} ${y2 * r} Z`;
      const out = { d, color: s.color, label: s.label, lx: Math.cos(mid) * r * 0.66, ly: Math.sin(mid) * r * 0.66, rot: (mid * 180) / Math.PI + (Math.cos(mid) < 0 ? 180 : 0), span, i };
      a += span;
      return out;
    });
  });
</script>

<svg viewBox="-100 -100 200 200" width={size} height={size} role="img" aria-label="Wheel preview">
  <circle r="98" fill="var(--surface-3)" />
  {#each slices as s}
    <path d={s.d} fill={s.color} stroke="#0008" stroke-width="0.8" opacity={index == null || index === s.i ? 1 : 0.45} />
    {#if s.span > 0.28}<text x={s.lx} y={s.ly} transform="rotate({s.rot} {s.lx} {s.ly})" text-anchor="middle" dominant-baseline="middle" font-size="9" font-weight="700" fill="#fff" stroke="#0007" stroke-width="2" paint-order="stroke">{s.label}</text>{/if}
  {/each}
  <circle r="10" fill="var(--surface)" stroke="var(--line)" />
  <path d="M-5 -102 L5 -102 L0 -90 Z" fill="var(--text)" />
</svg>
