<script lang="ts">
  import { Plus, Trash2, Edit, Link2, MoveVertical, Save, X, ArrowRight, Target, CheckCircle2 } from '@lucide/svelte/icons';
  import { get, post, put, del } from '../lib/api';
  import { toast } from '../lib/toast.svelte';
  import Modal from '../components/Modal.svelte';
  import Empty from '../components/Empty.svelte';

  type Quest = {
    id: string;
    title: string;
    description: string;
    quest_type: string;
    category: string;
    stat_type: string;
    target_count: number;
    xp_reward: number;
    icon: string;
    chain_id: string | null;
    chain_step: number;
    active: boolean;
  };

  type Chain = {
    id: string;
    name: string;
    description: string;
    quests: Quest[];
  };

  type ChainRecord = { id: string; name: string; description: string };
  let chains = $state<Chain[]>([]);
  let allQuests = $state<Quest[]>([]);
  let loading = $state(false);
  let modalOpen = $state(false);
  let editChainId = $state<string | null>(null);
  let chainDraft = $state({ name: '', description: '' });
  let selectedChain = $state<Chain | null>(null);
  let addQuestModalOpen = $state(false);
  let availableQuests = $state<Quest[]>([]);

  async function loadData() {
    loading = true;
    try {
      const res = await get<Quest[]>('/api/admin/quests');
      allQuests = res || [];
      
      const records = (await get<ChainRecord[]>('/api/admin/quest-chains')) || [];
      const byChain = new Map<string, Quest[]>();
      const noChain: Quest[] = [];
      allQuests.forEach((q) => {
        if (q.chain_id) {
          if (!byChain.has(q.chain_id)) byChain.set(q.chain_id, []);
          byChain.get(q.chain_id)!.push(q);
        } else noChain.push(q);
      });
      // Empty chains show up too: they are saved records, not just something quests happen to share.
      chains = records.map((r) => {
        const sorted = (byChain.get(r.id) ?? []).sort((x, y) => x.chain_step - y.chain_step);
        return { id: r.id, name: r.name, description: r.description, quests: sorted };
      });

      availableQuests = noChain;
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      loading = false;
    }
  }

  function openCreateChain() {
    chainDraft = { name: '', description: '' };
    editChainId = null;
    modalOpen = true;
  }

  function openEditChain(chain: Chain) {
    chainDraft = { name: chain.name, description: chain.description };
    editChainId = chain.id;
    modalOpen = true;
  }

  async function saveChain() {
    const name = chainDraft.name.trim();
    if (!name) {
      toast('Chain name is required', 'error');
      return;
    }

    try {
      const body = { name, description: chainDraft.description.trim() };
      if (editChainId) await put(`/api/admin/quest-chains/${encodeURIComponent(editChainId)}`, body);
      else await post('/api/admin/quest-chains', body);
      modalOpen = false;
      toast(editChainId ? 'Chain updated' : 'Chain created');
      await loadData();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  function openAddQuestToChain(chain: Chain) {
    selectedChain = chain;
    addQuestModalOpen = true;
  }

  async function addQuestToChain(quest: Quest) {
    if (!selectedChain) return;
    
    const nextStep = selectedChain.quests.length;
    
    try {
      await put(`/api/admin/quests/${quest.id}`, {
        ...quest,
        chain_id: selectedChain.id,
        chain_step: nextStep,
      });
      
      toast('Quest added to chain');
      addQuestModalOpen = false;
      await loadData();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function removeQuestFromChain(chain: Chain, quest: Quest) {
    if (!confirm(`Remove "${quest.title}" from this chain?`)) return;
    
    try {
      await put(`/api/admin/quests/${quest.id}`, {
        ...quest,
        chain_id: null,
        chain_step: 0,
      });
      
      // Reorder remaining quests
      const remaining = chain.quests.filter(q => q.id !== quest.id);
      for (let i = 0; i < remaining.length; i++) {
        await put(`/api/admin/quests/${remaining[i].id}`, {
          ...remaining[i],
          chain_step: i,
        });
      }
      
      toast('Quest removed from chain');
      await loadData();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function moveQuestInChain(chain: Chain, quest: Quest, direction: 'up' | 'down') {
    const currentIndex = chain.quests.findIndex(q => q.id === quest.id);
    if (currentIndex === -1) return;
    
    const newIndex = direction === 'up' ? currentIndex - 1 : currentIndex + 1;
    if (newIndex < 0 || newIndex >= chain.quests.length) return;
    
    try {
      const otherQuest = chain.quests[newIndex];
      
      // Swap steps
      await put(`/api/admin/quests/${quest.id}`, { ...quest, chain_step: newIndex });
      await put(`/api/admin/quests/${otherQuest.id}`, { ...otherQuest, chain_step: currentIndex });
      
      toast('Quest reordered');
      await loadData();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function deleteChain(chain: Chain) {
    if (!confirm(`Delete the "${chain.name}" chain? All quests will become independent.`)) return;
    
    try {
      await del(`/api/admin/quest-chains/${encodeURIComponent(chain.id)}`);
      toast('Chain deleted');
      await loadData();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  $effect(() => {
    void loadData();
  });
</script>

<div class="page">
  <header>
    <div>
      <h1><Link2 size={22} /> Quest Chains</h1>
      <p>Group quests into ordered multi-step sequences.</p>
    </div>
    <button class="primary" onclick={openCreateChain}><Plus size={16} /> New chain</button>
  </header>

  {#if loading && !chains.length}
    <div class="loading"><span class="spin">◌</span></div>
  {:else if !chains.length}
    <Empty icon={Link2} title="No quest chains yet" text="Create your first chain to link quests together in a sequential story or progression path." />
  {:else}
    <div class="chains">
      {#each chains as chain (chain.id)}
        <article class="chain-card">
          <div class="chain-header">
            <div class="chain-info">
              <h3>{chain.name}</h3>
              <small>{chain.quests.length} step{chain.quests.length !== 1 ? 's' : ''}{chain.description ? ` · ${chain.description}` : ''}</small>
            </div>
            <div class="chain-actions">
              <button class="ghost icon" onclick={() => openEditChain(chain)} title="Rename chain"><Edit size={16} /></button>
              <button class="ghost icon" onclick={() => openAddQuestToChain(chain)} title="Add quest"><Plus size={16} /></button>
              <button class="ghost icon" onclick={() => deleteChain(chain)} title="Delete chain"><Trash2 size={16} /></button>
            </div>
          </div>

          <div class="quest-flow">
            {#each chain.quests as quest, idx (quest.id)}
              <div class="quest-node">
                <div class="quest-node-badge">
                  <span class="step-num">{idx + 1}</span>
                </div>
                <div class="quest-node-content">
                  <strong>{quest.title}</strong>
                  <small>{quest.category} · {quest.stat_type.replace(/_/g, ' ')} ({quest.target_count})</small>
                  <span class="reward">+{quest.xp_reward} XP</span>
                </div>
                <div class="quest-node-actions">
                  <button class="ghost icon tiny" disabled={idx === 0} onclick={() => moveQuestInChain(chain, quest, 'up')} title="Move up">↑</button>
                  <button class="ghost icon tiny" disabled={idx === chain.quests.length - 1} onclick={() => moveQuestInChain(chain, quest, 'down')} title="Move down">↓</button>
                  <button class="ghost icon tiny" onclick={() => removeQuestFromChain(chain, quest)} title="Remove"><X size={14} /></button>
                </div>
              </div>
              {#if idx < chain.quests.length - 1}
                <div class="quest-connector"><ArrowRight size={18} /></div>
              {/if}
            {/each}
          </div>

          {#if chain.quests.length === 0}
            <div class="empty-chain">
              <Target size={20} />
              <span>No quests in this chain yet</span>
              <button class="ghost small" onclick={() => openAddQuestToChain(chain)}>Add a quest</button>
            </div>
          {/if}
        </article>
      {/each}
    </div>
  {/if}

  {#if availableQuests.length > 0}
    <section class="available-section">
      <h2>Available quests ({availableQuests.length})</h2>
      <p class="muted">Quests not assigned to any chain. Use + on a chain to add them.</p>
      <div class="available-grid">
        {#each availableQuests.slice(0, 12) as quest (quest.id)}
          <div class="available-quest">
            <strong>{quest.title}</strong>
            <small>{quest.category}</small>
          </div>
        {/each}
      </div>
      {#if availableQuests.length > 12}
        <p class="hint">+{availableQuests.length - 12} more</p>
      {/if}
    </section>
  {/if}
</div>

<Modal bind:open={modalOpen} title={editChainId ? 'Edit chain' : 'New quest chain'} width={32}>
  <div class="form">
    <div class="field">
      <label for="chain-name">Chain name</label>
      <input id="chain-name" bind:value={chainDraft.name} placeholder="Beginner's Journey" maxlength="60" required />
    </div>

    <div class="field">
      <label for="chain-desc">Description (optional)</label>
      <textarea id="chain-desc" bind:value={chainDraft.description} placeholder="A series of quests for new players..." rows="2" maxlength="200"></textarea>
    </div>

    <div class="info-box">
      <strong>How chains work:</strong>
      <ul>
        <li>A chain is an ordered list of quests. Create it here, then add quests with the + button.</li>
        <li>Each quest keeps its own XP and rewards</li>
        <li>Deleting a chain keeps its quests; they just stop being grouped</li>
      </ul>
    </div>
  </div>

  {#snippet footer()}
    <button class="ghost" onclick={() => (modalOpen = false)}>Cancel</button>
    <button class="primary" onclick={saveChain}><Save size={15} /> Save chain</button>
  {/snippet}
</Modal>

<Modal bind:open={addQuestModalOpen} title="Add quest to chain" width={36}>
  {#if selectedChain}
    <p class="muted">Select a quest to add to "{selectedChain.name}". It will be added as step {selectedChain.quests.length + 1}.</p>
    
    {#if availableQuests.length === 0}
      <div class="empty-state">
        <Target size={24} />
        <span>All quests are already in chains</span>
        <small>Create new quests in the Quest Editor first</small>
      </div>
    {:else}
      <div class="quest-picker">
        {#each availableQuests as quest (quest.id)}
          <button class="quest-pick-item" onclick={() => addQuestToChain(quest)}>
            <div>
              <strong>{quest.title}</strong>
              <small>{quest.description}</small>
              <span class="quest-meta">{quest.category} · {quest.stat_type.replace(/_/g, ' ')} ({quest.target_count}) · +{quest.xp_reward} XP</span>
            </div>
            <Plus size={18} />
          </button>
        {/each}
      </div>
    {/if}
  {/if}

  {#snippet footer()}
    <button class="ghost" onclick={() => (addQuestModalOpen = false)}>Cancel</button>
  {/snippet}
</Modal>

<style>
  .page { padding: 1.6rem 2.2rem; max-width: 1400px; margin: 0 auto; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--line); }
  h1 { display: flex; align-items: center; gap: 10px; margin: 0; font-size: 1.4rem; }
  header p { margin: 6px 0 0; color: var(--muted); font-size: .88rem; }

  .chains { display: flex; flex-direction: column; gap: 20px; }
  .chain-card { padding: 20px; background: var(--surface); border: 1px solid var(--line); border-radius: 14px; }
  .chain-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid var(--line); }
  .chain-info h3 { margin: 0; font-size: 1.1rem; font-weight: 600; }
  .chain-info small { color: var(--muted); font-size: .8rem; }
  .chain-actions { display: flex; gap: 4px; }

  .quest-flow { display: flex; align-items: center; gap: 12px; overflow-x: auto; padding: 8px 0; }
  .quest-node { display: flex; align-items: center; gap: 12px; padding: 14px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 12px; min-width: 280px; flex-shrink: 0; }
  .quest-node-badge { width: 36px; height: 36px; border-radius: 10px; background: var(--accent-soft); color: var(--accent-2); display: grid; place-items: center; font-weight: 700; flex: none; }
  .step-num { font-size: .95rem; }
  .quest-node-content { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 0; }
  .quest-node-content strong { font-size: .92rem; font-weight: 600; }
  .quest-node-content small { font-size: .78rem; color: var(--muted); text-transform: capitalize; }
  .quest-node-content .reward { font-size: .8rem; color: var(--good); font-weight: 600; }
  .quest-node-actions { display: flex; flex-direction: column; gap: 2px; flex: none; }
  .quest-node-actions .tiny { padding: 4px 6px; font-size: .7rem; }

  .quest-connector { color: var(--accent); flex: none; }

  .empty-chain { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 40px; color: var(--muted); }

  .available-section { margin-top: 32px; padding-top: 24px; border-top: 1px solid var(--line); }
  .available-section h2 { margin: 0 0 6px; font-size: 1rem; font-weight: 600; }
  .available-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 10px; margin-top: 14px; }
  .available-quest { padding: 12px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; display: flex; flex-direction: column; gap: 4px; }
  .available-quest strong { font-size: .88rem; }
  .available-quest small { font-size: .76rem; color: var(--muted); text-transform: capitalize; }

  .loading { display: flex; justify-content: center; padding: 60px 0; color: var(--muted); }
  .spin { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .form { display: flex; flex-direction: column; gap: 14px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field label { font-size: .88rem; font-weight: 550; color: var(--text-2); }
  .muted { color: var(--muted); font-size: .86rem; margin: 0 0 12px; }
  .hint { font-size: .8rem; color: var(--muted); margin: 8px 0 0; text-align: center; }

  .info-box { padding: 14px; background: color-mix(in srgb, var(--accent) 8%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent); border-radius: 10px; }
  .info-box strong { font-size: .88rem; color: var(--text); }
  .info-box ul { margin: 8px 0 0; padding-left: 20px; }
  .info-box li { font-size: .84rem; color: var(--text-2); line-height: 1.6; }

  .empty-state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 40px; color: var(--muted); }
  
  .quest-picker { display: flex; flex-direction: column; gap: 8px; max-height: 400px; overflow-y: auto; }
  .quest-pick-item { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 12px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; text-align: left; }
  .quest-pick-item:hover { background: var(--surface-3); border-color: var(--accent); }
  .quest-pick-item strong { font-size: .92rem; font-weight: 600; display: block; margin-bottom: 4px; }
  .quest-pick-item small { font-size: .82rem; color: var(--text-2); display: block; margin-bottom: 4px; }
  .quest-pick-item .quest-meta { font-size: .76rem; color: var(--muted); text-transform: capitalize; }

  @media (max-width: 768px) {
    .page { padding: 1rem; }
    .quest-flow { flex-direction: column; align-items: stretch; }
    .quest-connector { transform: rotate(90deg); }
    .quest-node { min-width: 100%; }
  }
</style>
