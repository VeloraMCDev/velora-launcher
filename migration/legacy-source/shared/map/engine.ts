import type { LivePlayer, MapBounds, MapClaim, MapDimension, MapInfo, MapLayers, MapOverlay, MapPin, MapSelection } from './types';
import { dimSlug } from './types';

/** A framework-free map renderer: pans and zooms the tile pyramid on a canvas and draws claims, pins and players on top. */

export interface Theme {
  background: string;
  grid: string;
  label: string;
  labelBg: string;
  accent: string;
  self: string;
}

const DARK: Theme = { background: '#0d1220', grid: 'rgba(255,255,255,0.045)', label: '#f4f6ff', labelBg: 'rgba(10,14,26,0.78)', accent: '#8b6cff', self: '#34d399' };

const PIN_COLORS: Record<string, string> = {
  spawn: '#fbbf24', warp: '#38bdf8', home: '#a78bfa', guild_home: '#fb7185', market: '#34d399', shop: '#f59e0b'
};

export interface MapViewOptions {
  tileUrl: (slug: string, zoom: number, x: number, y: number) => string;
  avatarUrl?: (uuid: string) => string;
  theme?: Partial<Theme>;
  selfUuid?: string | null;
  onselect?: (selection: MapSelection | null) => void;
  onblock?: (point: { dimension: string; x: number; z: number }) => void;
  onhover?: (hover: { cursor: { x: number; z: number } | null; label: string | null }) => void;
  onview?: (view: { x: number; z: number; scale: number }) => void;
}

interface Tile { img: HTMLImageElement; state: 'loading' | 'ok' | 'missing'; at: number; used: number }

const MIN_SCALE = 1 / 128;
const MAX_SCALE = 48;
const TILE = 256;
const MAX_LOADS = 10;

export class MapView {
  private ctx: CanvasRenderingContext2D;
  private theme: Theme;
  private info: MapInfo | null = null;
  private dimension: MapDimension | null = null;
  private overlay: MapOverlay = { players: [], claims: [], pins: [] };
  private layers: MapLayers = { claims: true, pins: true, players: true };
  private showSkins = true;
  private tiles = new Map<string, Tile>();
  private queue: string[] = [];
  private loading = 0;
  private icons = new Map<string, HTMLImageElement | 'failed'>();
  private paths = new Map<string, Path2D>();
  private width = 300;
  private height = 200;
  private dpr = 1;
  private cx = 0;
  private cz = 0;
  private scale = 1;
  private hoverId: string | null = null;
  private selected: MapSelection | null = null;
  private following: string | null = null;
  private raf = 0;
  private fly: { fromX: number; fromZ: number; fromS: number; toX: number; toZ: number; toS: number; start: number; ms: number } | null = null;
  private pointers = new Map<number, { x: number; y: number }>();
  private drag: { x: number; y: number; moved: number } | null = null;
  private pinchStart: { dist: number; scale: number } | null = null;
  private disposed = false;
  private resizeObserver: ResizeObserver;
  private texture: CanvasPattern | null = null;
  private hoverBlock: { x: number; z: number } | null = null;

  constructor(private canvas: HTMLCanvasElement, private opts: MapViewOptions) {
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('canvas is not available');
    this.ctx = ctx;
    this.theme = { ...DARK, ...(opts.theme ?? {}) };
    canvas.style.touchAction = 'none';
    canvas.addEventListener('pointerdown', this.down);
    canvas.addEventListener('pointermove', this.move);
    canvas.addEventListener('pointerup', this.up);
    canvas.addEventListener('pointercancel', this.up);
    canvas.addEventListener('pointerleave', this.leave);
    canvas.addEventListener('wheel', this.wheel, { passive: false });
    canvas.addEventListener('dblclick', this.dbl);
    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(canvas);
    this.resize();
  }

  // ---- public API --------------------------------------------------------------------------------------

  setInfo(info: MapInfo) {
    const tokenChanged = this.info?.token !== info.token || this.info?.tile_base !== info.tile_base;
    this.info = info;
    if (tokenChanged) {
      // New tile key: tiles that failed earlier may work now; loaded ones stay.
      for (const [k, t] of this.tiles) if (t.state !== 'ok') this.tiles.delete(k);
    }
    const keep = this.dimension ? info.dimensions.find((d) => d.slug === this.dimension!.slug) : undefined;
    this.dimension = keep ?? info.dimensions[0] ?? null;
    if (!keep && this.dimension) this.frameDimension();
    this.invalidate();
  }

  setOverlay(overlay: MapOverlay) {
    this.overlay = overlay;
    this.paths.clear();
    if (this.following) {
      const p = overlay.players.find((q) => q.uuid === this.following);
      if (p) {
        if (dimSlug(p.dimension) !== this.dimension?.slug) this.setDimension(p.dimension);
        this.fly = { fromX: this.cx, fromZ: this.cz, fromS: this.scale, toX: p.x, toZ: p.z, toS: this.scale, start: performance.now(), ms: 700 };
      }
    }
    if (this.selected?.type === 'player') {
      const fresh = overlay.players.find((p) => p.uuid === (this.selected as { player: LivePlayer }).player.uuid);
      if (fresh) this.selected = { type: 'player', player: fresh };
    }
    this.invalidate();
  }

