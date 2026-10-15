<script lang="ts">
  import type { Snippet } from 'svelte';
  let { tone = 'info', children, class: className = '' }: {
    tone?: 'info' | 'success' | 'warning' | 'danger' | 'busy'; children: Snippet; class?: string;
  } = $props();
</script>

<p class={`state-message ${className}`} data-state={tone} role={tone === 'danger' ? 'alert' : 'status'} aria-atomic="true">
  {@render children()}
</p>

<style>
  .state-message { color: var(--state-info-text); overflow-wrap: anywhere; }
  [data-state='success'] { color: var(--state-success-text); }
  [data-state='warning'] { color: var(--state-warning-text); }
  [data-state='danger'] { color: var(--state-danger-text); }
  [data-state='busy'] { color: var(--state-busy-text); }
</style>
