<script lang="ts">

  import PlayerGuide from '@velora/experience/PlayerGuide.svelte';

  import { experienceBranding, selectedInstance } from '../lib/store.svelte';

  const instance = $derived(selectedInstance());

  const details = $derived(instance ? [

    { label: 'Modpack', value: instance.name },

    { label: 'Minecraft', value: instance.mc_version },

    { label: 'Loader', value: instance.loader + (instance.loader_version ? ' ' + instance.loader_version : '') },

    { label: 'Pack', value: instance.source_label ?? instance.description },

    { label: 'Server', value: instance.server ? instance.server.name + ' - ' + instance.server.address + ':' + instance.server.port : 'No server configured' },

  ] : []);

</script>

<PlayerGuide guide={experienceBranding()?.player_guide} {details} />