  setLayers(layers: MapLayers) { this.layers = { ...layers }; this.invalidate(); }
  setSkins(show: boolean) { this.showSkins = show; this.invalidate(); }

  setSelf(uuid: string | null | undefined) { this.opts.selfUuid = uuid ?? null; this.invalidate(); }

  /** Switch dimension by id (`minecraft:the_nether`) or slug. */
  setDimension(idOrSlug: string) {
    const slug = dimSlug(idOrSlug);
    const next = this.info?.dimensions.find((d) => d.slug === slug);
    if (!next || next.slug === this.dimension?.slug) return;
    this.dimension = next;
    this.select(null);
    this.frameDimension();
    this.invalidate();
  }

  get currentDimension(): MapDimension | null { return this.dimension; }

  follow(uuid: string | null) { this.following = uuid; if (uuid) this.invalidate(); }

  flyTo(x: number, z: number, scale?: number) {
    this.following = null;
    this.fly = { fromX: this.cx, fromZ: this.cz, fromS: this.scale, toX: x, toZ: z, toS: clamp(scale ?? this.scale, MIN_SCALE, MAX_SCALE), start: performance.now(), ms: 520 };
    this.invalidate();
  }

  zoomBy(factor: number) {
    this.fly = null;
    this.zoomAround(this.width / 2, this.height / 2, factor, true);
  }

  select(selection: MapSelection | null) {
    this.selected = selection;
    this.opts.onselect?.(selection);
    this.invalidate();
  }

  frameDimension() {
    const d = this.dimension;
    // Frame the populated heart of the map: a single far-off stray region must not drag the first view into empty space.
    const b = d && d.tiles >= 8 && d.core ? d.core : d?.bounds;
    if (!b) return;
    this.fly = null;
    const [minX, maxX, minZ, maxZ] = worldBounds(b);
    this.cx = (minX + maxX) / 2;
    this.cz = (minZ + maxZ) / 2;
    const fit = Math.min(this.width / Math.max(1, maxX - minX), this.height / Math.max(1, maxZ - minZ)) * 0.9;
    this.scale = clamp(fit, 0.05, 2);
    this.emitView();
  }

  pinsOfKind(kind: string): MapPin[] { return this.overlay.pins.filter((p) => p.kind === kind && dimSlug(p.dimension) === this.dimension?.slug); }

  getView() { return { x: this.cx, z: this.cz, scale: this.scale }; }

  destroy() {
    this.disposed = true;
    cancelAnimationFrame(this.raf);
    this.resizeObserver.disconnect();
    const c = this.canvas;
    c.removeEventListener('pointerdown', this.down);
    c.removeEventListener('pointermove', this.move);
    c.removeEventListener('pointerup', this.up);
    c.removeEventListener('pointercancel', this.up);
    c.removeEventListener('pointerleave', this.leave);
    c.removeEventListener('wheel', this.wheel);
    c.removeEventListener('dblclick', this.dbl);
    this.tiles.clear();
  }

  // ---- geometry ---------------------------------------------------------------------------------------

  private resize() {
    const rect = this.canvas.getBoundingClientRect();
    this.dpr = Math.min(window.devicePixelRatio || 1, 2.5);
    this.width = Math.max(50, rect.width);
    this.height = Math.max(50, rect.height);
    this.canvas.width = Math.round(this.width * this.dpr);
    this.canvas.height = Math.round(this.height * this.dpr);
    this.invalidate();
  }

  private toScreen(x: number, z: number) { return { x: (x - this.cx) * this.scale + this.width / 2, y: (z - this.cz) * this.scale + this.height / 2 }; }
  private toWorld(sx: number, sy: number) { return { x: (sx - this.width / 2) / this.scale + this.cx, z: (sy - this.height / 2) / this.scale + this.cz }; }

  private zoomAround(sx: number, sy: number, factor: number, animate = false) {
    const before = this.toWorld(sx, sy);
    const target = clamp(this.scale * factor, MIN_SCALE, MAX_SCALE);
    if (animate) {
      // Keep the point under (sx, sy) fixed while easing the scale.
      const toX = before.x - (sx - this.width / 2) / target;
      const toZ = before.z - (sy - this.height / 2) / target;
      this.fly = { fromX: this.cx, fromZ: this.cz, fromS: this.scale, toX, toZ, toS: target, start: performance.now(), ms: 220 };
    } else {
      this.scale = target;
      this.cx = before.x - (sx - this.width / 2) / this.scale;
      this.cz = before.z - (sy - this.height / 2) / this.scale;
    }
    this.following = null;
    this.invalidate();
  }

  // ---- input --------------------------------------------------------------------------------------------

