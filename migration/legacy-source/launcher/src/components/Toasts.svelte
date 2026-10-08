<script lang="ts">
  import { CircleCheck, CircleAlert, Info } from '@lucide/svelte';
  import { app } from '../lib/store.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast glass {t.kind}">
      {#if t.kind === 'ok'}<CircleCheck size={17} />{:else if t.kind === 'error'}<CircleAlert size={17} />{:else}<Info size={17} />{/if}
      <span class="selectable">{t.text}</span>
    </div>
  {/each}
</div>

<style>
  .toasts { position: fixed; bottom: 1.25rem; right: 1.25rem; z-index: 100; display: flex; flex-direction: column; gap: 0.6rem; max-width: 26rem; }
  .toast { display: flex; gap: 0.6rem; align-items: flex-start; padding: 0.75rem 1rem; font-size: 0.88rem; line-height: 1.4; background: var(--surface); border-color: var(--line-strong); box-shadow: 0 0.6rem 1.5rem -0.8rem rgba(0, 0, 0, 0.5); animation: fade 0.15s ease; }
  .toast :global(svg) { flex-shrink: 0; margin-top: 0.1rem; }
  .ok :global(svg) { color: var(--success); }
  .error :global(svg) { color: var(--danger); }
  .info :global(svg) { color: var(--accent-2); }
</style>
