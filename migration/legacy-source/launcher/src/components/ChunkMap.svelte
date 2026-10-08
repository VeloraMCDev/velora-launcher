<script lang="ts">
  import { onMount } from 'svelte';
  import { Compass, Globe, Landmark, LocateFixed, RefreshCw, Server as ServerIcon, Shield } from '@lucide/svelte';
  import { invoke } from '../lib/tauri';
  import { activeAccount, toast } from '../lib/store.svelte';
  import type { GuildClaim } from '../lib/types';
  import ClaimTerrain from './ClaimTerrain.svelte';

  let { instanceId, myGuildId = null, myGuildTag = '', onclaim, onunclaim }: {
    instanceId: string; myGuildId?: string | null; myGuildTag?: string;
    onclaim?: (claim: GuildClaim) => void; onunclaim?: (claimId: number) => void;
  } = $props();
  type GameServer = { id: number; name: string; instance_id: string; map?: boolean };
  type Player = { x: number; z: number; dimension: string };
  let servers = $state<GameServer[]>([]);
  let serverId = $state<number | null>(null);
  let dimension = $state<'minecraft:overworld' | 'minecraft:the_nether' | 'minecraft:the_end'>('minecraft:overworld');
  let centerX = $state(0), centerZ = $state(0);
  let player = $state<Player | null>(null);
  let claims = $state<GuildClaim[]>([]);
  /** All of my guild's land in the current dimension, wherever it is (the nearby list above is limited to the view). */
  let myLand = $state<GuildClaim[]>([]);
  let focus = $state<{ minX: number; maxX: number; minZ: number; maxZ: number; nonce: number } | null>(null);
  let landLoad = 0;
  let loading = $state(false), painting = $state(false);
  let claimLoad = 0;
  let centeredServer = 0;
  const instanceServers = $derived(servers.filter((s) => s.instance_id === instanceId));
  const selectedServer = $derived(instanceServers.find((s) => s.id === serverId));
  const dimensions = [
    { id: 'minecraft:overworld', label: 'Overworld', icon: Globe },
    { id: 'minecraft:the_nether', label: 'Nether', icon: Compass },
    { id: 'minecraft:the_end', label: 'End', icon: Compass }
  ] as const;
  const myClaimsCount = $derived(claims.filter((c) => c.guild_id === myGuildId).length);

  async function refreshPosition(id: number) {
    try {
      const info = await invoke<{ players: Array<Player & { uuid: string }> }>('get_map_overlay', { serverId: id });
      if (serverId !== id) return;
      const mine = activeAccount()?.uuid?.toLowerCase();
      player = info.players.find((p) => p.uuid.toLowerCase() === mine) ?? null;
      if (player && centeredServer !== id) {
        centeredServer = id;
        dimension = (dimensions.find((d) => d.id === player?.dimension)?.id ?? 'minecraft:overworld');
        centerX = Math.floor(player.x / 16);
        centerZ = Math.floor(player.z / 16);
      }
    } catch { player = null; }
  }

  async function loadClaims() {
    const request = ++claimLoad;
    if (!serverId || !instanceId) { claims = []; return; }
    loading = true;
    try {
      const next = await invoke<GuildClaim[]>('get_guild_claims', { instanceId, serverId, dimension, centerX, centerZ });
      if (request === claimLoad) claims = next;
    } catch (e: any) {
      if (request === claimLoad) toast(typeof e === 'string' ? e : e?.message ?? 'Could not load claims', 'error');
    } finally { if (request === claimLoad) loading = false; }
  }

  async function fetchMyLand(forDimension: string): Promise<GuildClaim[]> {
    if (!serverId || !instanceId || !myGuildId) return [];
    return invoke<GuildClaim[]>('get_guild_claims', {
      instanceId, serverId, dimension: forDimension, centerX: 0, centerZ: 0, guildId: myGuildId, radius: 1_000_000
    });
  }

  async function loadMyLand() {
    const request = ++landLoad;
    try {
      const land = await fetchMyLand(dimension);
      if (request === landLoad) myLand = land;
    } catch { if (request === landLoad) myLand = []; }
  }

  /** Frames all of my guild's land, switching to the dimension it is in if the current one has none. */
  async function showMyLand() {
    if (!myGuildId) return;
    let land = myLand;
    if (!land.length) {
      for (const d of dimensions) {
        if (d.id === dimension) continue;
        try { land = await fetchMyLand(d.id); } catch { land = []; }
        if (land.length) { dimension = d.id; myLand = land; break; }
      }
    }
    if (!land.length) { toast('Your guild has no claimed land yet.', 'info'); return; }
    focus = {
      minX: Math.min(...land.map((c) => c.chunk_x)), maxX: Math.max(...land.map((c) => c.chunk_x)),
      minZ: Math.min(...land.map((c) => c.chunk_z)), maxZ: Math.max(...land.map((c) => c.chunk_z)), nonce: (focus?.nonce ?? 0) + 1
    };
  }

  async function paintChunks(chunks: { x: number; z: number }[], mode: 'claim' | 'unclaim') {
    if (!serverId || !myGuildId || painting) return;
    const existing = new Map(claims.map((c) => [`${c.chunk_x}:${c.chunk_z}`, c]));
    const wanted = chunks.filter(({ x, z }) => {
      const current = existing.get(`${x}:${z}`);
      return mode === 'claim' ? !current : current?.guild_id === myGuildId;
    });
    if (!wanted.length) return;
    painting = true;
    let done = 0;
    try {
      for (const chunk of wanted) {
        try {
          if (mode === 'claim') {
            const claim = await invoke<GuildClaim>('claim_guild_chunk', {
              guildId: myGuildId, instanceId, serverId, dimension, chunkX: chunk.x, chunkZ: chunk.z
            });
            claims = [...claims, claim];
            onclaim?.(claim);
          } else {
            const claim = existing.get(`${chunk.x}:${chunk.z}`)!;
            await invoke('unclaim_guild_chunk', { claimId: claim.id });
            claims = claims.filter((c) => c.id !== claim.id);
            onunclaim?.(claim.id);
          }
          done++;
        } catch (e: any) {
          toast(`${done} of ${wanted.length} chunks updated. ${typeof e === 'string' ? e : e?.message ?? 'Claim failed'}`, 'error');
          break;
        }
      }
      if (done === wanted.length) toast(`${done} ${done === 1 ? 'chunk' : 'chunks'} ${mode === 'claim' ? 'claimed' : 'unclaimed'}`, 'ok');
    } finally { painting = false; await Promise.all([loadClaims(), loadMyLand()]); }
  }

  function centerOnPlayer() {
    if (!player) return;
    dimension = (dimensions.find((d) => d.id === player?.dimension)?.id ?? 'minecraft:overworld');
    centerX = Math.floor(player.x / 16);
    centerZ = Math.floor(player.z / 16);
  }

  onMount(() => {
    invoke<{ servers: GameServer[] }>('get_public_servers').then((r) => { servers = r.servers; })
      .catch((e: any) => { toast(typeof e === 'string' ? e : 'Could not load servers', 'error'); });
    const timer = setInterval(() => { if (serverId) refreshPosition(serverId); }, 10_000);
    return () => clearInterval(timer);
  });
  $effect(() => {
    const available = instanceServers;
    if (!available.some((s) => s.id === serverId)) serverId = available[0]?.id ?? null;
  });
  $effect(() => { if (serverId) { player = null; centeredServer = 0; refreshPosition(serverId); } });
  $effect(() => { if (serverId && instanceId && dimension) loadClaims(); });
  $effect(() => { if (serverId && instanceId && dimension && myGuildId) loadMyLand(); else myLand = []; });