  private localPoint(e: PointerEvent | WheelEvent | MouseEvent) {
    const r = this.canvas.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  private down = (e: PointerEvent) => {
    this.canvas.setPointerCapture(e.pointerId);
    const p = this.localPoint(e);
    this.pointers.set(e.pointerId, p);
    this.fly = null;
    if (this.pointers.size === 1) this.drag = { x: p.x, y: p.y, moved: 0 };
    if (this.pointers.size === 2) {
      const [a, b] = [...this.pointers.values()];
      this.pinchStart = { dist: Math.hypot(a.x - b.x, a.y - b.y), scale: this.scale };
      this.drag = null;
    }
  };

  private move = (e: PointerEvent) => {
    const p = this.localPoint(e);
    if (this.pointers.has(e.pointerId)) this.pointers.set(e.pointerId, p);
    if (this.pinchStart && this.pointers.size === 2) {
      const [a, b] = [...this.pointers.values()];
      const dist = Math.hypot(a.x - b.x, a.y - b.y);
      this.zoomAround((a.x + b.x) / 2, (a.y + b.y) / 2, (this.pinchStart.scale * dist / this.pinchStart.dist) / this.scale);
      return;
    }
    if (this.drag && this.pointers.has(e.pointerId)) {
      const dx = p.x - this.drag.x, dy = p.y - this.drag.y;
      this.drag.moved += Math.abs(dx) + Math.abs(dy);
      this.drag.x = p.x; this.drag.y = p.y;
      if (this.drag.moved > 3) {
        this.cx -= dx / this.scale;
        this.cz -= dy / this.scale;
        this.following = null;
        this.canvas.style.cursor = 'grabbing';
        this.invalidate();
        this.emitView();
      }
      return;
    }
    const hit = this.hitTest(p.x, p.y);
    const id = hit ? hitId(hit) : null;
    if (id !== this.hoverId) { this.hoverId = id; this.invalidate(); }
    this.canvas.style.cursor = hit ? 'pointer' : 'grab';
    const w = this.toWorld(p.x, p.y);
    const block = { x: Math.floor(w.x), z: Math.floor(w.z) };
    if (this.scale >= 8 && (!this.hoverBlock || this.hoverBlock.x !== block.x || this.hoverBlock.z !== block.z)) { this.hoverBlock = block; this.invalidate(); }
    this.opts.onhover?.({ cursor: { x: w.x, z: w.z }, label: hit ? hitLabel(hit) : null });
  };

  private up = (e: PointerEvent) => {
    const wasClick = this.drag && this.drag.moved <= 3 && this.pointers.size === 1;
    this.pointers.delete(e.pointerId);
    if (this.pointers.size < 2) this.pinchStart = null;
    if (wasClick) {
      const p = this.localPoint(e);
      const world = this.toWorld(p.x, p.y);
      if (this.currentDimension) this.opts.onblock?.({ dimension: this.currentDimension.id, x: Math.floor(world.x), z: Math.floor(world.z) });
      const hit = this.hitTest(p.x, p.y);
      this.select(hit);
    }
    if (this.pointers.size === 0) { this.drag = null; this.canvas.style.cursor = 'grab'; }
  };

  private leave = () => {
    if (this.hoverId || this.hoverBlock) { this.hoverId = null; this.hoverBlock = null; this.invalidate(); }
    this.opts.onhover?.({ cursor: null, label: null });
  };

  private wheel = (e: WheelEvent) => {
    e.preventDefault();
    const p = this.localPoint(e);
    const delta = e.deltaMode === 1 ? e.deltaY * 18 : e.deltaY;
    this.fly = null;
    this.zoomAround(p.x, p.y, Math.exp(-delta * 0.0016));
    this.emitView();
  };

  private dbl = (e: MouseEvent) => {
    const p = this.localPoint(e);
    if (this.hitTest(p.x, p.y)) return;
    this.zoomAround(p.x, p.y, 2, true);
  };

  private emitView() { this.opts.onview?.({ x: this.cx, z: this.cz, scale: this.scale }); }

  // ---- hit testing --------------------------------------------------------------------------------------

  private visibleSlug() { return this.dimension?.slug ?? ''; }

  private hitTest(sx: number, sy: number): MapSelection | null {
    const slug = this.visibleSlug();
    if (this.layers.players) {
      let best: LivePlayer | null = null, bestD = 15 * 15;
      for (const p of this.overlay.players) {
        if (dimSlug(p.dimension) !== slug) continue;
        const s = this.toScreen(p.x, p.z);
        const d = (s.x - sx) ** 2 + (s.y - sy) ** 2;
        if (d < bestD) { best = p; bestD = d; }
      }
      if (best) return { type: 'player', player: best };
    }
    if (this.layers.pins) {
      let best: MapPin | null = null, bestD = 15 * 15;
      for (const p of this.overlay.pins) {
        if (dimSlug(p.dimension) !== slug) continue;
        const s = this.toScreen(p.x, p.z);
        const d = (s.x - sx) ** 2 + (s.y - sy) ** 2;
        if (d < bestD) { best = p; bestD = d; }
      }
      if (best) return { type: 'pin', pin: best };
    }
    if (this.layers.claims) {
      const w = this.toWorld(sx, sy);
      for (const c of this.overlay.claims) {
        if (dimSlug(c.dimension) !== slug) continue;
        if (inside(c.outer, w.x, w.z) && !c.holes.some((h) => inside(h, w.x, w.z))) return { type: 'claim', claim: c };
      }
    }
    return null;
  }

  // ---- tiles --------------------------------------------------------------------------------------------

  private tileKey(slug: string, z: number, x: number, y: number) { return `${slug}/${z}/${x}/${y}`; }

  private tileAt(slug: string, z: number, x: number, y: number, load: boolean): Tile | null {
    const key = this.tileKey(slug, z, x, y);
    const t = this.tiles.get(key);
    if (t) {
      t.used = performance.now();
      if (t.state === 'missing' && load && performance.now() - t.at > 20000) { this.tiles.delete(key); return this.tileAt(slug, z, x, y, load); }
      return t;
    }
    if (!load || !this.info?.token) return null;
    const img = new Image();
    img.decoding = 'async';
    const tile: Tile = { img, state: 'loading', at: performance.now(), used: performance.now() };
    this.tiles.set(key, tile);
    this.queue.push(key);
    this.pump();
    this.evict();
    return tile;
  }

  private pump() {
    while (this.loading < MAX_LOADS && this.queue.length) {
      const key = this.queue.pop()!; // newest first: what is on screen now matters most
      const tile = this.tiles.get(key);
      if (!tile || tile.state !== 'loading') continue;
      const [slug, z, x, y] = key.split('/');
      this.loading++;
      tile.img.onload = () => { tile.state = 'ok'; this.loading--; this.pump(); this.invalidate(); };
      tile.img.onerror = () => { tile.state = 'missing'; tile.at = performance.now(); this.loading--; this.pump(); this.invalidate(); };
      tile.img.src = this.opts.tileUrl(slug, Number(z), Number(x), Number(y));
    }
  }

  private evict() {
    if (this.tiles.size <= 900) return;
    const old = [...this.tiles.entries()].sort((a, b) => a[1].used - b[1].used).slice(0, 250);
    for (const [k] of old) this.tiles.delete(k);
  }

  private inBounds(z: number, x: number, y: number): boolean {
    const b = this.dimension?.bounds;
    if (!b) return true;
    const span = 2 ** z;
    return x * span <= b.max_x && (x + 1) * span - 1 >= b.min_x && y * span <= b.max_y && (y + 1) * span - 1 >= b.min_y;
  }

  // ---- drawing ------------------------------------------------------------------------------------------

  invalidate() {
    if (this.disposed || this.raf) return;
    this.raf = requestAnimationFrame((t) => { this.raf = 0; this.frame(t); });
  }

  private frame(now: number) {
    if (this.fly) {
      const f = this.fly;
      const k = Math.min(1, (now - f.start) / f.ms);
      const e = 1 - (1 - k) ** 3;
      this.scale = f.fromS * (f.toS / f.fromS) ** e;
      this.cx = f.fromX + (f.toX - f.fromX) * e;
      this.cz = f.fromZ + (f.toZ - f.fromZ) * e;
      if (k >= 1) this.fly = null;
      this.emitView();
      this.invalidate();
    }
    this.draw();
  }

  private draw() {
    const ctx = this.ctx;
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    ctx.fillStyle = this.theme.background;
    ctx.fillRect(0, 0, this.width, this.height);
    if (!this.dimension || !this.info) return;
    this.drawTiles(ctx);
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    if (this.scale >= 6) this.drawBlocks(ctx);
    if (this.layers.claims) this.drawClaims(ctx);
    if (this.layers.pins) this.drawPins(ctx);
    if (this.layers.players) this.drawPlayers(ctx);
  }

  private drawTiles(ctx: CanvasRenderingContext2D) {
    const slug = this.dimension!.slug;
    const maxZoom = this.info!.max_zoom;
    const zoom = this.usableZoom(clamp(Math.floor(Math.log2(1 / this.scale)), 0, maxZoom));
    const span = TILE * 2 ** zoom; // world blocks per tile
    const px = span * this.scale;  // screen pixels per tile
    const tl = this.toWorld(0, 0), br = this.toWorld(this.width, this.height);
    const x0 = Math.floor(tl.x / span), x1 = Math.floor(br.x / span), y0 = Math.floor(tl.z / span), y1 = Math.floor(br.z / span);
    // Zoomed far out on a level that was never built, thousands of tiles would be in view: draw what is cached, ask for nothing.
    const crowded = (x1 - x0 + 1) * (y1 - y0 + 1) > 1200;
    ctx.imageSmoothingEnabled = this.scale < 1;
    ctx.imageSmoothingQuality = 'high';
    // A faint chunk grid when close enough to matter.
    if (this.scale >= 3 && this.scale < 6) this.drawGrid(ctx);
    for (let ty = y0; ty <= y1; ty++) {
      for (let tx = x0; tx <= x1; tx++) {
        if (!this.inBounds(zoom, tx, ty)) continue;
        const dx = Math.floor((tx * span - this.cx) * this.scale + this.width / 2);
        const dy = Math.floor((ty * span - this.cz) * this.scale + this.height / 2);
        const dsize = Math.ceil(px) + 1; // overlap by a pixel so seams never show
        const tile = this.tileAt(slug, zoom, tx, ty, !crowded);
        if (tile?.state === 'ok') { ctx.drawImage(tile.img, dx, dy, dsize, dsize); continue; }
        // Missing at this level (never uploaded, or not built yet): sharper tiles underneath may still exist.
        if (tile?.state === 'missing' && zoom > 0 && this.drawFromChildren(ctx, slug, zoom, tx, ty, dx, dy, dsize)) continue;
        // Not here yet: show the blurry parent underneath, then the real one replaces it.
        for (let up = 1; zoom + up <= maxZoom; up++) {
          const parent = this.tileAt(slug, zoom + up, tx >> up, ty >> up, tile?.state === 'loading' && up === 1);
          if (parent?.state === 'ok') {
            const part = TILE / 2 ** up;
            const sx = (tx & (2 ** up - 1)) * part, sy = (ty & (2 ** up - 1)) * part;
            ctx.drawImage(parent.img, sx, sy, part, part, dx, dy, dsize, dsize);
            break;
          }
        }
      }
    }
  }

  /** The deepest zoom level at or below `wanted` that actually holds tiles (the panel reports how many each level has). */
  private usableZoom(wanted: number): number {
    const levels = this.dimension?.levels;
    if (!levels?.length) return wanted;
    for (let z = Math.min(wanted, levels.length - 1); z > 0; z--) if (levels[z] > 0) return z;
    return 0;
  }

  /** Paints a coarse tile out of its four finer tiles. True if at least one of them was ready. */
  private drawFromChildren(ctx: CanvasRenderingContext2D, slug: string, zoom: number, tx: number, ty: number, dx: number, dy: number, dsize: number): boolean {
    let drew = false;
    const half = dsize / 2;
    for (let j = 0; j < 2; j++) {
      for (let i = 0; i < 2; i++) {
        const cx = tx * 2 + i, cy = ty * 2 + j;
        if (!this.inBounds(zoom - 1, cx, cy)) continue;
        const child = this.tileAt(slug, zoom - 1, cx, cy, true);
        if (child?.state === 'ok') {
          ctx.drawImage(child.img, dx + i * half, dy + j * half, Math.ceil(half) + 1, Math.ceil(half) + 1);
          drew = true;
        }
      }
    }
    return drew;
  }

  /**
   * Close up, every block is its own cell: a fine texture, a light and a dark edge, stronger chunk lines, and an outline on the
   * block under the cursor. The tiles only know each block's colour, so the texture is drawn on top rather than stored.
   */
  private drawBlocks(ctx: CanvasRenderingContext2D) {
    const strength = clamp((this.scale - 6) / 10, 0, 1);
    if (!this.texture) this.texture = makeTexture(ctx);
    if (this.texture) {
      ctx.save();
      ctx.globalAlpha = 0.55 * strength + 0.25;
      ctx.globalCompositeOperation = 'overlay';
      const k = this.scale / 16 * this.dpr;
      // Pattern covers 8 x 8 blocks (128 px); align it to the world so blocks stay put while panning.
      this.texture.setTransform(new DOMMatrix([k, 0, 0, k, this.dpr * (this.width / 2 - this.cx * this.scale), this.dpr * (this.height / 2 - this.cz * this.scale)]));
      ctx.fillStyle = this.texture;
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
      ctx.restore();
    }
    ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    // Chunk borders every 16 blocks.
    const a = this.toWorld(0, 0);
    ctx.strokeStyle = 'rgba(255,255,255,0.34)';
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    for (let x = Math.floor(a.x / 16) * 16; (x - this.cx) * this.scale + this.width / 2 < this.width + 20; x += 16) {
      const sx = Math.round((x - this.cx) * this.scale + this.width / 2) + 0.5;
      ctx.moveTo(sx, 0); ctx.lineTo(sx, this.height);
    }
    for (let z = Math.floor(a.z / 16) * 16; (z - this.cz) * this.scale + this.height / 2 < this.height + 20; z += 16) {
      const sy = Math.round((z - this.cz) * this.scale + this.height / 2) + 0.5;
      ctx.moveTo(0, sy); ctx.lineTo(this.width, sy);
    }
    ctx.stroke();
    if (this.hoverBlock && this.scale >= 8) {
      const s = this.toScreen(this.hoverBlock.x, this.hoverBlock.z);
      ctx.lineWidth = 2;
      ctx.strokeStyle = '#ffffff';
      ctx.strokeRect(s.x + 1, s.y + 1, this.scale - 2, this.scale - 2);
      ctx.lineWidth = 1;
      ctx.strokeStyle = 'rgba(0,0,0,0.6)';
      ctx.strokeRect(s.x - 0.5, s.y - 0.5, this.scale + 1, this.scale + 1);
    }
  }

  private drawGrid(ctx: CanvasRenderingContext2D) {
    const step = 16 * this.scale;
    const a = this.toWorld(0, 0);
    ctx.strokeStyle = this.theme.grid;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let x = Math.floor(a.x / 16) * 16; (x - this.cx) * this.scale + this.width / 2 < this.width + step; x += 16) {
      const sx = Math.round((x - this.cx) * this.scale + this.width / 2) + 0.5;
      ctx.moveTo(sx, 0); ctx.lineTo(sx, this.height);
    }
    for (let z = Math.floor(a.z / 16) * 16; (z - this.cz) * this.scale + this.height / 2 < this.height + step; z += 16) {
      const sy = Math.round((z - this.cz) * this.scale + this.height / 2) + 0.5;
      ctx.moveTo(0, sy); ctx.lineTo(this.width, sy);
    }
    ctx.stroke();
  }

