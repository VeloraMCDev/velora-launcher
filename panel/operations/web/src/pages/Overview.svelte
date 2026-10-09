<script lang="ts">
  import { onMount } from 'svelte';
  import { Cpu, MemoryStick, HardDrive, Archive, Globe, ShieldCheck, TriangleAlert, Users, Rocket, RefreshCw } from '@lucide/svelte';
  import { api } from '../lib/api';
  import { app, navigate, runAction, ask } from '../lib/state.svelte';
  import { ago, bytes, duration, pct, until, when } from '../lib/format';
  import type { HistoryPoint, PanelSummary } from '../lib/types';
  import Chart from '../components/Chart.svelte';

  let history = $state<HistoryPoint[]>([]);
  let summary = $state<PanelSummary | null>(null);
  onMount(() => {
    const load = () => {
      api<HistoryPoint[]>('/api/history').then(h => (history = h.slice(-360))).catch(() => {});
      api<PanelSummary>('/api/panel/summary').then(s => (summary = s)).catch(() => {});
    };
    load();
    const t = setInterval(load, 60_000);
    return () => clearInterval(t);
  });

  const snap = $derived(app.overview?.snapshot);
  const stack = $derived(snap?.containers.filter(c => c.project === app.overview?.project) ?? []);
  const healthy = $derived(stack.filter(c => c.state === 'running' && c.health !== 'unhealthy').length);
  const backup = $derived(snap?.backups.last);
  const tone = (p: number | undefined, warn = 75, bad = 90) => (p === undefined ? '' : p >= bad ? 'bad' : p >= warn ? 'warn' : '');
</script>

<div class="head row wrap">
  <div><h1>Overview</h1><p class="muted">Production stack <code>{app.overview?.project ?? '…'}</code> on the OVH VPS{snap ? ` · updated ${ago(snap.at)}` : ''}</p></div>
  <span class="spacer"></span>
  <button onclick={async () => { if (await ask('Redeploy the production stack?', 'Applies compose.yaml to every Velora service. Running services with unchanged configuration are left alone.', 'Redeploy')) runAction('/api/actions/redeploy'); }}><RefreshCw size={16} /> Redeploy stack</button>
</div>

