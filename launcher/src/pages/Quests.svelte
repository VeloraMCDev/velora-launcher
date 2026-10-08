<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Target, Trophy, Clock, Sparkles, Check, Gift, Pickaxe, Swords, Hammer,
    Wheat, Users, Compass, RefreshCw, LoaderCircle, Flame, Star, Award, Shield
  } from '@lucide/svelte';
  import { abs } from '../lib/store.svelte';
  import { imageIcon, resolveIcon } from '../lib/achievementIcons';
  import RankTitle from '../components/RankTitle.svelte';
  import AchievementIcon from '../components/AchievementIcon.svelte';
  import { invoke } from '../lib/tauri';
  import { toast } from '../lib/store.svelte';
  import type { UserLevelInfo, ServerLevelInfo, UserQuest, Achievement } from '../lib/types';

  let activeTab = $state<'daily' | 'weekly' | 'achievements'>('daily');
  let selectedCategory = $state<string>('all');
  let loading = $state(false);
  let claimingQuestId = $state<string | null>(null);

  let levelInfo = $state<UserLevelInfo | null>(null);
  let userQuests = $state<UserQuest[]>([]);
  let achievements = $state<Achievement[]>([]);

  const categoryIcons: Record<string, any> = {
    mining: Pickaxe,
    combat: Swords,
    building: Hammer,
    farming: Wheat,
    social: Users,
    exploration: Compass
  };
  const questIcons: Record<string, any> = { pickaxe: Pickaxe, swords: Swords, blocks: Hammer, wheat: Wheat, users: Users, compass: Compass, target: Target };

  let extras = $state<Record<string, string[]>>({});

  async function loadData() {
    invoke<Record<string, string[]>>('get_reward_summaries').then((r) => (extras = r ?? {})).catch(() => {});
    loading = true;
    try {
      const [lvl, quests, achs] = await Promise.all([
        invoke<UserLevelInfo>('get_my_level').catch(() => null),
        invoke<UserQuest[]>('get_my_quests').catch(() => []),
        invoke<Achievement[]>('get_my_achievements').catch(() => [])
      ]);
      if (lvl) levelInfo = lvl;
      userQuests = quests;
      achievements = achs;
    } catch (e: any) {
      toast(e?.message ?? 'Failed to load quests data', 'error');
    } finally {
      loading = false;
    }
  }

  async function handleClaimQuest(questId: string) {
    claimingQuestId = questId;
    try {
      const updatedLevel = await invoke<UserLevelInfo>('claim_quest', { questId });
      levelInfo = updatedLevel;
      // Mark as claimed locally
      userQuests = userQuests.map((uq) => {
        const id = uq.quest?.id || (uq as any).id;
        if (id === questId) {
          return { ...uq, claimed: true };
        }
        return uq;
      });
      toast('Quest reward claimed! XP awarded to your Global Level.', 'ok');
    } catch (e: any) {
      toast(e?.message ?? 'Failed to claim reward', 'error');
    } finally {
      claimingQuestId = null;
    }
  }

  const filteredQuests = $derived(
    userQuests.filter((uq) => {
      const q = uq.quest || uq;
      const qType = q.quest_type || (q as any).period || 'daily';
      if (qType !== activeTab) return false;
      const cat = q.category || 'mining';
      if (selectedCategory !== 'all' && cat !== selectedCategory) return false;
      return true;
    })
  );

  const completedCount = $derived(
    userQuests.filter((uq) => {
      const q = uq.quest || uq;
      const qType = q.quest_type || (q as any).period || 'daily';
      return qType === activeTab && uq.completed;
    }).length
  );

  const totalTabQuests = $derived(
    userQuests.filter((uq) => {
      const q = uq.quest || uq;
      const qType = q.quest_type || (q as any).period || 'daily';
      return qType === activeTab;
    }).length
  );

  const unlockedAchievementsCount = $derived(
    achievements.filter((a) => a.unlocked).length
  );

  function getTimeUntilReset(type: 'daily' | 'weekly'): string {
    const now = new Date();
    if (type === 'daily') {
      const tomorrow = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() + 1));
      const diffMs = tomorrow.getTime() - now.getTime();
      const hours = Math.floor(diffMs / (1000 * 60 * 60));
      const mins = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
      return `${hours}h ${mins}m`;
    } else {
      const day = now.getUTCDay();
      const daysUntilMonday = (8 - day) % 7 || 7;
      const nextMonday = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() + daysUntilMonday));
      const diffMs = nextMonday.getTime() - now.getTime();
      const days = Math.floor(diffMs / (1000 * 60 * 60 * 24));
      const hours = Math.floor((diffMs % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
      return `${days}d ${hours}h`;
    }
  }

  onMount(() => {
    loadData();
  });