  private claimPath(c: MapClaim): Path2D {
    let p = this.paths.get(c.id);
    if (!p) {
      p = new Path2D();
      for (const ring of [c.outer, ...c.holes]) {
        ring.forEach(([x, z], i) => (i ? p!.lineTo(x, z) : p!.moveTo(x, z)));
        p.closePath();
      }
      this.paths.set(c.id, p);
    }
    return p;
  }

  private drawClaims(ctx: CanvasRenderingContext2D) {
    const slug = this.visibleSlug();
    const tl = this.toWorld(0, 0), br = this.toWorld(this.width, this.height);
    const picked = this.selected?.type === 'claim' ? this.selected.claim.id : null;
    for (const c of this.overlay.claims) {
      if (dimSlug(c.dimension) !== slug || !c.outer.length) continue;
      const xs = c.outer.map((p) => p[0]), zs = c.outer.map((p) => p[1]);
      if (Math.max(...xs) < tl.x || Math.min(...xs) > br.x || Math.max(...zs) < tl.z || Math.min(...zs) > br.z) continue;
      const hot = c.id === this.hoverId || c.id === picked;
      ctx.save();
      ctx.setTransform(this.dpr * this.scale, 0, 0, this.dpr * this.scale, this.dpr * (this.width / 2 - this.cx * this.scale), this.dpr * (this.height / 2 - this.cz * this.scale));
      const path = this.claimPath(c);
      ctx.fillStyle = withAlpha(c.color, hot ? 0.46 : 0.28);
      ctx.fill(path, 'evenodd');
      ctx.lineJoin = 'round';
      ctx.lineWidth = (hot ? 3 : 2) / this.scale;
      ctx.strokeStyle = withAlpha(c.color, 0.95);
      ctx.stroke(path);
      ctx.restore();
    }
    // Labels on top of all shapes, once they are big enough to read.
    for (const c of this.overlay.claims) {
      if (dimSlug(c.dimension) !== slug || !c.outer.length) continue;
      const xs = c.outer.map((p) => p[0]), zs = c.outer.map((p) => p[1]);
      const w = (Math.max(...xs) - Math.min(...xs)) * this.scale, h = (Math.max(...zs) - Math.min(...zs)) * this.scale;
      if (w < 46 || h < 30) continue;
      const s = this.toScreen(c.label_x, c.label_z);
      if (s.x < -80 || s.y < -40 || s.x > this.width + 80 || s.y > this.height + 40) continue;
      const showName = w > 150 && h > 50;
      this.chip(ctx, s.x, s.y, showName ? `${c.tag ? `[${c.tag}] ` : ''}${c.name}` : c.tag || c.name, c.color, c.icon);
    }
  }

