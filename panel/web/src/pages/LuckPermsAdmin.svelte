<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Shield, Plus, Trash2, RefreshCw, Link2, Users, Search, Check, X, TriangleAlert, ArrowRightLeft, Layers, History, Settings2, Lock, ChevronRight,
  } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import McTextInput from '../components/McTextInput.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { api, del, get, post, put, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import { spans } from '../lib/mccolor';
  import { OTHER_NODES, VELORA_NODES } from '../lib/lpNodes';

  type Perm = { key: string; value: boolean; server?: string; world?: string; temporary?: boolean };
  type LpGroup = { name: string; display?: string; weight: number; prefix: string; suffix: string; parents: string[]; permissions: Perm[]; members?: number };
  type PanelGroup = { id: number; name: string; color: string; luckperms_group: string; members: number };
  type Link = { level: number; group: string; source: 'manager' | 'reward' };
  type Command = { id: number; server_id: number; summary: string; actor: string; created_at: string; done_at: string | null; status: 'pending' | 'done' | 'failed'; error: string | null };
  type Server = { id: number; name: string; luckperms: boolean; version: string | null; reported_at: string | null };
  type Overview = {
    servers: Server[]; server_id: number | null; connected: boolean; reported_at: string | null; plugin_version: string | null; can_manage: boolean; manager_supported: boolean;
    groups: LpGroup[]; panel_groups: PanelGroup[]; level_links: Link[]; sync_mode: string; commands: Command[];
  };
  type Player = { uuid: string; name: string; primary: string | null; prefix: string | null; groups: string[]; last_reported: string | null };

  const TABS = [
    { id: 'groups', label: 'Groups', icon: Shield },
    { id: 'levels', label: 'Level milestones', icon: Layers },
    { id: 'panel', label: 'Panel groups', icon: Link2 },
    { id: 'players', label: 'Players', icon: Users },
    { id: 'activity', label: 'Activity', icon: History },
  ] as const;
  const SYNC_MODES = [
    { value: 'off', label: 'Off', help: 'Show ranks only. Nothing is moved between the panel and the game.' },
    { value: 'game_to_panel', label: 'Game → Panel', help: 'Players in a LuckPerms group are added to the panel group it is linked to.' },
    { value: 'panel_to_game', label: 'Panel → Game', help: 'Players in a panel group are added to the LuckPerms group it is linked to (needs apply-panel-groups in the plugin config).' },
    { value: 'both', label: 'Both ways', help: 'Memberships are added in both directions. Nothing is ever removed this way.' },
  ];

  let tab = $state<(typeof TABS)[number]['id']>('groups');
  let data = $state<Overview | null>(null);
  let loadError = $state('');
  let serverId = $state<number | null>(null);
  let selected = $state('');
  let filter = $state('');

  const groups = $derived(data?.groups ?? []);
  const current = $derived(groups.find((g) => g.name === selected) ?? null);
  const shown = $derived(groups.filter((g) => !filter.trim() || g.name.includes(filter.trim().toLowerCase()) || (g.display ?? '').toLowerCase().includes(filter.trim().toLowerCase())));
  const pending = $derived((data?.commands ?? []).filter((c) => c.status === 'pending').length);
  const failed = $derived((data?.commands ?? []).filter((c) => c.status === 'failed').length);
  const canManage = $derived(!!data?.connected && !!data.can_manage);
  const groupNames = $derived(groups.map((g) => g.name));
  const linkedPanel = (name: string) => (data?.panel_groups ?? []).filter((p) => p.luckperms_group === name);
  const linkedLevels = (name: string) => (data?.level_links ?? []).filter((l) => l.group === name);

  async function load(quiet = false) {
    try {
      const d = await get<Overview>('/api/admin/luckperms' + (serverId ? `?server_id=${serverId}` : ''));
      data = d;
      serverId = d.server_id;
      loadError = '';
      if (!d.groups.some((g) => g.name === selected)) selected = d.groups[0]?.name ?? '';
      if (!levelDirty) levelRows = d.level_links.filter((l) => l.source === 'manager').map((l) => ({ level: l.level, group: l.group }));
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e);
      if (!quiet) toastError(e);
    }
  }
  // While instructions are waiting, look again every few seconds so the result appears by itself.
  onMount(() => {
    load();
    const timer = setInterval(() => load(true), 5000);
    return () => clearInterval(timer);
  });

  /** Run an instruction and refresh. Returns whether it was accepted. */
  async function send(label: string, run: () => Promise<unknown>): Promise<boolean> {
    try {
      await run();
      toast(`${label} — sent to the game server`);
      await load(true);
      return true;
    } catch (e) {
      toastError(e);
      return false;
    }
  }
  const sid = () => (serverId ? { server_id: serverId } : {});

  // ---- create group -------------------------------------------------------------------------------------------
  let createOpen = $state(false);
  let draft = $state({ name: '', display: '', weight: 10, prefix: '', suffix: '', parents: [] as string[], permissions: '' });
  function openCreate() {
    draft = { name: '', display: '', weight: 10, prefix: '', suffix: '', parents: groupNames.includes('default') ? ['default'] : [], permissions: '' };
    createOpen = true;
  }
  async function create() {
    const permissions = draft.permissions.split('\n').map((l) => l.trim()).filter(Boolean).map((l) => (l.startsWith('-') ? { permission: l.slice(1).trim(), value: false } : { permission: l.replace(/^\+/, '').trim(), value: true }));
    const name = draft.name.trim().toLowerCase();
    if (await send(`Group ${name} created`, () => post('/api/admin/luckperms/groups', { ...sid(), ...draft, name, permissions }))) {
      createOpen = false;
      selected = name;
      tab = 'groups';
    }
  }
  function toggleParent(name: string) {
    draft.parents = draft.parents.includes(name) ? draft.parents.filter((p) => p !== name) : [...draft.parents, name];
  }

  // ---- edit group ---------------------------------------------------------------------------------------------
  let edit = $state({ display: '', weight: 0, prefix: '', suffix: '' });
  let editFor = '';
  $effect(() => {
    // Reload the editor fields when another group is picked, not on every refresh (that would undo typing).
    if (current && current.name !== editFor) {
      editFor = current.name;
      edit = { display: current.display ?? '', weight: current.weight, prefix: current.prefix, suffix: current.suffix };
    }
  });
  const editDirty = $derived(!!current && (edit.display !== (current.display ?? '') || edit.weight !== current.weight || edit.prefix !== current.prefix || edit.suffix !== current.suffix));
  async function saveGroup() {
    if (!current) return;
    await send(`Saved ${current.name}`, () => put(`/api/admin/luckperms/groups/${current.name}`, { ...sid(), ...edit, weight: Number(edit.weight) || 0 }));
    editFor = '';
  }

  let perm = $state({ permission: '', value: true, server: '', world: '' });
  let permAdvanced = $state(false);
  const permHints = $derived([...VELORA_NODES, ...OTHER_NODES].filter((n) => !perm.permission || n.includes(perm.permission.toLowerCase())).slice(0, 8));
  async function addPermission() {
    if (!current || !perm.permission.trim()) return;
    if (await send(`${perm.value ? 'Allowed' : 'Denied'} ${perm.permission.trim()}`, () => post(`/api/admin/luckperms/groups/${current.name}/permissions`, { ...sid(), ...perm, permission: perm.permission.trim() }))) {
      perm = { permission: '', value: true, server: '', world: '' };
    }
  }
  const removePermission = (g: LpGroup, p: Perm) =>
    send(`Removed ${p.key}`, () => api_delete(`/api/admin/luckperms/groups/${g.name}/permissions`, { ...sid(), permission: p.key, server: p.server ?? '', world: p.world ?? '' }));
  const flipPermission = (g: LpGroup, p: Perm) =>
    send(`${p.value ? 'Denied' : 'Allowed'} ${p.key}`, () => post(`/api/admin/luckperms/groups/${g.name}/permissions`, { ...sid(), permission: p.key, value: !p.value, server: p.server ?? '', world: p.world ?? '' }));
  // DELETE with a JSON body: the shared `del` helper does not take one.
  const api_delete = (path: string, body: unknown) => api(path, { method: 'DELETE', body });

  let parentPick = $state('');
  async function addParent() {
    if (!current || !parentPick) return;
    if (await send(`${current.name} now inherits ${parentPick}`, () => post(`/api/admin/luckperms/groups/${current.name}/parents`, { ...sid(), parent: parentPick }))) parentPick = '';
  }
  const removeParent = (g: LpGroup, parent: string) => send(`Removed ${parent}`, () => del(`/api/admin/luckperms/groups/${g.name}/parents/${parent}${serverId ? `?server_id=${serverId}` : ''}`));

  let deleteOpen = $state(false);
  async function removeGroup() {
    if (!current) return;
    const name = current.name;
    if (await send(`Group ${name} deleted`, () => del(`/api/admin/luckperms/groups/${name}${serverId ? `?server_id=${serverId}` : ''}`))) deleteOpen = false;
  }

  // ---- level milestones ---------------------------------------------------------------------------------------
  let levelRows = $state<{ level: number; group: string }[]>([]);
  let levelDirty = $state(false);
  const touchLevels = () => (levelDirty = true);
  const addLevelRow = () => { levelRows.push({ level: (levelRows.at(-1)?.level ?? 0) + 10, group: '' }); touchLevels(); };
  const dropLevelRow = (i: number) => { levelRows.splice(i, 1); touchLevels(); };
  async function saveLevels() {
    const links = levelRows.filter((r) => r.group.trim()).map((r) => ({ level: Number(r.level), group: r.group.trim() }));
    try {
      await put('/api/admin/luckperms/level-links', { links });
      levelDirty = false;
      toast('Milestones saved. Players are moved the next time their server reports (within a minute).');
      await load(true);
    } catch (e) {
      toastError(e);
    }
  }

  // ---- panel groups & sync ------------------------------------------------------------------------------------
  async function linkPanelGroup(g: PanelGroup, value: string) {
    try {
      await put(`/api/admin/groups/${g.id}/luckperms`, { luckperms_group: value });
      toast(value ? `${g.name} follows ${value}` : `${g.name} is no longer linked`);
      await load(true);
    } catch (e) {
      toastError(e);
      await load(true);
    }
  }
  async function setSyncMode(mode: string) {
    try {
      await put('/api/admin/integrations/settings', { luckperms_sync: mode });
      toast('Sync mode saved');
      await load(true);
    } catch (e) {
      toastError(e);
    }
  }

  // ---- players ------------------------------------------------------------------------------------------------
  let playerQuery = $state('');
  let players = $state<Player[] | null>(null);
  let chosen = $state<string[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;
  async function loadPlayers() {
    try {
      players = (await get<{ players: Player[] }>(`/api/admin/luckperms/players?q=${encodeURIComponent(playerQuery.trim())}`)).players;
    } catch (e) {
      toastError(e);
    }
  }
  $effect(() => {
    if (tab !== 'players') return;
    void playerQuery;
    clearTimeout(timer);
    timer = setTimeout(loadPlayers, 250);
    return () => clearTimeout(timer);
  });
  let moveOpen = $state(false);
  let moveTarget = $state<Player | null>(null);
  let moveAdd = $state<string[]>([]);
  let moveRemove = $state<string[]>([]);
  function openMove(p: Player) {
    moveTarget = p;
    moveAdd = [];
    moveRemove = [];
    moveOpen = true;
  }
  const toggleIn = (list: string[], v: string) => (list.includes(v) ? list.filter((x) => x !== v) : [...list, v]);
  async function doMove() {
    if (!moveTarget) return;
    const target = moveTarget;
    if (await send(`Updated ${target.name}`, () => post(`/api/admin/luckperms/players/${target.uuid}/groups`, { ...sid(), add: moveAdd, remove: moveRemove }))) moveOpen = false;
  }
  let bulkFrom = $state('');
  let bulkTo = $state('');
  async function doBulk() {
    if (!bulkFrom || !bulkTo || !chosen.length) return;
    if (await send(`Moved ${chosen.length} player${chosen.length === 1 ? '' : 's'}`, () => post('/api/admin/luckperms/players/bulk-move', { ...sid(), from: bulkFrom, to: bulkTo, uuids: chosen }))) chosen = [];
  }

  // ---- activity -----------------------------------------------------------------------------------------------
  const retry = (c: Command) => send('Retrying', () => post(`/api/admin/luckperms/commands/${c.id}/retry`, {}));
  const clearLog = () => send('Log cleared', () => del('/api/admin/luckperms/commands'));

  const statusBadge = { pending: 'warn', done: 'good', failed: 'bad' } as const;
</script>

{#snippet mc(text: string)}
  <span class="mc">{#each spans(text || '') as s}<span style:color={s.color} class:b={s.bold} class:i={s.italic} class:u={s.underline} class:st={s.strike}>{s.text}</span>{/each}</span>
{/snippet}

<div class="page">
  <div class="page-head">
    <div>
      <h1>LuckPerms</h1>
      <p>Create ranks, set their prefixes and permissions, grant them at level milestones and move players. Changes are carried out in game within seconds.</p>
    </div>
    <div class="row wrap">
      {#if data && data.servers.length > 1}
        <select class="server-pick" bind:value={serverId} onchange={() => load()} aria-label="Game server">
          {#each data.servers as s (s.id)}<option value={s.id}>{s.name}{s.luckperms ? '' : ' (no LuckPerms)'}</option>{/each}
        </select>
      {/if}
      <button class="ghost icon" onclick={() => load()} aria-label="Refresh" title="Refresh"><RefreshCw size={16} /></button>
      <button class="primary" onclick={openCreate} disabled={!canManage}><Plus size={16} /> New group</button>
    </div>
  </div>

  {#if loadError}
    <div class="notice bad"><TriangleAlert size={18} /><div><b>Could not load LuckPerms.</b> {loadError}</div></div>
  {:else if data && !data.servers.length}
    <div class="notice"><TriangleAlert size={18} /><div><b>No game server yet.</b> Add one on the Servers page and install the Velora plugin next to LuckPerms.</div></div>
  {:else if data && !data.connected}
    <div class="notice"><TriangleAlert size={18} /><div><b>This server has not reported LuckPerms.</b> Install the Velora plugin next to LuckPerms and start the server. Groups appear here within a minute.</div></div>
  {:else if data && !data.manager_supported}
    <div class="notice"><Lock size={18} /><div><b>Read-only: the Velora plugin on this server is out of date.</b> Replace the plugin jar with the latest build and restart (or reload) the server. This page lights up on the plugin's next report, within a minute. If you just updated it, check that the old jar is gone from <code>plugins/</code>.</div></div>
  {:else if data && !data.can_manage}
    <div class="notice"><Lock size={18} /><div><b>Read-only.</b> The plugin has <code>integrations.luckperms.allow-panel-commands</code> set to <code>false</code> in its <code>config.yml</code>. Set it to <code>true</code> and run <code>/scopenet reload</code> (or restart), and this page unlocks within a minute.</div></div>
  {:else if data}
    <div class="status row wrap">
      <span class="badge good"><Check size={12} /> Connected</span>
      <span class="muted small">Plugin reported {timeAgo(data.reported_at)} · LuckPerms {data.plugin_version}</span>
      {#if pending}<span class="badge warn">{pending} waiting for the game server</span>{/if}
      {#if failed}<button class="badge bad linkish" onclick={() => (tab = 'activity')}>{failed} failed — see Activity</button>{/if}
    </div>
  {/if}

  <div class="tabs" role="tablist">
    {#each TABS as t (t.id)}
      <button role="tab" aria-selected={tab === t.id} class:active={tab === t.id} onclick={() => (tab = t.id)}><t.icon size={15} /> {t.label}{#if t.id === 'activity' && failed}<span class="dot"></span>{/if}</button>
    {/each}
  </div>

  {#if tab === 'groups'}
    <div class="split">
      <aside class="card list">
        <label class="search"><Search size={15} /><input bind:value={filter} placeholder="Find a group…" aria-label="Find a group" /></label>
        {#if !data}
          <p class="muted small pad">Loading…</p>
        {:else if !groups.length}
          <p class="muted small pad">No groups reported yet.</p>
        {/if}
        {#each shown as g (g.name)}
          <button class="item" class:on={g.name === selected} onclick={() => (selected = g.name)}>
            <span class="meta">
              <b>{g.display || g.name}</b>
              <small>{g.name} · weight {g.weight}{#if g.members !== undefined} · {g.members} member{g.members === 1 ? '' : 's'}{/if}</small>
              {#if g.prefix}{@render mc(g.prefix + (g.display || g.name))}{/if}
            </span>
            <ChevronRight size={14} />
          </button>
        {/each}
      </aside>

      <section class="editor">
        {#if !current}
          <div class="card empty"><Shield size={32} /><h3>{groups.length ? 'Pick a group' : 'No groups yet'}</h3><p>{groups.length ? 'Choose a group on the left to edit it.' : 'Create the first group with “New group”.'}</p></div>
        {:else}
          <div class="card col">
            <div class="section-title"><h3>{current.display || current.name}</h3><span class="badge">{current.name}</span>
              <span class="hint">
                {#each linkedPanel(current.name) as p (p.id)}<span class="badge accent"><Link2 size={11} /> {p.name}</span> {/each}
                {#each linkedLevels(current.name) as l}<span class="badge accent"><Layers size={11} /> Level {l.level}</span> {/each}
              </span>
            </div>
            <div class="grid two">
              <label class="field">Display name<input bind:value={edit.display} maxlength="64" placeholder={current.name} disabled={!canManage} /></label>
              <label class="field">Weight<input type="number" bind:value={edit.weight} disabled={!canManage} /><span class="help">Higher weight wins when a player has several groups (and decides prefix order).</span></label>
            </div>
            <div class="grid two">
              <div class="field"><span>Prefix</span><McTextInput bind:value={edit.prefix} placeholder="&6[VIP] " compact showPreview={false} /></div>
              <div class="field"><span>Suffix</span><McTextInput bind:value={edit.suffix} placeholder=" &7★" compact showPreview={false} /></div>
            </div>
            <div class="chat-preview" aria-label="Chat preview">
              {@render mc(edit.prefix)}<span class="name">Steve</span>{@render mc(edit.suffix)}<span class="msg">: hello everyone!</span>
            </div>
            <div class="row"><button class="primary" onclick={saveGroup} disabled={!editDirty || !canManage}>Save changes</button>
              <span class="spacer"></span>
              <button class="danger" onclick={() => (deleteOpen = true)} disabled={!canManage || current.name === 'default'}><Trash2 size={15} /> Delete group</button></div>
          </div>

          <div class="card col">
            <div class="section-title"><h3>Inherits from</h3><span class="hint">Everything these groups can do, this group can do too.</span></div>
            <div class="chips">
              {#each current.parents as parent}
                <span class="chip"><b>{parent}</b>{#if canManage}<button class="x" aria-label="Remove {parent}" onclick={() => removeParent(current, parent)}><X size={12} /></button>{/if}</span>
              {:else}<span class="muted small">Nothing yet.</span>{/each}
            </div>
            {#if canManage}
              <div class="row">
                <select bind:value={parentPick} aria-label="Group to inherit from">
                  <option value="">Add a parent group…</option>
                  {#each groupNames.filter((n) => n !== current.name && !current.parents.includes(n)) as n}<option value={n}>{n}</option>{/each}
                </select>
                <button onclick={addParent} disabled={!parentPick}>Add</button>
              </div>
            {/if}
          </div>

          <div class="card col">
            <div class="section-title"><h3>Permissions</h3><span class="hint">{current.permissions.length} set on this group</span></div>
            {#if canManage}
              <form class="perm-form" onsubmit={(e) => { e.preventDefault(); addPermission(); }}>
                <div class="combo">
                  <input bind:value={perm.permission} placeholder="scopenet.command.guild.create" list="lp-nodes" aria-label="Permission" autocomplete="off" spellcheck="false" />
                  <datalist id="lp-nodes">{#each permHints as n}<option value={n}></option>{/each}</datalist>
                </div>
                <div class="segmented" role="group" aria-label="Allow or deny">
                  <button type="button" class:active={perm.value} onclick={() => (perm.value = true)}>Allow</button>
                  <button type="button" class:active={!perm.value} onclick={() => (perm.value = false)}>Deny</button>
                </div>
                <button class="primary" type="submit" disabled={!perm.permission.trim()}><Plus size={15} /> Add</button>
                <button type="button" class="ghost sm" onclick={() => (permAdvanced = !permAdvanced)}><Settings2 size={14} /> Context</button>
              </form>
              {#if permAdvanced}
                <div class="grid two">
                  <label class="field">Only on server<input bind:value={perm.server} placeholder="all servers" maxlength="64" /></label>
                  <label class="field">Only in world<input bind:value={perm.world} placeholder="all worlds" maxlength="64" /></label>
                </div>
              {/if}
            {/if}
            {#if current.permissions.length}
              <table class="table">
                <thead><tr><th>Permission</th><th>Applies</th><th></th></tr></thead>
                <tbody>
                  {#each current.permissions as p (p.key + (p.server ?? '') + (p.world ?? ''))}
                    <tr>
                      <td><code class:denied={!p.value}>{p.key}</code>{#if p.temporary}<span class="badge warn">temporary</span>{/if}</td>
                      <td>
                        <button class="badge perm-state {p.value ? 'good' : 'bad'}" disabled={!canManage} onclick={() => flipPermission(current, p)} title="Click to switch">{p.value ? 'Allowed' : 'Denied'}</button>
                        {#if p.server}<span class="badge">server: {p.server}</span>{/if}{#if p.world}<span class="badge">world: {p.world}</span>{/if}
                      </td>
                      <td class="end">{#if canManage}<button class="ghost icon" aria-label="Remove {p.key}" onclick={() => removePermission(current, p)}><Trash2 size={15} /></button>{/if}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <p class="muted small">No permissions set directly. {current.parents.length ? 'It still gets everything from its parents.' : ''}</p>
            {/if}
          </div>
        {/if}
      </section>
    </div>

  {:else if tab === 'levels'}
    <div class="card col">
      <div class="section-title"><h3>Level milestones</h3><span class="hint">Reaching a level moves the player into that group.</span></div>
      <p class="muted small">A player is in the group of the highest milestone they have reached, and leaves the groups of the others. Only groups listed here are ever touched. They are applied when the player's server next reports (about every minute).</p>
      {#each levelRows as row, i}
        <div class="level-row">
          <label class="field">Level<input type="number" min="1" max="10000" bind:value={row.level} oninput={touchLevels} /></label>
          <label class="field">Group
            <select bind:value={row.group} onchange={touchLevels}>
              <option value="">Choose a group…</option>
              {#each groupNames as n}<option value={n}>{n}</option>{/each}
              {#if row.group && !groupNames.includes(row.group)}<option value={row.group}>{row.group}</option>{/if}
            </select>
          </label>
          <button class="ghost icon" aria-label="Remove milestone" onclick={() => dropLevelRow(i)}><Trash2 size={16} /></button>
        </div>
      {:else}
        <p class="muted small">No milestones yet.</p>
      {/each}
      <div class="row">
        <button onclick={addLevelRow}><Plus size={15} /> Add milestone</button>
        <span class="spacer"></span>
        <button class="primary" onclick={saveLevels} disabled={!levelDirty}>Save milestones</button>
      </div>
      {#if data?.level_links.some((l) => l.source === 'reward')}
        <div class="sep"></div>
        <p class="small"><b>Also set on level rewards</b> <span class="muted">(Leveling &amp; XP). Edit these there; a milestone above for the same level wins.</span></p>
        <div class="chips">{#each data.level_links.filter((l) => l.source === 'reward') as l}<span class="chip">Level <b>{l.level}</b> → {l.group}</span>{/each}</div>
      {/if}
    </div>

  {:else if tab === 'panel'}
    <div class="card col">
      <div class="section-title"><h3>Sync mode</h3></div>
      <div class="modes">
        {#each SYNC_MODES as m}
          <button class="mode" class:on={data?.sync_mode === m.value} onclick={() => setSyncMode(m.value)}><b>{m.label}</b><small>{m.help}</small></button>
        {/each}
      </div>
    </div>
    <div class="card col">
      <div class="section-title"><h3>Panel groups</h3><span class="hint">Link a panel group to the LuckPerms group it follows.</span></div>
      {#if data && !data.panel_groups.length}
        <p class="muted small">There are no panel groups yet. Create them on the Players page.</p>
      {/if}
      <table class="table">
        <thead><tr><th>Panel group</th><th>Members</th><th>Follows LuckPerms group</th></tr></thead>
        <tbody>
          {#each data?.panel_groups ?? [] as g (g.id)}
            <tr>
              <td><span class="swatch" style:background={g.color}></span> <b>{g.name}</b></td>
              <td>{g.members}</td>
              <td>
                <select value={g.luckperms_group} onchange={(e) => linkPanelGroup(g, e.currentTarget.value)} aria-label="LuckPerms group for {g.name}">
                  <option value="">Not linked</option>
                  {#each groupNames as n}<option value={n}>{n}</option>{/each}
                  {#if g.luckperms_group && !groupNames.includes(g.luckperms_group)}<option value={g.luckperms_group}>{g.luckperms_group}</option>{/if}
                </select>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

  {:else if tab === 'players'}
    <div class="card col">
      <div class="row wrap">
        <label class="search grow"><Search size={15} /><input bind:value={playerQuery} placeholder="Find a player…" aria-label="Find a player" /></label>
        {#if chosen.length}
          <div class="bulk row wrap">
            <b>{chosen.length} selected</b>
            <select bind:value={bulkFrom} aria-label="Move from"><option value="">from…</option>{#each groupNames as n}<option value={n}>{n}</option>{/each}</select>
            <ArrowRightLeft size={15} />
            <select bind:value={bulkTo} aria-label="Move to"><option value="">to…</option>{#each groupNames as n}<option value={n}>{n}</option>{/each}</select>
            <button class="primary sm" onclick={doBulk} disabled={!bulkFrom || !bulkTo || !canManage}>Move</button>
            <button class="ghost sm" onclick={() => (chosen = [])}>Clear</button>
          </div>
        {/if}
      </div>
      {#if players && !players.length}
        <p class="muted small">No players match.</p>
      {:else if players}
        <table class="table">
          <thead><tr><th></th><th>Player</th><th>Groups</th><th></th></tr></thead>
          <tbody>
            {#each players as p (p.uuid)}
              <tr>
                <td><input type="checkbox" checked={chosen.includes(p.uuid)} onchange={() => (chosen = toggleIn(chosen, p.uuid))} aria-label="Select {p.name}" /></td>
                <td><b>{p.name}</b>{#if p.prefix}<div>{@render mc(p.prefix)}</div>{/if}</td>
                <td>
                  <div class="chips">
                    {#each p.groups as g}<span class="chip" class:primary={g === p.primary}>{g}</span>{:else}<span class="muted small">{p.last_reported ? 'No groups' : 'Not seen in game yet'}</span>{/each}
                  </div>
                  {#if p.last_reported}<small class="muted">as of {timeAgo(p.last_reported)}</small>{/if}
                </td>
                <td class="end"><button class="sm" onclick={() => openMove(p)} disabled={!canManage}><ArrowRightLeft size={14} /> Change groups</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted small">Loading…</p>
      {/if}
      <p class="muted small">Groups shown are what the game last reported for the player, so they can be a little behind. Moving works even when the player is offline.</p>
    </div>

  {:else}
    <div class="card col">
      <div class="section-title"><h3>Activity</h3><span class="hint">What was sent to the game server and whether it worked.</span>
        <button class="ghost sm" onclick={clearLog} disabled={!data?.commands.some((c) => c.status !== 'pending')}>Clear finished</button></div>
      {#if data && !data.commands.length}
        <p class="muted small">Nothing yet. Changes you make on this page show up here.</p>
      {:else}
        <table class="table">
          <thead><tr><th>What</th><th>By</th><th>When</th><th>Result</th><th></th></tr></thead>
          <tbody>
            {#each data?.commands ?? [] as c (c.id)}
              <tr>
                <td>{c.summary}</td><td>{c.actor}</td><td class="muted">{timeAgo(c.created_at)}</td>
                <td><span class="badge {statusBadge[c.status]}">{c.status === 'pending' ? 'Waiting' : c.status === 'done' ? 'Done' : 'Failed'}</span>{#if c.error}<div class="small err">{c.error}</div>{/if}</td>
                <td class="end">{#if c.status === 'failed'}<button class="sm" onclick={() => retry(c)}>Retry</button>{/if}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  {/if}
</div>

<Modal bind:open={createOpen} title="New group" width={620}>
  <div class="grid two">
    <label class="field">Name<input bind:value={draft.name} placeholder="vip" maxlength="64" autocomplete="off" /><span class="help">Letters, digits, _ - and . Used in commands and mappings.</span></label>
    <label class="field">Display name<input bind:value={draft.display} placeholder="VIP" maxlength="64" /></label>
  </div>
  <label class="field">Weight<input type="number" bind:value={draft.weight} /><span class="help">Higher is more important. Ranks usually go 10, 20, 30…</span></label>
  <div class="grid two">
    <div class="field"><span>Prefix</span><McTextInput bind:value={draft.prefix} placeholder="&6[VIP] " compact showPreview={false} /></div>
    <div class="field"><span>Suffix</span><McTextInput bind:value={draft.suffix} placeholder=" &7★" compact showPreview={false} /></div>
  </div>
  <div class="chat-preview">{@render mc(draft.prefix)}<span class="name">Steve</span>{@render mc(draft.suffix)}<span class="msg">: hello everyone!</span></div>
  {#if groupNames.length}
    <div class="field"><span>Inherits from</span>
      <div class="chips">{#each groupNames as n}<button type="button" class="chip pick" class:on={draft.parents.includes(n)} onclick={() => toggleParent(n)}>{n}</button>{/each}</div>
    </div>
  {/if}
  <label class="field">Permissions<textarea bind:value={draft.permissions} rows="4" placeholder={'scopenet.command.guild.create\n-essentials.fly   (a leading - denies it)'} spellcheck="false"></textarea><span class="help">One per line. You can add more later.</span></label>
  {#snippet footer()}
    <button onclick={() => (createOpen = false)}>Cancel</button>
    <button class="primary" onclick={create} disabled={!draft.name.trim()}>Create group</button>
  {/snippet}
</Modal>

<Modal bind:open={deleteOpen} title="Delete {current?.name ?? 'group'}?" width={460}>
  <p>Players in <b>{current?.name}</b> lose it, and its permissions are gone. Level milestones and panel links to it are removed too. This cannot be undone.</p>
  {#snippet footer()}
    <button onclick={() => (deleteOpen = false)}>Keep it</button>
    <button class="danger" onclick={removeGroup}><Trash2 size={15} /> Delete group</button>
  {/snippet}
</Modal>

<Modal bind:open={moveOpen} title="Change groups: {moveTarget?.name ?? ''}" width={560}>
  <div class="field"><span>Currently in</span>
    <div class="chips">{#each moveTarget?.groups ?? [] as g}<button type="button" class="chip pick" class:remove={moveRemove.includes(g)} onclick={() => (moveRemove = toggleIn(moveRemove, g))} title="Click to take them out of this group">{g}{#if moveRemove.includes(g)} ✕{/if}</button>{:else}<span class="muted small">Unknown (not seen in game yet).</span>{/each}</div>
    <span class="help">Click a group to take the player out of it.</span>
  </div>
  <div class="field"><span>Add to</span>
    <div class="chips">{#each groupNames.filter((n) => !(moveTarget?.groups ?? []).includes(n)) as n}<button type="button" class="chip pick" class:on={moveAdd.includes(n)} onclick={() => (moveAdd = toggleIn(moveAdd, n))}>{n}</button>{/each}</div>
  </div>
  {#snippet footer()}
    <button onclick={() => (moveOpen = false)}>Cancel</button>
    <button class="primary" onclick={doMove} disabled={!moveAdd.length && !moveRemove.length}>Apply</button>
  {/snippet}
</Modal>

<style>
  .page-head > div:first-child { flex: 1 1 420px; }
  .notice { display: flex; gap: 12px; align-items: flex-start; padding: 14px 16px; border-radius: var(--radius); border: 1px solid rgba(251, 191, 36, 0.28); background: rgba(251, 191, 36, 0.07); color: var(--text-2); margin-bottom: 20px; }
  .notice.bad { border-color: rgba(244, 63, 94, 0.3); background: rgba(244, 63, 94, 0.07); }
  .notice :global(svg) { margin-top: 2px; color: #fcd34d; }
  .notice.bad :global(svg) { color: #fda4af; }
  .status { margin-bottom: 20px; gap: 10px; }
  .server-pick { width: auto; min-width: 160px; }
  .linkish { cursor: pointer; font: inherit; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--bad); margin-left: 4px; }
  .tabs :global(svg) { margin-right: 6px; vertical-align: -2px; }
  .tabs button { white-space: nowrap; }

  .split { display: grid; grid-template-columns: 300px minmax(0, 1fr); gap: 20px; align-items: start; }
  .editor { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .list { padding: 12px; display: flex; flex-direction: column; gap: 4px; position: sticky; top: 16px; max-height: calc(100vh - 120px); overflow: auto; }
  .search { display: flex; align-items: center; gap: 8px; padding: 0 12px; border: 1px solid var(--line-strong); border-radius: var(--radius-sm); background: var(--bg-2); }
  .search input { border: none; background: transparent; padding: 9px 0; }
  .search input:focus { background: transparent; }
  .search.grow { flex: 1; min-width: 220px; }
  .search :global(svg) { color: var(--muted); }
  .pad { padding: 10px 6px; }
  .item { justify-content: space-between; text-align: left; background: transparent; border: 1px solid transparent; padding: 10px 12px; gap: 10px; }
  .item.on { background: var(--accent-soft); border-color: rgba(109, 106, 245, 0.35); }
  .item .meta { display: flex; flex-direction: column; gap: 2px; min-width: 0; align-items: flex-start; }
  .item small { color: var(--muted); font-weight: 400; }
  .item :global(svg) { color: var(--muted); }

  .col { display: flex; flex-direction: column; gap: 14px; }
  .grid.two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; }
  .field > span:first-child { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .field .help { color: var(--muted); font-weight: 400; font-size: 0.78rem; }
  .chat-preview { padding: 10px 14px; border-radius: 6px; background: #100010f2; border: 2px solid #2a0a5e; font: 14px/1.5 ui-monospace, 'Cascadia Mono', Consolas, monospace; color: #fff; text-shadow: 1px 1px 0 #0007; overflow-wrap: anywhere; }
  .chat-preview .name { color: #ddd; }
  .chat-preview .msg { color: #fff; }
  .mc { font: 13px/1.4 ui-monospace, 'Cascadia Mono', Consolas, monospace; white-space: pre-wrap; text-shadow: 1px 1px 0 #0007; }
  .mc .b { font-weight: 700; } .mc .i { font-style: italic; } .mc .u { text-decoration: underline; } .mc .st { text-decoration: line-through; }

  .chips { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chip { display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; border-radius: 999px; font-size: 0.8rem; background: rgba(255, 255, 255, 0.05); border: 1px solid var(--line-strong); color: var(--text-2); }
  .chip.primary { border-color: rgba(109, 106, 245, 0.5); color: #c9c8fb; background: var(--accent-soft); }
  button.chip.pick { padding: 4px 11px; font-weight: 450; cursor: pointer; }
  button.chip.pick.on { background: var(--accent-soft); border-color: var(--accent); color: #fff; }
  button.chip.pick.remove { background: rgba(244, 63, 94, 0.12); border-color: rgba(244, 63, 94, 0.5); color: #fda4af; text-decoration: line-through; }
  .chip .x { padding: 2px; background: transparent; border: none; color: var(--muted); }
  .chip .x:hover { color: var(--bad); }

  .perm-form { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .perm-form .combo { flex: 1; min-width: 240px; }
  .perm-form input { font-family: var(--mono); font-size: 0.88rem; }
  td code.denied { color: #fda4af; text-decoration: line-through; }
  .perm-state { cursor: pointer; font: inherit; font-size: 0.75rem; }
  .perm-state:disabled { cursor: default; opacity: 1; }
  .end { text-align: right; white-space: nowrap; }
  .err { color: #fda4af; margin-top: 4px; max-width: 360px; }
  .sep { height: 1px; background: var(--line); margin: 6px 0; }

  .level-row { display: grid; grid-template-columns: 140px minmax(0, 1fr) auto; gap: 12px; align-items: end; }
  .modes { display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 10px; }
  .mode { flex-direction: column; align-items: flex-start; text-align: left; gap: 4px; padding: 14px; background: var(--bg-2); }
  .mode small { color: var(--muted); font-weight: 400; }
  .mode.on { border-color: var(--accent); background: var(--accent-soft); }
  .swatch { display: inline-block; width: 10px; height: 10px; border-radius: 3px; margin-right: 4px; }
  .bulk { gap: 8px; }
  .bulk select { width: auto; min-width: 130px; }

  @media (max-width: 960px) {
    .split { grid-template-columns: 1fr; }
    .list { position: static; max-height: 260px; }
    .grid.two { grid-template-columns: 1fr; }
    .level-row { grid-template-columns: 1fr auto; }
    .level-row label:first-child { grid-column: 1 / -1; }
  }
</style>
