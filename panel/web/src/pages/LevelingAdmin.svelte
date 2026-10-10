<script lang="ts">
  import { go } from '../lib/router.svelte';
  import PlayerLink from '../components/PlayerLink.svelte';
  import { onMount } from 'svelte';
  import { Sparkles, Trophy, Plus, Pencil, Trash2, Award, Shield, Crown, RefreshCw } from '@lucide/svelte';
  import ImageInput from '../components/ImageInput.svelte';
  import PrefixGenerator from '../components/PrefixGenerator.svelte';
  import Modal from '../components/Modal.svelte';
  import RewardBundleEditor from '../components/RewardBundleEditor.svelte';
  import { loadBundle, saveBundle, type RewardAction } from '../lib/rewards';
  import { get, post, put, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { LevelReward, UserLevelInfo } from '../lib/types';

  let rewards = $state<LevelReward[]>([]);
  let leaderboard = $state<UserLevelInfo[]>([]);
  let loading = $state(true);
  let curve = $state({ base: 100, exponent: 1.5, max: 0 });

  let bundle = $state<RewardAction[]>([]);
  let draft = $state<Partial<LevelReward> | null>(null);
  let draftOpen = $state(false);
  let saving = $state(false);
  let deleteConfirm = $state<LevelReward | null>(null);
  let deleteOpen = $state(false);

  const rewardTypes = [
    { id: 'title', label: 'Player Title / Prefix' },
    { id: 'profile_badge', label: 'Profile Badge' },
    { id: 'cosmetic', label: 'Cosmetic / Particle Effect' },
    { id: 'item', label: 'In-Game Kit / Item' },
  ] as const;

  async function loadData() {
    loading = true;
    try {
      const [r, lb] = await Promise.all([
        get<any[]>('/api/v1/levels/rewards'),
        get<UserLevelInfo[]>('/api/v1/levels/leaderboard?limit=10'),
      ]);
      rewards = (r || []).map((x: any) => ({
        id: x.id,
        level: Number(x.level ?? x.level_req ?? 1),
        reward_type: x.reward_type || 'title',
        reward_value: x.reward_value ?? x.reward_name ?? '',
        description: x.description ?? '',
        icon: x.icon ?? '🎁',
        title_image: x.reward_data?.title_image ?? null,
        reward_data: x.reward_data ?? {},
        discord_role_id: x.reward_data?.discord_role_id ?? '',
        luckperms_group: x.reward_data?.luckperms_group ?? '',
      })).sort((a, b) => a.level - b.level);
      leaderboard = lb || [];
      const p = await get<{ settings: { level_base: number; level_exponent: number; max_level: number } }>('/api/admin/progression');
      curve = { base: p.settings.level_base, exponent: p.settings.level_exponent, max: p.settings.max_level };
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadData();
  });

  function openNew() {
    draft = {
      level: 10,
      reward_type: 'title',
      reward_value: 'Explorer',
      description: 'Grants the exclusive [Explorer] title across all servers and in launcher profile.',
      icon: '🧭',
    };
    bundle = [];
    draftOpen = true;
  }

  function openEdit(r: LevelReward) {
    draft = { ...r };
    bundle = [];
    loadBundle('level_reward', r.id).then((b) => (bundle = b)).catch(() => {});
    draftOpen = true;
  }

  async function save() {
    if (!draft || !draft.level || !draft.reward_value) {
      toast('Level and reward value are required', 'error');
      return;
    }
    saving = true;
    try {
      if (draft.id) {
        await put(`/api/admin/levels/rewards/${draft.id}`, { ...draft, reward_data: { ...draft.reward_data, title_image: draft.title_image ?? null, discord_role_id: draft.discord_role_id ?? '', luckperms_group: draft.luckperms_group ?? '' } });
        await saveBundle('level_reward', draft.id, bundle);
        toast('Reward updated');
      } else {
        const made = await post<{ id: number }>('/api/admin/levels/rewards', { ...draft, reward_data: { ...draft.reward_data, title_image: draft.title_image ?? null, discord_role_id: draft.discord_role_id ?? '', luckperms_group: draft.luckperms_group ?? '' } });
        if (bundle.length) await saveBundle('level_reward', made.id, bundle);
        toast('Reward created');
      }
      draftOpen = false;
      draft = null;
      await loadData();
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  /** Some rewards were seeded with an icon *name* ("sparkles") instead of an emoji; show something sensible for those. */
  const NAMED_ICONS: Record<string, string> = { sparkles: '✨', star: '⭐', crown: '👑', shield: '🛡️', trophy: '🏆', sword: '⚔️', gem: '💎', heart: '❤️', flame: '🔥', gift: '🎁' };
  const glyph = (icon: string | undefined) => (icon && /^[a-z-]+$/i.test(icon) ? NAMED_ICONS[icon.toLowerCase()] ?? '🎁' : icon || '🎁');

  let syncing = $state(false);
  /** Recomputes every player's title from the current rewards, for copies left stale by renames. */
  async function syncTitles() {
    if (syncing) return;
    syncing = true;
    try {
      const r = await post<{ checked: number; changed: number }>('/api/admin/levels/sync-titles', {});
      toast(r.changed ? `Updated ${r.changed} player ${r.changed === 1 ? 'title' : 'titles'} (checked ${r.checked})` : `Every title is already up to date (checked ${r.checked})`);
      await loadData();
    } catch (e) {
      toastError(e);
    } finally {
      syncing = false;
    }
  }

  async function remove(r: LevelReward) {
    try {
      await del(`/api/admin/levels/rewards/${r.id}`);
      toast('Reward deleted');
      deleteConfirm = null;
      await loadData();
    } catch (e) {
      toastError(e);
    }
  }

  function calcXp(lvl: number): number {
    if (lvl <= 1) return 0;
    return Math.round(Math.pow(lvl, curve.exponent) * curve.base);
  }
</script>

<div class="page">
  <header>
    <div>
      <h1>Global Leveling & Rewards</h1>
      <p>Configure account-wide global levels, unlockable titles, cosmetics, badges, and view global leaderboards.</p>
    </div>
    <div class="head-actions">
      <button class="ghost" onclick={syncTitles} disabled={syncing} title="Re-apply the current reward names to every player's title and rank. Use after renaming levels.">
        <RefreshCw size={15} class={syncing ? 'spin' : ''} /> Update all players' titles
      </button>
      <button class="primary" onclick={openNew}>
        <Plus size={16} /> New Level Reward
      </button>
    </div>
  </header>

  <div class="grid-layout">
    <!-- Rewards Section -->
    <div class="main-col">
      <div class="card-header">
        <h2>Milestone Rewards ({rewards.length})</h2>
        <span class="sub">Granted automatically as players earn XP across all connected servers</span>
      </div>

      {#if loading}
        <div class="empty">Loading rewards...</div>
      {:else}
        <div class="rewards-list">
          {#each rewards as r}
            <div class="reward-row">
              <div class="level-bubble">
                <span class="lvl-label">LVL</span>
                <span class="lvl-num">{r.level}</span>
              </div>

              <div class="reward-content">
                <div class="reward-top">
                  <span class="reward-icon">{#if r.title_image}<img src={r.title_image} alt={r.reward_value} style="max-width: 120px; height: 28px; object-fit: contain" />{:else}{glyph(r.icon)}{/if}</span>
                  <strong>{r.reward_value}</strong>
                  <span class="type-pill {r.reward_type}">{r.reward_type.replace('_', ' ').toUpperCase()}</span>
                </div>
                <p>{r.description}</p>
                <div class="reward-xp-needed">
                  Requires ~{calcXp(r.level).toLocaleString()} Total XP
                </div>
              </div>

              <div class="reward-actions">
                <button class="ghost icon" onclick={() => openEdit(r)} title="Edit"><Pencil size={15} /></button>
                <button class="ghost icon danger" onclick={() => { deleteConfirm = r; deleteOpen = true; }} title="Delete"><Trash2 size={15} /></button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Sidebar Info / Leaderboard Preview -->
    <div class="side-col">
      <div class="side-card">
        <h3><Sparkles size={16} class="text-accent" /> Leveling Formula</h3>
        <p class="formula-desc">
          Total XP required for level <em>L</em> is given by:
        </p>
        <code class="formula-box">XP = round(L ^ {curve.exponent} &times; {curve.base})</code>
        <p class="formula-desc">{curve.max ? `Level cap: ${curve.max}.` : 'No level cap.'} <button class="ghost sm" onclick={() => go('progression')}>Tune the curve</button></p>
        <ul class="curve-points">
          <li><span>Level 5:</span> <strong>{calcXp(5).toLocaleString()} XP</strong></li>
          <li><span>Level 10:</span> <strong>{calcXp(10).toLocaleString()} XP</strong></li>
          <li><span>Level 25:</span> <strong>{calcXp(25).toLocaleString()} XP</strong></li>
          <li><span>Level 50:</span> <strong>{calcXp(50).toLocaleString()} XP</strong></li>
          <li><span>Level 100:</span> <strong>{calcXp(100).toLocaleString()} XP</strong></li>
        </ul>
      </div>

      <div class="side-card">
        <h3><Crown size={16} class="text-gold" /> Top Global Players</h3>
        {#if leaderboard.length === 0}
          <p class="muted-note">No player progression recorded yet.</p>
        {:else}
          <div class="lb-list">
            {#each leaderboard as p, i}
              <div class="lb-item">
                <span class="lb-rank">#{i + 1}</span>
                <div class="lb-info">
                  <strong><PlayerLink uuid={p.uuid} name={(p as any).username ?? `UUID: ${p.uuid.slice(0, 8)}…`} /></strong>
                  <span>{Number((p as any).global_xp ?? p.current_xp ?? 0).toLocaleString()} XP</span>
                </div>
                <span class="lb-lvl">Lvl {(p as any).global_level ?? p.level}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

{#if draft}
  <Modal bind:open={draftOpen} title={draft.id ? 'Edit Level Reward' : 'New Level Reward'}>
    <form onsubmit={(e) => { e.preventDefault(); save(); }} class="form">
      <div class="grid-2">
        <div class="field">
          <label for="r-lvl">Target Global Level</label>
          <input id="r-lvl" type="number" min="1" max="500" bind:value={draft.level} required />
        </div>
        <div class="field">
          <label for="r-type">Reward Type</label>
          <select id="r-type" bind:value={draft.reward_type}>
            {#each rewardTypes as rt}
              <option value={rt.id}>{rt.label}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="grid-2">
        <div class="field">
          <label for="r-val">Reward Value / Name</label>
          <input id="r-val" type="text" bind:value={draft.reward_value} required placeholder="e.g. Master Builder or Diamond Cape" />
        </div>
        <div class="field">
          <label for="r-icon">Icon (Emoji / Symbol)</label>
          <input id="r-icon" type="text" bind:value={draft.icon} placeholder="👑" />
        </div>
      </div>

      <div class="field">
        <label for="r-desc">Description</label>
        <textarea id="r-desc" rows="3" bind:value={draft.description} required placeholder="Describe what the player unlocks..."></textarea>
      </div>
      {#if draft.reward_type === 'title'}
        <div class="grid-2">
          <ImageInput bind:value={() => draft?.title_image ?? null, (v) => { if (draft) draft.title_image = v; }} label="Rank title PNG" accept="image/png" help="Optional image displayed in launcher profiles. Keep the name for in-game text and accessibility." />
          <div class="field"><span class="lbl">No artwork yet?</span><PrefixGenerator initialText={draft.reward_value ?? ''} onmade={(url) => { if (draft) draft.title_image = url; }} /></div>
          <div class="field"><label>Discord role ID<input bind:value={draft.discord_role_id} inputmode="numeric" placeholder="Optional role for this rank" /></label></div>
          <div class="field"><label>LuckPerms group<input bind:value={draft.luckperms_group} placeholder="Optional in-game group" /></label></div>
        </div>
      {/if}

      <RewardBundleEditor bind:actions={bundle} />

      <div class="modal-actions">
        <button type="button" class="ghost" onclick={() => (draftOpen = false)}>Cancel</button>
        <button type="submit" class="primary" disabled={saving}>
          {saving ? 'Saving...' : 'Save Reward'}
        </button>
      </div>
    </form>
  </Modal>
{/if}

{#if deleteConfirm}
  <Modal bind:open={deleteOpen} title="Delete Reward">
    <p>Are you sure you want to delete the Level {deleteConfirm.level} reward (<strong>{deleteConfirm.reward_value}</strong>)?</p>
    <div class="modal-actions">
      <button class="ghost" onclick={() => { deleteOpen = false; deleteConfirm = null; }}>Cancel</button>
      <button class="danger" onclick={() => { if (deleteConfirm) { remove(deleteConfirm); deleteOpen = false; } }}>Delete Reward</button>
    </div>
  </Modal>
{/if}

<style>
  .head-actions { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; justify-content: flex-end; flex-shrink: 0; }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .page { display: flex; flex-direction: column; gap: 24px; }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap; }
  header > div:first-child { flex: 1; min-width: 280px; }
  h1 { font-size: 1.75rem; font-weight: 700; margin: 0 0 6px; }
  header p { color: var(--muted); margin: 0; font-size: 0.95rem; }

  .grid-layout { display: grid; grid-template-columns: 2fr 1fr; gap: 24px; }
  @media (max-width: 1000px) {
    .grid-layout { grid-template-columns: 1fr; }
  }

  .main-col { background: var(--bg-2); border: 1px solid var(--line); border-radius: 14px; padding: 24px; display: flex; flex-direction: column; gap: 18px; }
  .card-header h2 { font-size: 1.2rem; font-weight: 600; margin: 0 0 4px; }
  .card-header .sub { font-size: 0.85rem; color: var(--muted); }

  .rewards-list { display: flex; flex-direction: column; gap: 12px; }
  .reward-row {
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 16px 20px;
    display: flex;
    align-items: center;
    gap: 18px;
    transition: border-color 0.15s;
  }
  .reward-row:hover { border-color: rgba(255,255,255,0.15); }

  .level-bubble {
    width: 52px;
    height: 52px;
    border-radius: 12px;
    background: linear-gradient(135deg, rgba(99,102,241,0.2), rgba(168,85,247,0.2));
    border: 1px solid rgba(99,102,241,0.4);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .lvl-label { font-size: 0.65rem; font-weight: 700; letter-spacing: 0.05em; color: #a5b4fc; }
  .lvl-num { font-size: 1.25rem; font-weight: 800; color: #ffffff; line-height: 1; }

  .reward-content { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .reward-top { display: flex; align-items: center; gap: 10px; }
  .reward-icon { font-size: 1.2rem; }
  .reward-top strong { font-size: 1.05rem; }

  .type-pill { font-size: 0.68rem; font-weight: 700; letter-spacing: 0.05em; padding: 2px 7px; border-radius: 5px; }
  .type-pill.title { background: rgba(234,179,8,0.15); color: #eab308; }
  .type-pill.profile_badge { background: rgba(168,85,247,0.15); color: #c084fc; }
  .type-pill.cosmetic { background: rgba(236,72,153,0.15); color: #f472b6; }
  .type-pill.item { background: rgba(34,197,94,0.15); color: #4ade80; }

  .reward-content p { margin: 0; color: var(--muted); font-size: 0.86rem; }
  .reward-xp-needed { font-size: 0.75rem; color: #94a3b8; font-family: monospace; }

  .reward-actions { display: flex; align-items: center; gap: 6px; }
  @media (max-width: 700px) { .reward-row { flex-wrap: wrap; } .reward-content { flex-basis: 60%; } }

  .side-col { display: flex; flex-direction: column; gap: 20px; }
  .side-card { background: var(--bg-2); border: 1px solid var(--line); border-radius: 14px; padding: 20px; display: flex; flex-direction: column; gap: 12px; }
  .side-card h3 { display: flex; align-items: center; gap: 8px; font-size: 1rem; font-weight: 600; margin: 0; }
  .text-accent { color: var(--accent, #6366f1); }
  .text-gold { color: #eab308; }

  .formula-desc { font-size: 0.85rem; color: var(--muted); margin: 0; }
  .formula-box { background: rgba(0,0,0,0.3); border: 1px solid var(--line); padding: 8px 12px; border-radius: 8px; font-family: monospace; font-size: 0.85rem; color: #a5b4fc; }

  .curve-points { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
  .curve-points li { display: flex; justify-content: space-between; font-size: 0.85rem; border-bottom: 1px solid rgba(255,255,255,0.04); padding-bottom: 4px; }
  .curve-points li span { color: var(--muted); }

  .muted-note { color: var(--muted); font-size: 0.85rem; margin: 0; }
  .lb-list { display: flex; flex-direction: column; gap: 8px; }
  .lb-item { display: flex; align-items: center; gap: 10px; padding: 8px; background: var(--bg); border: 1px solid var(--line); border-radius: 8px; font-size: 0.85rem; }
  .lb-rank { font-weight: 700; width: 24px; color: var(--muted); }
  .lb-info { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .lb-info strong { font-size: 0.85rem; }
  .lb-info span { font-size: 0.72rem; color: var(--muted); }
  .lb-lvl { font-weight: 700; color: #eab308; }

  .form { display: flex; flex-direction: column; gap: 14px; padding-top: 8px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 0.82rem; font-weight: 500; color: var(--muted); }
  .field input, .field textarea, .field select { background: var(--bg); border: 1px solid var(--line); border-radius: 8px; padding: 10px 12px; color: var(--text); font-size: 0.9rem; }
  .grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }
  .empty { text-align: center; padding: 40px 20px; color: var(--muted); }
</style>
