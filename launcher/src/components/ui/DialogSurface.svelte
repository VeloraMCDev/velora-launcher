<script lang="ts">
  import type { Snippet } from 'svelte';
  import { focusTrap } from '../../lib/focus';
  let { label, onclose, initialFocus, children, class: className = '', maxWidth }: {
    label: string; onclose: () => void; initialFocus?: () => HTMLElement | null | undefined;
    children: Snippet; class?: string; maxWidth?: string;
  } = $props();
</script>

<div class={`dialog-surface glass ${className}`} role="dialog" aria-modal="true" aria-label={label}
  tabindex="-1" style:max-width={maxWidth} use:focusTrap={{ onclose, initialFocus }}>
  {@render children()}
</div>
