<script lang="ts">
  import Sidebar from './components/Sidebar.svelte';
  import AdminTopbar from './components/AdminTopbar.svelte';
  import Palette, { type PaletteItem } from './components/Palette.svelte';
  import { experienceContext } from './lib/experience.svelte';
  import { pageEnabled, PLATFORM_PAGES } from '@scopenet/experience';
  import InstanceControl from './pages/InstanceControl.svelte';
  import ExperienceEditor from './pages/ExperienceEditor.svelte';
  import EconomyAdmin from './pages/EconomyAdmin.svelte';
  import { adminGroups } from './lib/adminNav';
  import { COMMAND_GROUPS } from '@scopenet/commands';
  import { LogOut, Smartphone, Globe } from '@lucide/svelte';
  import Toasts from './components/Toasts.svelte';
  import Login from './pages/Login.svelte';
  import Dashboard from './pages/Dashboard.svelte';
  import Instances from './pages/Instances.svelte';
  import InstanceEditor from './pages/InstanceEditor.svelte';
  import Users from './pages/Users.svelte';
  import Branding from './pages/Branding.svelte';
  import Settings from './pages/Settings.svelte';
  import Servers from './pages/Servers.svelte';
  import ServerDetail from './pages/ServerDetail.svelte';
  import Capes from './pages/Capes.svelte';
  import Activity from './pages/Activity.svelte';
  import LandingBuilder from './pages/LandingBuilder.svelte';
  import PublicLanding from './pages/PublicLanding.svelte';
  import QuestEditor from './pages/QuestEditor.svelte';
  import QuestChains from './pages/QuestChains.svelte';
  import AchievementEditor from './pages/AchievementEditor.svelte';
  import RewardQueue from './pages/RewardQueue.svelte';
  import CompanionBuilder from './pages/CompanionBuilder.svelte';
  import LevelingAdmin from './pages/LevelingAdmin.svelte';
  import Progression from './pages/Progression.svelte';
  import BulkEditor from './pages/BulkEditor.svelte';
  import DiscordStudio from './pages/DiscordStudio.svelte';
  import Emails from './pages/Emails.svelte';
  import Events from './pages/Events.svelte';
  import GuildsAdmin from './pages/GuildsAdmin.svelte';
  import Commands from './pages/Commands.svelte';
  import ContentStudio from './pages/ContentStudio.svelte';
  import CosmeticsStudio from './pages/CosmeticsStudio.svelte';
  import LuckPermsAdmin from './pages/LuckPermsAdmin.svelte';
  import Casino from './pages/Casino.svelte';
  import BoardAdmin from './pages/BoardAdmin.svelte';
  import AdminClaims from './pages/AdminClaims.svelte';
  import ChatFormat from './pages/ChatFormat.svelte';
  import ScheduledTasks from './pages/ScheduledTasks.svelte';
  import { get } from './lib/api';
  import { go, route } from './lib/router.svelte';
  import { nativeApp } from './lib/native';
  import { logout, session, type Me } from './lib/session.svelte';

  $effect(() => {
    const id = route.instanceId;
    experienceContext.instance = null;
    experienceContext.loading = !!id;
    if (!id || !session.token || route.name === 'play' || session.user?.role !== 'admin') { experienceContext.loading = false; return; }
    let alive = true;
    get(`/api/admin/instances/${encodeURIComponent(id)}`).then(r => {
      if (alive) experienceContext.instance = r.instance;
    }).catch(() => { if (alive) go('instances'); }).finally(() => { if (alive) experienceContext.loading = false; });
    return () => { alive = false; };
  });

  let brand = $state<{ name: string; logo_url: string | null }>({ name: 'SCOPENET', logo_url: null });
  let registration = $state<'closed' | 'approval' | 'open'>('closed');
  let landingEnabled = $state<boolean | null>(null);
  const preview = new URLSearchParams(location.search).has('landingPreview');
  $effect(() => { get<{enabled: boolean}>('/api/v1/landing').then(c => landingEnabled = c.enabled).catch(() => landingEnabled = false); });
  let checking = $state(!!session.token);
  let menuOpen = $state(false);
  let paletteOpen = $state(false);

  const paletteItems = $derived<PaletteItem[]>([
    ...adminGroups(route.instanceId, experienceContext.instance?.experience).flatMap(g => g.items.map(i => ({ ...i, group: g }))).map((i) => ({ id: `page-${i.id}`, label: i.label, group: i.group.label, hint: i.hint, icon: i.icon, run: () => go(i.id) })),
    { id: 'player', label: 'Open the player panel', group: 'Shortcuts', hint: 'Market, casino, friends and more, on any device', icon: Smartphone, run: () => go('play') },
    { id: 'landing', label: 'View the landing page', group: 'Shortcuts', icon: Globe, run: () => window.open('#/landing', '_blank') },
    { id: 'signout', label: 'Sign out', group: 'Shortcuts', icon: LogOut, run: () => { logout(); go('login'); } },
    ...(route.instanceId && pageEnabled(experienceContext.instance?.experience, 'commands') ? COMMAND_GROUPS : []).flatMap((g) => g.cmds.map((c) => ({ id: `cmd-${c.usage}`, label: c.usage, group: 'In-game commands', hint: c.does, keywords: `${g.title} ${(c.aliases ?? []).join(' ')} ${c.perm}`, run: () => go('commands') }))),
  ]);

  $effect(() => {
    if (!session.token) {
      checking = false;
      return;
    }
    get<Me>('/api/v1/auth/me')
      .then((me) => { session.user = me; })
      .catch(() => logout())
      .finally(() => { checking = false; });
  });

  // Players live in the player panel; admins land in the admin panel after signing in and can open the player panel from there.
  $effect(() => {
    const u = session.user;
    if (!u) return;
    if (u.role !== 'admin' && route.name !== 'play' && route.name !== 'landing') go('play');
    else if (u.role === 'admin' && (route.name === 'login' || route.name === 'reset-password')) go(nativeApp ? 'play' : 'instances');
    else if (nativeApp && route.name === 'landing') go('play');
  });

  $effect(() => {
    get('/api/v1/launcher/manifest')
      .then((m) => {
        brand = m.branding;
        registration = m.auth?.panel_accounts === false ? 'closed' : (m.auth?.registration ?? 'closed');
        if (route.name !== 'landing' && route.name !== 'play') {
          document.title = `${m.branding.name} · Admin`;
        }
      })
      .catch(() => {});
  });
