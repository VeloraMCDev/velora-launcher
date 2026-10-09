<script lang="ts">
  import { RotateCw, Download, FileText, X, LoaderCircle, BookOpen } from '@lucide/svelte';
  import { api } from '../lib/api';
  import { app, ask, runAction, toast } from '../lib/state.svelte';
  import { bytes, pct } from '../lib/format';
  import type { Container } from '../lib/types';

  const containers = $derived(app.overview?.snapshot?.containers ?? []);
  const stack = $derived(containers.filter(c => c.project === app.overview?.project).sort((a, b) => (a.service ?? '').localeCompare(b.service ?? '')));
  const others = $derived(containers.filter(c => c.project !== app.overview?.project).sort((a, b) => (a.project ?? '~').localeCompare(b.project ?? '~') || a.name.localeCompare(b.name)));
  let logs = $state<{ name: string; text: string; loading: boolean } | null>(null);
  let filter = $state('');

  const tone = (c: Container) => c.state !== 'running' ? 'bad' : c.health === 'unhealthy' ? 'bad' : c.health === 'health: starting' ? 'warn' : 'ok';
  async function showLogs(c: Container) {
    logs = { name: c.name, text: '', loading: true };
    try {
      const r = await api<{ logs: string }>(`/api/containers/${encodeURIComponent(c.name)}/logs?tail=400`);
      logs = { name: c.name, text: r.logs, loading: false };
    } catch (e) {
      logs = null;
      toast((e as Error).message, 'error');
    }
  }
  async function restart(c: Container) {
    if (await ask(`Restart ${c.service}?`, `${c.name} will be unavailable for a few seconds.`, 'Restart')) runAction('/api/actions/restart', { service: c.service });
  }
  async function pull(c: Container) {
    if (await ask(`Pull and recreate ${c.service}?`, 'Pulls the image configured in compose.yaml and recreates the container if it changed.', 'Pull & recreate')) runAction('/api/actions/pull', { service: c.service });
  }
  const visibleOthers = $derived(others.filter(c => !filter || `${c.name} ${c.image} ${c.project}`.toLowerCase().includes(filter.toLowerCase())));
</script>

<div class="head"><h1>Services</h1><p class="muted">The Velora stack can be managed here. Other containers on the VPS are shown read-only.</p></div>

<div class="card flush">
  <div class="card-head pad"><h2>Velora stack</h2><span class="spacer"></span>
    <button class="sm" onclick={async () => { if (await ask('Rebuild the documentation?', 'Fetches main, runs the docs tests, builds the site and publishes it to docs.velora.scopedd.lol.', 'Rebuild')) runAction('/api/actions/rebuild-docs', { ref: 'main' }); }}><BookOpen size={15} /> Rebuild docs</button>
  </div>
  <div class="table-scroll">
    <table>
      <thead><tr><th>Service</th><th>Status</th><th>CPU</th><th>Memory</th><th>Image</th><th></th></tr></thead>
      <tbody>
        {#each stack as c}
          <tr>
            <td><div class="row"><span class="dot {tone(c)}"></span><div><b class="name">{c.service}</b><div class="tiny faint">{c.name}</div></div></div></td>
            <td><span class="badge {tone(c)}">{c.health ?? c.state}</span><div class="tiny faint">{c.status}</div></td>
            <td>{pct(c.stats?.cpu, 1)}</td>
            <td>{bytes(c.stats?.memory)}{#if c.stats?.memoryLimit}<span class="tiny faint"> / {bytes(c.stats.memoryLimit)}</span>{/if}</td>
            <td class="mono tiny img" title={c.image}>{c.image}</td>
            <td class="actions"><div class="row">
              <button class="sm ghost" title="Logs" onclick={() => showLogs(c)}><FileText size={15} /></button>
              <button class="sm ghost" title="Pull & recreate" onclick={() => pull(c)}><Download size={15} /></button>
              <button class="sm" onclick={() => restart(c)}><RotateCw size={14} /> Restart</button>
            </div></td>
          </tr>
        {:else}<tr><td colspan="6" class="empty">No Velora containers found.</td></tr>{/each}
      </tbody>
    </table>
  </div>
</div>

<div class="card flush section">
  <div class="card-head pad"><h2>Other containers on this host</h2><span class="badge">{others.length}</span><span class="spacer"></span><input class="search" placeholder="Filter…" bind:value={filter} /></div>
  <div class="table-scroll">
    <table>
      <thead><tr><th>Container</th><th>Project</th><th>Status</th><th>Ports</th><th>Image</th></tr></thead>
      <tbody>
        {#each visibleOthers as c}
          <tr>
            <td><div class="row"><span class="dot {tone(c)}"></span>{c.name}</div></td>
            <td class="muted">{c.project ?? '—'}</td>
            <td><span class="tiny {tone(c) === 'ok' ? 'muted' : ''}">{c.status}</span></td>
            <td class="tiny mono muted">{c.ports.join(', ') || '—'}</td>
            <td class="mono tiny img" title={c.image}>{c.image}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

{#if logs}
  <div class="scrim" role="presentation" onclick={() => (logs = null)}></div>
  <div class="logs card">
    <div class="card-head"><FileText size={17} /><h2>{logs.name}</h2><span class="tiny muted">last 400 lines</span><span class="spacer"></span>
      <button class="sm ghost" onclick={() => logs && showLogs({ name: logs.name } as Container)}><RotateCw size={14} /></button>
      <button class="ghost icon" onclick={() => (logs = null)}><X size={18} /></button></div>
    {#if logs.loading}<div class="empty"><LoaderCircle class="spin" /></div>{:else}<pre class="log">{logs.text || 'No output.'}</pre>{/if}
  </div>
{/if}

<style>
  .head { margin-bottom: 22px; }
  .head p { margin: 4px 0 0; }
  .section { margin-top: 18px; }
  .pad { padding: 16px 18px 0; margin-bottom: 12px; }
  .name { color: var(--head); text-transform: capitalize; }
  .img { max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); }
  .actions { text-align: right; }
  .actions .row { justify-content: flex-end; gap: 4px; }
  .search { width: 220px; height: 32px; padding: 0 10px; }
  .scrim { position: fixed; inset: 0; background: #05050acc; z-index: 45; }
  .logs { position: fixed; inset: 6vh 6vw; z-index: 46; display: flex; flex-direction: column; box-shadow: var(--shadow); }
  .logs pre { flex: 1; }
</style>
