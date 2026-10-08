<script lang="ts">
  import type { Snippet } from 'svelte';
  import { app } from '../lib/store.svelte';

  // A player's name that opens their profile. Safe to place inside other clickable rows.
  let { uuid, name = '', class: cls = '', children }: { uuid?: string | null; name?: string; class?: string; children?: Snippet } = $props();
  const open = (e: Event) => { e.stopPropagation(); if (uuid) app.viewProfileUuid = uuid; };
</script>

{#if uuid}
  <button type="button" class="plink {cls}" title="View {name || 'player'}'s profile" onclick={open}
    onkeydown={(e) => e.stopPropagation()}>{#if children}{@render children()}{:else}{name}{/if}</button>
{:else}
  <span class={cls}>{#if children}{@render children()}{:else}{name}{/if}</span>
{/if}

<style>
  .plink { all: unset; cursor: pointer; font: inherit; color: inherit; border-radius: 4px; transition: color 0.12s ease; display: inline-block; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: inherit; vertical-align: bottom; }
  .plink:hover { color: var(--accent); text-decoration: underline; text-underline-offset: 3px; }
  .plink:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
