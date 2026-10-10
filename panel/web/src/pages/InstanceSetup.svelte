<script lang="ts">
  import ModDownloads from './ModDownloads.svelte';
  import ExperienceEditor from './ExperienceEditor.svelte';
  import InstanceEditor from './InstanceEditor.svelte';
  import Servers from './Servers.svelte';
  import ServerDetail from './ServerDetail.svelte';
  import { go, route } from '../lib/router.svelte';
  let { section = 'experience' }: { section?: string } = $props();
  const current = $derived(section === 'experience' && route.params[0] === 'downloads' ? 'experience/downloads' : section);
  const tabs = [{ id: 'experience', label: 'Design & modules' }, { id: 'installation', label: 'Version & modpack' }, { id: 'servers', label: 'Servers' }, { id: 'experience/downloads', label: 'Mod downloads' }];
</script>

<div class="setup">
  <header><h1>Instance setup</h1><p>Design the experience, configure its installation and connect game servers in one place.</p></header>
  <nav aria-label="Instance setup sections">{#each tabs as tab}<button class:on={current === tab.id} aria-current={current === tab.id ? 'page' : undefined} onclick={() => go(tab.id)}>{tab.label}</button>{/each}</nav>
  {#if current === 'experience/downloads'}<ModDownloads />
  {:else if section === 'installation' && route.instanceId}<InstanceEditor id={route.instanceId} />
  {:else if section === 'servers'}
    {#if route.params[0]}<ServerDetail id={route.params[0]} />{:else}<Servers />{/if}
  {:else}<ExperienceEditor />{/if}
</div>

<style>
  header { padding: 1.5rem 2rem 0; } h1 { margin: 0; } p { color: var(--muted); }
  nav { display: flex; flex-wrap: wrap; gap: .5rem; padding: 1rem 2rem; border-bottom: 1px solid var(--line); }
  nav button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--text); }
  @media (max-width: 600px) { header { padding: 1rem 1rem 0; } nav { padding: 1rem; } }
</style>
