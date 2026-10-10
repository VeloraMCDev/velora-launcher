<script lang="ts">
  import { pageEnabled } from '@velora/experience';
  import { experienceContext } from '../lib/experience.svelte';
  import { go } from '../lib/router.svelte';
  import { PROGRESSION_SECTIONS } from '../lib/adminNav';
  import Progression from './Progression.svelte';
  import LevelingAdmin from './LevelingAdmin.svelte';
  import QuestEditor from './QuestEditor.svelte';
  import QuestChains from './QuestChains.svelte';
  import AchievementEditor from './AchievementEditor.svelte';
  import BulkEditor from './BulkEditor.svelte';
  import RewardQueue from './RewardQueue.svelte';
  let { section }: { section: string } = $props();
  const tabs = $derived(PROGRESSION_SECTIONS.filter(tab => pageEnabled(experienceContext.instance?.experience, tab.id)));
</script>
<div class="workspace">
  <header><h1>Progression & rewards</h1><p>Set how players advance, build objectives and manage rewards in one place.</p></header>
  <nav aria-label="Progression sections">{#each tabs as tab}<button class:on={section === tab.id} aria-current={section === tab.id ? 'page' : undefined} onclick={() => go(tab.id)}>{tab.label}</button>{/each}</nav>
  {#if section === 'leveling'}<LevelingAdmin />
  {:else if section === 'quests'}<QuestEditor />
  {:else if section === 'quest-chains'}<QuestChains />
  {:else if section === 'achievements'}<AchievementEditor />
  {:else if section === 'bulk'}<BulkEditor />
  {:else if section === 'reward-queue'}<RewardQueue />
  {:else}<Progression />{/if}
</div>
<style>
  header { padding: 1.5rem 2rem 0; } h1 { margin: 0; } p { color: var(--muted); }
  nav { display: flex; flex-wrap: wrap; gap: .5rem; padding: 1rem 2rem; border-bottom: 1px solid var(--line); }
  nav button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--text); }
  @media (max-width: 600px) { header { padding: 1rem 1rem 0; } nav { padding: 1rem; } }
</style>
