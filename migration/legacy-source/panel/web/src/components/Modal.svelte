<script lang="ts">
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';

  let { open = $bindable(false), title, width = 520, children, footer }: {
    open: boolean; title: string; width?: number; children: Snippet; footer?: Snippet;
  } = $props();

  // Widths under 100 are treated as rem so a stray `width={32}` can never collapse the dialog.
  const maxWidth = $derived(width < 100 ? width * 16 : width);

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }
</script>

<svelte:window onkeydown={open ? onkey : undefined} />

{#if open}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && (open = false)}>
    <div class="modal" style:max-width="{maxWidth}px" role="dialog" aria-modal="true" aria-label={title}>
      <header>
        <h2>{title}</h2>
        <button class="ghost icon" onclick={() => (open = false)} aria-label="Close"><X size={18} /></button>
      </header>
      <div class="body">{@render children()}</div>
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0; background: rgba(5, 6, 9, 0.7);
    display: grid; place-items: center; z-index: 50; padding: 20px; animation: fade 0.15s ease;
  }
  .modal {
    width: 100%; max-height: calc(100vh - 40px); overflow: auto; background: var(--surface);
    border: 1px solid var(--line-strong); border-radius: 12px; box-shadow: var(--shadow);
    animation: rise 0.2s ease;
  }
  header { display: flex; align-items: center; justify-content: space-between; padding: 18px 20px 0; }
  .body { padding: 18px 20px 20px; display: flex; flex-direction: column; gap: 14px; }
  footer { display: flex; justify-content: flex-end; gap: 10px; padding: 14px 20px; border-top: 1px solid var(--line); }
  @keyframes fade { from { opacity: 0; } }
  @keyframes rise { from { opacity: 0; transform: translateY(4px); } }
</style>
