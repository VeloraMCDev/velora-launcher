<script lang="ts">
  import { onMount } from 'svelte';
  import MapViewer from '@scopenet/map/MapViewer.svelte';
  import { startFeed, type MapInfo, type MapLayers, type MapOverlay } from '@scopenet/map';
  import { get } from '../lib/api';

  // The public live map on the landing page. The admin chooses the server and which layers visitors may see
  // (by default guild land, spawn, warps and shops, never where players are).
  let { serverId = 0, height = 560, unavailable = $bindable(false) }: { serverId?: number; height?: number; unavailable?: boolean } = $props();

  let info = $state<(MapInfo & { layers?: MapLayers }) | null>(null);
  let overlay = $state<MapOverlay | null>(null);
  let error = $state('');
  const tileUrl = (slug: string, z: number, x: number, y: number) => `${info?.tile_base}/${slug}/${z}/${x}/${y}.png?t=${info?.token}`;
  const available = $derived(info?.layers ?? { claims: true, pins: true, players: false });
  const claimCount = $derived(overlay?.claims.length ?? 0);

  onMount(() => startFeed({
    loadInfo: () => get<MapInfo & { layers: MapLayers }>(`/api/v1/public/map/${serverId}`),
    loadOverlay: () => get<MapOverlay>(`/api/v1/public/map/${serverId}/overlay`),
    onInfo: (i) => { info = i; error = ''; unavailable = false; },
    onOverlay: (o) => { overlay = o; error = ''; },
    // No map to show (switched off, or no server has one yet): the section hides itself instead of showing an error to visitors.
    onError: (m) => { error = m; if (!info) unavailable = true; },
    overlayMs: 4000
  }));
</script>

<div class="window">
  <div class="bar">
    <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
    <span class="title">Live map</span>
    <span class="live"><b></b> Live{claimCount ? ` · ${claimCount} claim${claimCount === 1 ? '' : 's'}` : ''}</span>
  </div>
  <div class="frame" style:height="{Math.min(1000, Math.max(300, height))}px">
    <MapViewer {info} {overlay} {tileUrl} {error} {available} />
  </div>
</div>

<style>
  .window { border-radius: 20px; overflow: hidden; border: 1px solid rgba(255, 255, 255, 0.12); background: #0b0d17; box-shadow: 0 40px 90px -30px rgba(0, 0, 0, 0.8), 0 0 0 1px rgba(255, 255, 255, 0.03) inset, 0 0 80px -30px color-mix(in srgb, var(--accent, #8b6cff) 55%, transparent); --map-accent: var(--accent, #8b6cff); }
  .bar { display: flex; align-items: center; gap: 12px; padding: 11px 16px; background: linear-gradient(180deg, rgba(255, 255, 255, 0.06), rgba(255, 255, 255, 0.02)); border-bottom: 1px solid rgba(255, 255, 255, 0.08); font-size: 12.5px; color: rgba(255, 255, 255, 0.7); }
  .dots { display: inline-flex; gap: 6px; } .dots i { width: 10px; height: 10px; border-radius: 50%; background: rgba(255, 255, 255, 0.16); }
  .dots i:nth-child(1) { background: #ff5f57; } .dots i:nth-child(2) { background: #febc2e; } .dots i:nth-child(3) { background: #28c840; }
  .title { font-weight: 650; letter-spacing: 0.02em; }
  .live { margin-left: auto; display: inline-flex; align-items: center; gap: 7px; font-weight: 560; } .live b { width: 7px; height: 7px; border-radius: 50%; background: #3fb97c; box-shadow: 0 0 10px #3fb97c; animation: ping 2s ease-in-out infinite; }
  @keyframes ping { 50% { opacity: 0.35; } }
  .frame { width: 100%; }
  @media (prefers-reduced-motion: reduce) { .live b { animation: none; } }
</style>
