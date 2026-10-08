<script lang="ts">
  import PlayerLink from '../components/PlayerLink.svelte';
  import { onMount } from 'svelte';
  import { Shield, Users, MapPin, Trash2, Search, ExternalLink, Flag, Pencil } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import { get, del, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Guild, Instance } from '../lib/types';

  let guilds = $state<Guild[]>([]);
  let instances = $state<Instance[]>([]);
  let loading = $state(true);
  let search = $state('');
  let selectedInstance = $state<string>('all');
  let deleteConfirm = $state<Guild | null>(null);
  let deleteOpen = $state(false);
  let renaming = $state<Guild | null>(null);
  let renameOpen = $state(false);
  let renameName = $state('');
  let renameTag = $state('');
  let renameBusy = $state(false);

  async function loadData() {
    loading = true;
    try {
      const [g, inst] = await Promise.all([
        get<any[]>('/api/admin/guilds').catch(() => []),
        get<Instance[]>('/api/admin/instances').catch(() => []),
      ]);
      guilds = (g || []).map((x: any) => ({
        id: x.id || '',
        instance_id: x.instance_id || '',
        name: x.name || 'Unnamed Guild',
        tag: x.tag || '',
        description: x.description || '',
        motd: x.motd || '',
        leader_uuid: x.leader_uuid || '',
        icon_url: x.icon_url || null,
        banner_url: x.banner_url || null,
        level: x.level || 1,
        xp: x.xp || 0,
        max_claims: x.max_claims || 10,
        member_count: x.member_count ?? 1,
        max_members: x.max_members || 50,
        claims_count: x.claims_count ?? 0,
        created_at: x.created_at || new Date().toISOString(),
      }));
      instances = inst || [];
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadData();
  });

  let filtered = $derived(
    guilds.filter((g) => {
      if (selectedInstance !== 'all' && g.instance_id !== selectedInstance) return false;
      if (search.trim()) {
        const s = search.toLowerCase();
        return g.name.toLowerCase().includes(s) || g.tag.toLowerCase().includes(s) || g.id.toLowerCase().includes(s);
      }
      return true;
    })
  );

  function openRename(g: Guild) {
    renaming = g; renameName = g.name; renameTag = g.tag; renameOpen = true;
  }

  async function saveRename() {
    if (!renaming || renameBusy) return;
    renameBusy = true;
    try {
      await put(`/api/admin/guilds/${renaming.id}`, { name: renameName.trim(), tag: renameTag.trim() });
      toast(`Guild renamed to [${renameTag.trim().toUpperCase()}] ${renameName.trim()}`);
      renameOpen = false; renaming = null;
      await loadData();
    } catch (e) {
      toastError(e);
    } finally {
      renameBusy = false;
    }
  }

  async function remove(g: Guild) {
    try {
      await del(`/api/admin/guilds/${g.id}`);
      toast(`Guild [${g.tag}] ${g.name} disbanded`);
      deleteConfirm = null;
      await loadData();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page">
  <header>
    <div>
      <h1>Guilds & Land Claims Management</h1>
      <p>View instances' guilds, member counts, active chunk claims, and moderate guilds.</p>
    </div>
  </header>

  <div class="stats-row">
    <div class="stat-card">
      <span class="label">Total Guilds</span>
      <span class="value">{guilds.length}</span>
    </div>
    <div class="stat-card">
      <span class="label">Total Members</span>
      <span class="value text-accent">{guilds.reduce((acc, g) => acc + g.member_count, 0)}</span>
    </div>
    <div class="stat-card">
      <span class="label">Claimed Chunks</span>
      <span class="value text-success">{guilds.reduce((acc, g) => acc + g.claims_count, 0)}</span>
    </div>
  </div>

  <div class="toolbar">
    <div class="search-box">
      <Search size={16} />
      <input type="text" placeholder="Search guilds by name, tag, or id..." bind:value={search} />
    </div>

    <div class="filters">
      <select bind:value={selectedInstance}>
        <option value="all">All Instances</option>
        {#each instances as inst}
          <option value={inst.id}>{inst.name}</option>
        {/each}
      </select>
    </div>
  </div>

  {#if loading}
    <div class="empty">Loading guilds...</div>
  {:else if filtered.length === 0}
    <div class="empty">No guilds found.</div>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Guild</th>
            <th>Tag</th>
            <th>Instance</th>
            <th>Leader</th>
            <th>Members</th>
            <th>Claimed Chunks</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as g}
            <tr>
              <td>
                <div class="guild-cell">
                  {#if g.icon_url}
                    <img src={g.icon_url} alt="" class="guild-avatar" />
                  {:else}
                    <div class="guild-avatar fallback">
                      <Shield size={18} />
                    </div>
                  {/if}
                  <div>
                    <strong>{g.name}</strong>
                    {#if g.description}
                      <p class="guild-desc">{g.description}</p>
                    {/if}
                  </div>
                </div>
              </td>
              <td>
                <span class="tag-badge">[{g.tag}]</span>
              </td>
              <td>
                <code class="instance-code">{g.instance_id}</code>
              </td>
              <td>
                <PlayerLink uuid={g.leader_uuid} name={g.leader_uuid.slice(0, 8) + '…'} />
              </td>
              <td>
                <span class="count-pill">
                  <Users size={13} /> {g.member_count} / {g.max_members}
                </span>
              </td>
              <td>
                <span class="claims-pill">
                  <MapPin size={13} /> {g.claims_count} chunks
                </span>
              </td>
              <td>
                <span class="date-text">{new Date(g.created_at).toLocaleDateString()}</span>
              </td>
              <td>
                <button class="ghost icon" title="Rename guild" aria-label="Rename guild" onclick={() => openRename(g)}>
                  <Pencil size={15} />
                </button>
                <button class="ghost icon danger" title="Disband guild" onclick={() => { deleteConfirm = g; deleteOpen = true; }}>
                  <Trash2 size={15} />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

{#if renaming}
  <Modal bind:open={renameOpen} title="Rename Guild">
    <div class="rename-form">
      <label>Name<input bind:value={renameName} maxlength="32" /></label>
      <label>Tag<input bind:value={renameTag} maxlength="6" /></label>
      <p class="muted small">Players see the new name and tag straight away, in the launcher, the map and in game.</p>
    </div>
    <div class="modal-actions">
      <button class="ghost" onclick={() => { renameOpen = false; renaming = null; }}>Cancel</button>
      <button class="primary" onclick={saveRename} disabled={renameBusy || renameName.trim().length < 3}>Save</button>
    </div>
  </Modal>
{/if}

{#if deleteConfirm}
  <Modal bind:open={deleteOpen} title="Disband Guild">
    <p>Are you sure you want to forcibly disband <strong>[{deleteConfirm.tag}] {deleteConfirm.name}</strong>?</p>
    <p class="warn-note">This will kick all {deleteConfirm.member_count} members and unclaim all {deleteConfirm.claims_count} chunks of land. Whatever is in the guild treasury is paid to its leader.</p>
    <div class="modal-actions">
      <button class="ghost" onclick={() => { deleteOpen = false; deleteConfirm = null; }}>Cancel</button>
      <button class="danger" onclick={() => { if (deleteConfirm) { remove(deleteConfirm); deleteOpen = false; } }}>Disband Guild</button>
    </div>
  </Modal>
{/if}

<style>
  .rename-form { display: flex; flex-direction: column; gap: 12px; }
  .rename-form label { display: flex; flex-direction: column; gap: 4px; font-size: 0.85rem; color: var(--muted); }
  .page { display: flex; flex-direction: column; gap: 24px; }
  header h1 { font-size: 1.75rem; font-weight: 700; margin: 0 0 6px; }
  header p { color: var(--muted); margin: 0; font-size: 0.95rem; }

  .stats-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; }
  .stat-card { background: var(--bg-2); border: 1px solid var(--line); border-radius: 12px; padding: 18px 20px; display: flex; flex-direction: column; gap: 4px; }
  .stat-card .label { font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--muted); }
  .stat-card .value { font-size: 1.8rem; font-weight: 700; }
  .text-success { color: var(--success, #22c55e); }
  .text-accent { color: var(--accent, #6366f1); }

  .toolbar { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .search-box { display: flex; align-items: center; gap: 10px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 14px; flex: 1; min-width: 260px; max-width: 500px; }
  .search-box input { background: transparent; border: none; outline: none; color: var(--text); width: 100%; font-size: 0.9rem; }
  select { background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 14px; color: var(--text); font-size: 0.85rem; outline: none; }

  .table-wrap { background: var(--bg-2); border: 1px solid var(--line); border-radius: 14px; overflow: hidden; }
  table { width: 100%; border-collapse: collapse; text-align: left; font-size: 0.9rem; }
  th { padding: 12px 16px; background: rgba(255,255,255,0.02); border-bottom: 1px solid var(--line); color: var(--muted); font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; }
  td { padding: 14px 16px; border-bottom: 1px solid rgba(255,255,255,0.04); vertical-align: middle; }
  tr:last-child td { border-bottom: none; }
  tr:hover td { background: rgba(255,255,255,0.015); }

  .guild-cell { display: flex; align-items: center; gap: 12px; }
  .guild-avatar { width: 36px; height: 36px; border-radius: 8px; object-fit: cover; }
  .guild-avatar.fallback { background: rgba(255,255,255,0.06); display: flex; align-items: center; justify-content: center; color: var(--muted); }
  .guild-desc { margin: 2px 0 0; font-size: 0.78rem; color: var(--muted); }

  .tag-badge { font-family: monospace; font-weight: 700; color: #a5b4fc; background: rgba(99,102,241,0.12); padding: 2px 8px; border-radius: 4px; border: 1px solid rgba(99,102,241,0.25); }
  .instance-code { font-size: 0.8rem; color: var(--muted); background: rgba(255,255,255,0.04); padding: 2px 6px; border-radius: 4px; }

  .count-pill { display: inline-flex; align-items: center; gap: 6px; font-size: 0.85rem; color: var(--text-2); }
  .claims-pill { display: inline-flex; align-items: center; gap: 6px; font-size: 0.85rem; color: #4ade80; }
  .date-text { font-size: 0.8rem; color: var(--muted); }

  .warn-note { color: #f87171; font-size: 0.88rem; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }
  .empty { text-align: center; padding: 60px 20px; color: var(--muted); background: var(--bg-2); border-radius: 12px; border: 1px solid var(--line); }
</style>
