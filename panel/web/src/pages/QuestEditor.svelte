<script lang="ts">
  import { onMount } from 'svelte';
  import { Search, Plus, Pencil, Trash2, CheckCircle2, XCircle, Sparkles, Pin, SlidersHorizontal, Pickaxe, Swords, Blocks, Wheat, Users, Compass, Target } from '@lucide/svelte';
  import ImageInput from '../components/ImageInput.svelte';
  import IconPicker from '../components/IconPicker.svelte';
  import { imageIcon } from '../lib/achievementIcons';
  import Modal from '../components/Modal.svelte';
  import RewardBundleEditor from '../components/RewardBundleEditor.svelte';
  import { loadBundle, saveBundle, type RewardAction } from '../lib/rewards';
  import { get, post, put, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Quest } from '../lib/types';

  let quests = $state<Quest[]>([]);
  let loading = $state(true);
  let filterType = $state<'all' | 'daily' | 'weekly'>('all');
  let filterCategory = $state<string>('all');
  let search = $state('');

  let bundle = $state<RewardAction[]>([]);
  let draft = $state<Partial<Quest> | null>(null);
  let draftOpen = $state(false);
  let saving = $state(false);
  let deleteConfirm = $state<Quest | null>(null);
  let deleteOpen = $state(false);

  const categories = ['mining', 'combat', 'building', 'farming', 'social', 'exploration'];
  const iconChoices = [
    { id: 'pickaxe', label: 'Mining', component: Pickaxe },
    { id: 'swords', label: 'Combat', component: Swords },
    { id: 'blocks', label: 'Building', component: Blocks },
    { id: 'wheat', label: 'Farming', component: Wheat },
    { id: 'users', label: 'Social', component: Users },
    { id: 'compass', label: 'Exploration', component: Compass },
    { id: 'target', label: 'General', component: Target },
  ];
  const isPreset = (id?: string) => iconChoices.some((i) => i.id === id);
  function iconFor(q: Partial<Quest>) {
    const icon = (q.icon || '').toLowerCase();
    return iconChoices.find((i) => i.id === icon)?.component || iconChoices.find((i) => i.label.toLowerCase() === q.category)?.component || Target;
  }
  const statTypes = [
    { id: 'blocks_broken', label: 'Blocks Broken' },
    { id: 'mob_kills', label: 'Mob Kills' },
    { id: 'player_kills', label: 'Player Kills' },
    { id: 'blocks_placed', label: 'Blocks Placed' },
    { id: 'playtime_secs', label: 'Playtime (Seconds)' },
    { id: 'messages', label: 'Chat Messages Sent' },
    { id: 'joins', label: 'Server Joins' },
    { id: 'friend_add', label: 'Add a Friend' },
    { id: 'guild_create', label: 'Create a Faction' },
  ];

  async function loadQuests() {
    loading = true;
    try {
      const res = await get<any[]>('/api/admin/quests');
      quests = (res || []).map((q: any) => ({
        id: q.id || '',
        title: q.title || '',
        description: q.description || '',
        quest_type: q.quest_type || q.period || 'daily',
        category: q.category || 'mining',
        xp_reward: q.xp_reward || 0,
        stat_type: q.stat_type || q.target_stat || 'blocks_broken',
        target_count: q.target_count || 1,
        icon: q.icon || 'pickaxe',
        active: q.active !== undefined ? q.active : (q.enabled !== undefined ? q.enabled : true),
      pinned: !!q.pinned,
        instance_id: q.instance_id ?? null,
        server_id: q.server_id ?? null,
        difficulty: q.difficulty || 'standard',
        chain_id: q.chain_id ?? null,
        chain_step: Number(q.chain_step || 0),
      }));
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  let limits = $state<{ daily_quest_limit: number; weekly_quest_limit: number; quest_rotation: string } | null>(null);
  onMount(() => {
    loadQuests();
    get<{ settings: NonNullable<typeof limits> }>('/api/admin/progression').then((v) => (limits = v.settings)).catch(() => {});
  });

  let filtered = $derived(
    quests.filter((q) => {
      if (filterType !== 'all' && q.quest_type !== filterType) return false;
      if (filterCategory !== 'all' && q.category !== filterCategory) return false;
      if (search.trim()) {
        const s = search.toLowerCase();
        return q.title.toLowerCase().includes(s) || q.description.toLowerCase().includes(s) || q.id.toLowerCase().includes(s);
      }
      return true;
    })
  );

  let dailyCount = $derived(quests.filter((q) => q.quest_type === 'daily').length);
  let weeklyCount = $derived(quests.filter((q) => q.quest_type === 'weekly').length);
  let activeCount = $derived(quests.filter((q) => q.active).length);

  function openNew() {
    draft = {
      id: `quest_${Date.now()}`,
      title: '',
      description: '',
      quest_type: 'daily',
      category: 'mining',
      xp_reward: 150,
      stat_type: 'blocks_broken',
      target_count: 50,
      icon: 'pickaxe',
      active: true,
      pinned: false,
      instance_id: null,
      server_id: null,
      difficulty: 'standard',
      chain_id: null,
      chain_step: 0,
    };
    bundle = [];
    draftOpen = true;
  }

  function openEdit(q: Quest) {
    draft = { ...q };
    bundle = [];
    loadBundle('quest', q.id).then((b) => (bundle = b)).catch(() => {});
    draftOpen = true;
  }

  async function save() {
    if (!draft || !draft.id?.trim() || !draft.title?.trim() || !draft.description?.trim() || !draft.stat_type?.trim() || Number(draft.target_count) < 1 || Number(draft.xp_reward) < 0) {
      toast('Enter an ID, title, description, tracked stat, target and reward', 'error');
      return;
    }
    saving = true;
    try {
      const exists = quests.some((q) => q.id === draft?.id);
      if (exists) {
        await put(`/api/admin/quests/${draft.id}`, draft);
        toast('Quest updated');
      } else {
        await post('/api/admin/quests', draft);
        toast('Quest created');
      }
      await saveBundle('quest', draft.id!, bundle);
      draftOpen = false;
      draft = null;
      await loadQuests();
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  async function remove(q: Quest) {
    try {
      await del(`/api/admin/quests/${q.id}`);
      toast(`Quest "${q.title}" removed`);
      deleteConfirm = null;
      await loadQuests();
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page">
  <header>
    <div>
      <h1>Quests Manager</h1>
      <p>Configure daily and weekly player objectives, XP rewards, and stat progression tracking.</p>
    </div>
    <button class="primary" onclick={openNew}>
      <Plus size={16} /> New Quest
    </button>
  </header>

  {#if limits}
    <a class="limit-banner" href="#/progression">
      <SlidersHorizontal size={16} />
      <span>
        Each player gets <strong>{limits.daily_quest_limit || 'every'}</strong> of {dailyCount} daily and
        <strong>{limits.weekly_quest_limit || 'every'}</strong> of {weeklyCount} weekly quests
        ({limits.quest_rotation === 'shared' ? 'same set for everyone' : 'random per player'}).
      </span>
      <span class="edit">Change limits →</span>
    </a>
  {/if}

  <div class="stats-row">
    <div class="stat-card">
      <span class="label">Total Quests</span>
      <span class="value">{quests.length}</span>
    </div>
    <div class="stat-card">
      <span class="label">Active Quests</span>
      <span class="value text-success">{activeCount}</span>
    </div>
    <div class="stat-card">
      <span class="label">Daily Quests</span>
      <span class="value text-accent">{dailyCount}</span>
    </div>
    <div class="stat-card">
      <span class="label">Weekly Quests</span>
      <span class="value text-accent-2">{weeklyCount}</span>
    </div>
  </div>

  <div class="toolbar">
    <div class="search-box">
      <Search size={16} />
      <input type="text" placeholder="Search quests by title, description, or id..." bind:value={search} />
    </div>

    <div class="filters">
      <div class="segmented">
        <button class:active={filterType === 'all'} onclick={() => (filterType = 'all')}>All ({quests.length})</button>
        <button class:active={filterType === 'daily'} onclick={() => (filterType = 'daily')}>Daily ({dailyCount})</button>
        <button class:active={filterType === 'weekly'} onclick={() => (filterType = 'weekly')}>Weekly ({weeklyCount})</button>
      </div>

      <select bind:value={filterCategory}>
        <option value="all">All Categories</option>
        {#each categories as cat}
          <option value={cat}>{cat.toUpperCase()}</option>
        {/each}
      </select>
    </div>
  </div>

  {#if loading}
    <div class="empty">Loading quests...</div>
  {:else if filtered.length === 0}
    <div class="empty">No quests match your filters.</div>
  {:else}
    <div class="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Icon</th>
            <th>Title & Description</th>
            <th>Type</th>
            <th>Category</th>
            <th>Target Objective</th>
            <th>XP Reward</th>
            <th>Status</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as q}
            {@const Icon = iconFor(q)}
            <tr>
              <td class="icon-cell">
                <span class="quest-icon" title={q.icon}>{#if imageIcon(q.icon)}<img src={imageIcon(q.icon) ?? ''} alt="" style="width: 32px; height: 32px; object-fit: contain" />{:else if !/^[a-z_]+$/i.test(q.icon)}{q.icon}{:else}<Icon size={21} />{/if}</span>
              </td>
              <td class="info-cell">
                <strong>{q.title}{#if q.pinned}<span class="pin-tag" title="Always handed out first"><Pin size={11} /> Pinned</span>{/if}</strong>
                <p>{q.description}</p>
                <code class="id-tag">{q.id}</code>
              </td>
              <td>
                <span class="badge {q.quest_type}">
                  {q.quest_type.toUpperCase()}
                </span>
              </td>
              <td>
                <span class="cat-tag">{q.category}</span>
              </td>
              <td>
                <span class="stat-target">
                  <strong>{q.target_count.toLocaleString()}</strong>
                  <code>{q.stat_type}</code>
                </span>
              </td>
              <td>
                <span class="xp-badge">
                  <Sparkles size={13} />
                  +{q.xp_reward} XP
                </span>
              </td>
              <td>
                {#if q.active}
                  <span class="status-active"><CheckCircle2 size={14} /> Active</span>
                {:else}
                  <span class="status-inactive"><XCircle size={14} /> Disabled</span>
                {/if}
              </td>
              <td>
                <div class="actions">
                  <button class="ghost icon" title="Edit quest" onclick={() => openEdit(q)}>
                    <Pencil size={15} />
                  </button>
                  <button class="ghost icon danger" title="Delete quest" onclick={() => { deleteConfirm = q; deleteOpen = true; }}>
                    <Trash2 size={15} />
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

{#if draft}
  <Modal bind:open={draftOpen} width={720} title={draft.id && quests.some((q) => q.id === draft?.id) ? 'Edit Quest' : 'New Quest'}>
    <form onsubmit={(e) => { e.preventDefault(); save(); }} class="form">
      <div class="field">
        <label for="q-id">Quest Identifier</label>
        <input id="q-id" type="text" bind:value={draft.id} required disabled={quests.some((q) => q.id === draft?.id)} placeholder="e.g. daily_mine_iron" />
      </div>

      <div class="grid-2">
        <div class="field">
          <label for="q-title">Title</label>
          <input id="q-title" type="text" bind:value={draft.title} required placeholder="e.g. Iron Seeker" />
        </div>
        <div class="field">
          <label for="q-icon">Icon</label>
          <select id="q-icon" value={isPreset(draft.icon) ? draft.icon : '__custom'} onchange={(e) => { if (draft) draft.icon = e.currentTarget.value === '__custom' ? (isPreset(draft.icon) ? '✨' : draft.icon) : e.currentTarget.value; }}>
            {#each iconChoices as icon}<option value={icon.id}>{icon.label}</option>{/each}
            <option value="__custom">Custom emoji or PNG…</option>
          </select>
          {#if !isPreset(draft.icon)}
            <input aria-label="Custom icon or emoji" bind:value={draft.icon} placeholder="Emoji, or a PNG address" />
            <IconPicker onpick={(v) => { if (draft) draft.icon = v; }} />
            <ImageInput bind:value={() => imageIcon(draft?.icon) ? draft?.icon ?? null : null, (v) => { if (draft) draft.icon = v || '✨'; }} previewValue={imageIcon(draft.icon)} label="PNG icon" accept="image/png" help="Upload a PNG, or paste its HTTPS address in the box above." />
          {/if}
        </div>
      </div>

      <div class="grid-2">
        <div class="field">
          <label for="q-instance">Launcher instance scope</label>
          <input id="q-instance" bind:value={draft.instance_id} placeholder="All instances" />
        </div>
        <div class="field">
          <label for="q-server">Game server scope</label>
          <input id="q-server" type="number" min="1" bind:value={draft.server_id} placeholder="All servers" />
        </div>
      </div>
      <div class="grid-3">
        <div class="field">
          <label for="q-difficulty">Difficulty</label>
          <select id="q-difficulty" bind:value={draft.difficulty}>
            <option value="easy">Easy</option><option value="standard">Standard</option><option value="hard">Hard</option><option value="challenge">Challenge</option>
          </select>
        </div>
        <div class="field">
          <label for="q-chain">Quest chain ID</label>
          <input id="q-chain" bind:value={draft.chain_id} placeholder="No chain" />
        </div>
        <div class="field">
          <label for="q-chain-step">Chain step</label>
          <input id="q-chain-step" type="number" min="0" bind:value={draft.chain_step} />
        </div>
      </div>

      <div class="field">
        <label for="q-desc">Description</label>
        <textarea id="q-desc" rows="2" bind:value={draft.description} required placeholder="Describe the objective..."></textarea>
      </div>

      <div class="grid-2">
        <div class="field">
          <label for="q-type">Cadence Type</label>
          <select id="q-type" bind:value={draft.quest_type}>
            <option value="daily">Daily Quest</option>
            <option value="weekly">Weekly Quest</option>
          </select>
        </div>
        <div class="field">
          <label for="q-cat">Category</label>
          <select id="q-cat" bind:value={draft.category}>
            {#each categories as cat}
              <option value={cat}>{cat.charAt(0).toUpperCase() + cat.slice(1)}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="grid-3">
        <div class="field">
          <label for="q-stat">Tracked Stat</label>
          <input id="q-stat" list="quest-stats" bind:value={draft.stat_type} required placeholder="blocks_broken or action:block_broken:DIAMOND_ORE" />
          <datalist id="quest-stats">{#each statTypes as st}<option value={st.id}>{st.label}</option>{/each}</datalist>
          <small>Use an action target such as <code>action:block_broken:DIAMOND_ORE</code> for a specific block.</small>
        </div>
        <div class="field">
          <label for="q-target">Target Count</label>
          <input id="q-target" type="number" min="1" bind:value={draft.target_count} required />
        </div>
        <div class="field">
          <label for="q-xp">XP Reward</label>
          <input id="q-xp" type="number" min="0" step="1" bind:value={draft.xp_reward} required />
        </div>
      </div>

      <div class="field checkbox">
        <label>
          <input type="checkbox" bind:checked={draft.active} />
          Active (players can receive and complete this quest)
        </label>
      </div>
      <div class="field checkbox">
        <label>
          <input type="checkbox" bind:checked={draft.pinned} />
          Pinned — always included when a player's quests are limited
        </label>
      </div>

      <RewardBundleEditor bind:actions={bundle} />

      <div class="modal-actions">
        <button type="button" class="ghost" onclick={() => (draftOpen = false)}>Cancel</button>
        <button type="submit" class="primary" disabled={saving}>
          {saving ? 'Saving...' : 'Save Quest'}
        </button>
      </div>
    </form>
  </Modal>
{/if}

{#if deleteConfirm}
  <Modal bind:open={deleteOpen} title="Delete Quest">
    <p>Are you sure you want to delete quest <strong>{deleteConfirm.title}</strong> (<code>{deleteConfirm.id}</code>)?</p>
    <div class="modal-actions">
      <button class="ghost" onclick={() => { deleteOpen = false; deleteConfirm = null; }}>Cancel</button>
      <button class="danger" onclick={() => { if (deleteConfirm) { remove(deleteConfirm); deleteOpen = false; } }}>Delete Quest</button>
    </div>
  </Modal>
{/if}

<style>
  .page { min-width: 0; display: flex; flex-direction: column; gap: 24px; }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  h1 { font-size: 1.75rem; font-weight: 700; margin: 0 0 6px; }
  header p { color: var(--muted); margin: 0; font-size: 0.95rem; }

  .limit-banner { display: flex; align-items: center; gap: 12px; padding: 12px 16px; border-radius: 12px; background: color-mix(in srgb, var(--accent) 9%, var(--bg-2)); border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent); color: var(--text); font-size: 0.9rem; }
  @media (max-width: 700px) { .limit-banner { flex-wrap: wrap; } }
  .limit-banner:hover { border-color: color-mix(in srgb, var(--accent) 60%, transparent); }
  .limit-banner .edit { margin-left: auto; color: var(--accent-2); white-space: nowrap; }
  .pin-tag { display: inline-flex; align-items: center; gap: 3px; margin-left: 8px; padding: 1px 7px; border-radius: 99px; font-size: 0.68rem; font-weight: 600; background: color-mix(in srgb, #fcd34d 16%, transparent); color: #fcd34d; vertical-align: middle; }
  .stats-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; }
  .stat-card { background: var(--bg-2); border: 1px solid var(--line); border-radius: 12px; padding: 18px 20px; display: flex; flex-direction: column; gap: 4px; }
  .stat-card .label { font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--muted); }
  .stat-card .value { font-size: 1.8rem; font-weight: 700; }
  .text-success { color: var(--success, #22c55e); }
  .text-accent { color: var(--accent, #6366f1); }
  .text-accent-2 { color: #a855f7; }

  .toolbar { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .search-box { display: flex; align-items: center; gap: 10px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 14px; flex: 1; min-width: 260px; max-width: 500px; }
  .search-box input { background: transparent; border: none; outline: none; color: var(--text); width: 100%; font-size: 0.9rem; }
  .filters { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; min-width: 0; }
  .segmented { display: flex; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; padding: 3px; gap: 2px; }
  .segmented button { padding: 6px 14px; border-radius: 7px; border: none; background: transparent; color: var(--muted); font-size: 0.85rem; font-weight: 500; cursor: pointer; transition: all 0.15s; }
  .segmented button.active { background: var(--surface); color: var(--text); box-shadow: 0 1px 3px rgba(0,0,0,0.3); }
  select { background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 14px; color: var(--text); font-size: 0.85rem; outline: none; }

  .table-wrap { width: 100%; min-width: 0; background: var(--bg-2); border: 1px solid var(--line); border-radius: 14px; overflow-x: auto; }
  table { width: 100%; min-width: 1040px; border-collapse: collapse; text-align: left; font-size: 0.9rem; }
  th { padding: 12px 16px; background: rgba(255,255,255,0.02); border-bottom: 1px solid var(--line); color: var(--muted); font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; }
  td { padding: 14px 16px; border-bottom: 1px solid rgba(255,255,255,0.04); vertical-align: middle; }
  tr:last-child td { border-bottom: none; }
  tr:hover td { background: rgba(255,255,255,0.015); }

  .icon-cell { width: 48px; text-align: center; }
  .quest-icon { display: inline-flex; align-items: center; justify-content: center; color: var(--accent); }

  .info-cell strong { font-size: 0.95rem; display: block; margin-bottom: 2px; }
  .info-cell p { margin: 0 0 4px; color: var(--muted); font-size: 0.82rem; }
  .id-tag { font-size: 0.72rem; color: var(--muted); background: rgba(255,255,255,0.04); padding: 2px 6px; border-radius: 4px; }

  .badge { display: inline-block; padding: 4px 10px; border-radius: 6px; font-size: 0.72rem; font-weight: 700; letter-spacing: 0.05em; }
  .badge.daily { background: rgba(99, 102, 241, 0.15); color: #818cf8; border: 1px solid rgba(99, 102, 241, 0.3); }
  .badge.weekly { background: rgba(168, 85, 247, 0.15); color: #c084fc; border: 1px solid rgba(168, 85, 247, 0.3); }

  .cat-tag { display: inline-block; padding: 3px 8px; border-radius: 6px; background: rgba(255,255,255,0.05); font-size: 0.75rem; text-transform: capitalize; color: var(--text-2); }

  .stat-target { display: flex; flex-direction: column; gap: 2px; }
  .stat-target strong { font-size: 0.95rem; }
  .stat-target code { font-size: 0.72rem; color: var(--muted); }

  .xp-badge { display: inline-flex; align-items: center; gap: 5px; padding: 4px 9px; border-radius: 6px; background: rgba(234, 179, 8, 0.12); color: #eab308; font-weight: 600; font-size: 0.82rem; border: 1px solid rgba(234, 179, 8, 0.25); }

  .status-active { display: inline-flex; align-items: center; gap: 5px; color: var(--success, #22c55e); font-size: 0.82rem; font-weight: 500; }
  .status-inactive { display: inline-flex; align-items: center; gap: 5px; color: var(--muted); font-size: 0.82rem; }

  .actions { display: flex; align-items: center; gap: 6px; }

  .form { display: flex; flex-direction: column; gap: 16px; padding-top: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 0.82rem; font-weight: 500; color: var(--muted); }
  .field input, .field textarea, .field select { background: var(--bg); border: 1px solid var(--line); border-radius: 8px; padding: 10px 12px; color: var(--text); font-size: 0.9rem; }
  .field.checkbox label { display: flex; align-items: center; gap: 8px; cursor: pointer; color: var(--text); font-size: 0.88rem; }
  .grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .grid-3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }
  .empty { text-align: center; padding: 60px 20px; color: var(--muted); background: var(--bg-2); border-radius: 12px; border: 1px solid var(--line); }
  @media (max-width: 720px) {
    header { flex-wrap: wrap; }
    .stats-row { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .search-box { min-width: 0; max-width: none; flex-basis: 100%; }
    .filters, .segmented { width: 100%; }
    .segmented button { flex: 1; padding-inline: 4px; }
    .grid-2, .grid-3 { grid-template-columns: minmax(0, 1fr); }
    .form .field { min-width: 0; }
  }
</style>
