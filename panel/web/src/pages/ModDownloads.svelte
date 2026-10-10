<script lang="ts">
  import { onMount } from 'svelte';
  import CoreReleases from '../components/CoreReleases.svelte';
  import { get } from '../lib/api';
  import { go } from '../lib/router.svelte';
  let versions = $state<Array<string | null>>([]);
  onMount(() => { get<Array<{ plugin_version: string | null }>>('/api/admin/servers').then(servers => { versions = servers.map(s => s.plugin_version); }).catch(() => {}); });
</script>
<div class="page">
  <header><h2>Mod downloads</h2><p>Download verified Velora builds and choose where to install them.</p></header>
  <CoreReleases {versions} />
  <section class="card"><h3>Installing the Client jar in your modpack?</h3><p>Add it to your instance files under <code>mods/</code> alongside Fabric API. Players receive the files included in your published instance.</p><button onclick={() => go('installation')}>Open version & modpack</button></section>
</div>
<style>.page { padding: 1.5rem 2rem; display: grid; gap: 1rem; } header p, section p { color: var(--muted); line-height: 1.6; } h2 { margin: 0; } @media(max-width: 600px) { .page { padding: 1rem; } }</style>
