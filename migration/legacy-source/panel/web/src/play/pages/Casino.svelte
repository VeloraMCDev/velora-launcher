<script lang="ts">
  import { Dices } from '@lucide/svelte';
  import CasinoPage from '@scopenet/casino/CasinoPage.svelte';
  import { setCasinoHost } from '@scopenet/casino';
  import Empty from '../ui/Empty.svelte';
  import { get, post } from '../../lib/api';
  import { toast } from '../../lib/toast.svelte';
  import { balanceChanged, currentServer, play } from '../store.svelte';

  // The casino is shared with the launcher; this page plugs it into the website's API and the player panel's balance.
  setCasinoHost({
    get: (serverId, path) => get(`/api/v1/casino/${serverId}${path}`),
    post: (serverId, path, body) => post(`/api/v1/casino/${serverId}${path}`, body),
    toast,
    avatar: (uuid, size) => (uuid ? `/api/v1/avatar/${encodeURIComponent(uuid)}?size=${size}` : null),
  });

  const server = $derived(currentServer());
</script>

<div class="pl-page wide">
  <div class="pl-head">
    <div><h1><Dices size={26} /> Casino</h1><p class="lead">Spin, drop and gamble your in-game cash from anywhere. Winnings are in your balance instantly.</p></div>
  </div>
  {#if !play.loaded}
    <div class="pl-skel" style="height: 280px"></div>
  {:else if server}
    {#key server.id}
      <CasinoPage servers={[{ id: server.id, name: server.name }]} initialServerId={server.id} compact onbalance={(v) => balanceChanged(v)} />
    {/key}
  {:else}
    <div class="pl-card"><Empty icon={Dices} title="No servers yet" text="The casino opens as soon as a game server is connected." /></div>
  {/if}
</div>

<style>
  .wide { max-width: min(1780px, 100%); }
  @media (max-width: 880px) { .lead { display: none; } .pl-head { margin-bottom: 10px; } }
</style>