</script>

{#if checking || landingEnabled === null}
  <div class="boot"><span class="pulse"></span></div>
{:else if !nativeApp && route.name === 'landing' && (landingEnabled || (preview && !!session.user))}
  <PublicLanding />
{:else if !session.user}
  {#if nativeApp || route.name === 'login' || route.name === 'reset-password' || !landingEnabled}
    <Login brandName={brand.name} logo={brand.logo_url} {registration} app={nativeApp} />
  {:else}
    <PublicLanding />
  {/if}
{:else if route.name === 'play' || session.user.role !== 'admin'}
  {#await import('./play/PlayApp.svelte')}
    <div class="boot"><span class="pulse"></span></div>
  {:then mod}
    {@const PlayApp = mod.default}
    <PlayApp />
  {/await}
{:else}
  <div class="shell">
    <Sidebar brandName={brand.name} logo={brand.logo_url} bind:open={menuOpen} onsearch={() => (paletteOpen = true)} />
    <div class="stage">
    <AdminTopbar brandName={brand.name} logo={brand.logo_url} onmenu={() => (menuOpen = true)} onsearch={() => (paletteOpen = true)} />
    <main>
      {#key (route.instanceId ?? 'platform') + route.name + route.params.join('/')}
        {#if experienceContext.loading}
          <div class="boot"><span class="pulse"></span></div>
        {:else if !route.instanceId && !PLATFORM_PAGES.has(route.name)}
          <Instances />
        {:else if route.instanceId && !pageEnabled(experienceContext.instance?.experience, route.name)}
          <InstanceControl />
        {:else if route.name === 'control'}
          <InstanceControl />
        {:else if route.name === 'experience'}
          <ExperienceEditor />
        {:else if route.name === 'installation' && route.instanceId}
          <InstanceEditor id={route.instanceId} />
        {:else if route.name === 'instances' && route.params[0]}
          <InstanceEditor id={route.params[0]} />
        {:else if route.name === 'instances'}
          <Instances />
        {:else if route.name === 'servers' && route.params[0]}
          <ServerDetail id={route.params[0]} />
        {:else if route.name === 'servers'}
          <Servers />
        {:else if route.name === 'quests'}
          <QuestEditor />
        {:else if route.name === 'quest-chains'}
          <QuestChains />
        {:else if route.name === 'achievements'}
          <AchievementEditor />
        {:else if route.name === 'leveling'}
          <LevelingAdmin />
        {:else if route.name === 'bulk'}
          <BulkEditor />
        {:else if route.name === 'progression'}
          <Progression />
        {:else if route.name === 'events'}
          <Events />
        {:else if route.name === 'cosmetics'}
          <CosmeticsStudio />
        {:else if route.name === 'guilds'}
          <GuildsAdmin />
        {:else if route.name === 'commands'}
          <Commands />
        {:else if route.name === 'items'}
          <ContentStudio />
        {:else if route.name === 'casino'}
          <Casino />
        {:else if route.name === 'board'}
          <BoardAdmin />
        {:else if route.name === 'economy'}
          <EconomyAdmin />
        {:else if route.name === 'admin-claims'}
          <AdminClaims />
        {:else if route.name === 'chat'}
          <ChatFormat />
        {:else if route.name === 'tasks'}
          <ScheduledTasks />
        {:else if route.name === 'luckperms'}
          <LuckPermsAdmin />
        {:else if route.name === 'capes'}
          <Capes />
        {:else if route.name === 'activity'}
          <Activity />
        {:else if route.name === 'users'}
          <Users />
        {:else if route.name === 'branding'}
          <Branding onsaved={(b) => (brand = b)} />
        {:else if route.name === 'emails'}
          <Emails />
        {:else if route.name === 'discord'}
          <DiscordStudio />
        {:else if route.name === 'companion'}
          <CompanionBuilder />
        {:else if route.name === 'reward-queue'}
          <RewardQueue />
        {:else if route.name === 'landing-builder'}
          <LandingBuilder />
        {:else if route.name === 'settings'}
          <Settings />
        {:else}
          <Dashboard />
        {/if}
      {/key}
    </main>
    </div>
  </div>
  <Palette bind:open={paletteOpen} items={paletteItems} placeholder="Jump to a page or search commands…" />
{/if}

<Toasts />

<style>
  .shell { display: flex; min-height: 100vh; }
  .stage { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  main { flex: 1; min-width: 0; }
  .boot { height: 100vh; display: grid; place-items: center; }
  .pulse { width: 28px; height: 28px; border-radius: 50%; border: 3px solid var(--surface-3); border-top-color: var(--accent); animation: spin 0.8s linear infinite; }
  @keyframes pulse { from { transform: scale(0.85); opacity: 0.6; } to { transform: scale(1); opacity: 1; } }
</style>