</script>

<div class="quests-page">
  <!-- Page Header -->
  <div class="page-header">
    <div class="title-wrap">
      <h1><Target class="target-icon" size={26} /> Quests & Progression</h1>
      <p class="lead">Complete daily and weekly quests across servers to level up your Velora account.</p>
    </div>
    <button class="ghost icon" onclick={loadData} title="Refresh" aria-label="Refresh">
      <RefreshCw size={17} class={loading ? 'spin' : ''} />
    </button>
  </div>

  <!-- Global Level Summary Hero Card -->
  {#if levelInfo}
    <div class="hero-level glass">
      <div class="level-badge-wrap">
        <div class="level-circle">
          <span class="lvl-label">LVL</span>
          <span class="lvl-number">{levelInfo.level}</span>
        </div>
        {#if levelInfo.rank}
          <div class="rank-badge-pill">
            <Trophy size={11} /> #{levelInfo.rank}
          </div>
        {/if}
      </div>
      <div class="level-details">
        <div class="level-title-row">
          <div class="title-meta">
            <span class="level-name">Velora Progression</span>
            {#if levelInfo.rank}
              <span class="badge rank-title-badge"><Trophy size={12} /> Global Rank #{levelInfo.rank}</span>
            {/if}
            {#if levelInfo.title}
              <span class="badge title-badge"><RankTitle text={levelInfo.title} image={levelInfo.title_image} /></span>
            {/if}
          </div>
          <span class="xp-ratio">
            <strong>{levelInfo.current_xp.toLocaleString()}</strong> / {levelInfo.next_level_xp.toLocaleString()} XP
          </span>
        </div>
        <div class="progress-bar-bg">
          <div class="progress-fill" style:width="{Math.min(100, Math.max(0, levelInfo.progress_pct))}%"></div>
        </div>
        <div class="level-footer">
          <span class="muted tiny">
            {#if levelInfo.rank}
              You are currently ranked #{levelInfo.rank} globally! Earn XP across all servers to climb the leaderboard.
            {:else}
              Playing on any Velora server earns Global XP. Unlocks exclusive titles, badges, and cosmetic items.
            {/if}
          </span>
          {#if levelInfo.badges && levelInfo.badges.length > 0}
            <div class="badges-row">
              {#each levelInfo.badges as badge}
                <span class="badge icon-badge" title={badge}><Star size={11} /> {badge}</span>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Segmented Tabs Navigation -->
  <div class="nav-bar">
    <div class="tabs-segmented">
      <button class:active={activeTab === 'daily'} onclick={() => (activeTab = 'daily')}>
        <Clock size={15} /> Daily Quests
      </button>
      <button class:active={activeTab === 'weekly'} onclick={() => (activeTab = 'weekly')}>
        <Flame size={15} /> Weekly Quests
      </button>
      <button class:active={activeTab === 'achievements'} onclick={() => (activeTab = 'achievements')}>
        <Trophy size={15} /> Achievements ({unlockedAchievementsCount}/{achievements.length})
      </button>
    </div>

    {#if activeTab !== 'achievements'}
      <div class="reset-countdown muted tiny">
        <Clock size={13} />
        <span>Resets in: <strong>{getTimeUntilReset(activeTab)}</strong></span>
        <span class="divider">•</span>
        <span>Completed: <strong>{completedCount} / {totalTabQuests}</strong></span>
      </div>
    {/if}
  </div>

  <!-- Content Section -->
  {#if loading && !userQuests.length}
    <div class="center-state"><LoaderCircle class="spin" size={32} /></div>
  {:else if activeTab === 'achievements'}
    <!-- Achievements List with Minecraft Frame Styling -->
    <div class="achievements-section">
      <div class="achievements-grid">
        {#each achievements as ach (ach.id)}
          <div class="achievement-card glass" class:unlocked={ach.unlocked} class:frame-task={ach.frame_type === 'task'} class:frame-goal={ach.frame_type === 'goal'} class:frame-challenge={ach.frame_type === 'challenge'}>
            <AchievementIcon {ach} locked={!ach.unlocked} />
            <div class="achievement-body">
              <div class="achievement-header">
                <span class="ach-title">{ach.title}</span>
                <span class="badge frame-badge {ach.frame_type}">{ach.frame_type.toUpperCase()}</span>
              </div>
              <p class="ach-desc">{ach.description}</p>
              <div class="ach-footer">
                <span class="xp-tag">+{ach.xp_reward} XP</span>
                {#each extras[`achievement:${ach.id}`] ?? [] as extra}<span class="extra-tag"><Gift size={10} /> {extra}</span>{/each}
                {#if ach.unlocked}
                  <span class="badge unlocked-tag"><Check size={11} /> Unlocked</span>
                {:else}
                  <span class="badge locked-tag">Locked</span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <!-- Quests List -->
    <div class="quests-content">
      <!-- Category Filter Pills -->
      <div class="category-filters">
        {#each ['all', 'mining', 'combat', 'building', 'farming', 'social', 'exploration'] as cat}
          {@const CatIcon = categoryIcons[cat]}
          <button
            class="pill-btn"
            class:active={selectedCategory === cat}
            onclick={() => (selectedCategory = cat)}
          >
            {#if cat !== 'all' && CatIcon}
              <CatIcon size={13} />
            {/if}
            <span class="cap">{cat}</span>
          </button>
        {/each}
      </div>

      {#if filteredQuests.length === 0}
        <div class="empty-state glass">
          <Target size={36} class="muted" />
          <p>No {activeTab} quests found in this category.</p>
        </div>
      {:else}
        <div class="quests-grid">
          {#each filteredQuests as uq ((uq.quest?.id || (uq as any).id))}
            {@const q = uq.quest || uq}
            {@const currentCount = uq.current_count ?? (uq as any).progress ?? 0}
            {@const targetCount = q.target_count || (q as any).target_count || 1}
            {@const pct = Math.min(100, Math.round((currentCount / Math.max(1, targetCount)) * 100))}
            {@const IconComponent = questIcons[q.icon] || categoryIcons[q.category] || Target}
            {@const qId = q.id || (uq as any).id}

            <div class="quest-card glass" class:completed={uq.completed} class:claimed={uq.claimed}>
              <div class="quest-top">
                <div class="quest-icon-wrapper">
                  {#if imageIcon(q.icon)}<img src={abs(imageIcon(q.icon))} alt="" style="width: 32px; height: 32px; object-fit: contain" />{:else if q.icon && !questIcons[q.icon]}<span>{resolveIcon({ icon_item: q.icon }).emoji}</span>{:else}<IconComponent size={20} class="cat-icon" />{/if}
                </div>
                <div class="quest-meta">
                  <div class="quest-title-row">
                    <span class="quest-title">{q.title}</span>
                    <span class="badge cat-badge">{q.category}</span>
                  </div>
                  <p class="quest-desc">{q.description}</p>
                </div>
              </div>

              <div class="quest-progress-wrap">
                <div class="progress-labels">
                  <span class="progress-ratio">
                    <strong>{currentCount}</strong> / {targetCount} ({pct}%)
                  </span>
                  <span class="xp-badge">+{q.xp_reward} XP</span>
                  {#each extras[`quest:${q.id}`] ?? [] as extra}<span class="extra-tag"><Gift size={10} /> {extra}</span>{/each}
                </div>
                <div class="progress-bar-bg">
                  <div class="progress-fill" style:width="{pct}%"></div>
                </div>
              </div>

              <div class="quest-action-row">
                {#if uq.claimed}
                  <span class="badge claimed-badge"><Check size={13} /> Reward Claimed</span>
                {:else if uq.completed}
                  <button
                    class="primary claim-btn"
                    onclick={() => handleClaimQuest(qId)}
                    disabled={claimingQuestId === qId}
                  >
                    {#if claimingQuestId === qId}
                      <LoaderCircle size={14} class="spin" />
                    {:else}
                      <Gift size={14} />
                    {/if}
                    Claim +{q.xp_reward} XP
                  </button>
                {:else}
                  <span class="badge in-progress-badge">In Progress</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .quests-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 1.6rem 2.2rem 2.5rem;
    gap: 1.2rem;
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    border-bottom: 1px solid var(--line);
    padding-bottom: 1rem;
  }
  .title-wrap h1 {
    font-size: 1.5rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  :global(.target-icon) {
    color: var(--accent);
  }
  .lead {
    color: var(--muted);
    font-size: 0.88rem;
    margin-top: 0.2rem;
  }
  .hero-level {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 1.2rem 1.6rem;
    border-radius: var(--radius);
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 15%, var(--surface)) 0%, var(--surface) 100%);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .level-badge-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .level-circle {
    width: 4.5rem;
    height: 4.5rem;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 25%, transparent);
    border: 3px solid var(--accent);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    box-shadow: 0 0 16px color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .lvl-label {
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--accent);
  }
  .lvl-number {
    font-size: 1.6rem;
    font-weight: 800;
    line-height: 1;
    color: var(--text);
  }
  .rank-badge-pill {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: color-mix(in srgb, #f59e0b 25%, transparent);
    color: #f59e0b;
    border: 1px solid color-mix(in srgb, #f59e0b 50%, transparent);
    border-radius: 99rem;
    padding: 2px 7px;
    font-size: 0.72rem;
    font-weight: 700;
    margin-top: -6px;
    z-index: 2;
  }
  .rank-title-badge {
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #f59e0b;
    border: 1px solid color-mix(in srgb, #f59e0b 45%, transparent);
    font-weight: 700;
  }
  .level-details {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }
  .level-title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .title-meta {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .level-name {
    font-size: 1.15rem;
    font-weight: 700;
  }
  .title-badge {
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #f59e0b;
    border: 1px solid color-mix(in srgb, #f59e0b 40%, transparent);
  }
  .xp-ratio {
    font-size: 0.88rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .xp-ratio strong {
    color: var(--text);
  }
  .progress-bar-bg {
    width: 100%;
    height: 0.55rem;
    border-radius: 99rem;
    background: color-mix(in srgb, var(--text) 10%, transparent);
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    border-radius: 99rem;
    background: linear-gradient(90deg, var(--accent), color-mix(in srgb, var(--accent) 60%, white));
    transition: width 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .level-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.6rem;
  }
  .badges-row {
    display: flex;
    gap: 0.4rem;
  }
  .icon-badge {
    background: color-mix(in srgb, var(--text) 8%, transparent);
    font-size: 0.72rem;
  }
  .nav-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.8rem;
  }
  .tabs-segmented {
    display: flex;
    background: color-mix(in srgb, var(--surface) 80%, transparent);
    padding: 0.25rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    gap: 0.2rem;
  }
  .tabs-segmented button {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: none;
    padding: 0.45rem 1rem;
    font-size: 0.85rem;
    font-weight: 560;
    color: var(--muted);
    border-radius: calc(var(--radius-sm) - 2px);
    transition: all 0.15s;
  }
  .tabs-segmented button.active {
    background: color-mix(in srgb, var(--text) 10%, transparent);
    color: var(--text);
  }
  .reset-countdown {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .divider {
    color: var(--line-strong);
  }
  .quests-content {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .category-filters {
    display: flex;
    gap: 0.4rem;
    overflow-x: auto;
    padding-bottom: 0.2rem;
  }
  .pill-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
    font-weight: 560;
    border-radius: 99rem;
    background: color-mix(in srgb, var(--text) 6%, transparent);
    border: 1px solid transparent;
    color: var(--muted);
    transition: all 0.15s;
  }
  .pill-btn.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    color: var(--text);
  }
  .cap {
    text-transform: capitalize;
  }
  .quests-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
    gap: 1rem;
  }
  .quest-card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 1.1rem 1.25rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
    gap: 0.9rem;
    transition: transform 0.15s, border-color 0.15s;
  }
  .quest-card:hover {
    transform: translateY(-2px);
    border-color: var(--line-strong);
  }
  .quest-card.completed {
    border-color: color-mix(in srgb, var(--success) 45%, transparent);
  }
  .quest-card.claimed {
    opacity: 0.75;
  }
  .quest-top {
    display: flex;
    gap: 0.8rem;
    align-items: flex-start;
  }
  .quest-icon-wrapper {
    width: 2.2rem;
    height: 2.2rem;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    color: var(--accent);
  }
  .quest-meta {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .quest-title-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.4rem;
  }
  .quest-title {
    font-size: 0.92rem;
    font-weight: 650;
    line-height: 1.2;
  }
  .cat-badge {
    text-transform: uppercase;
    font-size: 0.65rem;
    letter-spacing: 0.05em;
    background: color-mix(in srgb, var(--text) 8%, transparent);
    color: var(--muted);
  }
  .quest-desc {
    font-size: 0.82rem;
    color: var(--muted);
    line-height: 1.35;
  }
  .quest-progress-wrap {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .progress-labels {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    justify-content: space-between;
    font-size: 0.8rem;
  }
  .xp-badge {
    color: var(--accent);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .quest-action-row {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    border-top: 1px solid color-mix(in srgb, var(--line) 40%, transparent);
    padding-top: 0.6rem;
  }
  .claim-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.82rem;
    padding: 0.4rem 0.9rem;
    background: var(--success);
    border-color: var(--success);
    animation: pulse 2s infinite ease-in-out;
  }
  @keyframes pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--success) 50%, transparent); }
    50% { box-shadow: 0 0 0 6px color-mix(in srgb, var(--success) 0%, transparent); }
  }
  .claimed-badge {
    background: color-mix(in srgb, var(--success) 15%, transparent);
    color: var(--success);
  }
  .in-progress-badge {
    background: color-mix(in srgb, var(--text) 8%, transparent);
    color: var(--muted);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.72rem;
    font-weight: 600;
    padding: 0.16rem 0.5rem;
    border-radius: 99rem;
  }
  .center-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.8rem;
    padding: 3rem 1.5rem;
    text-align: center;
    border-radius: var(--radius);
  }
  /* Achievements Section */
  .achievements-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
    gap: 1rem;
  }
  .achievement-card {
    display: flex;
    gap: 0.9rem;
    padding: 1.1rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
    position: relative;
    opacity: 0.65;
    transition: opacity 0.15s, border-color 0.15s;
  }
  .achievement-card.unlocked {
    opacity: 1;
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .achievement-card.frame-challenge.unlocked {
    border-color: #f59e0b88;
    background: linear-gradient(135deg, color-mix(in srgb, #f59e0b 8%, var(--surface)) 0%, var(--surface) 100%);
  }
  .achievement-icon-frame {
    width: 2.8rem;
    height: 2.8rem;
    border-radius: var(--radius-sm);
    border: 2px solid var(--line);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    box-shadow: inset 0 0 6px rgba(0, 0, 0, 0.4);
  }
  .sprite-icon {
    font-size: 1.4rem;
  }
  .achievement-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .achievement-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .ach-title {
    font-size: 0.92rem;
    font-weight: 700;
  }
  .frame-badge {
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.05em;
  }
  .frame-badge.task { background: #64748b33; color: #94a3b8; }
  .frame-badge.goal { background: #3b82f633; color: #60a5fa; }
  .frame-badge.challenge { background: #f59e0b33; color: #fbbf24; }
  .ach-desc {
    font-size: 0.8rem;
    color: var(--muted);
    line-height: 1.35;
  }
  .ach-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.2rem;
  }
  .extra-tag { display: inline-flex; align-items: center; gap: 0.25rem; padding: 0.15rem 0.5rem; border-radius: 99px; font-size: 0.7rem; font-weight: 600; color: #fbbf24; background: color-mix(in srgb, #fbbf24 14%, transparent); border: 1px solid color-mix(in srgb, #fbbf24 30%, transparent); }
  .xp-tag {
    font-size: 0.78rem;
    font-weight: 700;
    color: var(--accent);
  }
  .unlocked-tag {
    background: color-mix(in srgb, var(--success) 20%, transparent);
    color: var(--success);
  }
  .locked-tag {
    background: color-mix(in srgb, var(--text) 8%, transparent);
    color: var(--muted);
  }
</style>
