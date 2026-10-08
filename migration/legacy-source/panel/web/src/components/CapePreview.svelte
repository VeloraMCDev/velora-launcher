<script lang="ts">
  // Outer face of a cape texture (64×32 and HD multiples, or legacy 22×17).
  let { src, scale = 5 }: { src: string | null; scale?: number } = $props();
  let canvas = $state<HTMLCanvasElement>();

  $effect(() => {
    const el = canvas;
    const url = src;
    if (!el) return;
    const ctx = el.getContext('2d')!;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, el.width, el.height);
    if (!url) return;
    let cancelled = false;
    const img = new Image();
    img.onload = () => {
      if (cancelled) return;
      const u = img.width === 22 ? 1 : img.width / 64;
      ctx.drawImage(img, u, u, 10 * u, 16 * u, 0, 0, el.width, el.height);
    };
    img.src = url;
    return () => (cancelled = true);
  });
</script>

<canvas bind:this={canvas} width={10 * scale} height={16 * scale} aria-hidden="true"></canvas>

<style>
  canvas { image-rendering: pixelated; display: block; border-radius: 3px; background: rgba(255, 255, 255, 0.04); }
</style>
