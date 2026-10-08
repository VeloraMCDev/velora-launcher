<script lang="ts">
  import { Award, Check, RefreshCw, Sparkles, Crown, Star, Wand2, Bird, Flame, MessageSquare, LogOut, LogIn, Trophy } from '@lucide/svelte/icons';
  import { onMount } from 'svelte';
  import { get, post } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { toast } from '../../lib/toast.svelte';
  import Empty from '../ui/Empty.svelte';

  type Unlock = { 
    key: string; 
    type: string; 
    source_type: string; 
    source_id: string; 
    equipped: boolean; 
    metadata: Record<string, unknown>;
  };

  let unlocks = $state<Unlock[]>([]);
  let loading = $state(false);
  let filter = $state<'all' | 'equipped'>('all');

  const typeIcons: Record<string, any> = {
    title: Crown,
    badge: Star,
    cosmetic: Wand2,
    particle: Sparkles,
    pet: Bird,
    join_message: LogIn,
    leave_message: LogOut,
    showcase: Trophy,
  };

  const typeColors: Record<string, string> = {
    title: '#fbbf24',
    badge: '#38bdf8',
    cosmetic: '#f472b6',
    particle: '#a78bfa',
    pet: '#fb923c',
    join_message: '#4ade80',
    leave_message: '#ef4444',
    showcase: '#fcd34d',
  };

  async function load() {
    loading = true;
    try {
      const data = await get('/api/v1/collections/me');
      unlocks = Array.isArray(data) ? data : [];
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      loading = false;
    }
  }

  async function equip(item: Unlock) {
    try {
      await post('/api/v1/collections/equip', { unlock_key: item.key, equipped: !item.equipped });
      await load();
      toast(item.equipped ? 'Unequipped' : 'Equipped', 'ok');
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  const title = (item: Unlock) => String(item.metadata.label ?? item.metadata.name ?? item.key);
  const description = (item: Unlock) => String(item.metadata.description ?? item.metadata.desc ?? '');

  const shown = $derived(unlocks.filter((u) => filter === 'all' || u.equipped));
  const byType = $derived(
    shown.reduce((acc, u) => {
      const t = u.type;
      if (!acc[t]) acc[t] = [];
      acc[t].push(u);
      return acc;
    }, {} as Record<string, Unlock[]>)
  );

  onMount(load);
</script>

<main class="pl-page">
  <header class="pl-header">
    <div>
      <h1><Award size={26} /> Collection</h1>
      <p>Titles, badges, cosmetics, particles, pets and more unlocked on your account.</p>
    </div>
    <button class="pl-icon-button" onclick={load} aria-label="Refresh" title="Refresh"><RefreshCw size={18} /></button>
  </header>

  <div class="pl-tabs" style="margin-bottom: 16px;">
    <button class:on={filter === 'all'} onclick={() => (filter = 'all')}>All <span class="pl-chip">{unlocks.length}</span></button>
    <button class:on={filter === 'equipped'} onclick={() => (filter = 'equipped')}>Equipped <span class="pl-chip">{unlocks.filter((u) => u.equipped).length}</span></button>
  </div>

  {#if loading && !unlocks.length}
    <div class="pl-loading"><div class="pl-spinner"></div></div>
  {:else if !shown.length}
    <Empty icon={Sparkles} title="No unlocks yet" text={filter === 'equipped' ? 'Equip items from your collection to see them here.' : 'Complete quests, reach level milestones or earn achievements to unlock cosmetics.'} />
  {:else}
    <div class="sections">
      {#each Object.entries(byType) as [type, items] (type)}
        <section class="type-section">
          <div class="type-header">
            <svelte:component this={typeIcons[type] ?? Award} size={20} style="color: {typeColors[type] ?? 'var(--accent)'}" />
            <h2>{type.replaceAll('_', ' ')}</h2>
            <span class="count">{items.length}</span>
          </div>
          <div class="grid">
            {#each items as item (item.key)}
              <article class="unlock-card" class:equipped={item.equipped}>
                <div class="unlock-icon" style="background: color-mix(in srgb, {typeColors[type] ?? 'var(--accent)'} 14%, transparent); color: {typeColors[type] ?? 'var(--accent)'}">
                  <svelte:component this={typeIcons[type] ?? Award} size={24} />
                </div>
                <div class="unlock-body">
                  <strong>{title(item)}</strong>
                  {#if description(item)}
                    <p>{description(item)}</p>
                  {/if}
                  <small class="source">{item.source_type.replaceAll('_', ' ')}</small>
                </div>
                <button class="equip-btn" class:on={item.equipped} onclick={() => equip(item)}>
                  {#if item.equipped}<Check size={16} /> Equipped{:else}Equip{/if}
                </button>
              </article>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</main>

<style>
  .sections { display: flex; flex-direction: column; gap: 28px; }
  .type-section { display: flex; flex-direction: column; gap: 12px; }
  .type-header { display: flex; align-items: center; gap: 10px; }
  .type-header h2 { margin: 0; font-size: 1.1rem; font-weight: 600; text-transform: capitalize; flex: 1; }
  .count { font-size: .8rem; color: var(--muted); font-weight: 500; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 12px; }
  .unlock-card { 
    display: flex; 
    align-items: center; 
    gap: 14px; 
    padding: 14px; 
    background: var(--bg-2); 
    border: 1px solid var(--line); 
    border-radius: 12px; 
    transition: border-color .15s, box-shadow .15s;
  }
  .unlock-card.equipped { border-color: var(--accent); box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 30%, transparent); }
  .unlock-icon { 
    width: 48px; 
    height: 48px; 
    border-radius: 11px; 
    display: grid; 
    place-items: center; 
    flex: none;
  }
  .unlock-body { 
    display: flex; 
    flex-direction: column; 
    gap: 4px; 
    flex: 1; 
    min-width: 0;
  }
  .unlock-body strong { font-size: .95rem; font-weight: 600; }
  .unlock-body p { margin: 0; font-size: .85rem; color: var(--text-2); line-height: 1.4; }
  .unlock-body .source { font-size: .75rem; color: var(--muted); text-transform: capitalize; }
  .equip-btn { 
    display: inline-flex; 
    align-items: center; 
    gap: 6px; 
    padding: 8px 14px; 
    border-radius: 8px; 
    border: 1px solid var(--line); 
    background: transparent; 
    color: var(--text); 
    font-size: .85rem; 
    font-weight: 500; 
    cursor: pointer; 
    transition: all .15s;
    flex: none;
  }
  .equip-btn:hover { background: var(--surface-3); }
  .equip-btn.on { 
    background: var(--accent-soft); 
    border-color: var(--accent); 
    color: var(--accent-2);
  }
  @media (max-width: 768px) {
    .grid { grid-template-columns: 1fr; }
    .unlock-card { flex-direction: column; align-items: flex-start; }
    .equip-btn { width: 100%; justify-content: center; }
  }
</style>
