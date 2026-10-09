<script lang="ts">
  import { onMount } from 'svelte';
  import { LoaderCircle, Download, X } from '@lucide/svelte';
  import Background from './components/Background.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import Rail from './components/Rail.svelte';
  import Modal from './components/Modal.svelte';
  import SignIn from './components/SignIn.svelte';
  import InstanceSettings from './components/InstanceSettings.svelte';
  import CrashDialog from './components/CrashDialog.svelte';
  import Toasts from './components/Toasts.svelte';
  import TexturesBanner from './components/TexturesBanner.svelte';
  import { initTextures } from './lib/textures.svelte';
  import Setup from './pages/Setup.svelte';
  import Login from './pages/Login.svelte';
  import Market from './pages/Market.svelte';
  import Casino from './pages/Casino.svelte';
  import Collections from './pages/Collections.svelte';
  import SocialSidebar from './components/SocialSidebar.svelte';
  import { pageEnabled } from '@velora/experience';
  import ExperienceWidgets from '@velora/experience/ExperienceWidgets.svelte';
  import { selectedInstance } from './lib/store.svelte';
  import Home from './pages/Home.svelte';
  import Settings from './pages/Settings.svelte';
  import Stats from './pages/Stats.svelte';
  import Quests from './pages/Quests.svelte';
  import Guilds from './pages/Guilds.svelte';
  import Commands from './pages/Commands.svelte';
  import Social from './pages/Social.svelte';
  import PlayerProfileModal from './components/PlayerProfileModal.svelte';
  import CommandPalette from './components/CommandPalette.svelte';
  import { app, init, refreshUpdates } from './lib/store.svelte';
  import { errorText } from './lib/tauri';

  $effect(() => {
    if (!pageEnabled(selectedInstance()?.experience, app.view)) app.view = 'home';
  });
  let fatal = $state('');
  let dismissedUpdate = $state<string | null>(null);

  onMount(() => {
    initTextures();
    init().catch((e) => (fatal = errorText(e)));
    const timer = setInterval(() => void refreshUpdates(), 5 * 60_000);
    const focus = () => void refreshUpdates();
    window.addEventListener('focus', focus);
    return () => { clearInterval(timer); window.removeEventListener('focus', focus); };
  });
</script>

<Background />

<div class="shell">
  <TitleBar />
  {#if fatal && !app.manifest && app.view !== 'settings'}
    <div class="center"><div class="glass msg"><h2>Can't reach the server</h2><p class="muted selectable">{fatal}</p><button class="primary" onclick={() => location.reload()}>Try again</button></div></div>
  {:else if !app.ready}
    <div class="center"><LoaderCircle class="spin" size={28} /></div>
  {:else if app.view === 'settings'}
    <div class="body">
      <Rail /><main><Settings /></main>
      {#if app.settings?.show_social_sidebar && app.accounts.length && app.manifest}<SocialSidebar />{/if}
    </div>
  {:else if !app.panelUrl || !app.manifest}
    <Setup />
  {:else if app.accounts.length === 0}
    <Login />
  {:else}
    <div class="body">
      <Rail />
      <main>
        {#key (app.selected ?? '') + app.view}
        {#if app.view === 'experience'}
          <div class="experience-page"><h1>{selectedInstance()?.name}</h1><ExperienceWidgets experience={selectedInstance()?.experience} /></div>
        {:else if app.view === 'stats'}
          <Stats />
        {:else if app.view === 'quests'}
          <Quests />
        {:else if app.view === 'collections'}
          <Collections />
        {:else if app.view === 'guilds'}
          <Guilds />
        {:else if app.view === 'social'}
          <Social />
        {:else if app.view === 'market'}
          <Market />
        {:else if app.view === 'casino'}
          <Casino />
        {:else if app.view === 'commands'}
          <Commands />
        {:else}
          <Home />
        {/if}
        {/key}
      </main>
      {#if app.settings?.show_social_sidebar}<SocialSidebar />{/if}
    </div>
  {/if}
</div>

{#if app.update && dismissedUpdate !== app.update.version && app.view !== 'settings'}
  <div class="update glass">
    <Download size={16} />
    <span>New update: Launcher {app.update.version}</span>
    <button class="sm primary" onclick={() => { app.view = 'settings'; app.settingsTab = 'about'; }}>View</button>
    <button class="ghost icon sm" aria-label="Dismiss" onclick={() => (dismissedUpdate = app.update?.version ?? null)}><X size={14} /></button>
  </div>
{/if}

<Modal bind:open={app.addAccount} title="Add account" width={26}>
  <SignIn />
</Modal>
<InstanceSettings />
{#if app.viewProfileUuid}
  <PlayerProfileModal uuid={app.viewProfileUuid} onclose={() => (app.viewProfileUuid = null)} />
{/if}
<CrashDialog />
<CommandPalette />
<Toasts />
<TexturesBanner belowUpdate={!!app.update && dismissedUpdate !== app.update.version && app.view !== 'settings'} />

<style>
  .experience-page { padding: 2rem; overflow-y: auto; height: 100%; }
  .shell { position: relative; z-index: 1; height: 100%; display: flex; flex-direction: column; }
  .body { flex: 1; display: flex; min-height: 0; }
  main { flex: 1; min-width: 0; min-height: 0; position: relative; }
  .center { flex: 1; display: grid; place-items: center; color: var(--accent); }
  .msg { padding: 2rem; max-width: 28rem; display: flex; flex-direction: column; gap: 0.8rem; color: var(--text); }
  .update { position: fixed; top: 3.2rem; left: 50%; transform: translateX(-50%); z-index: 40; display: flex; align-items: center; gap: 0.7rem; padding: 0.45rem 0.5rem 0.45rem 1rem; font-size: 0.87rem; border-radius: var(--radius); background: var(--surface); animation: fade 0.2s ease; }
  .update :global(svg:first-child) { color: var(--accent); }
</style>
