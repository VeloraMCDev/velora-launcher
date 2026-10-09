<script lang="ts">
  import { onMount } from 'svelte';
  import { Map as MapIcon, RefreshCw, RotateCcw, Image as ImageIcon, Users } from '@lucide/svelte';
  import MapViewer from '@velora/map/MapViewer.svelte';
  import { startFeed, type LivePlayer, type MapInfo, type MapOverlay } from '@velora/map';
  import Avatar from './Avatar.svelte';
  import PlayerLink from './PlayerLink.svelte';
  import Modal from './Modal.svelte';
  import { del, post, formatBytes, get, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { MapStatus } from '../lib/types';

  let { id, enabled }: { id: number; enabled: boolean } = $props();

  let status = $state<MapStatus | null>(null);
  let info = $state<MapInfo | null>(null);
  let overlay = $state<MapOverlay | null>(null);
  let error = $state('');
  let resetOpen = $state(false);
  let resetting = $state(false);
  let nonce = $state(0);
  type Corner = { dimension: string; x: number; z: number };
  let claiming = $state(false);
  let first = $state<Corner | null>(null);
  let second = $state<Corner | null>(null);
  let claimName = $state('');
  let claimDescription = $state('');
  let claimColor = $state('#f59e0b');
  let savingClaim = $state(false);
  const chunkCount = $derived(first && second ? (Math.abs(first.x - second.x) + 1) * (Math.abs(first.z - second.z) + 1) : 0);
  const displayOverlay = $derived.by(() => {
    if (!overlay || !claiming || !first) return overlay;
    const end = second ?? first;
    const x1 = Math.min(first.x, end.x) * 16, z1 = Math.min(first.z, end.z) * 16;
    const x2 = (Math.max(first.x, end.x) + 1) * 16, z2 = (Math.max(first.z, end.z) + 1) * 16;
    return { ...overlay, claims: [...overlay.claims, { id: 'admin-selection', guild_id: '', name: 'Selected area', tag: '', icon: '', color: claimColor, dimension: first.dimension, chunks: chunkCount || 1, label_x: (x1 + x2) / 2, label_z: (z1 + z2) / 2, outer: [[x1, z1], [x2, z1], [x2, z2], [x1, z2]] as [number, number][], holes: [], lines: ['Click two corners, then create the claim below the map.'] }] };
  });
  function pickChunk(point: Corner) {
    if (!claiming) return;
    const corner = { dimension: point.dimension, x: Math.floor(point.x / 16), z: Math.floor(point.z / 16) };
    if (!first || second || first.dimension !== corner.dimension) { first = corner; second = null; }
    else second = corner;
  }
  async function saveClaim() {
    if (!first || !second || chunkCount > 4096 || savingClaim) return;
    savingClaim = true;
    let created: string | null = null;
    try {
      const result = await post<{ id: string }>(`/api/admin/servers/${id}/admin-claims`, { name: claimName, description: claimDescription, color: claimColor });
      created = result.id;
      const area = await post<{ added: number; skipped: number }>(`/api/admin/admin-claims/${created}/area`, { dimension: first.dimension, chunks: true, x1: first.x, z1: first.z, x2: second.x, z2: second.z });
      created = null;
      toast(`Claim created: ${area.added} chunks added, ${area.skipped} already claimed.`);
      claiming = false; first = null; second = null; claimName = ''; claimDescription = '';
      overlay = await get<MapOverlay>(`/api/v1/servers/${id}/map/overlay`);
    } catch (e) {
      if (created) { try { await del(`/api/admin/admin-claims/${created}`); } catch (cleanup) { toastError(cleanup); } }
      toastError(e);
    } finally { savingClaim = false; }
  }

  const totalTiles = $derived((status?.dimensions ?? []).reduce((a, d) => a + d.tiles, 0));
  const totalBytes = $derived((status?.dimensions ?? []).reduce((a, d) => a + d.bytes, 0));
  const tileUrl = (slug: string, z: number, x: number, y: number) => `${info?.tile_base}/${slug}/${z}/${x}/${y}.png?t=${info?.token}`;
  const avatarUrl = (uuid: string) => `/api/v1/avatar/${uuid}?size=64`;

  async function refresh() {
    try { status = await get<MapStatus>(`/api/admin/servers/${id}/map`); } catch (e) { toastError(e); }
  }

  onMount(() => {
    refresh();
    const timer = setInterval(refresh, 15_000);
    const stop = enabled ? startFeed({
      loadInfo: () => get<MapInfo>(`/api/v1/servers/${id}/map`),
      loadOverlay: () => get<MapOverlay>(`/api/v1/servers/${id}/map/overlay`),
      onInfo: (i) => { info = i; error = ''; },
      onOverlay: (o) => { overlay = o; error = ''; },
      onError: (m) => { error = m; }
    }) : () => {};
    return () => { clearInterval(timer); stop(); };
  });

  async function reset() {
    resetting = true;
    try {
      await del(`/api/admin/servers/${id}/map`);
      toast('Map cleared. The server will draw it again.');
      resetOpen = false;
      info = null; overlay = null; nonce++;
      await refresh();
    } catch (e) { toastError(e); } finally { resetting = false; }
  }
</script>

<section class="card map-card">
  <div class="heading">
    <h2><MapIcon size={17} /> Velora Map</h2>
    <span class="pill" class:ok={enabled}>{enabled ? 'On' : 'Off'}</span>
    <button class="ghost icon" aria-label="Refresh" onclick={refresh}><RefreshCw size={15} /></button>
    <button class="ghost sm" onclick={() => (resetOpen = true)} disabled={!enabled}><RotateCcw size={14} /> Redraw world</button>
  </div>

  {#if !enabled}
    <p class="muted small">The map is off for this server. Switch it on in the server's settings and the plugin or mod starts drawing the world.</p>
  {:else}
    <div class="stats">
      <div><ImageIcon size={14} /><span>Tiles</span><strong>{totalTiles.toLocaleString()}</strong></div>
      <div><span>Size</span><strong>{formatBytes(totalBytes)}</strong></div>
      <div><Users size={14} /><span>Online</span><strong>{status?.players ?? 0}</strong></div>
      <div><span>Last tiles</span><strong>{status?.stats.last_upload_at ? timeAgo(status.stats.last_upload_at) : '—'}</strong></div>
      <div><span>Last players</span><strong>{status?.stats.last_players_at ? timeAgo(status.stats.last_players_at) : '—'}</strong></div>
    </div>
    <div class="diagnostics" role="status">
      {#if status?.diagnostics}
        <div class="diagnostic-head"><strong>Game server: {status.diagnostics.stage}</strong><span class:stale={status.diagnostics.age_seconds > 30}>{status.diagnostics.age_seconds > 30 ? 'No recent report' : `Reported ${status.diagnostics.age_seconds}s ago`}</span></div>
        <div class="diagnostic-grid">
          <span>Region files <strong>{status.diagnostics.region_files}</strong></span>
          <span>Rendered <strong>{status.diagnostics.regions_rendered}</strong></span>
          <span>Queued tiles <strong>{status.diagnostics.queued_tiles}</strong></span>
          <span>Uploaded <strong>{status.diagnostics.tiles_uploaded}</strong></span>
        </div>
        {#if status.diagnostics.error}<p class="diagnostic-error">{status.diagnostics.error}</p>{/if}
        <details><summary>Paths and upload details</summary><p>World: <code>{status.diagnostics.world_root}</code></p><p>Last region: {status.diagnostics.last_region || '—'}</p><p>Last overlay: {status.stats.last_overlay_at ? timeAgo(status.stats.last_overlay_at) : 'never'}</p></details>
      {:else}
        <p>No map report from the game server yet. Check <code>/map</code> in game, the server's <code>map.enabled</code> setting, and its panel token.</p>
      {/if}
    </div>
    {#if status?.dimensions.length}
      <p class="dims muted small">{#each status.dimensions as d, i}{i ? ' · ' : ''}{d.label}: {d.tiles.toLocaleString()} tiles ({formatBytes(d.bytes)}){/each}</p>
    {/if}
    <div class="stage">
      {#key nonce}
        <MapViewer {info} overlay={displayOverlay} {tileUrl} {avatarUrl} {error} onblock={pickChunk}>
          {#snippet toolbar()}
            <button onclick={() => { claiming = !claiming; first = null; second = null; }} disabled={savingClaim}>{claiming ? 'Cancel claim selection' : 'Create admin claim'}</button>
          {/snippet}
          {#snippet player(p: LivePlayer, close: () => void)}
            <div class="pcard">
              <button class="x" onclick={close} aria-label="Close">×</button>
              <div class="who"><Avatar name={p.name} uuid={p.uuid} size={44} /><div><strong><PlayerLink uuid={p.uuid} name={p.name} /></strong><span>{p.dimension.replace('minecraft:', '').replace('the_', '')}</span></div></div>
              <p>X {Math.round(p.x)} · Y {Math.round(p.y)} · Z {Math.round(p.z)}</p>
              <code>/tp {p.name}</code>
            </div>
          {/snippet}
        </MapViewer>
      {/key}
    </div>
    {#if claiming}
      <div class="claim-editor">
        <p>Click two corners on the map to select a rectangle of chunks. Existing claims are skipped.</p>
        {#if first}<p>{first.dimension}: first corner ({first.x}, {first.z}){#if second}, second corner ({second.x}, {second.z}) — {chunkCount} chunks{/if}</p>{/if}
        <label>Name<input bind:value={claimName} maxlength="32" /></label>
        <label>Description<input bind:value={claimDescription} maxlength="300" /></label>
        <label>Colour<input type="color" bind:value={claimColor} /></label>
        {#if chunkCount > 4096}<p>Select at most 4096 chunks.</p>{/if}
        <button onclick={saveClaim} disabled={!second || !claimName.trim() || chunkCount > 4096 || savingClaim}>{savingClaim ? 'Creating…' : 'Create claim'}</button>
      </div>
    {/if}
  {/if}
</section>

<Modal bind:open={resetOpen} title="Redraw the world?">
  <p>This deletes every map tile stored on the panel. The game server notices and draws the world again from its region files, starting near the players. The map stays empty for a while on big worlds.</p>
  <div class="actions">
    <button class="ghost" onclick={() => (resetOpen = false)}>Cancel</button>
    <button class="danger" onclick={reset} disabled={resetting}>{resetting ? 'Clearing…' : 'Clear and redraw'}</button>
  </div>
</Modal>

<style>
  .map-card { display: flex; flex-direction: column; gap: 0.75rem; }
  .heading { display: flex; align-items: center; gap: 0.5rem; }
  .heading h2 { margin: 0 auto 0 0; font-size: 1rem; display: flex; align-items: center; gap: 0.5rem; }
  .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(7.5rem, 1fr)); gap: 0.6rem; }
  .stats > div { display: flex; flex-direction: column; gap: 2px; padding: 0.55rem 0.75rem; border: 1px solid var(--line); border-radius: 10px; font-size: 0.78rem; color: var(--muted); }
  .stats strong { color: var(--text); font-size: 1.05rem; }
  .dims { margin: 0; }
  .diagnostics { padding: 0.85rem 1rem; border: 1px solid var(--line); border-radius: 10px; background: var(--surface-2); line-height: 1.5; }
  .diagnostic-head { display: flex; gap: 0.75rem; align-items: center; justify-content: space-between; flex-wrap: wrap; }
  .diagnostic-head span { color: var(--good); font-size: 0.82rem; }
  .diagnostic-head span.stale, .diagnostic-error { color: var(--bad); }
  .diagnostic-grid { display: flex; flex-wrap: wrap; gap: 0.5rem 1.25rem; margin: 0.5rem 0; color: var(--muted); font-size: 0.85rem; }
  .diagnostic-grid strong { color: var(--text); margin-left: 0.25rem; }
  .diagnostics details { margin-top: 0.4rem; color: var(--muted); font-size: 0.85rem; }
  .diagnostics summary { cursor: pointer; }
  .diagnostics code { overflow-wrap: anywhere; }
  .stage { height: min(68vh, 640px); min-height: 380px; border-radius: 14px; overflow: hidden; border: 1px solid var(--line); --map-accent: var(--accent); }
  .pcard { height: 100%; display: flex; flex-direction: column; gap: 0.5rem; padding: 1rem; border-radius: 18px; background: rgba(14, 19, 34, 0.94); border: 1px solid rgba(255,255,255,0.12); backdrop-filter: blur(14px); color: #f4f6ff; position: relative; align-self: flex-start; height: auto; width: 100%; }
  .pcard .who { display: flex; gap: 0.7rem; align-items: center; }
  .pcard .who div { display: flex; flex-direction: column; }
  .pcard .who span { color: #8d97b8; font-size: 0.8rem; text-transform: capitalize; }
  .pcard p { margin: 0; color: #d4daf0; font-size: 0.85rem; }
  .pcard code { align-self: flex-start; padding: 0.25rem 0.5rem; border-radius: 8px; background: rgba(255,255,255,0.08); font-size: 0.8rem; }
  .pcard .x { position: absolute; top: 8px; right: 10px; width: 26px; height: 26px; border-radius: 50%; border: 0; background: rgba(255,255,255,0.08); color: #fff; cursor: pointer; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem; }
  .claim-editor { display: flex; flex-wrap: wrap; align-items: end; gap: 0.75rem; padding: 1rem; border: 1px solid var(--line); border-radius: 12px; }
  .claim-editor p { flex-basis: 100%; margin: 0; }
  .claim-editor label { display: flex; flex-direction: column; gap: 0.25rem; }
</style>
