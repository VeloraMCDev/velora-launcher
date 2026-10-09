<script lang="ts">
  import { onMount } from 'svelte';
  import { LogIn, Users, Rocket, ShieldAlert, Search } from '@lucide/svelte';
  import { api } from '../lib/api';
  import { toast } from '../lib/state.svelte';
  import { ago, when } from '../lib/format';
  import type { ActivityEntry, PanelSummary } from '../lib/types';

  let summary = $state<PanelSummary | null>(null);
  let entries = $state<ActivityEntry[]>([]);
  let source = $state('');
  let player = $state('');
  let offset = $state(0);
  let loading = $state(false);

  async function load() {
    loading = true;
    try {
      const params = new URLSearchParams({ ...(source ? { source } : {}), ...(player ? { player } : {}), ...(offset ? { offset: String(offset) } : {}) });
      entries = await api<ActivityEntry[]>(`/api/panel/activity?${params}`);
    } catch (e) { toast((e as Error).message, 'error'); }
    loading = false;
  }
  onMount(() => {
    api<PanelSummary>('/api/panel/summary').then(s => (summary = s)).catch(e => toast(e.message, 'error'));
    load();
  });
  const sources = ['', 'auth', 'panel', 'launcher', 'server'];
</script>

<div class="head"><h1>Activity</h1><p class="muted">Sign-ins, launches and admin changes recorded by the Velora Panel.</p></div>

<div class="grid g4">
  <div class="card stat"><span class="label"><LogIn size={14} /> Logins (24h)</span><span class="value">{summary?.logins_24h ?? '—'}</span><span class="sub">{summary?.logins_7d ?? '—'} in the last 7 days</span></div>
  <div class="card stat"><span class="label"><Users size={14} /> Active players (7d)</span><span class="value">{summary?.unique_logins_7d ?? '—'}</span><span class="sub">of {summary?.users ?? '—'} accounts</span></div>
  <div class="card stat"><span class="label"><Rocket size={14} /> Game launches (24h)</span><span class="value">{summary?.launches_24h ?? '—'}</span><span class="sub">{summary?.launcher_sign_ins_24h ?? '—'} launcher sign-ins</span></div>
  <div class="card stat"><span class="label"><ShieldAlert size={14} /> Admin changes (7d)</span><span class="value">{summary?.admin_changes_7d ?? '—'}</span><span class="sub">{summary?.admins ?? '—'} active administrators</span></div>
</div>

<div class="grid g2 section">
  <div class="card">
    <div class="card-head"><LogIn size={17} /><h2>Recent sign-ins</h2></div>
    <div class="col list">
      {#each summary?.recent_logins ?? [] as l}<div class="row"><span class="avatar">{(l.name ?? '?').slice(0, 1).toUpperCase()}</span><span class="grow">{l.name ?? 'unknown'}</span><span class="tiny muted" title={when(l.created_at)}>{ago(l.created_at)}</span></div>
      {:else}<div class="empty">No sign-ins recorded yet.</div>{/each}
    </div>
  </div>
  <div class="card">
    <div class="card-head"><ShieldAlert size={17} /><h2>Recent admin changes</h2></div>
    <div class="col list">
      {#each summary?.recent_admin_changes ?? [] as c}<div class="row"><span class="avatar admin">{(c.name ?? '?').slice(0, 1).toUpperCase()}</span><span class="grow"><b>{c.name ?? 'unknown'}</b> <code class="tiny muted">{c.detail}</code></span><span class="tiny muted" title={when(c.created_at)}>{ago(c.created_at)}</span></div>
      {:else}<div class="empty">No admin changes this week.</div>{/each}
    </div>
  </div>
</div>

<div class="card flush section">
  <div class="card-head pad wrap">
    <h2>Activity log</h2><span class="spacer"></span>
    <select bind:value={source} onchange={() => { offset = 0; load(); }} class="narrow">{#each sources as s}<option value={s}>{s || 'All sources'}</option>{/each}</select>
    <form class="row" onsubmit={e => { e.preventDefault(); offset = 0; load(); }}><input class="narrow" placeholder="Player or UUID" bind:value={player} /><button class="sm icon" type="submit" aria-label="Search"><Search size={15} /></button></form>
  </div>
  <div class="table-scroll">
    <table>
      <thead><tr><th>When</th><th>Source</th><th>Player</th><th>Event</th><th>Detail</th></tr></thead>
      <tbody>
        {#each entries as e}
          <tr class:dim={loading}>
            <td class="tiny nowrap" title={e.created_at}>{when(e.created_at)}</td>
            <td><span class="badge">{e.server ?? e.source}</span></td>
            <td>{e.name ?? '—'}</td>
            <td><code class="tiny">{e.kind}</code></td>
            <td class="tiny muted detail">{e.detail ?? ''}</td>
          </tr>
        {:else}<tr><td colspan="5" class="empty">{loading ? 'Loading…' : 'Nothing matches.'}</td></tr>{/each}
      </tbody>
    </table>
  </div>
  <div class="row pager">
    <button class="sm" disabled={offset === 0 || loading} onclick={() => { offset = Math.max(0, offset - 100); load(); }}>Newer</button>
    <span class="tiny muted">{offset + 1}–{offset + entries.length}</span>
    <button class="sm" disabled={entries.length < 100 || loading} onclick={() => { offset += 100; load(); }}>Older</button>
  </div>
</div>

<style>
  .head { margin-bottom: 22px; }
  .head p { margin: 4px 0 0; }
  .section { margin-top: 16px; }
  .pad { padding: 16px 18px 0; margin-bottom: 12px; }
  .list { gap: 10px; max-height: 340px; overflow: auto; }
  .grow { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .avatar { width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; background: #23204099; color: var(--accent); font-weight: 700; font-size: 13px; flex-shrink: 0; }
  .avatar.admin { color: var(--warn); }
  .narrow { width: auto; min-width: 160px; height: 32px; padding: 0 10px; }
  .nowrap { white-space: nowrap; }
  .detail { max-width: 420px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dim { opacity: 0.5; }
  .pager { justify-content: center; padding: 12px; border-top: 1px solid var(--line); }
</style>
