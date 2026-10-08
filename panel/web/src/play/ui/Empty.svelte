<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  let { icon: Icon, title, text = '', children }: { icon?: Component<any>; title: string; text?: string; children?: Snippet } = $props();
</script>

<div class="pl-empty">
  {#if Icon}<Icon size={30} />{/if}
  <b>{title}</b>
  {#if text}<span>{text}</span>{/if}
  {@render children?.()}
</div>
