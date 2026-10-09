<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { toast } from '../lib/state.svelte';
  import { when } from '../lib/format';
  import type { AuditEntry } from '../lib/types';

  let entries = $state<AuditEntry[]>([]);
  let filter = $state('');
  onMount(() => { api<AuditEntry[]>('/api/audit').then(e => (entries = e)).catch(e => toast(e.message, 'error')); });
  const shown = $derived(entries.filter(e => !filter || JSON.stringify(e).toLowerCase().includes(filter.toLowerCase())));
  const tone = (r?: string) => (r === 'succeeded' || r === 'ok' || r === 'sent' || r === 'saved' ? 'ok' : r === 'failed' ? 'bad' : r === 'started' ? 'info' : '');
</script>

<div class="head row wrap">
  <div><h1>Audit log</h1><p class="muted">Every sign-in, operation and alert handled by this dashboard.</p></div>
  <span class="spacer"></span><input class="filter" placeholder="Filter…" bind:value={filter} />
</div>

<div class="card flush">
  <div class="table-scroll">
    <table>
      <thead><tr><th>When</th><th>Who</th><th>Action</th><th>Target</th><th>Result</th></tr></thead>
      <tbody>
        {#each shown as e}
          <tr>
            <td class="tiny nowrap">{when(e.at)}</td>
            <td>{e.user}{#if e.ip}<span class="tiny faint"> · {e.ip}</span>{/if}</td>
            <td><code class="small">{e.action}</code></td>
            <td class="small muted">{e.target ?? ''}</td>
            <td><span class="badge {tone(e.result)}">{e.result ?? ''}</span></td>
          </tr>
        {:else}<tr><td colspan="5" class="empty">Nothing recorded yet.</td></tr>{/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .head { margin-bottom: 22px; }
  .head p { margin: 4px 0 0; }
  .filter { width: 240px; }
  .nowrap { white-space: nowrap; }
</style>