</script>

<div class="claim-map glass">
  <div class="toolbar">
    <div class="group">
      {#if instanceServers.length > 1}
        <label class="server"><ServerIcon size={14} /><select bind:value={serverId} aria-label="Game server">
          {#each instanceServers as s}<option value={s.id}>{s.name}</option>{/each}
        </select></label>
      {:else if selectedServer}<span class="chip"><ServerIcon size={14} /> {selectedServer.name}</span>{/if}
      <div class="dimensions" role="tablist" aria-label="Dimension">
        {#each dimensions as dim}
          {@const Icon = dim.icon}
          <button role="tab" aria-selected={dimension === dim.id} class:active={dimension === dim.id}
            onclick={() => (dimension = dim.id)}><Icon size={14} /> {dim.label}</button>
        {/each}
      </div>
    </div>
    <div class="group">
      {#if player}<span class="position">You: {Math.floor(player.x)}, {Math.floor(player.z)}</span>{:else}<span class="position">Player position unavailable</span>{/if}
      <button class="ghost sm" onclick={showMyLand} disabled={!myGuildId} title="Zoom the map to all of your guild's claimed land"><Landmark size={15} /> My land</button>
      <button class="ghost sm" onclick={centerOnPlayer} disabled={!player} title="Center on your current position"><LocateFixed size={15} /> Find me</button>
      <button class="ghost icon sm" onclick={loadClaims} title="Refresh claims" aria-label="Refresh claims"><RefreshCw size={15} class={loading ? 'spin' : ''} /></button>
    </div>
  </div>
  {#if selectedServer?.map}
    {#key serverId}
      <ClaimTerrain serverId={selectedServer.id} {dimension} {centerX} {centerZ}
        {claims} {myLand} {myGuildId} {player} {focus} onpaint={paintChunks}
        oncenterchange={(x, z) => { centerX = x; centerZ = z; }} />
    {/key}
  {:else}
    <div class="notice">The map is switched off for this server. An admin can turn it on under Servers in the panel.</div>
  {/if}
  <div class="footer">
    <span><Shield size={14} /> Your guild: {myLand.length || myClaimsCount} {(myLand.length || myClaimsCount) === 1 ? 'chunk' : 'chunks'} in this dimension</span>
    <span class="legend"><i class="mine"></i> Your guild <i class="other"></i> Other guilds</span>
    {#if painting}<span>Updating claims…</span>{/if}
    {#if !player}<span class="hint">Your position appears once you are in game and the server's map is running.</span>{/if}
  </div>
</div>

<style>
  .claim-map { display: flex; flex-direction: column; gap: 0.8rem; padding: 1rem; border: 1px solid var(--line); border-radius: var(--radius); }
  .toolbar, .group, .footer, .server, .chip, .legend { display: flex; align-items: center; gap: 0.6rem; }
  .toolbar, .footer { justify-content: space-between; flex-wrap: wrap; }
  .group { flex-wrap: wrap; }
  .server select { width: auto; min-width: 10rem; }
  .dimensions { display: flex; gap: 0.2rem; padding: 0.2rem; background: color-mix(in srgb, var(--bg) 70%, transparent); border-radius: 99px; }
  .dimensions button { display: flex; gap: 0.3rem; align-items: center; border: 0; border-radius: 99px; background: transparent; color: var(--muted); padding: 0.35rem 0.65rem; font-size: 0.8rem; }
  .dimensions button.active { background: color-mix(in srgb, var(--accent) 24%, transparent); color: var(--text); }
  .position, .footer, .notice { color: var(--muted); font-size: 0.8rem; }
  .notice { min-height: 420px; display: grid; place-items: center; text-align: center; border-radius: 10px; background: #142333; padding: 1rem; }
  .footer { border-top: 1px solid var(--line); padding-top: 0.65rem; }
  .footer span { display: inline-flex; align-items: center; gap: 0.3rem; }
  .legend i { width: 10px; height: 10px; border-radius: 2px; }
  .legend .mine { background: #4ade80; }.legend .other { background: #fb7185; }
</style>
