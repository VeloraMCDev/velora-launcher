<script lang="ts">
  // A prize wheel. `rotation` is the total angle in degrees and is applied as-is (the caller animates it frame by frame),
  // `pointer` is how far the pointer is flicked by the pegs (degrees).
  let { segments, rotation = 0, size = 280, highlight = null, pointer = 0 }: { segments: { label: string; weight: number; color: string }[]; rotation?: number; size?: number; highlight?: number | null; pointer?: number } = $props();
  const R = 94;
  const uid = `w${Math.random().toString(36).slice(2, 8)}`;
  const total = $derived(segments.reduce((a, s) => a + Math.max(0.0001, s.weight), 0));
  const slices = $derived.by(() => {
    let a = -Math.PI / 2;
    return segments.map((s, i) => {
      const span = (Math.max(0.0001, s.weight) / total) * Math.PI * 2;
      const [x1, y1, x2, y2] = [Math.cos(a), Math.sin(a), Math.cos(a + span), Math.sin(a + span)];
      const mid = a + span / 2;
      const out = { i, d: `M0 0 L${x1 * R} ${y1 * R} A${R} ${R} 0 ${span > Math.PI ? 1 : 0} 1 ${x2 * R} ${y2 * R} Z`, color: s.color, label: s.label, lx: Math.cos(mid) * R * 0.68, ly: Math.sin(mid) * R * 0.68, deg: (mid * 180) / Math.PI + (Math.cos(mid) < 0 ? 180 : 0), span, ex: Math.cos(a) * R, ey: Math.sin(a) * R };
      a += span;
      return out;
    });
  });
  const BULBS = 24;
  const bulbs = Array.from({ length: BULBS }, (_, i) => ({ x: Math.sin((i / BULBS) * Math.PI * 2) * 100.5, y: -Math.cos((i / BULBS) * Math.PI * 2) * 100.5 }));
  // The bulbs chase around the rim while the wheel turns.
  const chase = $derived(Math.floor(rotation / 11));
</script>

<div class="wheel" style:width="{size}px">
  <svg viewBox="-104 -104 208 208" role="img" aria-label="Prize wheel" style:transform="rotate({rotation}deg)">
    <circle r="102" fill="#0b0c10" />
    <circle r="99" fill="none" stroke="url(#{uid}rim)" stroke-width="5" />
    {#each slices as s}
      <path d={s.d} fill={s.color} stroke="#0009" stroke-width="0.7" opacity={highlight == null || highlight === s.i ? 1 : 0.4} />
      <circle cx={s.ex} cy={s.ey} r="1.7" fill="#f5d97a" stroke="#0007" stroke-width="0.5" />
      {#if s.span > 0.2}
        <text x={s.lx} y={s.ly} transform="rotate({s.deg} {s.lx} {s.ly})" text-anchor="middle" dominant-baseline="middle" font-size={s.span > 0.5 ? 11 : 8} font-weight="800" fill="#fff" stroke="#0008" stroke-width="2.4" paint-order="stroke">{s.label}</text>
      {/if}
    {/each}
    <defs><linearGradient id="{uid}rim" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f5d97a" /><stop offset="0.5" stop-color="#b8892b" /><stop offset="1" stop-color="#f5d97a" /></linearGradient></defs>
  </svg>
  <svg class="bulbs" viewBox="-104 -104 208 208" aria-hidden="true">
    {#each bulbs as b, i}
      <circle cx={b.x} cy={b.y} r="2.2" fill={(i + chase) % 2 === 0 ? '#fff3b0' : '#7a5a14'} />
    {/each}
  </svg>
  <div class="hub"></div>
  <svg class="pointer" viewBox="-12 -2 24 30" aria-hidden="true" style:transform="translateX(-50%) rotate({pointer}deg)"><path d="M-10 0 H10 L0 26 Z" fill="#f5d97a" stroke="#0009" stroke-width="2" /></svg>
</div>

<style>
  .wheel { position: relative; aspect-ratio: 1; filter: drop-shadow(0 12px 28px #000a); max-width: 100%; }
  svg:not(.pointer) { width: 100%; height: 100%; display: block; will-change: transform; }
  .bulbs { position: absolute; inset: 0; pointer-events: none; }
  .hub { position: absolute; inset: 41%; border-radius: 50%; background: radial-gradient(circle at 35% 30%, #fff5c9, #b8892b 60%, #6b4c10); box-shadow: 0 0 0 3px #0008, 0 2px 10px #000a; }
  .pointer { position: absolute; top: -10px; left: 50%; width: 26px; transform-origin: 50% 12%; filter: drop-shadow(0 2px 3px #000a); }
</style>
