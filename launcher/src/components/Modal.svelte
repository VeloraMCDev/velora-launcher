<script lang="ts">
  import InterfaceIcon from '@velora/experience/InterfaceIcon.svelte';
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  import DialogSurface from './ui/DialogSurface.svelte';
  let { open = $bindable(false), title, width = 30, children, footer, onclose }: { open: boolean; title: string; width?: number; children: Snippet; footer?: Snippet; onclose?: () => void } = $props();
  function close() {
    open = false;
    onclose?.();
  }
</script>

{#if open}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
    <DialogSurface class="modal" label={title} onclose={close} maxWidth={`${width}rem`}>
      <header><h3>{title}</h3><button type="button" class="ghost icon" onclick={close} aria-label={`Close ${title}`}><InterfaceIcon name="X" fallback={X} size={17} /></button></header>
      <div class="body">{@render children()}</div>
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </DialogSurface>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 1.5rem; background: color-mix(in srgb, var(--bg) 62%, transparent); backdrop-filter: blur(6px); -webkit-backdrop-filter: blur(6px); animation: fade 0.18s ease; }
  .backdrop :global(.modal) { width: 100%; min-width: 0; max-height: calc(100dvh - 3rem); display: flex; flex-direction: column; overflow: hidden; background: var(--surface); border-color: var(--line-strong); box-shadow: 0 1rem 3rem -1rem rgba(0, 0, 0, 0.5); animation: pop 0.24s cubic-bezier(0.2, 1.1, 0.3, 1); }
  header { display: flex; align-items: center; justify-content: space-between; padding: 1.1rem 1.25rem 0; }
  header { gap: 0.75rem; flex-shrink: 0; }
  h3 { overflow-wrap: anywhere; }
  .body { min-height: 0; overflow: auto; padding: 1rem 1.25rem 1.25rem; display: flex; flex-direction: column; gap: 0.9rem; }
  footer { display: flex; justify-content: flex-end; gap: 0.6rem; padding: 0.9rem 1.25rem; border-top: 1px solid var(--line); }
  @keyframes pop { from { opacity: 0; transform: translateY(0.8rem) scale(0.97); } }
  @media (max-width: 640px), (max-height: 540px) {
    .backdrop { padding: 0.5rem; }
    .backdrop :global(.modal) { max-height: calc(100dvh - 1rem); }
    header { padding: 0.75rem 0.85rem 0; }
    .body { padding: 0.75rem 0.85rem; }
    footer { padding: 0.75rem 0.85rem; flex-wrap: wrap; flex-shrink: 0; }
  }
</style>
