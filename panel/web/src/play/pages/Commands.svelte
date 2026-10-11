<script lang="ts">

  import PlayerGuide from '@velora/experience/PlayerGuide.svelte';

  import { play } from '../store.svelte';

  import { route } from '../../lib/router.svelte';

  const instance = $derived(play.manifest?.instances.find(i => i.id === route.instanceId));

  const brand = $derived(instance?.experience?.branding ?? play.manifest?.branding);

  const initialQuery = (() => { try { const q = sessionStorage.getItem('scopenet.play.cmdq') ?? ''; sessionStorage.removeItem('scopenet.play.cmdq'); return q; } catch { return ''; } })();

  const details = $derived(instance ? [

    { label: 'Modpack', value: instance.name },

    { label: 'Minecraft', value: instance.mc_version },

    { label: 'Loader', value: instance.loader + (instance.loader_version ? ' ' + instance.loader_version : '') },

    { label: 'Server', value: instance.server ? (instance.server.name ?? instance.name) + ' - ' + instance.server.address + ':' + (instance.server.port ?? 25565) : 'No server configured' },

  ] : []);

</script>

<PlayerGuide guide={brand?.player_guide} {details} {initialQuery} />

