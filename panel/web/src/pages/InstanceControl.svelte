<script lang="ts">
  import { experienceContext } from '../lib/experience.svelte';
  import { adminGroups } from '../lib/adminNav';
  import { route, go } from '../lib/router.svelte';
  import ExperienceWidgets from '@scopenet/experience/ExperienceWidgets.svelte';
  const instance = $derived(experienceContext.instance);
  const groups = $derived(adminGroups(route.instanceId, instance?.experience).filter(g => !['platform', 'control'].includes(g.id)));
</script>

<div class="control">
  <p class="eyebrow">Velora / Instance control center</p>
  <h1>{instance?.name ?? 'Select an instance'}</h1>
  <p class="muted">{instance?.description}</p>
  <div class="identity"><span>{instance?.experience?.kind ?? 'smp'}</span><span>{instance?.mc_version} · {instance?.loader}</span></div>
  <div class="actions"><button class="primary" onclick={() => go('experience')}>Design this experience</button><button onclick={() => go('installation')}>Version & modpack</button></div>
  <ExperienceWidgets experience={instance?.experience} />
  {#each groups as group (group.id)}
    <section><h2>{group.label}</h2><div class="systems">
      {#each group.items as item (item.id)}
        <button class="system" onclick={() => go(item.id)}><item.icon size={20} /><strong>{item.label}</strong><span>{item.hint}</span></button>
      {/each}
    </div></section>
  {/each}
</div>

<style>
  .control { padding: 2rem; max-width: 1200px; margin: auto; } .eyebrow { color: var(--muted); font-size: .8rem; }
  h1 { margin: .5rem 0; } .identity, .actions { display: flex; gap: .75rem; margin: 1.5rem 0; flex-wrap: wrap; }
  .identity span { border: 1px solid var(--line); border-radius: 8px; padding: .4rem .7rem; }
  section { margin-top: 2rem; } .systems { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1rem; }
  .system { display: grid; grid-template-columns: 24px 1fr; text-align: left; gap: .7rem; padding: 1.2rem; background: var(--surface); border: 1px solid var(--line); }
  .system span { grid-column: 2; font-size: .85rem; font-weight: 400; color: var(--muted); }
</style>
