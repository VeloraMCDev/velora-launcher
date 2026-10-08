<script lang="ts">
  import { CircleCheck, CircleAlert, Info, X } from '@lucide/svelte';
  import { dismiss, toasts } from '../lib/toast.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast {t.kind}">
      {#if t.kind === 'ok'}<CircleCheck size={18} />{:else if t.kind === 'error'}<CircleAlert size={18} />{:else}<Info size={18} />{/if}
      <span>{t.text}</span>
      <button class="ghost icon" onclick={() => dismiss(t.id)} aria-label="Dismiss"><X size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts { position: fixed; right: 20px; bottom: 20px; display: flex; flex-direction: column; gap: 10px; z-index: 100; max-width: 420px; }
  .toast {
    display: flex; align-items: center; gap: 10px; padding: 10px 10px 10px 14px; border-radius: 12px;
    background: var(--surface-2); border: 1px solid var(--line-strong);
    box-shadow: var(--shadow); animation: pop 0.2s ease; font-size: 0.9rem;
  }
  .toast span { flex: 1; }
  .ok :global(svg:first-child) { color: var(--good); }
  .error :global(svg:first-child) { color: var(--bad); }
  .info :global(svg:first-child) { color: var(--accent-2); }
  @keyframes pop { from { opacity: 0; transform: translateY(8px) scale(0.98); } }
</style>
