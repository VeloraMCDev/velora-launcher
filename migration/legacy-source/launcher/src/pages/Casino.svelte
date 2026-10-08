<script lang="ts">
  import { onMount } from 'svelte';
  import CasinoPage, { type CasinoServer } from '@scopenet/casino/CasinoPage.svelte';
  import { setCasinoHost } from '@scopenet/casino';
  import { app, selectedInstance, toast } from '../lib/store.svelte';
  import { errorText, invoke } from '../lib/tauri';

  // The casino itself is shared with the web player panel; this page only plugs it into the launcher.
  setCasinoHost({
    get: (serverId, path) => invoke('casino_get', { serverId, path }),
    post: (serverId, path, body) => invoke('casino_post', { serverId, path, body }),
    toast,
    avatar: (uuid, size) => (uuid && app.panelUrl ? `${app.panelUrl}/api/v1/avatar/${uuid}?size=${size}&v=${app.skinVersion}` : null),
  });

  let servers = $state<(CasinoServer & { instance_id: string })[]>([]);
  let initial = $state<number | null>(null);
  let error = $state('');
  let ready = $state(false);

  onMount(async () => {
    try {
      const r = await invoke<{ servers: (CasinoServer & { instance_id: string })[] }>('get_public_servers');
      servers = r.servers.filter(s => s.instance_id === selectedInstance()?.id);
      const inst = selectedInstance();
      initial = servers.find((s) => s.instance_id === inst?.id)?.id ?? servers[0]?.id ?? null;
    } catch (e) { error = errorText(e); }
    ready = true;
  });
</script>

{#if ready}
  <CasinoPage {servers} initialServerId={initial} />
{:else if error}
  <p class="err">{error}</p>
{/if}

<style>
  .err { padding: 2rem; color: var(--danger); }
</style>
