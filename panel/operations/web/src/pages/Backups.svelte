<script lang="ts">
  import { Archive, CloudUpload, Play, CircleCheck, CircleX, Clock } from '@lucide/svelte';
  import { app, ask, runAction } from '../lib/state.svelte';
  import { ago, bytes, when } from '../lib/format';

  const b = $derived(app.overview?.snapshot?.backups);
  async function backupNow() {
    if (await ask('Back up now?', 'The Panel pauses for about a second while the final copy is taken. The archive is verified, kept on the VPS and copied to Hermes.', 'Back up now')) runAction('/api/actions/backup');
  }
</script>

<div class="head row wrap">
  <div><h1>Backups</h1><p class="muted">Nightly at 09:30 UTC: a cold, verified copy of all Panel data, kept 7 days on the VPS and 30 days on Hermes.</p></div>
  <span class="spacer"></span>
  <button class="primary" disabled={b?.pending} onclick={backupNow}><Play size={16} /> {b?.pending ? 'Backup queued…' : 'Back up now'}</button>
</div>

<div class="grid g3">
  <div class="card stat">
    <span class="label">{#if b?.last?.status === 'ok'}<CircleCheck size={14} color="var(--ok)" />{:else}<CircleX size={14} color="var(--bad)" />{/if} Last backup</span>
    <span class="value">{b?.last ? ago(b.last.finished_at) : 'never'}</span>
    <span class="sub">{b?.last?.message ?? 'No backup has run yet.'}</span>
  </div>
  <div class="card stat">
    <span class="label"><CloudUpload size={14} /> Offsite copy</span>
    <span class="value">{b?.last ? (b.last.offsite ? 'Hermes' : 'missing') : '—'}</span>
    <span class="sub">Write-only rsync over Tailscale</span>
  </div>
  <div class="card stat">
    <span class="label"><Clock size={14} /> Duration</span>
    <span class="value">{b?.last ? `${Math.round((Date.parse(b.last.finished_at) - Date.parse(b.last.started_at)) / 1000)}s` : '—'}</span>
    <span class="sub">{bytes(b?.last?.bytes)} compressed</span>
  </div>
</div>

<div class="card flush section">
  <div class="card-head pad"><Archive size={18} /><h2>Archives on the VPS</h2><span class="spacer"></span><span class="tiny muted mono">/var/backups/velora</span></div>
  <table>
    <thead><tr><th>Archive</th><th>Created</th><th>Size</th><th>Checksum</th></tr></thead>
    <tbody>
      {#each b?.archives ?? [] as a}
        <tr><td class="mono small">{a.name}</td><td>{when(a.at)} <span class="tiny faint">{ago(a.at)}</span></td><td>{bytes(a.bytes)}</td>
          <td class="mono tiny muted">{a.name === b?.last?.archive ? `${b.last.sha256?.slice(0, 16)}…` : ''}</td></tr>
      {:else}<tr><td colspan="4" class="empty">No archives yet.</td></tr>{/each}
    </tbody>
  </table>
</div>

<div class="card section">
  <h2>Restoring</h2>
  <p class="muted small">Stop the Panel, extract an archive over a fresh <code>panel-data</code> directory as root (keeping owner 65532), then start the Panel and confirm its signing key matches. Archives on Hermes live in <code>/mnt/2TB-1/backups/velora-nightly</code> with a <code>.sha256</code> beside each.</p>
</div>

<style>
  .head { margin-bottom: 22px; }
  .head p { margin: 4px 0 0; }
  .section { margin-top: 16px; }
  .pad { padding: 16px 18px 0; margin-bottom: 12px; }
  .card p { margin: 8px 0 0; line-height: 1.6; }
</style>
