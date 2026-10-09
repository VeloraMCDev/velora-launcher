<script lang="ts">
  import { Map as MapIcon, Users } from '@lucide/svelte';
  import MapViewer from '@velora/map/MapViewer.svelte';
  import { startFeed, type MapInfo, type MapOverlay } from '@velora/map';
  import Avatar from '../../components/Avatar.svelte';
  import Empty from '../ui/Empty.svelte';
  import { get } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { currentServer, play } from '../store.svelte';

  const server = $derived(currentServer());
  const mapOn = $derived(!!server?.map);
  let info = $state<MapInfo | null>(null);
  let overlay = $state<MapOverlay | null>(null);
  let error = $state('');
  let showPlayers = $state(false);

  const tileUrl = (slug: string, z: number, x: number, y: number) => `${info?.tile_base}/${slug}/${z}/${x}/${y}.png?t=${info?.token}`;
  const avatarUrl = (uuid: string) => `/api/v1/avatar/${encodeURIComponent(uuid)}?size=64`;

  // The feed refreshes players every few seconds and renews the tile key before it expires. It restarts when the server changes.
  $effect(() => {
    const id = play.serverId;
    info = null; overlay = null; error = '';
    if (id == null || !mapOn) return;
    return startFeed({
      loadInfo: () => get<MapInfo>(`/api/v1/servers/${id}/map`),
      loadOverlay: () => get<MapOverlay>(`/api/v1/servers/${id}/map/overlay`),
      onInfo: (i) => { info = i; error = ''; },
      onOverlay: (o) => { overlay = o; error = ''; },
      onError: (m) => { error = m; },
    });
  });
  const players = $derived(overlay?.players ?? []);
</script>

<div class="pl-page mapPage">
  <div class="pl-head">
    <div><h1><MapIcon size={26} /> Live map</h1><p>{server ? `${server.name}: ` : ''}see the world, claimed land and who is online right now.</p></div>
    {#if server?.map}<button class="pl-btn" onclick={() => (showPlayers = !showPlayers)} aria-pressed={showPlayers}><Users size={16} /> {players.length} online</button>{/if}
  </div>

  {#if !play.loaded}
    <div class="pl-skel" style="height: 60vh"></div>
  {:else if !server}
    <div class="pl-card"><Empty icon={MapIcon} title="No servers yet" text="The map appears once a game server is connected." /></div>
  {:else if !server.map}
    <div class="pl-card"><Empty icon={MapIcon} title="The map is off for {server.name}" text="An admin can switch the live map on in the server's settings." /></div>
  {:else}
    <div class="frame">
      <MapViewer {info} {overlay} {tileUrl} {avatarUrl} {error} selfUuid={session.user?.uuid ?? null} />
      {#if showPlayers}
        <aside class="who pl-card tight" aria-label="Players online">
          {#each players as p (p.uuid)}
            <div class="pl-item"><Avatar name={p.name} uuid={p.uuid} size={28} /><div class="grow"><b>{p.name}</b><span class="sub">{p.dimension ?? ''}</span></div></div>
          {:else}
            <p class="none">Nobody is online.</p>
          {/each}
        </aside>
      {/if}
    </div>
  {/if}
</div>

<style>
  .mapPage { max-width: 1400px; }
  .frame { position: relative; height: clamp(360px, calc(100dvh - 230px), 820px); border-radius: 20px; overflow: hidden; border: 1px solid var(--pl-line); box-shadow: 0 30px 80px -30px #000; --map-accent: var(--accent); }
  .who { position: absolute; top: 12px; right: 12px; width: min(260px, calc(100% - 24px)); max-height: 60%; overflow-y: auto; z-index: 5; background: color-mix(in srgb, var(--surface) 92%, transparent) !important; }
  .none { color: var(--muted); font-size: 0.86rem; padding: 8px; }
  @media (max-width: 880px) { .frame { height: calc(100dvh - var(--pl-top) - var(--pl-tab) - 150px); min-height: 340px; border-radius: 16px; } }
</style>
