<script lang="ts">
  import { X, LoaderCircle, CircleCheck, CircleX } from '@lucide/svelte';
  import { app } from '../lib/state.svelte';
  import { ago } from '../lib/format';

  let pre = $state<HTMLPreElement>();
  $effect(() => {
    void app.job?.log?.length;
    if (pre) pre.scrollTop = pre.scrollHeight;
  });
</script>

{#if app.jobOpen && app.job}
  <aside class="drawer" aria-label="Operation log">
    <header>
      {#if app.job.status === 'running'}<LoaderCircle size={18} class="spin" color="var(--accent)" />
      {:else if app.job.status === 'succeeded'}<CircleCheck size={18} color="var(--ok)" />
      {:else}<CircleX size={18} color="var(--bad)" />{/if}
      <div class="title">
        <strong>{app.job.kind}</strong>
        <span class="tiny muted">{app.job.target ?? ''} · {app.job.user} · started {ago(app.job.started)}</span>
      </div>
      <button class="ghost icon" onclick={() => (app.jobOpen = false)} aria-label="Close"><X size={18} /></button>
    </header>
    <pre class="log" bind:this={pre}>{(app.job.log ?? []).join('\n') || 'Starting…'}</pre>
  </aside>
{/if}

<style>
  .drawer { position: fixed; right: 20px; bottom: 20px; width: min(640px, calc(100vw - 40px)); max-height: 60vh; display: flex; flex-direction: column; background: var(--panel); border: 1px solid var(--line-2); border-radius: 16px; box-shadow: var(--shadow); z-index: 40; overflow: hidden; }
  header { display: flex; align-items: center; gap: 12px; padding: 12px 14px 12px 18px; border-bottom: 1px solid var(--line); }
  .title { display: flex; flex-direction: column; flex: 1; min-width: 0; }
  .title strong { color: var(--head); text-transform: capitalize; }
  pre { border: 0; border-radius: 0; flex: 1; min-height: 160px; }
</style>
