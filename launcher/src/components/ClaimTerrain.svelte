<script lang="ts">
  import { onMount } from 'svelte';
  import { Eraser, Hand, Landmark, Minus, Plus, SquarePlus } from '@lucide/svelte';
  import { invoke } from '../lib/tauri';
  import type { GuildClaim } from '../lib/types';

  type Point = { x: number; z: number };
  type Mode = 'move' | 'claim' | 'unclaim';
  type Bounds = { minX: number; maxX: number; minZ: number; maxZ: number };

  let { serverId, dimension, centerX, centerZ, claims, myLand = [], myGuildId, player, focus = null, onpaint, oncenterchange }: {
    serverId: number; dimension: string; centerX: number; centerZ: number;
    claims: GuildClaim[]; myLand?: GuildClaim[]; myGuildId: string | null; player: { x: number; z: number; dimension: string } | null;
    /** Frames a rectangle of chunks when `nonce` changes (the "my land" button). */
    focus?: (Bounds & { nonce: number }) | null;
    onpaint: (chunks: Point[], mode: 'claim' | 'unclaim') => void;
    oncenterchange: (x: number, z: number) => void;
  } = $props();

  const TILE = 256;
  const MIN_PPB = 0.2, MAX_PPB = 8;
  /** Below this zoom a chunk is too small to hit reliably, so the map only moves. */
  const PAINT_MIN_PPB = 1.5;

  let host: HTMLDivElement;
  let width = $state(800), height = $state(480);
  let pixelsPerBlock = $state(3);
  let worldX = $state(8), worldZ = $state(8);
  let info = $state<{ ready: boolean; message: string; tile_base: string; token: string | null; token_ttl_secs: number; max_zoom: number;
    dimensions: Array<{ slug: string; levels?: number[]; bounds: { min_x: number; max_x: number; min_y: number; max_y: number } | null }> } | null>(null);
  let error = $state('');
  let loading = $state(true);
  let mode = $state<Mode>('move');
  let paint = $state<Point[]>([]);
  let pointer = $state<{ id: number; kind: Mode | 'pan'; x: number; y: number; last: Point | null } | null>(null);
  let broken = $state<Record<string, true>>({});

  const canEdit = $derived(!!myGuildId);
  const slug = $derived(dimension.replace('minecraft:', '').replace(':', '__'));
  const here = $derived(info?.dimensions.find((d) => d.slug === slug) ?? null);
  const canPaint = $derived(canEdit && pixelsPerBlock >= PAINT_MIN_PPB);
  const effective = $derived<Mode>(mode !== 'move' && !canPaint ? 'move' : mode);

  // Tile keys last an hour; renew well before they expire.
  $effect(() => {
    const id = serverId;
    let alive = true, timer: ReturnType<typeof setTimeout>;
    loading = true; error = ''; broken = {};
    const load = async () => {
      try {
        const next = await invoke<NonNullable<typeof info>>('get_map_info', { serverId: id });
        if (!alive) return;
        info = next; loading = false; error = next.ready ? '' : next.message;
        timer = setTimeout(load, next.ready ? Math.max(30, Math.min(next.token_ttl_secs * 0.6, 1200)) * 1000 : 8000);
      } catch (e) {
        if (!alive) return;
        loading = false; error = typeof e === 'string' ? e : (e as Error)?.message ?? 'Could not load the map';
        timer = setTimeout(load, 15000);
      }
    };
    load();
    return () => { alive = false; clearTimeout(timer); };
  });

  $effect(() => { if (!canEdit && mode !== 'move') mode = 'move'; });
  $effect(() => { worldX = centerX * 16 + 8; worldZ = centerZ * 16 + 8; });

  // Frame a rectangle of chunks (e.g. "my land"): centre it and zoom so all of it is in view.
  let lastFocus = 0;
  $effect(() => {
    if (!focus || focus.nonce === lastFocus) return;
    lastFocus = focus.nonce;
    const spanX = (focus.maxX - focus.minX + 1) * 16, spanZ = (focus.maxZ - focus.minZ + 1) * 16;
    worldX = (focus.minX + focus.maxX + 1) * 8;
    worldZ = (focus.minZ + focus.maxZ + 1) * 8;
    pixelsPerBlock = Math.max(MIN_PPB, Math.min(4, Math.min(width / (spanX * 1.25), height / (spanZ * 1.25))));
    oncenterchange(Math.floor(worldX / 16), Math.floor(worldZ / 16));
  });

  const left = $derived(worldX - width / (2 * pixelsPerBlock));
  const top = $derived(worldZ - height / (2 * pixelsPerBlock));

  /** The sharpest tile level worth loading at this zoom (each level up halves the resolution). */
  const level = $derived.by(() => {
    const wanted = Math.max(0, Math.min(info?.max_zoom ?? 6, Math.floor(Math.log2(1 / pixelsPerBlock))));
    const levels = here?.levels;
    if (!levels?.length) return wanted;
    for (let z = Math.min(wanted, levels.length - 1); z > 0; z--) if (levels[z] > 0) return z;
    return 0;
  });

  const tileUrl = (z: number, x: number, y: number) => `${info!.tile_base}/${slug}/${z}/${x}/${y}.png?t=${info!.token}`;

  const visibleTiles = $derived.by(() => {
    if (!info?.ready || !info.token || !here) return [] as { key: string; url: string; left: number; top: number; size: number }[];
    const out: { key: string; url: string; left: number; top: number; size: number }[] = [];
    const b = here.bounds;
    const span = TILE * 2 ** level;
    const minX = Math.floor(left / span), maxX = Math.floor((left + width / pixelsPerBlock) / span);
    const minZ = Math.floor(top / span), maxZ = Math.floor((top + height / pixelsPerBlock) / span);
    if ((maxX - minX + 1) * (maxZ - minZ + 1) > 400) return out;
    for (let z = minZ; z <= maxZ; z++) for (let x = minX; x <= maxX; x++) {
      // bounds are in zoom-0 tiles; this tile covers 2^level of them each way
      if (b && (x * 2 ** level > b.max_x || (x + 1) * 2 ** level - 1 < b.min_x || z * 2 ** level > b.max_y || (z + 1) * 2 ** level - 1 < b.min_y)) continue;
      const url = tileUrl(level, x, z);
      if (broken[url]) continue;
      out.push({ key: url, url, left: (x * span - left) * pixelsPerBlock, top: (z * span - top) * pixelsPerBlock, size: span * pixelsPerBlock });
    }
    return out;
  });

  // Everyone's land near the view, plus all of my guild's land (which may be far away).
  const allClaims = $derived.by(() => {
    const seen = new Set<number>();
    const out: GuildClaim[] = [];
    for (const c of [...claims, ...myLand]) {
      if (c.dimension !== dimension || seen.has(c.id)) continue;
      seen.add(c.id);
      out.push(c);
    }
    return out;
  });
  const visibleClaims = $derived(allClaims.filter((c) =>
    c.chunk_x * 16 + 16 >= left && c.chunk_x * 16 <= left + width / pixelsPerBlock &&
    c.chunk_z * 16 + 16 >= top && c.chunk_z * 16 <= top + height / pixelsPerBlock).slice(0, 2500));
  const playerHere = $derived(player?.dimension === dimension ? player : null);
  const chunkPx = $derived(Math.max(1, 16 * pixelsPerBlock));

  function at(e: PointerEvent): Point {
    const rect = host.getBoundingClientRect();
    return { x: Math.floor((left + (e.clientX - rect.left) / pixelsPerBlock) / 16),
      z: Math.floor((top + (e.clientY - rect.top) / pixelsPerBlock) / 16) };
  }

  function addStroke(next: Point) {
    const last = pointer?.last ?? next;
    const steps = Math.max(Math.abs(next.x - last.x), Math.abs(next.z - last.z));
    const nextPaint = new Map(paint.map((p) => [`${p.x}:${p.z}`, p]));
    for (let i = 0; i <= steps; i++) {
      const x = Math.round(last.x + (next.x - last.x) * i / Math.max(1, steps));
      const z = Math.round(last.z + (next.z - last.z) * i / Math.max(1, steps));
      if (nextPaint.size < 128) nextPaint.set(`${x}:${z}`, { x, z });
    }
    paint = [...nextPaint.values()];
    if (pointer) pointer.last = next;
  }

  function down(e: PointerEvent) {
    if (!info?.ready || (e.button !== 0 && e.button !== 1)) return;
    e.preventDefault();
    // Move mode drags the map. Claim and un-claim modes paint, and Alt (or the middle button) drags the map instead.
    const kind: Mode | 'pan' = e.button === 1 || e.altKey || effective === 'move' ? 'pan' : effective;
    pointer = { id: e.pointerId, kind, x: e.clientX, y: e.clientY, last: null };
    host.setPointerCapture(e.pointerId);
    if (kind !== 'pan') { paint = []; addStroke(at(e)); }
  }
  function move(e: PointerEvent) {
    if (!pointer || pointer.id !== e.pointerId) return;
    if (pointer.kind === 'pan') {
      worldX -= (e.clientX - pointer.x) / pixelsPerBlock;
      worldZ -= (e.clientY - pointer.y) / pixelsPerBlock;
      pointer.x = e.clientX; pointer.y = e.clientY;
    } else addStroke(at(e));
  }
  function up(e: PointerEvent) {
    if (!pointer || pointer.id !== e.pointerId) return;
    const kind = pointer.kind;
    if (host.hasPointerCapture(e.pointerId)) host.releasePointerCapture(e.pointerId);
    pointer = null;
    if (kind === 'pan') oncenterchange(Math.floor(worldX / 16), Math.floor(worldZ / 16));
    else if (paint.length && kind !== 'move') onpaint(paint, kind);
    paint = [];
  }

  /** Zoom keeping the point under (sx, sy) where it is. */
  function zoomAt(factor: number, sx = width / 2, sy = height / 2) {
    const before = { x: left + sx / pixelsPerBlock, z: top + sy / pixelsPerBlock };
    pixelsPerBlock = Math.max(MIN_PPB, Math.min(MAX_PPB, pixelsPerBlock * factor));
    worldX = before.x - (sx - width / 2) / pixelsPerBlock;
    worldZ = before.z - (sy - height / 2) / pixelsPerBlock;
    oncenterchange(Math.floor(worldX / 16), Math.floor(worldZ / 16));
  }
  function wheel(e: WheelEvent) {
    e.preventDefault();
    const rect = host.getBoundingClientRect();
    zoomAt(e.deltaY < 0 ? 1.2 : 1 / 1.2, e.clientX - rect.left, e.clientY - rect.top);
  }
  function key(e: KeyboardEvent) {
    const step = 64 / pixelsPerBlock;
    const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    if (moves[e.key]) { e.preventDefault(); worldX += moves[e.key][0]; worldZ += moves[e.key][1]; oncenterchange(Math.floor(worldX / 16), Math.floor(worldZ / 16)); }
    else if (e.key === '+' || e.key === '=') zoomAt(1.25);
    else if (e.key === '-') zoomAt(1 / 1.25);
    else if (e.key === 'Enter' && effective !== 'move') onpaint([{ x: Math.floor(worldX / 16), z: Math.floor(worldZ / 16) }], effective);
  }

  const hints: Record<Mode, string> = {
    move: 'Drag to move the map · Wheel to zoom',
    claim: 'Click or drag to claim chunks · Alt + drag to move',
    unclaim: 'Click or drag to un-claim your chunks · Alt + drag to move'
  };
  const hint = $derived(mode !== 'move' && !canPaint ? 'Zoom in to claim or un-claim · Drag to move' : hints[mode]);

  onMount(() => {
    const observer = new ResizeObserver(([entry]) => { width = entry.contentRect.width; height = entry.contentRect.height; });
    observer.observe(host);
    return () => observer.disconnect();
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="terrain" class:claiming={effective === 'claim'} class:unclaiming={effective === 'unclaim'} class:panning={pointer?.kind === 'pan'}
  bind:this={host} role="application" tabindex="0" aria-label="Territory map. {hint}"
  onkeydown={key} onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onwheel={wheel} oncontextmenu={(e) => e.preventDefault()}>
  {#each visibleTiles as tile (tile.key)}
    <img class="tile" src={tile.url} alt="" draggable="false" onerror={() => (broken[tile.url] = true)} style:left="{tile.left}px" style:top="{tile.top}px" style:width="{tile.size}px" style:height="{tile.size}px" />
  {/each}
  {#if pixelsPerBlock >= 1.5}
    <div class="grid" style:background-size="{16 * pixelsPerBlock}px {16 * pixelsPerBlock}px"
      style:background-position="{-left * pixelsPerBlock}px {-top * pixelsPerBlock}px"></div>
  {/if}
  {#each visibleClaims as claim (claim.id)}
    <div class="claim" class:mine={claim.guild_id === myGuildId} class:thin={chunkPx < 6}
      title="[{claim.chunk_x}, {claim.chunk_z}] {claim.guild_name}"
      style:left="{(claim.chunk_x * 16 - left) * pixelsPerBlock}px" style:top="{(claim.chunk_z * 16 - top) * pixelsPerBlock}px"
      style:width="{chunkPx}px" style:height="{chunkPx}px"></div>
  {/each}
  {#each paint as chunk (`${chunk.x}:${chunk.z}`)}
    <div class="preview" class:remove={pointer?.kind === 'unclaim'}
      style:left="{(chunk.x * 16 - left) * pixelsPerBlock}px" style:top="{(chunk.z * 16 - top) * pixelsPerBlock}px"
      style:width="{chunkPx}px" style:height="{chunkPx}px"></div>
  {/each}
  {#if playerHere}<div class="player" title="Your position: {Math.floor(playerHere.x)}, {Math.floor(playerHere.z)}"
    style:left="{(playerHere.x - left) * pixelsPerBlock}px" style:top="{(playerHere.z - top) * pixelsPerBlock}px"></div>{/if}
  {#if loading}<div class="message">Loading the map…</div>{/if}
  {#if error && !loading}<div class="message">{error}</div>{/if}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modes" role="radiogroup" tabindex="-1" aria-label="Map tool" onpointerdown={(e) => e.stopPropagation()}>
    <button role="radio" aria-checked={mode === 'move'} class:on={mode === 'move'} onclick={() => (mode = 'move')} title="Click and drag to move the map (no claiming)">
      <Hand size={15} /> Move</button>
    <button role="radio" aria-checked={mode === 'claim'} class:on={mode === 'claim'} class="claim-btn" disabled={!canEdit} onclick={() => (mode = 'claim')}
      title={canEdit ? 'Click and drag to claim chunks. Alt + drag to move.' : 'Join a faction to claim land'}>
      <SquarePlus size={15} /> Claim</button>
    <button role="radio" aria-checked={mode === 'unclaim'} class:on={mode === 'unclaim'} class="unclaim-btn" disabled={!canEdit} onclick={() => (mode = 'unclaim')}
      title={canEdit ? 'Click and drag to un-claim your faction’s chunks. Alt + drag to move.' : 'Join a faction to manage land'}>
      <Eraser size={15} /> Un-claim</button>
  </div>
  <div class="zoom" role="group" aria-label="Zoom" onpointerdown={(e) => e.stopPropagation()}>
    <button onclick={() => zoomAt(1.4)} title="Zoom in" aria-label="Zoom in" disabled={pixelsPerBlock >= MAX_PPB}><Plus size={16} /></button>
    <button onclick={() => zoomAt(1 / 1.4)} title="Zoom out" aria-label="Zoom out" disabled={pixelsPerBlock <= MIN_PPB}><Minus size={16} /></button>
  </div>

  <div class="help" class:warn={mode !== 'move' && !canPaint}>{hint}</div>
  <div class="coords">{Math.floor(worldX)}, {Math.floor(worldZ)} · {pixelsPerBlock >= 1 ? `${pixelsPerBlock.toFixed(1)} px/block` : `${(1 / pixelsPerBlock).toFixed(1)} blocks/px`}</div>
</div>

<style>
  .terrain { position: relative; height: min(62vh, 640px); min-height: 420px; overflow: hidden; background: #142333; border-radius: 12px; cursor: grab; touch-action: none; outline: none; }
  .terrain:focus-visible { box-shadow: 0 0 0 2px var(--accent); }
  .terrain.panning { cursor: grabbing; }
  .terrain.claiming { cursor: crosshair; }
  .terrain.unclaiming { cursor: crosshair; }
  .tile { position: absolute; image-rendering: pixelated; pointer-events: none; }
  .grid { position: absolute; inset: 0; pointer-events: none; background-image: linear-gradient(to right, #ffffff30 1px, transparent 1px), linear-gradient(to bottom, #ffffff30 1px, transparent 1px); }
  .claim, .preview { position: absolute; pointer-events: none; box-sizing: border-box; }
  .claim { background: #fb71854a; border: 2px solid #fb7185; }
  .claim.mine { background: #4ade804a; border-color: #4ade80; }
  .claim.thin { border-width: 0; background: #fb7185aa; }
  .claim.thin.mine { background: #4ade80cc; }
  .preview { background: #4ade8090; border: 2px solid #fff; }
  .preview.remove { background: #fb718590; }
  .player { position: absolute; width: 15px; height: 15px; margin: -7px; border: 3px solid white; border-radius: 50%; background: #3b82f6; box-shadow: 0 0 0 3px #2563eb, 0 2px 9px #000; pointer-events: none; }
  .message { position: absolute; inset: 0; display: grid; place-items: center; color: white; background: #142333d9; pointer-events: none; }

  .modes { position: absolute; top: 10px; left: 10px; display: flex; padding: 3px; gap: 2px; border-radius: 999px; background: #08111cd9; border: 1px solid #ffffff22; backdrop-filter: blur(10px); cursor: default; }
  .modes button { display: inline-flex; align-items: center; gap: 0.35rem; border: 0; border-radius: 999px; padding: 0.4rem 0.8rem; background: transparent; color: #b8c2dc; font: 600 12.5px/1 inherit; cursor: pointer; transition: background .12s, color .12s; }
  .modes button:hover:not(:disabled):not(.on) { background: #ffffff14; color: #fff; }
  .modes button:disabled { opacity: 0.4; cursor: not-allowed; }
  .modes button.on { color: #fff; background: var(--accent, #8b6cff); }
  .modes .claim-btn.on { background: #16a34a; }
  .modes .unclaim-btn.on { background: #e11d48; }
  .zoom { position: absolute; right: 10px; top: 10px; display: flex; flex-direction: column; border-radius: 12px; overflow: hidden; background: #08111cd9; border: 1px solid #ffffff22; backdrop-filter: blur(10px); cursor: default; }
  .zoom button { display: grid; place-items: center; width: 34px; height: 34px; border: 0; background: transparent; color: #e7ebff; cursor: pointer; }
  .zoom button + button { border-top: 1px solid #ffffff1f; }
  .zoom button:hover:not(:disabled) { background: #ffffff1a; }
  .zoom button:disabled { opacity: 0.35; cursor: default; }
  .help, .coords { position: absolute; bottom: 10px; padding: 7px 11px; border-radius: 8px; color: white; background: #08111ccc; font-size: 12px; pointer-events: none; }
  .help { left: 10px; }.coords { right: 10px; font-variant-numeric: tabular-nums; }
  .help.warn { background: #78350fe6; }
</style>
