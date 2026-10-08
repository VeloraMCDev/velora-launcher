<script lang="ts">
  import { Users, UserCheck, Boxes, Activity, Download, ArrowRight, Palette, Rocket, CircleCheck, Circle, Radio, Server } from '@lucide/svelte';
  import BarChart from '../components/BarChart.svelte';
  import Avatar from '../components/Avatar.svelte';
  import { get, timeAgo } from '../lib/api';
  import { session } from '../lib/session.svelte';

  let stats = $state<any>(null);
  let instances = $state<any[]>([]);
  let branding = $state<any>(null);
  let showGettingStarted = $state(localStorage.getItem('scopenet_hide_getting_started') !== 'true');

  function hideGettingStarted() {
    showGettingStarted = false;
    localStorage.setItem('scopenet_hide_getting_started', 'true');
  }
  function showGuide() {
    showGettingStarted = true;
    localStorage.removeItem('scopenet_hide_getting_started');
  }

  $effect(() => {
    get('/api/admin/stats').then((s) => (stats = s));
    get('/api/admin/instances').then((i) => (instances = i));
    get('/api/admin/branding').then((b) => (branding = b));
  });

  const nameOf = (id: string | null) => instances.find((i) => i.id === id)?.name ?? id ?? 'Unknown';
  const hour = new Date().getHours();
  const greeting = hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';
  const total = $derived(stats ? stats.launches.reduce((a: number, d: any) => a + d.launches, 0) : 0);

  const steps = $derived([
    { done: !!branding && (branding.name !== 'SCOPENET' || branding.logo_url), label: 'Make the launcher yours', href: '#/branding', icon: Palette },
    { done: instances.length > 0, label: 'Create your first instance', href: '#/instances', icon: Boxes },
    { done: !!stats?.launcher_download_url, label: 'Share the launcher download link', href: '#/settings', icon: Download },
    { done: total > 0, label: 'Launch the game once', href: '#/instances', icon: Rocket },
    { done: !!stats?.live?.servers?.length, label: 'Connect a game server', href: '#/servers', icon: Server },
  ]);
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>{greeting}, {session.user?.username}</h1>
      <p>Here's what's happening with your launcher.</p>
    </div>
    {#if stats?.launcher_download_url}
      <a class="btn primary" href={stats.launcher_download_url} target="_blank" rel="noreferrer"><Download size={16} /> Launcher download</a>
    {/if}
    {#if !showGettingStarted}<button class="ghost" onclick={showGuide}>Show getting started</button>{/if}
  </div>

  <div class="grid stats">
    {#each [
      { label: 'Online now', value: stats?.live?.online_now, icon: Radio, tone: 'live', href: '#/servers' },
      { label: 'Active players (7d)', value: stats?.players_7d, icon: Activity, tone: 'accent' },
      { label: 'Accounts', value: stats?.users, icon: Users, tone: 'cyan' },
      { label: 'Awaiting approval', value: stats?.pending, icon: UserCheck, tone: 'warn', href: '#/users' },
      { label: 'Instances', value: stats?.instances, icon: Boxes, tone: 'green', href: '#/instances' },
    ] as s}
      <a class="card stat hover {s.tone}" href={s.href ?? '#/dashboard'}>
        <span class="icon"><s.icon size={18} /></span>
        <span class="value">{s.value ?? '—'}</span>
        <span class="label">{s.label}</span>
      </a>
    {/each}
  </div>

  <div class="grid main">
    <section class="card">
      <div class="section-title">
        <h2>Game launches</h2>
        <span class="hint">{total.toLocaleString()} in the last 14 days</span>
      </div>
      {#if stats}
        <BarChart data={stats.launches} label="Game launches per day, last 14 days" />
      {:else}
        <div class="skeleton" style="height: 200px"></div>
      {/if}
    </section>

    {#if showGettingStarted}<section class="card">
      <div class="section-title"><h2>Getting started</h2><button class="hint" onclick={hideGettingStarted} aria-label="Hide Getting started">Hide</button></div>
      <div class="steps">
        {#each steps as s}
          <a class="step" class:done={s.done} href={s.href}>
            {#if s.done}<CircleCheck size={18} />{:else}<Circle size={18} />{/if}
            <span>{s.label}</span>
            <ArrowRight size={14} />
          </a>
        {/each}
      </div>
    </section>{/if}

    {#if stats?.live?.servers?.length}
      <section class="card servers">
        <div class="section-title"><h2>Servers</h2><a class="hint" href="#/servers">View all</a></div>
        <div class="list">
          {#each stats.live.servers as s}
            <a class="srv" href="#/servers/{s.id}">
              <span class="dot" class:on={s.online}></span>
              <span class="name">{s.name}</span>
              <span class="muted small num">{s.online ? `${s.players} / ${s.max_players || '–'}` : 'offline'}</span>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    <section class="card">
      <div class="section-title"><h2>Popular instances</h2></div>
      {#if stats?.top_instances?.length}
        <div class="list">
          {#each stats.top_instances as t}
            {@const pct = (t.launches / stats.top_instances[0].launches) * 100}
            <div class="top">
              <span class="name">{nameOf(t.instance_id)}</span>
              <span class="meter"><span style:width="{pct}%"></span></span>
              <span class="num">{t.launches}</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="muted small">No launches yet. Stats appear once players start the game.</p>
      {/if}
    </section>

    <section class="card">
      <div class="section-title"><h2>Recent activity</h2></div>
      {#if stats?.recent?.length}
        <div class="list">
          {#each stats.recent as r}
            <div class="activity">
              <Avatar name={r.username ?? 'Player'} size={28} />
              <span><strong>{r.username ?? 'Someone'}</strong> <span class="muted">launched</span> {nameOf(r.instance_id)}</span>
              <span class="muted tiny">{timeAgo(r.at)}</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="muted small">Nothing yet.</p>
      {/if}
    </section>
  </div>
</div>

<style>
  .stats { grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); margin-bottom: 16px; }
  .stat { display: flex; flex-direction: column; gap: 4px; color: var(--text); position: relative; overflow: hidden; }
  .stat .icon { width: 36px; height: 36px; border-radius: 10px; display: grid; place-items: center; margin-bottom: 10px; }
  .stat .value { font-size: 1.75rem; font-weight: 600; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .stat .label { color: var(--muted); font-size: 0.85rem; }
  .accent .icon { background: var(--accent-soft); color: #b9b7fb; }
  .cyan .icon { background: rgba(34, 211, 238, 0.12); color: #67e8f9; }
  .warn .icon { background: rgba(251, 191, 36, 0.12); color: #fcd34d; }
  .live .icon { background: rgba(63, 185, 124, 0.12); color: #6ee7b7; }
  .srv { display: flex; align-items: center; gap: 10px; color: var(--text); font-size: 0.9rem; padding: 2px 0; }
  .srv .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .srv .num { font-variant-numeric: tabular-nums; }
  .srv .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--surface-3); }
  .srv .dot.on { background: var(--good); }
  .green .icon { background: rgba(52, 211, 153, 0.12); color: #6ee7b7; }
  .main { grid-template-columns: 1.6fr 1fr; }
  @media (max-width: 1100px) { .main { grid-template-columns: 1fr; } }
  .steps { display: flex; flex-direction: column; gap: 6px; }
  .step { display: flex; align-items: center; gap: 12px; padding: 12px; border-radius: 10px; color: var(--text); background: var(--bg-2); border: 1px solid var(--line); transition: border-color 0.15s; }
  .step:hover { border-color: var(--line-strong); }
  .step span { flex: 1; }
  .step :global(svg:first-child) { color: var(--muted); }
  .step.done { color: var(--muted); }
  .step.done span { text-decoration: line-through; }
  .step.done :global(svg:first-child) { color: var(--good); }
  .list { display: flex; flex-direction: column; gap: 12px; }
  .top { display: grid; grid-template-columns: 140px 1fr 40px; gap: 12px; align-items: center; font-size: 0.9rem; }
  .top .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meter { height: 8px; background: var(--bg-2); border-radius: 99px; overflow: hidden; }
  .meter span { display: block; height: 100%; background: #6d6af5; border-radius: 99px; }
  .num { text-align: right; font-variant-numeric: tabular-nums; color: var(--text-2); }
  .activity { display: flex; align-items: center; gap: 10px; font-size: 0.9rem; }
  .activity > span:nth-child(2) { flex: 1; }
  .skeleton { border-radius: 10px; background: linear-gradient(90deg, var(--surface-2), var(--surface-3), var(--surface-2)); background-size: 200% 100%; animation: shimmer 1.2s linear infinite; }
  @keyframes shimmer { to { background-position: -200% 0; } }
</style>
