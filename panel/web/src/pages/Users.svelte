<script lang="ts">
  import { UserPlus, Check, Pencil, Trash2, Shield, Plus, X, Search, Tag } from '@lucide/svelte';
  import Avatar from '../components/Avatar.svelte';
  import Modal from '../components/Modal.svelte';
  import PlayerPanel from '../components/PlayerPanel.svelte';
  import { del, duration, get, patch, post, put, timeAgo } from '../lib/api';
  import { session } from '../lib/session.svelte';
  import { route, go } from '../lib/router.svelte';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Cape, Group, User } from '../lib/types';

  let users = $state<User[]>([]);
  let groups = $state<Group[]>([]);
  let filter = $state('');
  let editing = $state<Partial<User> & { password?: string } | null>(null);
  let editOpen = $state(false);
  let confirm = $state<User | null>(null);
  let confirmOpen = $state(false);
  let newGroup = $state({ name: '', color: '#22d3ee' });
  let capes = $state<Cape[]>([]);
  let viewing = $state<User | null>(null);
  let viewOpen = $state(false);
  let avatarV = $state(0);

  const refresh = () => {
    get<User[]>('/api/admin/users').then((u) => (users = u)).catch(toastError);
    get<Group[]>('/api/admin/groups').then((g) => (groups = g)).catch(toastError);
    get<Cape[]>('/api/admin/capes').then((c) => (capes = c)).catch(() => {});
  };

  function openView(u: User) {
    viewing = u;
    viewOpen = true;
  }
  function updated(u: User) {
    viewing = u;
    users = users.map((x) => (x.id === u.id ? u : x));
    avatarV++;
  }
  const lastSeen = (u: User) => [u.last_login, u.last_seen_ingame].filter(Boolean).sort().at(-1) ?? null;
  $effect(refresh);
  // #/users/<uuid> (a player's name anywhere in the panel) opens their profile.
  $effect(() => {
    const target = route.params[0];
    if (target && users.length) {
      const found = users.find((x) => x.uuid === target);
      if (found) openView(found);
    }
  });
  $effect(() => { if (!viewOpen && route.params[0]) go('users'); });

  const pending = $derived(users.filter((u) => u.status === 'pending'));
  const shown = $derived(users.filter((u) => !filter || u.username.toLowerCase().includes(filter.toLowerCase()) || u.groups.some((g) => g.toLowerCase().includes(filter.toLowerCase()))));
  const colorOf = (name: string) => groups.find((g) => g.name === name)?.color ?? '#8b6cff';

  function openNew() {
    editing = { username: '', password: '', role: 'player', status: 'active', groups: [], email: '' };
    editOpen = true;
  }
  function openEdit(u: User) {
    editing = { ...u, password: '' };
    editOpen = true;
  }

  async function saveUser() {
    if (!editing) return;
    try {
      const body = { username: editing.username, password: editing.password || undefined, role: editing.role, status: editing.status, status_reason: editing.status === 'disabled' ? (editing.status_reason ?? '') : '', groups: editing.groups, email: editing.email ?? '' };
      if (editing.id) await patch(`/api/admin/users/${editing.id}`, body);
      else await post('/api/admin/users', body);
      toast(editing.id ? 'Player updated' : `Created ${editing.username}`);
      editOpen = false;
      refresh();
    } catch (e) {
      toastError(e);
    }
  }

  async function approve(u: User) {
    try {
      await patch(`/api/admin/users/${u.id}`, { status: 'active' });
      toast(`${u.username} approved`);
      refresh();
    } catch (e) {
      toastError(e);
    }
  }

  async function removeUser() {
    if (!confirm) return;
    try {
      const r = await del<{ report?: { removed: Record<string, number> } }>(`/api/admin/users/${confirm.id}`);
      const n = Object.values(r?.report?.removed ?? {}).reduce((a, b) => a + b, 0);
      toast(`Player deleted — ${n.toLocaleString()} records removed`);
      confirmOpen = false;
      refresh();
    } catch (e) {
      toastError(e);
    }
  }

  async function addGroup(e: SubmitEvent) {
    e.preventDefault();
    try {
      await post('/api/admin/groups', newGroup);
      newGroup = { name: '', color: newGroup.color };
      refresh();
    } catch (err) {
      toastError(err);
    }
  }

  async function removeGroup(g: Group) {
    try {
      await del(`/api/admin/groups/${g.id}`);
      refresh();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Players</h1>
      <p>Accounts, UUIDs, skins and capes all live here. Players sign in through the launcher, and your servers verify them with the panel.</p>
    </div>
    <button class="primary" onclick={openNew}><UserPlus size={18} /> Add player</button>
  </div>

  {#if pending.length}
    <section class="card pending">
      <strong>{pending.length} player{pending.length > 1 ? 's' : ''} waiting for approval</strong>
      <div class="row wrap">
        {#each pending as u}
          <span class="pend"><Avatar name={u.username} uuid={u.uuid} size={22} /> {u.username}
            <button class="sm primary" onclick={() => approve(u)}><Check size={14} /> Approve</button>
          </span>
        {/each}
      </div>
    </section>
  {/if}

  <div class="grid layout">
    <section class="card">
      <div class="section-title">
        <div class="search"><Search size={16} /><input bind:value={filter} placeholder="Search players or groups" /></div>
        <span class="hint">{users.length} accounts</span>
      </div>
      <div class="table-wrap">
        <table class="table">
          <thead><tr><th>Player</th><th>Role</th><th>Groups</th><th>Playtime</th><th>Last seen</th><th></th></tr></thead>
          <tbody>
            {#each shown as u (u.id)}
              <tr class="clickable" onclick={(e) => !(e.target as HTMLElement).closest('button') && openView(u)}>
                <td>
                  <div class="row who">
                    <Avatar name={u.username} uuid={u.uuid} size={34} v={avatarV} />
                    <div class="col tight">
                      <strong>{u.username}{#if u.id === session.user?.id} <span class="muted tiny">(you)</span>{/if}</strong>
                      <span class="mono tiny muted">{u.uuid}</span>
                    </div>
                  </div>
                </td>
                <td>
                  {#if u.role === 'admin'}<span class="badge accent"><Shield size={11} /> Admin</span>{:else}<span class="badge">Player</span>{/if}
                  {#if u.status === 'pending'}<span class="badge warn">Pending</span>{:else if u.status === 'disabled'}<span class="badge bad">Disabled</span>{/if}
                </td>
                <td><div class="row wrap chips">{#each u.groups as g}<span class="gchip" style:--c={colorOf(g)}>{g}</span>{/each}</div></td>
                <td class="small num">{u.playtime_secs ? duration(u.playtime_secs) : '–'}</td>
                <td class="muted small">{timeAgo(lastSeen(u))}</td>
                <td class="actions">
                  <button class="ghost icon" aria-label="Edit {u.username}" onclick={() => openEdit(u)}><Pencil size={15} /></button>
                  {#if u.id !== session.user?.id}
                    <button class="ghost icon" aria-label="Delete {u.username}" onclick={() => { confirm = u; confirmOpen = true; }}><Trash2 size={15} /></button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>

    <section class="card col">
      <div class="row"><Tag size={18} /><h3>Groups</h3></div>
      <p class="muted small">Use groups to give VIPs, staff or testers access to private instances.</p>
      <div class="col tight">
        {#each groups as g (g.id)}
          <div class="group">
            <span class="dot" style:background={g.color}></span>
            <strong>{g.name}</strong>
            <span class="muted tiny">{g.members} member{g.members === 1 ? '' : 's'}</span>
            <span class="spacer"></span>
            <button class="ghost icon" aria-label="Delete group {g.name}" onclick={() => removeGroup(g)}><X size={14} /></button>
          </div>
        {/each}
      </div>
      <form class="row" onsubmit={addGroup}>
        <input type="color" bind:value={newGroup.color} aria-label="Group colour" />
        <input bind:value={newGroup.name} placeholder="New group" maxlength="32" />
        <button class="icon" disabled={!newGroup.name.trim()} aria-label="Add group"><Plus size={16} /></button>
      </form>
    </section>
  </div>
</div>

<Modal bind:open={editOpen} title={editing?.id ? `Edit ${editing.username}` : 'Add player'}>
  {#if editing}
    {#if !editing.id}
      <label class="field">Username <span class="help">3–16 letters, numbers or _. This is their in-game name.</span><input bind:value={editing.username} /></label>
    {/if}
    <label class="field">{editing.id ? 'New password' : 'Password'} <span class="help">{editing.id ? 'Leave empty to keep the current password.' : 'At least 8 characters.'}</span>
      <input type="password" bind:value={editing.password} autocomplete="new-password" />
    </label>
    <label class="field">Email (optional)<input type="email" bind:value={editing.email} /></label>
    <div class="grid two">
      <label class="field">Role
        <select bind:value={editing.role}><option value="player">Player</option><option value="admin">Admin</option></select>
      </label>
      <label class="field">Status
        <select bind:value={editing.status}><option value="active">Active</option><option value="pending">Pending approval</option><option value="disabled">Disabled</option></select>
      </label>
    </div>
    {#if editing.status === 'disabled'}
      <label class="field">Reason <span class="help">Shown to the player in the launcher and when a server refuses them. Online players are kicked within 30 seconds.</span>
        <input bind:value={editing.status_reason} maxlength="200" placeholder="e.g. Griefing at spawn — appeal on Discord" />
      </label>
    {/if}
    {#if groups.length}
      <div class="field">
        <span class="lbl">Groups</span>
        <div class="row wrap">
          {#each groups as g}
            {@const on = editing.groups?.includes(g.name)}
            <button type="button" class="gpick" class:on style:--c={g.color}
              onclick={() => editing && (editing.groups = on ? editing.groups!.filter((x) => x !== g.name) : [...(editing.groups ?? []), g.name])}>
              <span class="dot" style:background={g.color}></span>{g.name}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
  {#snippet footer()}
    <button class="ghost" onclick={() => (editOpen = false)}>Cancel</button>
    <button class="primary" onclick={saveUser}>{editing?.id ? 'Save' : 'Create player'}</button>
  {/snippet}
</Modal>

<Modal bind:open={viewOpen} title={viewing?.username ?? ''} width={780}>
  {#if viewing}
    <div class="row head">
      <span class="mono tiny muted">{viewing.uuid}</span>
      {#if viewing.status === 'disabled'}<span class="badge bad">Disabled{viewing.status_reason ? ` · ${viewing.status_reason}` : ''}</span>{/if}
    </div>
    <PlayerPanel user={viewing} {capes} onchange={updated} />
  {/if}
  {#snippet footer()}
    <button class="ghost" onclick={() => { if (viewing) { viewOpen = false; openEdit(viewing); } }}><Pencil size={15} /> Edit account</button>
    <button class="primary" onclick={() => (viewOpen = false)}>Done</button>
  {/snippet}
</Modal>

<Modal bind:open={confirmOpen} title="Delete {confirm?.username}?">
  <p class="muted">This removes the account <strong>and all of its data</strong>: levels, XP, quests, achievements, stats, friends, messages, profile and posts, economy balance and market listings, and guild membership. Guilds they lead pass to the next member (or are dissolved if they're alone). Their name stays reserved so nobody can impersonate them. This can't be undone.</p>
  {#snippet footer()}
    <button class="ghost" onclick={() => (confirmOpen = false)}>Cancel</button>
    <button class="danger" onclick={removeUser}><Trash2 size={16} /> Delete</button>
  {/snippet}
</Modal>

<style>
  .layout { grid-template-columns: 1fr 300px; align-items: start; }
  @media (max-width: 1100px) { .layout { grid-template-columns: 1fr; } }
  .pending { display: flex; flex-direction: column; gap: 12px; margin-bottom: 16px; border-color: rgba(251, 191, 36, 0.3); background: rgba(251, 191, 36, 0.05); }
  .pend { display: inline-flex; align-items: center; gap: 8px; padding: 5px 5px 5px 8px; border-radius: 12px; background: var(--bg-2); border: 1px solid var(--line); }
  .search { display: flex; align-items: center; gap: 8px; flex: 1; max-width: 360px; background: var(--bg-2); border: 1px solid var(--line-strong); border-radius: var(--radius-sm); padding: 0 12px; color: var(--muted); }
  .search input { border: none; background: none; padding: 9px 0; }
  .table-wrap { overflow-x: auto; }
  .who { gap: 12px; }
  .tight { gap: 2px; }
  .chips { gap: 4px; }
  .gchip { font-size: 0.75rem; padding: 2px 8px; border-radius: 99px; background: color-mix(in srgb, var(--c) 16%, transparent); color: color-mix(in srgb, var(--c) 70%, white); border: 1px solid color-mix(in srgb, var(--c) 35%, transparent); }
  .clickable { cursor: pointer; }
  .num { font-variant-numeric: tabular-nums; }
  .head { justify-content: space-between; margin-top: -8px; }
  .actions { white-space: nowrap; text-align: right; }
  .group { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: 10px; background: var(--bg-2); }
  .group { flex-wrap: wrap; }
  .dot { width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }
  form.row input[type='color'] { width: 42px; flex-shrink: 0; }
  .two { grid-template-columns: 1fr 1fr; }
  .field { display: flex; flex-direction: column; gap: 7px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .gpick { padding: 6px 12px; border-radius: 99px; font-size: 0.85rem; background: var(--bg-2); }
  .gpick.on { border-color: var(--c); background: color-mix(in srgb, var(--c) 18%, transparent); }
</style>
