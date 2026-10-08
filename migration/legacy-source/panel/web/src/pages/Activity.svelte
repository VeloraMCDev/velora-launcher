<script lang="ts">
  import PlayerLink from '../components/PlayerLink.svelte';
  import { onMount } from 'svelte';
  import { get } from '../lib/api';
  type Entry = { id: number; source: string; server: string | null; uuid: string | null; name: string | null; kind: string; detail: string | null; created_at: string };
  let rows = $state<Entry[]>([]);
  let source = $state('');
  let player = $state('');
  let offset = $state(0);
  let busy = $state(false);
  let error = $state('');
  async function load(reset = false) {
    if (reset) offset = 0;
    busy = true; error = '';
    try { rows = await get<Entry[]>('/api/admin/activity?' + new URLSearchParams({ source, player: player.trim(), offset: String(offset) })); }
    catch (e) { error = String(e); }
    finally { busy = false; }
  }
  onMount(() => { void load(); });
</script>

<div class="page">
  <header><h1>Activity</h1><p class="muted">Server gameplay, launcher activity and account changes. Players keep the same UUID when renamed.</p></header>
  <form class="filters" onsubmit={(e) => { e.preventDefault(); load(true); }}>
    <label>Source<select bind:value={source}><option value="">All sources</option><option value="server">Servers</option><option value="launcher">Launcher</option><option value="auth">Sign-ins</option><option value="panel">Account & admin changes</option></select></label>
    <label>Player<input bind:value={player} placeholder="Exact username or permanent UUID" /></label>
    <button disabled={busy}>Refresh</button>
  </form>
  <p class="muted small">Chat counts only. Command arguments and passwords are never included. Repeated gameplay actions are grouped by reporting interval.</p>
  {#if error}<p role="alert">{error}</p>{/if}
  <div class="table-wrap">
    <table>
      <thead><tr><th>Time</th><th>Source</th><th>Player</th><th>Action</th><th>Details</th></tr></thead>
      <tbody>
        {#each rows as row (row.source + ':' + row.id)}
          <tr><td>{new Date(row.created_at).toLocaleString()}</td><td>{row.server ?? row.source}</td><td title={row.uuid ?? ''}>{#if row.uuid && row.name}<PlayerLink uuid={row.uuid} name={row.name} />{:else}{row.name ?? 'Server'}{/if}</td><td>{row.kind.replaceAll('_', ' ')}</td><td>{row.detail ?? '—'}</td></tr>
        {:else}<tr><td colspan="5">{busy ? 'Loading…' : 'No matching activity yet.'}</td></tr>{/each}
      </tbody>
    </table>
  </div>
  <div class="filters"><button class="ghost" disabled={busy || offset === 0} onclick={() => { offset = Math.max(0, offset - 100); load(); }}>Newer</button><span class="muted">Page {offset / 100 + 1}</span><button class="ghost" disabled={busy || rows.length < 100} onclick={() => { offset += 100; load(); }}>Older</button></div>
</div>
<style>
  .page { max-width: 1500px; }
  header { margin-bottom: 24px; }
  .filters { display: flex; gap: 16px; align-items: end; margin: 20px 0; flex-wrap: wrap; }
  label { display: flex; flex-direction: column; gap: 6px; }
  input { min-width: 300px; }
  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; }
  th, td { padding: 12px; border-bottom: 1px solid var(--line); text-align: left; }
  td:last-child { overflow-wrap: anywhere; }
</style>
