<script lang="ts">
  import { onMount } from 'svelte';
  import { Activity, Gauge, Server, Rocket, Archive, Settings as Cog, ScrollText, LogOut, Bell, Menu, TerminalSquare } from '@lucide/svelte';
  import { app, boot, navigate, refreshOverview, signOut, type Page } from './lib/state.svelte';
  import Login from './pages/Login.svelte';
  import Overview from './pages/Overview.svelte';
  import Services from './pages/Services.svelte';
  import ActivityPage from './pages/Activity.svelte';
  import Updates from './pages/Updates.svelte';
  import Backups from './pages/Backups.svelte';
  import Settings from './pages/Settings.svelte';
  import Audit from './pages/Audit.svelte';
  import JobDrawer from './components/JobDrawer.svelte';
  import Confirm from './components/Confirm.svelte';

  const nav: { page: Page; label: string; icon: typeof Gauge }[] = [
    { page: 'overview', label: 'Overview', icon: Gauge },
    { page: 'services', label: 'Services', icon: Server },
    { page: 'activity', label: 'Activity', icon: Activity },
    { page: 'updates', label: 'Updates', icon: Rocket },
    { page: 'backups', label: 'Backups', icon: Archive },
    { page: 'audit', label: 'Audit log', icon: ScrollText },
    { page: 'settings', label: 'Settings', icon: Cog },
  ];
  let menu = $state(false);

  onMount(() => { boot(); });
  $effect(() => {
    if (!app.user) return;
    refreshOverview().catch(() => {});
    const timer = setInterval(() => refreshOverview().catch(() => {}), 15_000);
    return () => clearInterval(timer);
  });

  const alerts = $derived(app.overview?.alerts ?? []);
  const critical = $derived(alerts.some(a => a.severity === 'critical'));
  const pendingUpdates = $derived(app.overview ? app.overview.updates.launcher_pending + Object.values(app.overview.updates.images).filter(Boolean).length : 0);
  const running = $derived(app.overview?.jobs.find(j => j.status === 'running'));
</script>

