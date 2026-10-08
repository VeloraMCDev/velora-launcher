<script lang="ts">
  import { Plus, Save, Trash2, Wand2, Bird, Sparkles, MessageSquare, LogIn, LogOut, Crown, Star, Copy, Check } from '@lucide/svelte/icons';
  import { get, post, put, del } from '../lib/api';
  import { toast } from '../lib/toast.svelte';
  import Modal from '../components/Modal.svelte';
  import Empty from '../components/Empty.svelte';
  import AssetPicker from '../components/AssetPicker.svelte';
  import ModelViewer from '../components/ModelViewer.svelte';

  type CosmeticTemplate = {
    id?: number;
    key: string;
    type: 'cosmetic' | 'pet' | 'particle' | 'join_message' | 'leave_message' | 'title' | 'badge';
    label: string;
    description: string;
    metadata: Record<string, any>;
    auto_grant?: boolean;
    grant_on_level?: number | null;
    grant_on_achievement?: string | null;
  };

  let templates = $state<CosmeticTemplate[]>([]);
  let loading = $state(false);
  let modalOpen = $state(false);
  let draft = $state<CosmeticTemplate>({
    key: '',
    type: 'cosmetic',
    label: '',
    description: '',
    metadata: {},
  });
  let editingId = $state<number | null>(null);
  let filter = $state<string>('all');
  let copiedKey = $state<string | null>(null);

  const typeIcons: Record<string, any> = {
    cosmetic: Wand2,
    particle: Sparkles,
    pet: Bird,
    join_message: LogIn,
    leave_message: LogOut,
    title: Crown,
    badge: Star,
  };

  const typeLabels: Record<string, string> = {
    cosmetic: 'Cosmetic',
    particle: 'Particle Effect',
    pet: 'Pet',
    join_message: 'Join Message',
    leave_message: 'Leave Message',
    title: 'Title',
    badge: 'Badge',
  };

  const particleTypes = ['flame', 'heart', 'note', 'portal', 'enchant', 'crit', 'explosion', 'cloud', 'smoke', 'snow', 'drip_water', 'drip_lava', 'splash', 'bubble', 'end_rod', 'dragon_breath', 'totem', 'firework', 'cherry_leaves', 'sculk_soul'];
  const petTypes = ['wolf', 'cat', 'parrot', 'fox', 'bee', 'rabbit', 'turtle', 'panda', 'axolotl', 'frog', 'allay', 'strider', 'hoglin', 'piglin'];

  // The 3D look of a modelled cosmetic: whatever texture or model is picked right now.
  let view = $state<any>(null);
  $effect(() => {
    const t = draft.metadata?.texture ?? '', m = draft.metadata?.model ?? '';
    if (!modalOpen || draft.type !== 'cosmetic' || (!t && !m)) { view = null; return; }
    let live = true;
    get<{ view: any }>(`/api/admin/content/view?texture=${encodeURIComponent(t)}&model=${encodeURIComponent(m)}`).then((r) => { if (live) view = r.view; }).catch(() => { if (live) view = null; });
    return () => { live = false; };
  });
  const slots = [['head', 'Head (hat)'], ['back', 'Back (cape / wings)'], ['hand', 'Held in hand'], ['aura', 'Around the body']] as const;

  async function load() {
    loading = true;
    try {
      const resp = await get('/api/admin/cosmetics/templates');
      templates = Array.isArray(resp) ? resp : (resp.templates ?? []);
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      loading = false;
    }
  }

  function openNew(type?: CosmeticTemplate['type']) {
    draft = {
      key: '',
      type: type ?? 'cosmetic',
      label: '',
      description: '',
      auto_grant: false,
      grant_on_level: null,
      grant_on_achievement: null,
      metadata: type === 'cosmetic' ? { cosmetic_type: '', slot: 'head', scale: 1, offset_y: 0 } : type === 'particle' ? { particle_type: 'flame', color: '#ff6b35' } : type === 'pet' ? { pet_type: 'wolf', name: '' } : type === 'join_message' || type === 'leave_message' ? { message: '' } : type === 'title' ? { prefix: '', color: '#fbbf24' } : type === 'badge' ? { icon: 'star', color: '#38bdf8' } : {},
    };
    editingId = null;
    modalOpen = true;
  }

  function openEdit(item: CosmeticTemplate) {
    draft = { ...item, metadata: { ...item.metadata } };
    editingId = item.id ?? null;
    modalOpen = true;
  }

  async function save() {
    const key = draft.key.trim();
    if (!key || key.length > 80 || !/^[a-z0-9_:.-]+$/.test(key)) {
      toast('Key must use lowercase letters, numbers, _, :, - or .', 'error');
      return;
    }
    if (!draft.label.trim()) {
      toast('Label is required', 'error');
      return;
    }
    try {
      const payload = { ...draft, key };
      if (editingId) {
        await put(`/api/admin/cosmetics/templates/${editingId}`, payload);
        toast('Template updated');
      } else {
        await post('/api/admin/cosmetics/templates', payload);
        toast('Template created');
      }
      modalOpen = false;
      await load();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function remove(id: number) {
    if (!confirm('Delete this template? This will not revoke already-granted unlocks.')) return;
    try {
      await del(`/api/admin/cosmetics/templates/${id}`);
      toast('Template deleted');
      await load();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function copyKey(key: string) {
    try {
      await navigator.clipboard.writeText(key);
      copiedKey = key;
      setTimeout(() => (copiedKey = null), 2000);
    } catch {
      toast('Failed to copy', 'error');
    }
  }

  const shown = $derived(templates.filter((t) => filter === 'all' || t.type === filter));

  $effect(() => {
    void load();
  });
</script>

<div class="page">
  <header>
    <div>
      <h1><Wand2 size={22} /> Cosmetics Studio</h1>
      <p>Create cosmetics, pets, particles, titles, badges and custom messages. Grantable via rewards.</p>
    </div>
    <button class="primary" onclick={() => openNew()}><Plus size={16} /> New template</button>
  </header>

  <div class="filters">
    {#each ['all', 'cosmetic', 'particle', 'pet', 'title', 'badge', 'join_message', 'leave_message'] as f}
      <button class:on={filter === f} onclick={() => (filter = f)}>{f === 'all' ? 'All' : typeLabels[f] ?? f}</button>
    {/each}
  </div>

  {#if loading && !templates.length}
    <div class="loading"><span class="spin">◌</span></div>
  {:else if !shown.length}
    <Empty icon={Wand2} title="No cosmetics yet" text={filter === 'all' ? 'Create your first cosmetic template to make unlockable content for players.' : `No ${typeLabels[filter] ?? filter} templates yet.`} />
  {:else}
    <div class="grid">
      {#each shown as item (item.id)}
        <article class="card">
          <div class="card-icon" style="background: color-mix(in srgb, var(--accent) 14%, transparent); color: var(--accent)">
            <svelte:component this={typeIcons[item.type] ?? Wand2} size={24} />
          </div>
          <div class="card-body">
            <div class="row-head">
              <strong>{item.label}</strong>
              <span class="type-badge">{typeLabels[item.type] ?? item.type}</span>
            </div>
            {#if item.description}
              <p class="desc">{item.description}</p>
            {/if}
            <div class="key-row">
              <code class="key">{item.key}</code>
              <button class="icon ghost" onclick={() => copyKey(item.key)} title="Copy key" aria-label="Copy key">
                {#if copiedKey === item.key}<Check size={13} />{:else}<Copy size={13} />{/if}
              </button>
            </div>
            {#if item.auto_grant}
              <small class="auto">Auto-granted {item.grant_on_level ? `at level ${item.grant_on_level}` : item.grant_on_achievement ? `via ${item.grant_on_achievement}` : ''}</small>
            {/if}
          </div>
          <div class="card-actions">
            <button class="ghost" onclick={() => openEdit(item)}>Edit</button>
            <button class="ghost icon" onclick={() => remove(item.id!)} title="Delete" aria-label="Delete"><Trash2 size={15} /></button>
          </div>
        </article>
      {/each}
    </div>
  {/if}

  <div class="quick-create">
    <h3>Quick create</h3>
    <div class="quick-grid">
      {#each ['cosmetic', 'particle', 'pet', 'title', 'badge', 'join_message', 'leave_message'] as type}
        <button class="quick-btn" onclick={() => openNew(type as CosmeticTemplate['type'])}>
          <svelte:component this={typeIcons[type] ?? Wand2} size={20} />
          <span>New {typeLabels[type] ?? type}</span>
        </button>
      {/each}
    </div>
  </div>
</div>

<Modal bind:open={modalOpen} title={editingId ? 'Edit template' : 'New cosmetic template'} width={36}>
  <div class="form">
    <div class="field">
      <label for="type">Type</label>
      <select id="type" bind:value={draft.type} disabled={!!editingId}>
        {#each ['cosmetic', 'particle', 'pet', 'title', 'badge', 'join_message', 'leave_message'] as t}
          <option value={t}>{typeLabels[t] ?? t}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="key">Key <small>(unique identifier)</small></label>
      <input id="key" bind:value={draft.key} placeholder="cosmic_wings" maxlength="80" pattern="[a-z0-9_:.\\-]+" required />
      <small class="hint">Lowercase letters, numbers, _, :, - or .</small>
    </div>

    <div class="field">
      <label for="label">Display label</label>
      <input id="label" bind:value={draft.label} placeholder="Cosmic Wings" maxlength="100" required />
    </div>

    <div class="field">
      <label for="desc">Description</label>
      <textarea id="desc" bind:value={draft.description} placeholder="Shimmering celestial wings" rows="2" maxlength="200"></textarea>
    </div>

    {#if draft.type === 'particle'}
      <div class="fields2">
        <div class="field">
          <label for="particle_type">Particle type</label>
          <select id="particle_type" bind:value={draft.metadata.particle_type}>
            {#each particleTypes as p}
              <option value={p}>{p.replaceAll('_', ' ')}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="color">Color</label>
          <input id="color" type="color" bind:value={draft.metadata.color} />
        </div>
      </div>
    {:else if draft.type === 'pet'}
      <div class="fields2">
        <div class="field">
          <label for="pet_type">Pet type</label>
          <select id="pet_type" bind:value={draft.metadata.pet_type}>
            {#each petTypes as p}
              <option value={p}>{p}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="pet_name">Default name</label>
          <input id="pet_name" bind:value={draft.metadata.name} placeholder="Fluffy" maxlength="32" />
        </div>
      </div>
    {:else if draft.type === 'join_message' || draft.type === 'leave_message'}
      <div class="field">
        <label for="message">Message template</label>
        <input id="message" bind:value={draft.metadata.message} placeholder="has arrived in style!" maxlength="100" />
        <small class="hint">Use {'{player}'} for the player name</small>
      </div>
    {:else if draft.type === 'title'}
      <div class="fields2">
        <div class="field">
          <label for="prefix">Prefix/Title text</label>
          <input id="prefix" bind:value={draft.metadata.prefix} placeholder="[Legend]" maxlength="32" />
        </div>
        <div class="field">
          <label for="title_color">Color</label>
          <input id="title_color" type="color" bind:value={draft.metadata.color} />
        </div>
      </div>
    {:else if draft.type === 'badge'}
      <div class="fields2">
        <div class="field">
          <label for="icon">Icon name</label>
          <input id="icon" bind:value={draft.metadata.icon} placeholder="star" maxlength="32" />
        </div>
        <div class="field">
          <label for="badge_color">Color</label>
          <input id="badge_color" type="color" bind:value={draft.metadata.color} />
        </div>
      </div>
    {:else if draft.type === 'cosmetic'}
      <div class="field">
        <label for="cosmetic_type">Cosmetic type</label>
        <input id="cosmetic_type" bind:value={draft.metadata.cosmetic_type} placeholder="wings" maxlength="32" />
      </div>
      <div class="model-box">
        <strong>Model</strong>
        <div class="model-preview">
          {#if view}<ModelViewer {view} height={220} />{:else}<small>No model yet. Pick a texture or a 3D model from your server assets.</small>{/if}
        </div>
        <div class="model-actions">
          <AssetPicker kind="texture" onpick={(r) => { draft.metadata.texture = r; draft.metadata.model = undefined; }} />
          <AssetPicker kind="model" onpick={(r) => { draft.metadata.model = r; draft.metadata.texture = undefined; }} />
          {#if draft.metadata.texture || draft.metadata.model}
            <button type="button" class="ghost" onclick={() => { draft.metadata.texture = undefined; draft.metadata.model = undefined; }}>Clear</button>
          {/if}
        </div>
        <small class="hint">{draft.metadata.model ? `Model ${draft.metadata.model}` : draft.metadata.texture ? `Texture ${draft.metadata.texture}` : 'Upload PNGs, Blockbench models or a whole resource pack under Custom items → Server assets & resource pack.'}</small>
        <div class="fields2">
          <div class="field">
            <label for="slot">Worn on</label>
            <select id="slot" bind:value={draft.metadata.slot}>{#each slots as [v, l]}<option value={v}>{l}</option>{/each}</select>
          </div>
          <div class="field">
            <label for="scale">Size <small>({Number(draft.metadata.scale ?? 1).toFixed(2)}×)</small></label>
            <input id="scale" type="range" min="0.25" max="3" step="0.05" bind:value={draft.metadata.scale} />
          </div>
          <div class="field">
            <label for="offset_y">Height <small>({Number(draft.metadata.offset_y ?? 0).toFixed(2)})</small></label>
            <input id="offset_y" type="range" min="-1" max="1" step="0.05" bind:value={draft.metadata.offset_y} />
          </div>
        </div>
        <small class="hint">Players see it on their character in game (Paper), within about 20 seconds of equipping. Nudge Height and Size after trying it on: models differ.</small>
      </div>
    {/if}

    <div class="model-box">
      <label class="check"><input type="checkbox" bind:checked={draft.auto_grant} /> <strong>Grant automatically</strong></label>
      {#if draft.auto_grant}
        <div class="fields2">
          <div class="field">
            <label for="grant_level">At global level</label>
            <input id="grant_level" type="number" min="1" placeholder="e.g. 10" value={draft.grant_on_level ?? ''} oninput={(e) => (draft.grant_on_level = e.currentTarget.value ? Number(e.currentTarget.value) : null)} />
          </div>
          <div class="field">
            <label for="grant_ach">Or on achievement ID</label>
            <input id="grant_ach" placeholder="achievement id" value={draft.grant_on_achievement ?? ''} oninput={(e) => (draft.grant_on_achievement = e.currentTarget.value.trim() || null)} />
          </div>
        </div>
        <small class="hint">Players who already qualify get it as soon as you save. You can also give it as a quest, achievement or level reward (Extra rewards → Cosmetic), or grant it to one player from Progression.</small>
      {:else}
        <small class="hint">Off: grant it from a reward (Extra rewards → Cosmetic) or to one player from Progression.</small>
      {/if}
    </div>
  </div>

  {#snippet footer()}
    <button class="ghost" onclick={() => (modalOpen = false)}>Cancel</button>
    <button class="primary" onclick={save}><Save size={15} /> Save template</button>
  {/snippet}
</Modal>

<style>
  .model-box { display: flex; flex-direction: column; gap: 10px; padding: 12px; border: 1px solid var(--line); border-radius: 12px; background: var(--bg-2); }
  .model-preview { min-height: 80px; display: grid; place-items: center; border-radius: 10px; background: var(--surface); overflow: hidden; }
  .model-actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .check { display: flex; align-items: center; gap: 8px; }
  .page { padding: 1.6rem 2.2rem; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--line); }
  h1 { display: flex; align-items: center; gap: 10px; margin: 0; font-size: 1.4rem; }
  header p { margin: 6px 0 0; color: var(--muted); font-size: .88rem; }
  .filters { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 18px; }
  .filters button { padding: 7px 13px; border-radius: 9px; background: var(--bg-2); border: 1px solid var(--line); color: var(--text-2); font-size: .85rem; font-weight: 520; }
  .filters button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--accent-2); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 14px; margin-bottom: 32px; }
  .card { display: flex; flex-direction: column; gap: 12px; padding: 16px; background: var(--surface); border: 1px solid var(--line); border-radius: 14px; }
  .card-icon { width: 52px; height: 52px; border-radius: 12px; display: grid; place-items: center; }
  .card-body { display: flex; flex-direction: column; gap: 8px; flex: 1; }
  .row-head { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .row-head strong { font-size: .98rem; font-weight: 600; }
  .type-badge { padding: 3px 8px; border-radius: 6px; background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent-2); font-size: .72rem; font-weight: 600; text-transform: uppercase; letter-spacing: .02em; }
  .desc { margin: 0; font-size: .86rem; color: var(--text-2); line-height: 1.4; }
  .key-row { display: flex; align-items: center; gap: 6px; }
  .key { padding: 4px 8px; border-radius: 6px; background: var(--bg-2); color: var(--muted); font-size: .8rem; font-family: 'SF Mono', Consolas, monospace; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .auto { color: var(--good); font-size: .76rem; }
  .card-actions { display: flex; gap: 6px; padding-top: 8px; border-top: 1px solid var(--line); }
  .card-actions button:first-child { flex: 1; }
  .quick-create { margin-top: 32px; padding-top: 24px; border-top: 1px solid var(--line); }
  .quick-create h3 { margin: 0 0 12px; font-size: .95rem; color: var(--text-2); }
  .quick-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 10px; }
  .quick-btn { display: flex; align-items: center; gap: 8px; padding: 11px 14px; border-radius: 10px; background: var(--bg-2); border: 1px solid var(--line); color: var(--text); font-weight: 520; justify-content: flex-start; }
  .quick-btn:hover { background: var(--surface-3); border-color: var(--accent); }
  .loading { display: flex; justify-content: center; padding: 60px 0; color: var(--muted); }
  .spin { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .form { display: flex; flex-direction: column; gap: 14px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: .88rem; font-weight: 550; color: var(--text-2); }
  .field label small { font-weight: 400; color: var(--muted); }
  .hint { font-size: .78rem; color: var(--muted); margin-top: -2px; }
  .fields2 { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
  @media (max-width: 768px) {
    .page { padding: 1rem; }
    .grid { grid-template-columns: 1fr; }
    .fields2 { grid-template-columns: 1fr; }
  }
</style>