  private drawPins(ctx: CanvasRenderingContext2D) {
    const slug = this.visibleSlug();
    const picked = this.selected?.type === 'pin' ? this.selected.pin.id : null;
    for (const p of this.overlay.pins) {
      if (dimSlug(p.dimension) !== slug) continue;
      const s = this.toScreen(p.x, p.z);
      if (s.x < -30 || s.y < -30 || s.x > this.width + 30 || s.y > this.height + 30) continue;
      const hot = p.id === this.hoverId || p.id === picked;
      const color = PIN_COLORS[p.kind] ?? '#94a3b8';
      const r = hot ? 14 : 12;
      ctx.save();
      ctx.shadowColor = 'rgba(0,0,0,0.5)'; ctx.shadowBlur = 8; ctx.shadowOffsetY = 2;
      ctx.beginPath(); ctx.arc(s.x, s.y, r, 0, Math.PI * 2);
      ctx.fillStyle = color; ctx.fill();
      ctx.restore();
      ctx.lineWidth = 2; ctx.strokeStyle = '#fff'; ctx.stroke();
      drawGlyph(ctx, p.kind, s.x, s.y, r * 0.58);
      if (hot || this.scale >= 0.9) this.tag(ctx, s.x, s.y + r + 9, p.label);
    }
  }

