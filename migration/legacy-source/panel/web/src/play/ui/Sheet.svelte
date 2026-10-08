<script lang="ts">
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';

  // A bottom sheet on phones and a centred dialog on larger screens. Esc and a tap on the backdrop close it.
  let { open = $bindable(false), title, children, footer, width = 520 }: { open: boolean; title: string; children: Snippet; footer?: Snippet; width?: number } = $props();
</script>

<svelte:window onkeydown={(e) => { if (open && e.key === 'Escape') open = false; }} />

{#if open}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && (open = false)}>
    <div class="sheet" style:--w="{width}px" role="dialog" aria-modal="true" aria-label={title}>
      <div class="grab" aria-hidden="true"></div>
      <header><h2>{title}</h2><button class="x" onclick={() => (open = false)} aria-label="Close"><X size={18} /></button></header>
      <div class="body">{@render children()}</div>
      {#if footer}<footer>{@render footer()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 120; background: rgba(4, 5, 9, 0.66); backdrop-filter: blur(5px); display: grid; place-items: center; padding: 18px; animation: fade 0.16s ease; }
  .sheet { width: 100%; max-width: var(--w); max-height: min(86vh, 760px); display: flex; flex-direction: column; border-radius: 22px; background: var(--surface); border: 1px solid var(--line-strong); box-shadow: 0 30px 80px -20px #000; animation: pop 0.22s cubic-bezier(0.2, 1.1, 0.4, 1); overflow: hidden; }
  .grab { display: none; width: 40px; height: 4px; border-radius: 4px; background: var(--surface-3); margin: 8px auto 0; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 16px 18px 0; gap: 10px; }
  h2 { font-size: 1.1rem; }
  .x { width: 34px; height: 34px; padding: 0; border-radius: 10px; background: transparent; border-color: transparent; color: var(--muted); }
  .body { padding: 14px 18px 18px; overflow-y: auto; display: flex; flex-direction: column; gap: 14px; }
  footer { padding: 12px 18px calc(14px + env(safe-area-inset-bottom)); border-top: 1px solid var(--line); display: flex; gap: 10px; justify-content: flex-end; flex-wrap: wrap; }
  @keyframes fade { from { opacity: 0; } }
  @keyframes pop { from { opacity: 0; transform: translateY(14px) scale(0.97); } }
  @media (max-width: 640px) {
    .backdrop { place-items: end center; padding: 0; }
    .sheet { max-width: 100%; border-radius: 24px 24px 0 0; max-height: 90dvh; animation: slide 0.26s cubic-bezier(0.2, 0.9, 0.3, 1); }
    .grab { display: block; }
    footer > :global(button) { flex: 1; }
  }
  @keyframes slide { from { transform: translateY(100%); } }
</style>