{#if !app.ready}
  <div class="boot"><div class="logo"></div></div>
{:else if !app.user}
  <Login />
{:else}
  <div class="shell" class:menu>
    <nav class="side">
      <a class="brand" href="#/overview"><span class="logo"></span><span>Velora <b>Operations</b></span></a>
      <div class="links">
        {#each nav as item}
          {@const Icon = item.icon}
          <a href="#/{item.page}" class:active={app.page === item.page} onclick={() => (menu = false)}>
            <Icon size={18} /> {item.label}
            {#if item.page === 'overview' && alerts.length}<span class="count" class:bad={critical}>{alerts.length}</span>{/if}
            {#if item.page === 'updates' && pendingUpdates}<span class="count">{pendingUpdates}</span>{/if}
          </a>
        {/each}
      </div>
      <div class="foot">
        <div class="who"><span class="avatar">{app.user.slice(0, 1).toUpperCase()}</span><div><b>{app.user}</b><span class="tiny faint">Administrator</span></div></div>
        <button class="ghost icon" title="Sign out" onclick={signOut}><LogOut size={17} /></button>
      </div>
      <div class="tiny faint ver">Operations {app.version}</div>
    </nav>
    <div class="main">
      <header class="top">
        <button class="ghost icon mobile" onclick={() => (menu = !menu)} aria-label="Menu"><Menu size={20} /></button>
        <div class="status">
          {#if !app.overview}<span class="dot"></span> Connecting…
          {:else if alerts.length}<span class="dot {critical ? 'bad' : 'warn'}"></span> {alerts.length} {alerts.length === 1 ? 'issue needs' : 'issues need'} attention
          {:else}<span class="dot ok"></span> All systems operational{/if}
        </div>
        <span class="spacer"></span>
        {#if running}
          <button class="sm" onclick={() => { app.job = running; app.jobOpen = true; }}><TerminalSquare size={15} /> {running.kind} running…</button>
        {:else if app.job}
          <button class="sm ghost" onclick={() => (app.jobOpen = !app.jobOpen)}><TerminalSquare size={15} /> Last operation</button>
        {/if}
        <a class="btn sm ghost" href="#/settings" title="Notifications"><Bell size={16} /></a>
      </header>
      <main>
        {#key app.page}
          {#if app.page === 'overview'}<Overview />
          {:else if app.page === 'services'}<Services />
          {:else if app.page === 'activity'}<ActivityPage />
          {:else if app.page === 'updates'}<Updates />
          {:else if app.page === 'backups'}<Backups />
          {:else if app.page === 'settings'}<Settings />
          {:else}<Audit />{/if}
        {/key}
      </main>
    </div>
  </div>
  <JobDrawer />
{/if}
<Confirm />

<div class="toasts">
  {#each app.toasts as t (t.id)}<div class="toast {t.kind}">{t.text}</div>{/each}
</div>

<style>
  .boot { height: 100vh; display: grid; place-items: center; }
  .logo { width: 30px; height: 30px; border-radius: 9px; background: linear-gradient(135deg, #8f6bff, #3dd6c6); display: inline-block; flex-shrink: 0; }
  .boot .logo { width: 44px; height: 44px; animation: pulse 1.4s infinite; }
  .shell { display: grid; grid-template-columns: 248px minmax(0, 1fr); min-height: 100vh; }
  .side { position: sticky; top: 0; height: 100vh; display: flex; flex-direction: column; gap: 6px; padding: 18px 14px; border-right: 1px solid var(--line); background: #0d0d17cc; backdrop-filter: blur(10px); }
  .brand { display: flex; align-items: center; gap: 11px; color: var(--head); text-decoration: none; font-size: 15.5px; padding: 6px 8px 18px; }
  .brand b { font-weight: 700; }
  .links { display: flex; flex-direction: column; gap: 2px; flex: 1; }
  .links a { display: flex; align-items: center; gap: 11px; padding: 9px 12px; border-radius: 10px; color: var(--muted); text-decoration: none; font-weight: 550; font-size: 14.5px; }
  .links a:hover { background: var(--panel); color: var(--text); }
  .links a.active { background: var(--accent-soft); color: var(--head); box-shadow: inset 2px 0 0 var(--accent); }
  .count { margin-left: auto; min-width: 20px; height: 20px; border-radius: 99px; background: var(--accent); color: #fff; font-size: 11.5px; display: grid; place-items: center; padding: 0 6px; font-weight: 700; }
  .count.bad { background: var(--bad); }
  .foot { display: flex; align-items: center; gap: 8px; padding: 12px 8px 4px; border-top: 1px solid var(--line); }
  .who { display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0; }
  .who div { display: flex; flex-direction: column; line-height: 1.25; }
  .who b { color: var(--head); font-size: 14px; }
  .avatar { width: 32px; height: 32px; border-radius: 10px; background: linear-gradient(135deg, #2d2550, #1c3a42); display: grid; place-items: center; font-weight: 700; color: var(--head); }
  .ver { padding: 0 8px; }
  .main { min-width: 0; display: flex; flex-direction: column; }
  .top { position: sticky; top: 0; z-index: 10; height: 60px; display: flex; align-items: center; gap: 12px; padding: 0 28px; border-bottom: 1px solid var(--line); background: #0b0b14d9; backdrop-filter: blur(12px); }
  .status { display: flex; align-items: center; gap: 10px; font-weight: 550; font-size: 14px; }
  main { padding: 28px; max-width: 1480px; width: 100%; }
  .mobile { display: none; }
  .toasts { position: fixed; top: 16px; right: 16px; display: flex; flex-direction: column; gap: 8px; z-index: 60; }
  .toast { background: var(--panel-2); border: 1px solid var(--line-2); border-left: 3px solid var(--ok); padding: 10px 16px; border-radius: 10px; box-shadow: var(--shadow); max-width: 380px; font-size: 14px; }
  .toast.error { border-left-color: var(--bad); }
  @media (max-width: 900px) {
    .shell { grid-template-columns: 1fr; }
    .side { position: fixed; z-index: 30; width: 260px; transform: translateX(-100%); transition: transform 0.2s; }
    .shell.menu .side { transform: none; box-shadow: var(--shadow); }
    .mobile { display: inline-flex; }
    main, .top { padding-left: 16px; padding-right: 16px; }
  }
</style>