  private drawPlayers(ctx: CanvasRenderingContext2D) {
    const slug = this.visibleSlug();
    const picked = this.selected?.type === 'player' ? this.selected.player.uuid : null;
    for (const p of this.overlay.players) {
      if (dimSlug(p.dimension) !== slug) continue;
      const s = this.toScreen(p.x, p.z);
      if (s.x < -30 || s.y < -30 || s.x > this.width + 30 || s.y > this.height + 30) continue;
      const me = !!this.opts.selfUuid && p.uuid.toLowerCase() === this.opts.selfUuid.toLowerCase();
      const hot = p.uuid === this.hoverId || p.uuid === picked;
      const ring = me ? this.theme.self : this.theme.accent;
      const r = hot ? 14 : 12;
      // facing direction: Minecraft yaw 0 = south (+z), 90 = west (-x)
      const a = (p.yaw * Math.PI) / 180;
      const fx = -Math.sin(a), fz = Math.cos(a);
      ctx.save();
      ctx.translate(s.x, s.y);
      ctx.rotate(Math.atan2(fz, fx));
      ctx.beginPath(); ctx.moveTo(r + 9, 0); ctx.lineTo(r + 1, -5); ctx.lineTo(r + 1, 5); ctx.closePath();
      ctx.fillStyle = ring; ctx.fill();
      ctx.restore();
      ctx.save();
      ctx.shadowColor = 'rgba(0,0,0,0.55)'; ctx.shadowBlur = 8; ctx.shadowOffsetY = 2;
      ctx.beginPath(); ctx.arc(s.x, s.y, r + 2, 0, Math.PI * 2);
      ctx.fillStyle = ring; ctx.fill();
      ctx.restore();
      const img = this.showSkins && this.opts.avatarUrl ? this.icon(this.opts.avatarUrl(p.uuid)) : null;
      ctx.save();
      ctx.beginPath(); ctx.arc(s.x, s.y, r, 0, Math.PI * 2); ctx.clip();
      if (img) { ctx.imageSmoothingEnabled = false; ctx.drawImage(img, s.x - r, s.y - r, r * 2, r * 2); }
      else { ctx.fillStyle = '#1e293b'; ctx.fillRect(s.x - r, s.y - r, r * 2, r * 2); ctx.fillStyle = '#fff'; ctx.font = '700 12px system-ui, sans-serif'; ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.fillText((p.name[0] ?? '?').toUpperCase(), s.x, s.y + 1); }
      ctx.restore();
      this.tag(ctx, s.x, s.y - r - 12, p.name, me ? this.theme.self : undefined);
    }
  }

