<script lang="ts">
  import { onMount } from 'svelte';
  import { X, RefreshCw } from '@lucide/svelte';
  import MapViewer from '@velora/map/MapViewer.svelte';
  import { startFeed, type LivePlayer, type MapInfo, type MapOverlay } from '@velora/map';
  import { invoke } from '../lib/tauri';
  import { activeAccount, app } from '../lib/store.svelte';
  import MapPlayerPanel from './MapPlayerPanel.svelte';

  let { serverId, serverName, instanceId, onclose }: { serverId: number; serverName: string; instanceId: string; onclose: () => void } = $props();

  let info = $state<MapInfo | null>(null);
  let overlay = $state<MapOverlay | null>(null);
  let error = $state('');
  let nonce = $state(0);

  const panel = $derived(activeAccount()?.panel_url ?? app.panelUrl);
  const tileUrl = (slug: string, z: number, x: number, y: number) => `${info?.tile_base}/${slug}/${z}/${x}/${y}.png?t=${info?.token}`;
  const avatarUrl = (uuid: string) => (panel ? `${panel}/api/v1/avatar/${uuid}?size=64&v=${app.skinVersion}` : '');

  // The feed reloads the overlay every couple of seconds and renews the tile key before it expires.
  onMount(() => startFeed({
    loadInfo: () => invoke<MapInfo>('get_map_info', { serverId }),
    loadOverlay: () => invoke<MapOverlay>('get_map_overlay', { serverId }),
    onInfo: (i) => { info = i; error = ''; },
    onOverlay: (o) => { overlay = o; error = ''; },
    onError: (m) => { error = m; }
  }));

  const reload = () => { info = null; overlay = null; nonce++; };
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" role="dialog" aria-modal="true" aria-label="Live map">
  <header class="glass">
    <div class="title">
      <span class="live" class:on={!!info?.ready}></span>
      <div><strong>{serverName}</strong><span class="muted">Live Map{#if overlay} · {overlay.players.length} online{/if}</span></div>
    </div>
    <span class="grow"></span>
    <button class="ghost sm" onclick={reload} title="Reload the map"><RefreshCw size={14} /> Reload</button>
    <button class="ghost icon close" onclick={onclose} aria-label="Close live map"><X size={18} /></button>
  </header>

  <div class="stage">
    {#key nonce}
      <MapViewer {info} {overlay} {tileUrl} {avatarUrl} {error} selfUuid={activeAccount()?.uuid ?? null}>
        {#snippet player(p: LivePlayer, close: () => void)}
          <MapPlayerPanel {serverId} {instanceId} uuid={p.uuid} nameHint={p.name} onclose={close} />
        {/snippet}
      </MapViewer>
    {/key}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 90; display: flex; flex-direction: column; gap: 0.75rem; padding: 0.9rem; background: color-mix(in srgb, var(--bg) 88%, transparent); backdrop-filter: blur(10px); animation: fade 0.15s ease; }
  header { display: flex; align-items: center; gap: 0.6rem; padding: 0.55rem 0.7rem 0.55rem 1rem; background: var(--surface); flex-shrink: 0; }
  .title { display: flex; align-items: center; gap: 0.7rem; }
  .title div { display: flex; flex-direction: column; line-height: 1.2; }
  .title .muted { font-size: 0.75rem; }
  .live { width: 0.6rem; height: 0.6rem; border-radius: 50%; background: color-mix(in srgb, var(--muted) 55%, transparent); }
  .live.on { background: #34d399; box-shadow: 0 0 0 4px color-mix(in srgb, #34d399 22%, transparent); }
  .grow { flex: 1; }
  .sm { display: inline-flex; align-items: center; gap: 0.35rem; }
  .stage { position: relative; flex: 1; min-height: 0; border-radius: var(--radius); overflow: hidden; border: 1px solid var(--line); background: #0b0d12; --map-accent: var(--accent); }
</style>
