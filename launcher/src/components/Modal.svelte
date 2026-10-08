<script lang="ts">
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  let { open = $bindable(false), title, width = 30, children, footer, onclose }: { open: boolean; title: string; width?: number; children: Snippet; footer?: Snippet; onclose?: () => void } = $props();
  function close() {
    open = false;
    onclose?.();
  }
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && close()} />

{#if open}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
    <div class="modal glass" role="dialog" aria-modal="true" aria-label={title} style:max-width="{width}rem">
      <header><h3>{title}</h3><button class="ghost icon" onclick={close} aria-label="Close"><X size={17} /></button></header>
      <div class="body">{@render children()}</div>
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 1.5rem; background: color-mix(in srgb, var(--bg) 70%, transparent); animation: fade 0.12s ease; }
  .modal { width: 100%; max-height: calc(100vh - 4rem); overflow: auto; background: var(--surface); border-color: var(--line-strong); box-shadow: 0 1rem 3rem -1rem rgba(0, 0, 0, 0.5); animation: pop 0.15s ease; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 1.1rem 1.25rem 0; }
  .body { padding: 1rem 1.25rem 1.25rem; display: flex; flex-direction: column; gap: 0.9rem; }
  footer { display: flex; justify-content: flex-end; gap: 0.6rem; padding: 0.9rem 1.25rem; border-top: 1px solid var(--line); }
  @keyframes pop { from { opacity: 0; transform: translateY(0.3rem); } }
</style>