  private icon(url: string): HTMLImageElement | null {
    if (!url) return null;
    const got = this.icons.get(url);
    if (got === 'failed') return null;
    if (got) return got.complete && got.naturalWidth ? got : null;
    const img = new Image();
    img.onload = () => this.invalidate();
    img.onerror = () => this.icons.set(url, 'failed');
    img.src = url;
    this.icons.set(url, img);
    return null;
  }

  /** A small rounded label with an optional round icon, centred on (x, y). */
  private chip(ctx: CanvasRenderingContext2D, x: number, y: number, text: string, color: string, iconUrl: string) {
    ctx.font = '700 12px system-ui, -apple-system, Segoe UI, sans-serif';
    const img = iconUrl ? this.icon(iconUrl) : null;
    const iconW = img ? 20 : 0;
    const w = ctx.measureText(text).width + 18 + iconW, h = 24;
    ctx.save();
    ctx.shadowColor = 'rgba(0,0,0,0.45)'; ctx.shadowBlur = 8; ctx.shadowOffsetY = 2;
    roundRect(ctx, x - w / 2, y - h / 2, w, h, 12);
    ctx.fillStyle = this.theme.labelBg; ctx.fill();
    ctx.restore();
    ctx.lineWidth = 1.5; ctx.strokeStyle = withAlpha(color, 0.9); ctx.stroke();
    let left = x - w / 2 + 9;
    if (img) {
      ctx.save(); ctx.beginPath(); ctx.arc(left + 8, y, 8, 0, Math.PI * 2); ctx.clip(); ctx.drawImage(img, left, y - 8, 16, 16); ctx.restore();
      left += 20;
    }
    ctx.fillStyle = this.theme.label; ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
    ctx.fillText(text, left, y + 0.5);
  }

  private tag(ctx: CanvasRenderingContext2D, x: number, y: number, text: string, color?: string) {
    ctx.font = '600 11px system-ui, -apple-system, Segoe UI, sans-serif';
    const w = ctx.measureText(text).width + 12;
    roundRect(ctx, x - w / 2, y - 9, w, 18, 9);
    ctx.fillStyle = this.theme.labelBg; ctx.fill();
    if (color) { ctx.lineWidth = 1; ctx.strokeStyle = color; ctx.stroke(); }
    ctx.fillStyle = this.theme.label; ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
    ctx.fillText(text, x, y + 0.5);
  }
}

// ---- helpers --------------------------------------------------------------------------------------------

function clamp(v: number, lo: number, hi: number) { return Math.max(lo, Math.min(hi, v)); }

/** Zoom-0 tile bounds -> world block bounds [minX, maxX, minZ, maxZ]. */
function worldBounds(b: MapBounds): [number, number, number, number] {
  return [b.min_x * TILE, (b.max_x + 1) * TILE, b.min_y * TILE, (b.max_y + 1) * TILE];
}