{#if (app.overview?.alerts.length ?? 0) > 0}
  <div class="alerts col">
    {#each app.overview!.alerts as a}
      <div class="alert {a.severity}">
        <TriangleAlert size={18} />
        <div><b>{a.title}</b>{#if a.detail}<span class="muted small"> — {a.detail}</span>{/if}<div class="tiny faint">since {when(a.since)}{a.notified ? ' · email sent' : ''}</div></div>
      </div>
    {/each}
  </div>
{/if}

{#if !snap}
  <div class="grid g4">{#each Array(4) as _}<div class="card skeleton" style="height:120px"></div>{/each}</div>
{:else}
  <div class="grid g4">
    <div class="card stat">
      <span class="label"><Globe size={14} /> Services</span>
      <span class="value">{healthy}<span class="faint">/{stack.length}</span></span>
      <span class="sub">{healthy === stack.length ? 'All Velora services healthy' : `${stack.length - healthy} need attention`}</span>
    </div>
    <div class="card stat">
      <span class="label"><Cpu size={14} /> CPU</span>
      <span class="value">{pct(snap.host.cpu)}</span>
      <span class="sub">Load {snap.host.load.map(l => l.toFixed(2)).join(' · ')} on {snap.host.cores} cores</span>
    </div>
    <div class="card stat">
      <span class="label"><MemoryStick size={14} /> Memory</span>
      <span class="value">{pct(snap.host.memory.percent)}</span>
      <div class="bar {tone(snap.host.memory.percent, 85, 95)}"><span style:width="{snap.host.memory.percent}%"></span></div>
      <span class="sub">{bytes(snap.host.memory.used)} of {bytes(snap.host.memory.total)}</span>
    </div>
    <div class="card stat">
      <span class="label"><HardDrive size={14} /> Disk</span>
      <span class="value">{pct(snap.host.disk?.percent)}</span>
      <div class="bar {tone(snap.host.disk?.percent)}"><span style:width="{snap.host.disk?.percent ?? 0}%"></span></div>
      <span class="sub">{bytes(snap.host.disk?.used)} of {bytes(snap.host.disk?.total)} · up {duration(snap.host.uptime)}</span>
    </div>
  </div>

  <div class="grid g2 section">
    <div class="card">
      <div class="card-head"><h2>Host CPU</h2><span class="spacer"></span><span class="tiny muted">last 6 hours</span></div>
      <Chart points={history.map(h => ({ t: h.t, v: h.cpu }))} max={100} unit="%" />
    </div>
    <div class="card">
      <div class="card-head"><h2>Panel response time</h2><span class="spacer"></span><span class="tiny muted">public /health</span></div>
      <Chart points={history.map(h => ({ t: h.t, v: h.panel_ms }))} unit=" ms" color="#3dd6c6" />
    </div>
  </div>

  <div class="grid g3 section">
    <div class="card">
      <div class="card-head"><Globe size={17} /><h2>Endpoints</h2></div>
      <div class="col">
        {#each snap.endpoints as e}
          <div class="row line"><span class="dot {e.ok ? 'ok' : 'bad'}"></span><span class="grow">{e.name}<span class="tiny faint block">{e.url.replace(/^https?:\/\//, '')}</span></span>
            <span class="badge {e.ok ? 'ok' : 'bad'}">{e.ok ? `${e.ms} ms` : e.status || e.error}</span></div>
        {/each}
      </div>
    </div>
    <div class="card">
      <div class="card-head"><Archive size={17} /><h2>Backups</h2><span class="spacer"></span><a class="tiny" href="#/backups">Details</a></div>
      {#if backup}
        <div class="col">
          <div class="row line"><span class="dot {backup.status === 'ok' ? (backup.offsite ? 'ok' : 'warn') : 'bad'}"></span><span class="grow">Last backup<span class="tiny faint block">{when(backup.finished_at)} · {ago(backup.finished_at)}</span></span><span class="badge {backup.status === 'ok' ? 'ok' : 'bad'}">{backup.status}</span></div>
          <div class="row line"><span class="grow muted small">Size</span><span>{bytes(backup.bytes)}</span></div>
          <div class="row line"><span class="grow muted small">Copied to Hermes</span><span class="badge {backup.offsite ? 'ok' : 'warn'}">{backup.offsite ? 'yes' : 'no'}</span></div>
          <div class="row line"><span class="grow muted small">Archives kept on VPS</span><span>{snap.backups.archives.length}</span></div>
        </div>
      {:else}<div class="empty">No backup has run yet.</div>{/if}
    </div>
    <div class="card">
      <div class="card-head"><ShieldCheck size={17} /><h2>Certificates</h2></div>
      <div class="col">
        {#each snap.certificates as c}
          {@const days = c.expires ? (Date.parse(c.expires) - Date.now()) / 86400000 : -1}
          <div class="row line"><span class="dot {days > 20 ? 'ok' : days > 7 ? 'warn' : 'bad'}"></span><span class="grow">{c.host}<span class="tiny faint block">{c.issuer ?? c.error}</span></span><span class="badge">{until(c.expires)}</span></div>
        {:else}<div class="empty">Checked hourly.</div>{/each}
      </div>
    </div>
  </div>

  <div class="grid g2 section">
    <div class="card">
      <div class="card-head"><Users size={17} /><h2>Community</h2><span class="spacer"></span><a class="tiny" href="#/activity">Activity</a></div>
      {#if summary}
        <div class="grid g3 minis">
          <div class="stat"><span class="label">Logins 24h</span><span class="value">{summary.logins_24h}</span></div>
          <div class="stat"><span class="label">Players 7d</span><span class="value">{summary.unique_logins_7d}</span></div>
          <div class="stat"><span class="label">Launches 24h</span><span class="value">{summary.launches_24h}</span></div>
          <div class="stat"><span class="label">Accounts</span><span class="value">{summary.users}</span></div>
          <div class="stat"><span class="label">Admins</span><span class="value">{summary.admins}</span></div>
          <div class="stat"><span class="label">Panel</span><span class="value small-value">v{summary.version}</span></div>
        </div>
      {:else}<div class="skeleton" style="height:120px"></div>{/if}
    </div>
    <div class="card">
      <div class="card-head"><Rocket size={17} /><h2>Updates</h2><span class="spacer"></span><a class="tiny" href="#/updates">Review</a></div>
      <div class="col">
        <div class="row line"><span class="grow">Launcher releases awaiting approval</span><span class="badge {app.overview?.updates.launcher_pending ? 'accent' : ''}">{app.overview?.updates.launcher_pending ?? 0}</span></div>
        {#each Object.entries(app.overview?.updates.images ?? {}) as [name, available]}
          <div class="row line"><span class="grow">{name}</span><span class="badge {available ? 'accent' : 'ok'}">{available ? 'new build' : 'current'}</span></div>
        {/each}
        <div class="row line"><span class="grow">Container images with updates</span><span class="badge {app.overview?.updates.containers ? 'info' : ''}">{app.overview?.updates.containers ?? 0}</span></div>
        <div class="tiny faint">Checked {ago(app.overview?.updates.checked)}</div>
      </div>
    </div>
  </div>
{/if}

<style>
  .head { margin-bottom: 22px; }
  .head p { margin: 4px 0 0; }
  .section { margin-top: 16px; }
  .alerts { margin-bottom: 18px; gap: 8px; }
  .alert { display: flex; gap: 12px; align-items: flex-start; padding: 12px 16px; border-radius: 12px; border: 1px solid; }
  .alert.critical { background: #f2555a12; border-color: #f2555a50; color: #ffc9cb; }
  .alert.warning { background: #f5b04112; border-color: #f5b04150; color: #ffe0ad; }
  .alert b { color: var(--head); }
  .line { gap: 10px; }
  .grow { flex: 1; min-width: 0; }
  .block { display: block; }
  .minis { gap: 18px; }
  .minis .value { font-size: 24px; }
  .small-value { font-size: 18px !important; }
</style>
