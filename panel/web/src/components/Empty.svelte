<script lang="ts">
  import type { Component } from 'svelte';

  let { icon: Icon, title, text = '', message = '' }: { icon?: Component<any>; title?: string; text?: string; message?: string } = $props();
  const body = $derived(text || message);
</script>

<div class="empty">
  {#if Icon}<Icon size={30} />{/if}
  {#if title}<h3>{title}</h3>{/if}
  {#if body}<p>{body}</p>{/if}
</div>

<style>
  .empty { padding: 4rem 2rem; text-align: center; color: var(--muted); display: flex; flex-direction: column; align-items: center; gap: 0.6rem; }
  .empty h3 { margin: 0; font-size: 1rem; font-weight: 600; color: var(--text); }
  .empty p { margin: 0; font-size: 0.875rem; max-width: 32rem; }
</style>