function hitId(s: MapSelection): string { return s.type === 'player' ? s.player.uuid : s.type === 'pin' ? s.pin.id : s.claim.id; }
function hitLabel(s: MapSelection): string { return s.type === 'player' ? s.player.name : s.type === 'pin' ? s.pin.label : `${s.claim.tag ? `[${s.claim.tag}] ` : ''}${s.claim.name}`; }

function inside(ring: [number, number][], x: number, z: number): boolean {
  let in_ = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [ax, az] = ring[i], [bx, bz] = ring[j];
    if ((az > z) !== (bz > z) && x < ((bx - ax) * (z - az)) / (bz - az) + ax) in_ = !in_;
  }
  return in_;
}

export function withAlpha(color: string, alpha: number): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(color.trim());
  if (!m) return color;
  const n = parseInt(m[1], 16);
  return `rgba(${n >> 16}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

/** A tiny white symbol for each kind of pin, drawn as shapes so no icon font is needed. */
function drawGlyph(ctx: CanvasRenderingContext2D, kind: string, x: number, y: number, r: number) {
  ctx.save();
  ctx.translate(x, y);
  ctx.fillStyle = '#fff';
  ctx.strokeStyle = '#fff';
  ctx.beginPath();
  switch (kind) {
    case 'spawn': // star
      for (let i = 0; i < 10; i++) { const a = -Math.PI / 2 + (i * Math.PI) / 5, rr = i % 2 ? r * 0.45 : r * 1.15; ctx.lineTo(Math.cos(a) * rr, Math.sin(a) * rr); }
      ctx.closePath(); ctx.fill();
      break;
    case 'warp': // diamond with a hole
      ctx.moveTo(0, -r * 1.2); ctx.lineTo(r * 0.95, 0); ctx.lineTo(0, r * 1.2); ctx.lineTo(-r * 0.95, 0); ctx.closePath(); ctx.fill();
      break;
    case 'home': // house
      ctx.moveTo(-r * 1.1, 0); ctx.lineTo(0, -r * 1.1); ctx.lineTo(r * 1.1, 0); ctx.lineTo(r * 0.7, 0); ctx.lineTo(r * 0.7, r); ctx.lineTo(-r * 0.7, r); ctx.lineTo(-r * 0.7, 0); ctx.closePath(); ctx.fill();
      break;
    case 'guild_home': // flag
      ctx.fillRect(-r * 0.8, -r * 1.1, r * 0.28, r * 2.2);
      ctx.moveTo(-r * 0.52, -r * 1.1); ctx.lineTo(r * 1.0, -r * 0.5); ctx.lineTo(-r * 0.52, 0.1 * r); ctx.closePath(); ctx.fill();
      break;
    case 'market': // $
      ctx.font = `800 ${Math.round(r * 2.3)}px system-ui, sans-serif`; ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.fillText('$', 0, r * 0.1);
      break;
    default: // shop: a bag
      ctx.moveTo(-r * 0.9, -r * 0.3); ctx.lineTo(r * 0.9, -r * 0.3); ctx.lineTo(r * 0.7, r * 1.0); ctx.lineTo(-r * 0.7, r * 1.0); ctx.closePath(); ctx.fill();
      ctx.lineWidth = Math.max(1.2, r * 0.22); ctx.beginPath(); ctx.arc(0, -r * 0.3, r * 0.5, Math.PI, 0); ctx.stroke();
  }
  ctx.restore();
}

/**
 * An 8 x 8 block texture sheet (16 px per block): fine noise that differs block to block, a light top-left edge and a dark
 * bottom-right edge, as grey values for the "overlay" blend (mid grey = no change).
 */
function makeTexture(ctx: CanvasRenderingContext2D): CanvasPattern | null {
  const c = document.createElement('canvas');
  c.width = c.height = 128;
  const g = c.getContext('2d');
  if (!g) return null;
  g.fillStyle = '#808080';
  g.fillRect(0, 0, 128, 128);
  let seed = 1234567;
  const rnd = () => { seed = (seed * 1664525 + 1013904223) >>> 0; return seed / 4294967296; };
  for (let by = 0; by < 8; by++) {
    for (let bx = 0; bx < 8; bx++) {
      const ox = bx * 16, oy = by * 16;
      const base = 120 + Math.floor(rnd() * 16);           // each block a touch lighter or darker
      g.fillStyle = `rgb(${base},${base},${base})`;
      g.fillRect(ox, oy, 16, 16);
      for (let i = 0; i < 4; i++) {                          // 4 x 4 chunky speckles, like a texture
        for (let j = 0; j < 4; j++) {
          const v = base + Math.floor((rnd() - 0.5) * 34);
          g.fillStyle = `rgb(${v},${v},${v})`;
          g.fillRect(ox + i * 4, oy + j * 4, 4, 4);
        }
      }
      g.fillStyle = 'rgba(255,255,255,0.55)'; g.fillRect(ox, oy, 16, 1); g.fillRect(ox, oy, 1, 16);
      g.fillStyle = 'rgba(0,0,0,0.6)'; g.fillRect(ox, oy + 15, 16, 1); g.fillRect(ox + 15, oy, 1, 16);
    }
  }
  return ctx.createPattern(c, 'repeat');
}
