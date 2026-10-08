<script lang="ts">
  import { onMount } from 'svelte';
  import { Plus, Pencil, Trash2, Trophy, Sparkles, Shield, Swords, Pickaxe, Compass, Users } from '@lucide/svelte';
  import ImageInput from '../components/ImageInput.svelte';
  import IconPicker from '../components/IconPicker.svelte';
  import Modal from '../components/Modal.svelte';
  import RewardBundleEditor from '../components/RewardBundleEditor.svelte';
  import { loadBundle, saveBundle, type RewardAction } from '../lib/rewards';
  import { get, post, put, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Achievement } from '../lib/types';
  import { BACKGROUNDS, BORDERS, FRAMES, ITEM_ICONS, resolveIcon } from '../lib/achievementIcons';

  let achievements = $state<Achievement[]>([]);
  let loading = $state(true);
  let filterCategory = $state<string>('all');
  let filterFrame = $state<string>('all');

  let bundle = $state<RewardAction[]>([]);
  let draft = $state<Partial<Achievement> | null>(null);
  let draftOpen = $state(false);
  let saving = $state(false);
  let deleteConfirm = $state<Achievement | null>(null);
  let deleteOpen = $state(false);

  const categories = ['progression', 'mining', 'combat', 'building', 'social', 'guilds', 'exploration'];

  const frameTypes = FRAMES;
  const bgTextures = BACKGROUNDS;
  const borderGlows = BORDERS;
  const popularItems = ITEM_ICONS;

  async function loadAchievements() {
    loading = true;
    try {
      const res = await get<any[]>('/api/admin/achievements');
      achievements = (res || []).map((a: any) => ({
        id: a.id || '',
        title: a.title || '',
        description: a.description || '',
        category: a.category || 'progression',
        xp_reward: a.xp_reward || 0,
        frame_type: a.frame_type || a.icon_frame || 'task',
        icon_item: a.icon_item || 'diamond_pickaxe',
        icon_bg: a.icon_bg || 'deepslate',
        icon_border: a.icon_border || 'gold',
        requirement_type: a.requirement_type || 'stat',
        stat_type: a.stat_type || a.requirement_key || 'blocks_broken',
        target_count: a.target_count || a.requirement_value || 1,
        secret: !!a.secret,
      }));
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadAchievements();
  });

  let filtered = $derived(
    achievements.filter((a) => {
      if (filterCategory !== 'all' && a.category !== filterCategory) return false;
      if (filterFrame !== 'all' && a.frame_type !== filterFrame) return false;
      return true;
    })
  );

  function openNew() {
    draft = {
      id: `ach_${Date.now()}`,
      title: 'New Advancement',
      description: 'Describe the achievement requirements...',
      category: 'progression',
      xp_reward: 200,
      frame_type: 'task',
      icon_item: 'diamond_pickaxe',
      icon_bg: 'deepslate',
      icon_border: 'gold',
      stat_type: 'blocks_broken',
      target_count: 100,
    };
    bundle = [];
    draftOpen = true;
  }

  function openEdit(a: Achievement) {
    draft = { ...a };
    bundle = [];
    loadBundle('achievement', a.id).then((b) => (bundle = b)).catch(() => {});
    draftOpen = true;
  }

  async function save() {
    if (!draft || !draft.title || !draft.description) {
      toast('Title and description are required', 'error');
      return;
    }
    saving = true;
    try {
      const exists = achievements.some((a) => a.id === draft?.id);
      if (exists) {
        await put(`/api/admin/achievements/${draft.id}`, draft);
        toast('Achievement updated');
      } else {
        await post('/api/admin/achievements', draft);
        toast('Achievement created');
      }
      await saveBundle('achievement', draft.id!, bundle);
      draftOpen = false;
      draft = null;
      await loadAchievements();
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  async function remove(a: Achievement) {
    try {
      await del(`/api/admin/achievements/${a.id}`);
      toast(`Achievement "${a.title}" deleted`);
      deleteConfirm = null;
      await loadAchievements();
    } catch (e) {
      toastError(e);
    }
  }

  const getItemEmoji = (id?: string | null) => resolveIcon({ icon_item: id }).emoji;
  const getBorderColor = (border?: string | null) => resolveIcon({ icon_border: border }).borderColor;
  const getBorderGlow = (border?: string | null) => resolveIcon({ icon_border: border }).glow;
  const getBgPattern = (bg?: string | null) => resolveIcon({ icon_bg: bg }).background;
</script>

<div class="page">
  <header>
    <div>
      <h1>Minecraft Achievements & Advancements</h1>
      <p>Create and customize in-game achievements with the Minecraft-themed icon and advancement creator.</p>
    </div>
    <button class="primary" onclick={openNew}>
      <Plus size={16} /> New Achievement
    </button>
  </header>

  <div class="creator-spotlight">
    <div class="spotlight-header">
      <Trophy size={18} class="text-gold" />
      <span>Minecraft Advancement Toast Preview</span>
    </div>
    <div class="toast-preview-row">
      <!-- Task Preview -->
      <div class="mc-toast task">
        <div class="mc-frame-outer task border-gold">
          <div class="mc-frame-inner bg-stone">
            <span class="mc-icon">⛏️</span>
          </div>
        </div>
        <div class="mc-toast-body">
          <div class="mc-toast-title text-task">Advancement Made!</div>
          <div class="mc-toast-desc">Stone Age</div>
        </div>
      </div>

      <!-- Goal Preview -->
      <div class="mc-toast goal">
        <div class="mc-frame-outer goal border-diamond">
          <div class="mc-frame-inner bg-deepslate">
            <span class="mc-icon">✨</span>
          </div>
        </div>
        <div class="mc-toast-body">
          <div class="mc-toast-title text-goal">Goal Reached!</div>
          <div class="mc-toast-desc">Beaconator</div>
        </div>
      </div>

      <!-- Challenge Preview -->
      <div class="mc-toast challenge">
        <div class="mc-frame-outer challenge border-purple">
          <div class="mc-frame-inner bg-obsidian">
            <span class="mc-icon">🪽</span>
          </div>
        </div>
        <div class="mc-toast-body">
          <div class="mc-toast-title text-challenge">Challenge Complete!</div>
          <div class="mc-toast-desc">Sky's the Limit</div>
        </div>
      </div>
    </div>
  </div>

  <div class="toolbar">
    <div class="filters">
      <label for="filter-cat">Category:</label>
      <select id="filter-cat" bind:value={filterCategory}>
        <option value="all">All Categories</option>
        {#each categories as cat}
          <option value={cat}>{cat.toUpperCase()}</option>
        {/each}
      </select>

      <label for="filter-frame">Frame Style:</label>
      <select id="filter-frame" bind:value={filterFrame}>
        <option value="all">All Frames</option>
        <option value="task">Task (Normal)</option>
        <option value="goal">Goal (Rounded)</option>
        <option value="challenge">Challenge (Spiked)</option>
      </select>
    </div>
    <span class="count-badge">{filtered.length} Achievements</span>
  </div>

  {#if loading}
    <div class="empty">Loading achievements...</div>
  {:else if filtered.length === 0}
    <div class="empty">No achievements match your filters.</div>
  {:else}
    <div class="grid">
      {#each filtered as a}
        <div class="ach-card">
          <div class="ach-icon-container">
            <div
              class="mc-frame-outer {a.frame_type}"
              style="border-color: {getBorderColor(a.icon_border)}; box-shadow: {getBorderGlow(a.icon_border)};"
            >
              <div class="mc-frame-inner" style="background: {getBgPattern(a.icon_bg)};">
                {#if resolveIcon(a).image}<img src={resolveIcon(a).image} alt="" style="width: 32px; height: 32px; object-fit: contain; image-rendering: pixelated" />{:else}<span class="mc-icon">{getItemEmoji(a.icon_item)}</span>{/if}
              </div>
            </div>
          </div>

          <div class="ach-info">
            <div class="ach-header">
              <span class="ach-type {a.frame_type}">{a.frame_type.toUpperCase()}</span>
              <span class="ach-xp"><Sparkles size={12} /> +{a.xp_reward} XP</span>
            </div>
            <h3>{a.title}</h3>
            <p>{a.description}</p>
            <div class="ach-meta">
              <span class="meta-tag">{a.category}</span>
              {#if a.stat_type}
                <code class="meta-trigger">{a.stat_type} &ge; {a.target_count}</code>
              {:else if a.event_type}
                <code class="meta-trigger">event: {a.event_type}</code>
              {/if}
            </div>
          </div>

          <div class="card-actions">
            <button class="ghost icon" title="Edit" onclick={() => openEdit(a)}>
              <Pencil size={15} />
            </button>
            <button class="ghost icon danger" title="Delete" onclick={() => { deleteConfirm = a; deleteOpen = true; }}>
              <Trash2 size={15} />
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if draft}
  <Modal bind:open={draftOpen} title={draft.id && achievements.some((a) => a.id === draft?.id) ? 'Edit Achievement' : 'Achievement Creator'}>
    <form onsubmit={(e) => { e.preventDefault(); save(); }} class="form">
      <!-- Live Achievement Preview Box -->
      <div class="preview-panel">
        <span class="preview-label">Live In-Game Toast Preview</span>
        <div class="mc-toast {draft.frame_type}">
          <div
            class="mc-frame-outer {draft.frame_type}"
            style="border-color: {getBorderColor(draft.icon_border)}; box-shadow: {getBorderGlow(draft.icon_border)};"
          >
            <div class="mc-frame-inner" style="background: {getBgPattern(draft.icon_bg)};">
              {#if resolveIcon(draft).image}<img src={resolveIcon(draft).image} alt="" style="width: 32px; height: 32px; object-fit: contain; image-rendering: pixelated" />{:else}<span class="mc-icon">{getItemEmoji(draft.icon_item)}</span>{/if}
            </div>
          </div>
          <div class="mc-toast-body">
            <div class="mc-toast-title text-{draft.frame_type}">
              {frameTypes.find((f) => f.id === draft?.frame_type)?.bannerTitle}
            </div>
            <div class="mc-toast-desc">{draft.title || 'Untitled Achievement'}</div>
          </div>
        </div>
      </div>

      <div class="field">
        <label for="a-id">Achievement ID</label>
        <input id="a-id" type="text" bind:value={draft.id} required placeholder="ach_dragon_slayer" />
      </div>

      <div class="grid-2">
        <div class="field">
          <label for="a-title">Title</label>
          <input id="a-title" type="text" bind:value={draft.title} required placeholder="Free the End" />
        </div>
        <div class="field">
          <label for="a-xp">XP Reward</label>
          <input id="a-xp" type="number" min="50" step="50" bind:value={draft.xp_reward} required />
        </div>
      </div>

      <div class="field">
        <label for="a-desc">Description</label>
        <textarea id="a-desc" rows="2" bind:value={draft.description} required placeholder="Kill the Ender Dragon..."></textarea>
      </div>

      <div class="section-title">Minecraft Icon Styling</div>
      <div class="grid-3">
        <div class="field">
          <label for="a-frame">Frame Type</label>
          <select id="a-frame" bind:value={draft.frame_type}>
            {#each frameTypes as ft}
              <option value={ft.id}>{ft.label}</option>
            {/each}
          </select>
        </div>

        <div class="field">
          <label for="a-item">Icon Item</label>
          <select id="a-item" bind:value={draft.icon_item}>
            {#each popularItems as it}
              <option value={it.id}>{it.emoji} {it.label}</option>
            {/each}
          </select>
        </div>

        <div class="field">
          <label for="a-bg">Background Texture</label>
          <select id="a-bg" bind:value={draft.icon_bg}>
            {#each bgTextures as bt}
              <option value={bt.id}>{bt.label}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="grid-2">
        <div class="field">
          <label for="a-border">Glowing Border Accent</label>
          <select id="a-border" bind:value={draft.icon_border}>
            {#each borderGlows as bg}
              <option value={bg.id}>{bg.label}</option>
            {/each}
          </select>
        </div>

        <div class="field">
          <label for="a-cat">Category</label>
          <select id="a-cat" bind:value={draft.category}>
            {#each categories as cat}
              <option value={cat}>{cat.charAt(0).toUpperCase() + cat.slice(1)}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="field"><label>Custom item ID or emoji<input bind:value={draft.icon_item} placeholder="diamond_sword or ✨" /></label></div>
      <div class="field"><span class="lbl">Icon library</span><IconPicker onpick={(v) => { if (draft) draft.icon_item = v; }} /></div>
      <ImageInput bind:value={() => draft?.icon_item ?? null, (v) => { if (draft) draft.icon_item = v || 'diamond_pickaxe'; }} previewValue={resolveIcon(draft).image} label="Custom PNG icon" accept="image/png" help="Upload a PNG or enter its HTTPS URL. Choose an item above to switch back." />
      <div class="grid-2">
        <div class="field"><label>Custom background color<input type="color" value={draft.icon_bg?.startsWith('#') ? draft.icon_bg : '#27272a'} oninput={(e) => { if (draft) draft.icon_bg = e.currentTarget.value; }} /></label></div>
        <div class="field"><label>Custom border color<input type="color" value={draft.icon_border?.startsWith('#') ? draft.icon_border : '#eab308'} oninput={(e) => { if (draft) draft.icon_border = e.currentTarget.value; }} /></label></div>
      </div>
      <div class="section-title">Trigger Conditions</div>
      <div class="grid-2">
        <div class="field">
          <label for="a-stat">Tracked Stat (Optional)</label>
          <input id="a-stat" type="text" bind:value={draft.stat_type} placeholder="e.g. mob_kills or blocks_broken" />
        </div>
        <div class="field">
          <label for="a-target">Target Count (Optional)</label>
          <input id="a-target" type="number" min="1" bind:value={draft.target_count} placeholder="100" />
        </div>
      </div>

      <div class="field">
        <label for="a-event">Game Event Trigger (Optional)</label>
        <input id="a-event" type="text" bind:value={draft.event_type} placeholder="e.g. kill_dragon, enter_nether, craft_beacon" />
      </div>

      <RewardBundleEditor bind:actions={bundle} />

      <div class="modal-actions">
        <button type="button" class="ghost" onclick={() => (draftOpen = false)}>Cancel</button>
        <button type="submit" class="primary" disabled={saving}>
          {saving ? 'Saving...' : 'Save Achievement'}
        </button>
      </div>
    </form>
  </Modal>
{/if}

{#if deleteConfirm}
  <Modal bind:open={deleteOpen} title="Delete Achievement">
    <p>Are you sure you want to delete achievement <strong>{deleteConfirm.title}</strong> (<code>{deleteConfirm.id}</code>)?</p>
    <div class="modal-actions">
      <button class="ghost" onclick={() => { deleteOpen = false; deleteConfirm = null; }}>Cancel</button>
      <button class="danger" onclick={() => { if (deleteConfirm) { remove(deleteConfirm); deleteOpen = false; } }}>Delete Achievement</button>
    </div>
  </Modal>
{/if}

<style>
  .page { display: flex; flex-direction: column; gap: 24px; }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  h1 { font-size: 1.75rem; font-weight: 700; margin: 0 0 6px; }
  header p { color: var(--muted); margin: 0; font-size: 0.95rem; }

  .creator-spotlight { background: linear-gradient(135deg, rgba(24,24,27,0.8), rgba(9,9,11,0.9)); border: 1px solid var(--line); border-radius: 14px; padding: 20px 24px; display: flex; flex-direction: column; gap: 16px; }
  .spotlight-header { display: flex; align-items: center; gap: 10px; font-weight: 600; font-size: 0.95rem; }
  .text-gold { color: #eab308; }

  .toast-preview-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 16px; }

  /* Authentic Minecraft Toast Styling */
  .mc-toast {
    display: flex;
    align-items: center;
    gap: 14px;
    background: #212121;
    border: 2px solid #3f3f46;
    border-radius: 8px;
    padding: 10px 14px;
    box-shadow: 0 4px 14px rgba(0,0,0,0.6), inset 0 1px 0 rgba(255,255,255,0.1);
  }
  .mc-toast.goal { border-color: #0284c7; }
  .mc-toast.challenge { border-color: #9333ea; }

  .mc-toast-body { display: flex; flex-direction: column; gap: 2px; }
  .mc-toast-title { font-size: 0.8rem; font-weight: 700; letter-spacing: 0.05em; font-family: monospace, sans-serif; text-transform: uppercase; }
  .text-task { color: #ffff55; text-shadow: 1px 1px 0 #3f3f00; }
  .text-goal { color: #55ffff; text-shadow: 1px 1px 0 #003f3f; }
  .text-challenge { color: #ff55ff; text-shadow: 1px 1px 0 #3f003f; }

  .mc-toast-desc { font-size: 0.9rem; font-weight: 600; color: #ffffff; text-shadow: 1px 1px 0 #181818; }

  /* Minecraft Frame Creator Classes */
  .mc-frame-outer {
    width: 44px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px solid #eab308;
    background: #18181b;
    transition: all 0.2s ease;
  }
  .mc-frame-outer.task { border-radius: 6px; }
  .mc-frame-outer.goal { border-radius: 50%; }
  .mc-frame-outer.challenge {
    border-radius: 6px;
    clip-path: polygon(15% 0%, 85% 0%, 100% 15%, 100% 85%, 85% 100%, 15% 100%, 0% 85%, 0% 15%);
  }

  .mc-frame-inner {
    width: 34px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: inherit;
  }
  .mc-icon { font-size: 1.3rem; }

  .border-gold { border-color: #eab308 !important; box-shadow: 0 0 10px rgba(234, 179, 8, 0.6) !important; }
  .border-diamond { border-color: #06b6d4 !important; box-shadow: 0 0 12px rgba(6, 182, 212, 0.8) !important; }
  .border-purple { border-color: #c084fc !important; box-shadow: 0 0 14px rgba(192, 132, 252, 0.8) !important; }

  .bg-stone { background: radial-gradient(circle, #52525b 0%, #27272a 100%); }
  .bg-deepslate { background: radial-gradient(circle, #27272a 0%, #09090b 100%); }
  .bg-obsidian { background: radial-gradient(circle, #2e1065 0%, #0f172a 100%); }

  .toolbar { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .filters { display: flex; align-items: center; gap: 12px; }
  .filters label { font-size: 0.82rem; color: var(--muted); }
  select { background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 14px; color: var(--text); font-size: 0.85rem; outline: none; }
  .count-badge { font-size: 0.85rem; color: var(--muted); }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(360px, 1fr)); gap: 16px; }
  .ach-card {
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: 14px;
    padding: 18px;
    display: flex;
    gap: 16px;
    align-items: flex-start;
    position: relative;
    transition: transform 0.15s, border-color 0.15s;
  }
  .ach-card:hover { transform: translateY(-2px); border-color: rgba(255,255,255,0.15); }

  .ach-icon-container { flex-shrink: 0; padding-top: 2px; }

  .ach-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .ach-header { display: flex; align-items: center; gap: 8px; }
  .ach-type { font-size: 0.68rem; font-weight: 700; letter-spacing: 0.05em; padding: 2px 6px; border-radius: 4px; }
  .ach-type.task { background: rgba(234, 179, 8, 0.15); color: #eab308; }
  .ach-type.goal { background: rgba(6, 182, 212, 0.15); color: #06b6d4; }
  .ach-type.challenge { background: rgba(192, 132, 252, 0.15); color: #c084fc; }

  .ach-xp { display: inline-flex; align-items: center; gap: 4px; font-size: 0.75rem; font-weight: 600; color: #eab308; }
  .ach-info h3 { font-size: 1rem; font-weight: 600; margin: 2px 0 0; }
  .ach-info p { margin: 0; color: var(--muted); font-size: 0.84rem; line-height: 1.35; }
  .ach-meta { display: flex; align-items: center; gap: 6px; margin-top: 6px; flex-wrap: wrap; }
  .meta-tag { font-size: 0.72rem; padding: 2px 6px; border-radius: 4px; background: rgba(255,255,255,0.06); color: var(--text-2); text-transform: capitalize; }
  .meta-trigger { font-size: 0.72rem; color: #a1a1aa; background: rgba(255,255,255,0.03); padding: 2px 6px; border-radius: 4px; }

  .card-actions { display: flex; flex-direction: column; gap: 4px; }

  .preview-panel { background: rgba(0,0,0,0.3); border: 1px dashed var(--line); border-radius: 10px; padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .preview-label { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--muted); }

  .form { display: flex; flex-direction: column; gap: 14px; padding-top: 8px; }
  .section-title { font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--muted); margin-top: 6px; border-top: 1px solid var(--line); padding-top: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: 0.82rem; font-weight: 500; color: var(--muted); }
  .field input, .field textarea, .field select { background: var(--bg); border: 1px solid var(--line); border-radius: 8px; padding: 10px 12px; color: var(--text); font-size: 0.9rem; }
  .grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .grid-3 { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 16px; }
  .empty { text-align: center; padding: 60px 20px; color: var(--muted); background: var(--bg-2); border-radius: 12px; border: 1px solid var(--line); }
</style>
