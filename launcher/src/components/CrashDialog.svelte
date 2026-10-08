<script lang="ts">
  import { FolderOpen, FileText, Copy, RotateCcw } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { app, play, toast } from '../lib/store.svelte';
  import { invoke } from '../lib/tauri';

  let open = $state(false);
  $effect(() => {
    open = !!app.crash;
  });
  const c = $derived(app.crash);
  const hint = $derived.by(() => {
    const text = c?.tail.join('\n') ?? '';
    if (/OutOfMemoryError|Java heap space/.test(text)) return 'Minecraft ran out of memory. Give it more RAM in Settings → Game.';
    if (/UnsupportedClassVersionError/.test(text)) return 'Wrong Java version. Switch Java back to "Automatic" in Settings → Java.';
    if (/Could not reserve enough space|Invalid maximum heap size/.test(text)) return "Your PC can't give Minecraft that much memory. Lower it in Settings → Game.";
    if (/Mixin|mixin.*failed|Incompatible mod set|requires .* but/i.test(text)) return 'A mod failed to load. If you added mods yourself, try removing them.';
    return null;
  });
</script>

<Modal bind:open title="Minecraft closed unexpectedly" width={40} onclose={() => (app.crash = null)}>
  {#if c}
    <p class="muted small">The game exited with code {c.code}. {hint ?? 'The last lines of the log are below — they usually explain what went wrong.'}</p>
    {#if hint}<div class="hint">{hint}</div>{/if}
    <pre class="tail selectable">{c.tail.slice(-40).join('\n')}</pre>
  {/if}
  {#snippet footer()}
    <button class="ghost" onclick={() => { navigator.clipboard.writeText(c?.tail.join('\n') ?? ''); toast('Copied'); }}><Copy size={15} /> Copy</button>
    {#if c?.crash_report}<button onclick={() => invoke('open_file', { path: c?.crash_report })}><FileText size={15} /> Crash report</button>{/if}
    <button onclick={() => c && invoke('open_folder', { kind: 'logs', instanceId: c.instance_id })}><FolderOpen size={15} /> Logs</button>
    <button class="primary" onclick={() => { const id = c?.instance_id; app.crash = null; if (id) play(id); }}><RotateCcw size={15} /> Try again</button>
  {/snippet}
</Modal>

<style>
  .hint { padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--warn) 12%, transparent); border: 1px solid color-mix(in srgb, var(--warn) 30%, transparent); font-size: 0.88rem; }
  .tail { margin: 0; max-height: 16rem; overflow: auto; font-family: var(--mono); font-size: 0.72rem; line-height: 1.5; padding: 0.8rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--bg) 80%, transparent); border: 1px solid var(--line); white-space: pre-wrap; word-break: break-all; }
</style>
