<script lang="ts">
  import { route } from '../lib/router.svelte';
  import { enabled } from '@velora/experience';
  import { experienceContext } from '../lib/experience.svelte';
  import { Globe, UserRound, Tag } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { get } from '../lib/api';
  let instances = $state<Array<{id: string; name: string}>>([]);
  onMount(async () => { instances = await get('/api/admin/instances'); if (route.instanceId) { instances = instances.filter(i => i.id === route.instanceId); draft.instance_id = route.instanceId; } });
  import Toggle from './Toggle.svelte';
  import type { Group, ServerDraft } from '../lib/types';

  let { draft = $bindable(), groups }: { draft: ServerDraft; groups: Group[] } = $props();

  const accessOptions = [
    { id: 'all', label: 'Anyone', help: 'Anyone who can connect. Panel accounts are still checked for bans.', icon: Globe },
    { id: 'members', label: 'Panel accounts', help: 'Only players with an active account on this panel.', icon: UserRound },
    { id: 'groups', label: 'Groups', help: 'Only accounts in the groups you pick (admins always get in).', icon: Tag },
  ] as const;
</script>

<label class="field">Name<input bind:value={draft.name} maxlength="48" placeholder="Survival SMP" /></label>
<label class="field">Launcher instance<select bind:value={draft.instance_id}>{#if !route.instanceId}<option value="">Unlinked</option>{/if}{#each instances as instance}<option value={instance.id}>{instance.name}</option>{/each}</select></label>
{#if enabled(experienceContext.instance?.experience, 'maps')}<Toggle
  bind:checked={draft.map_enabled}
  label="Velora Map"
  help="The server plugin/mod draws this world in the background and sends only what changed, with players, guild land and shops. Players get a Live Map button in the launcher and you get it here. Nothing else to install or host."
/>{/if}
{#if enabled(experienceContext.instance?.experience, 'economy')}<label class="field">Shared economy group<input bind:value={draft.economy_group} maxlength="32" placeholder="Leave empty for a separate economy" /><span class="tiny muted">Servers with the same group name share player balances and guild banks, so one wallet follows players across servers in this instance. Existing per-server balances aren't merged.</span></label>{/if}
<div class="field">
  <span class="lbl">Who can join</span>
  <div class="opts">
    {#each accessOptions as o}
      <button type="button" class="opt" class:on={draft.access === o.id} onclick={() => (draft.access = o.id)}>
        <o.icon size={16} /><span><strong>{o.label}</strong><span class="tiny muted">{o.help}</span></span>
      </button>
    {/each}
  </div>
</div>
{#if draft.access === 'groups'}
  <div class="row wrap">
    {#each groups as g}
      {@const on = draft.allowed_groups.includes(g.name)}
      <button type="button" class="gpick" class:on style:--c={g.color} onclick={() => (draft.allowed_groups = on ? draft.allowed_groups.filter((x) => x !== g.name) : [...draft.allowed_groups, g.name])}>
        <span class="dot" style:background={g.color}></span>{g.name}
      </button>
    {:else}
      <span class="muted small">Create groups on the Players page first.</span>
    {/each}
  </div>
{/if}
<Toggle
  bind:checked={draft.require_launcher}
  label="Must join through the launcher"
  help="Accounts are only let in if they launched the game from the Velora launcher from the same IP address in the last 12 hours."
/>

<style>
  .field { display: flex; flex-direction: column; gap: 7px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .opts { display: flex; flex-direction: column; gap: 6px; }
  .opt { justify-content: flex-start; gap: 12px; padding: 10px 12px; text-align: left; white-space: normal; background: var(--bg-2); }
  .opt > span { display: flex; flex-direction: column; gap: 2px; }
  .opt :global(svg) { color: var(--muted); flex-shrink: 0; }
  .opt.on { border-color: color-mix(in srgb, var(--accent) 60%, transparent); background: var(--accent-soft); }
  .opt.on :global(svg) { color: var(--accent-2); }
  .gpick { padding: 6px 12px; border-radius: 99px; font-size: 0.85rem; background: var(--bg-2); }
  .gpick.on { border-color: var(--c); background: color-mix(in srgb, var(--c) 18%, transparent); }
  .dot { width: 10px; height: 10px; border-radius: 50%; }
</style>
