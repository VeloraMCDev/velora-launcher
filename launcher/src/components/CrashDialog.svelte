<script lang="ts">
  import InterfaceIcon from '@velora/experience/InterfaceIcon.svelte';
  import { FolderOpen, FileText, Copy, RotateCcw, Download } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { app, play, toast } from '../lib/store.svelte';
  import { invoke } from '../lib/tauri';
  import { createDiagnosticReport } from '../lib/diagnostics';

  let open = $state(false);
  $effect(() => {
    open = !!app.crash;
  });
  const crash = $derived(app.crash);
  const report = $derived(createDiagnosticReport(crash, app.launch));

  async function copyReport() {
    if (!crash) return;
    try {
      await navigator.clipboard.writeText(report.text);
      toast('Support report copied');
    } catch {
      toast('Could not copy the report. Select the report text or try exporting.', 'error');
    }
  }

  function exportReport() {
    if (!crash) return;
    let objectUrl: string | null = null;
    try {
      objectUrl = URL.createObjectURL(new Blob([report.text], { type: 'text/plain;charset=utf-8' }));
      const download = document.createElement('a');
      download.href = objectUrl;
      download.download = 'velora-support-diagnostics.txt';
      download.click();
    } catch {
      toast('Could not export the report. Select the report text or try copying.', 'error');
    } finally {
      if (objectUrl) setTimeout(() => URL.revokeObjectURL(objectUrl!), 1000);
    }
  }
</script>

<Modal bind:open title="Minecraft closed unexpectedly" width={40} onclose={() => (app.crash = null)}>
  {#if crash}
    <p class="muted small">This report summarizes only the existing crash/preparation snapshot. It does not inspect your computer or verify game health. Nothing is uploaded automatically.</p>
    <div class="hint"><strong>Categories: {report.categories.join(', ')}</strong><ul>{#each report.advice as step}<li>{step}</li>{/each}</ul></div>
    <p class="muted small">Review before sharing. Copy and export contain only the redacted report below. Original crash reports and logs may contain private information; opening them does not add them to this report.</p>
    <pre class="tail selectable">{report.text}</pre>
  {/if}
  {#snippet footer()}
    <button class="ghost" disabled={!crash} onclick={copyReport}><InterfaceIcon name="Copy" fallback={Copy} size={15} /> Copy report</button>
    <button class="ghost" disabled={!crash} onclick={exportReport}><InterfaceIcon name="Download" fallback={Download} size={15} /> Export report</button>
    {#if crash?.crash_report}<button onclick={() => invoke('open_file', { path: crash?.crash_report })}><InterfaceIcon name="FileText" fallback={FileText} size={15} /> Crash report</button>{/if}
    <button onclick={() => crash && invoke('open_folder', { kind: 'logs', instanceId: crash.instance_id })}><InterfaceIcon name="FolderOpen" fallback={FolderOpen} size={15} /> Logs</button>
    <button class="primary" onclick={() => { const instanceId = crash?.instance_id; app.crash = null; if (instanceId) play(instanceId); }}><InterfaceIcon name="RotateCcw" fallback={RotateCcw} size={15} /> Try again</button>
  {/snippet}
</Modal>

<style>
  .hint { padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--warn) 12%, transparent); border: 1px solid color-mix(in srgb, var(--warn) 30%, transparent); font-size: 0.88rem; }
  .tail { margin: 0; max-height: 16rem; overflow: auto; font-family: var(--mono); font-size: 0.72rem; line-height: 1.5; padding: 0.8rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--bg) 80%, transparent); border: 1px solid var(--line); white-space: pre-wrap; word-break: break-all; }
</style>
