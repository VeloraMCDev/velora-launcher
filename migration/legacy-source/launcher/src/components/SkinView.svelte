<script lang="ts">
  // Flat front/back render of a Minecraft skin (+ cape) on a canvas.
  let { skin, cape = null, slim = false, side = 'front', scale = 8 }: {
    skin: string | null; cape?: string | null; slim?: boolean; side?: 'front' | 'back'; scale?: number;
  } = $props();

  let canvas = $state<HTMLCanvasElement>();

  function load(src: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = reject;
      img.src = src;
    });
  }

  type Part = [sx: number, sy: number, w: number, h: number, dx: number, dy: number, mirror?: boolean];

  function parts(legacy: boolean): { base: Part[]; overlay: Part[] } {
    const aw = slim ? 3 : 4;
    const off = slim ? 1 : 0; // slim arms sit one pixel closer to the body
    if (side === 'front') {
      const base: Part[] = [
        [8, 8, 8, 8, 4, 0],
        [20, 20, 8, 12, 4, 8],
        [44, 20, aw, 12, off, 8],
        legacy ? [44, 20, aw, 12, 12, 8, true] : [36, 52, aw, 12, 12, 8],
        [4, 20, 4, 12, 4, 20],
        legacy ? [4, 20, 4, 12, 8, 20, true] : [20, 52, 4, 12, 8, 20],
      ];
      const overlay: Part[] = [[40, 8, 8, 8, 4, 0]];
      if (!legacy) overlay.push([20, 36, 8, 12, 4, 8], [44, 36, aw, 12, off, 8], [52, 52, aw, 12, 12, 8], [4, 36, 4, 12, 4, 20], [4, 52, 4, 12, 8, 20]);
      return { base, overlay };
    }
    const rb = slim ? 51 : 52; // back face of the right arm
    const lb = slim ? 43 : 44;
    const base: Part[] = [
      [24, 8, 8, 8, 4, 0],
      [32, 20, 8, 12, 4, 8],
      [rb, 20, aw, 12, 12, 8],
      legacy ? [rb, 20, aw, 12, off, 8, true] : [lb, 52, aw, 12, off, 8],
      [12, 20, 4, 12, 8, 20],
      legacy ? [12, 20, 4, 12, 4, 20, true] : [28, 52, 4, 12, 4, 20],
    ];
    const overlay: Part[] = [[56, 8, 8, 8, 4, 0]];
    if (!legacy) overlay.push([32, 36, 8, 12, 4, 8], [rb, 36, aw, 12, 12, 8], [lb + 16, 52, aw, 12, off, 8], [12, 36, 4, 12, 8, 20], [12, 52, 4, 12, 4, 20]);
    return { base, overlay };
  }

  function draw(ctx: CanvasRenderingContext2D, img: HTMLImageElement, [sx, sy, w, h, dx, dy, mirror]: Part) {
    if (mirror) {
      ctx.save();
      ctx.translate((dx + w) * scale, dy * scale);
      ctx.scale(-1, 1);
      ctx.drawImage(img, sx, sy, w, h, 0, 0, w * scale, h * scale);
      ctx.restore();
    } else {
      ctx.drawImage(img, sx, sy, w, h, dx * scale, dy * scale, w * scale, h * scale);
    }
  }

  $effect(() => {
    const el = canvas;
    const src = skin, capeSrc = cape, s = side, sl = slim;
    void s, sl;
    if (!el) return;
    const ctx = el.getContext('2d')!;
    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, el.width, el.height);
    let cancelled = false;
    (async () => {
      if (src) {
        const img = await load(src).catch(() => null);
        if (!img || cancelled) return;
        const { base, overlay } = parts(img.height === 32);
        base.forEach((p) => draw(ctx, img, p));
        overlay.forEach((p) => draw(ctx, img, p));
      } else {
        // Silhouette placeholder.
        ctx.fillStyle = 'rgba(255,255,255,0.08)';
        for (const [x, y, w, h] of [[4, 0, 8, 8], [4, 8, 8, 12], [0, 8, 4, 12], [12, 8, 4, 12], [4, 20, 4, 12], [8, 20, 4, 12]]) {
          ctx.fillRect(x * scale, y * scale, w * scale, h * scale);
        }
      }
      if (capeSrc && s === 'back') {
        const c = await load(capeSrc).catch(() => null);
        if (!c || cancelled) return;
        const u = c.width / 64; // HD capes
        ctx.drawImage(c, 1 * u, 1 * u, 10 * u, 16 * u, 3 * scale, 8 * scale, 10 * scale, 16 * scale);
      }
    })();
    return () => (cancelled = true);
  });
</script>

<canvas bind:this={canvas} width={16 * scale} height={32 * scale} aria-label="Skin {side}"></canvas>

<style>
  canvas { image-rendering: pixelated; display: block; }
</style>
