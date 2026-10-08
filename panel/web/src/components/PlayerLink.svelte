<script lang="ts">
  import type { Snippet } from 'svelte';
  import { go } from '../lib/router.svelte';

  // A player's name that opens their profile in the Players page.
  let { uuid, name = '', children }: { uuid?: string | null; name?: string; children?: Snippet } = $props();
</script>

{#if uuid}
  <a class="plink" href={`#/users/${uuid}`} title="View {name || 'player'}'s profile" onclick={(e) => { e.preventDefault(); go(`users/${uuid}`); }}>{#if children}{@render children()}{:else}{name}{/if}</a>
{:else}
  {#if children}{@render children()}{:else}{name}{/if}
{/if}

<style>
  .plink { color: inherit; text-decoration: none; border-radius: 4px; display: inline-block; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: inherit; vertical-align: bottom; }
  .plink:hover { color: var(--accent, #8b6cff); text-decoration: underline; text-underline-offset: 3px; }
</style>
