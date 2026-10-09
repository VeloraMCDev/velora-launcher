<script lang="ts">
  import { TriangleAlert } from '@lucide/svelte';
  import { app } from '../lib/state.svelte';

  function key(e: KeyboardEvent) {
    if (!app.confirm) return;
    if (e.key === 'Escape') app.confirm.resolve(false);
  }
</script>

<svelte:window onkeydown={key} />

{#if app.confirm}
  {@const c = app.confirm}
  <div class="scrim" role="presentation" onclick={() => c.resolve(false)}></div>
  <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
    <div class="row">
      {#if c.danger}<span class="icon"><TriangleAlert size={20} /></span>{/if}
      <h2 id="confirm-title">{c.title}</h2>
    </div>
    <p class="muted">{c.body}</p>
    <div class="row actions">
      <button class="ghost" onclick={() => c.resolve(false)}>Cancel</button>
      <!-- svelte-ignore a11y_autofocus -->
      <button class={c.danger ? 'danger' : 'primary'} autofocus onclick={() => c.resolve(true)}>{c.action}</button>
    </div>
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; background: #05050acc; backdrop-filter: blur(3px); z-index: 50; }
  .dialog { position: fixed; z-index: 51; left: 50%; top: 22vh; transform: translateX(-50%); width: min(480px, calc(100vw - 32px)); background: var(--panel); border: 1px solid var(--line-2); border-radius: 18px; padding: 22px 24px; box-shadow: var(--shadow); }
  p { line-height: 1.6; margin: 12px 0 20px; white-space: pre-line; }
  .actions { justify-content: flex-end; }
  .icon { color: var(--warn); display: grid; }
</style>
