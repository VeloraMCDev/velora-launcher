<script lang="ts">
  import { abs } from '../lib/store.svelte';
  let { text, image = null }: { text: string; image?: string | null } = $props();
  let failed = $state(false);
  $effect(() => { void image; failed = false; });
</script>

{#if image && !failed}
  <img src={abs(image)} alt={text} title={text} onerror={() => (failed = true)} />
{:else}
  <span>{text}</span>
{/if}

<style>
  img { max-width: 10rem; height: 1.6rem; object-fit: contain; image-rendering: pixelated; vertical-align: middle; }
</style>
