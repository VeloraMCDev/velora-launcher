<script lang="ts">
  import { onMount } from 'svelte';
  import { get, put } from '../lib/api';
  import type { Group } from '../lib/types';
  import { toast, toastError } from '../lib/toast.svelte';
  let groups = $state<Group[]>([]);
  onMount(() => { get<Group[]>('/api/admin/instance-groups').then(value => groups = value).catch(toastError); });
  async function save(group: Group) {
    try {
      await put(`/api/admin/groups/${group.id}/luckperms`, { luckperms_group: group.luckperms_group ?? '' });
      await put(`/api/admin/groups/${group.id}/discord-role`, { discord_role: group.discord_role ?? '' });
      toast(`Saved ${group.name} integrations for this instance`);
    } catch (e) { toastError(e); }
  }
</script>
<section>
  <h2>Group integration mappings</h2>
  <p>Player groups are shared. Each instance chooses its own LuckPerms group and Discord role mappings.</p>
  {#each groups as group (group.id)}
    <div class="mapping">
      <strong>{group.name}</strong>
      <label>LuckPerms group<input bind:value={group.luckperms_group} maxlength="64" /></label>
      <label>Discord role ID<input bind:value={group.discord_role} maxlength="24" inputmode="numeric" /></label>
      <button onclick={() => save(group)}>Save mapping</button>
    </div>
  {:else}<p>Create player groups in the platform’s Players page to configure their mappings here.</p>{/each}
</section>
<style>
  section { margin: 1.5rem 0; padding: 1.5rem; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  p { color: var(--muted); line-height: 1.6; }
  .mapping { display: flex; gap: 1rem; flex-wrap: wrap; align-items: center; margin: 1rem 0; }
  label { display: grid; gap: .4rem; }
</style>
