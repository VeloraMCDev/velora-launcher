<script lang="ts">
  import type { Component } from 'svelte';
  import { loadPage, type LazyPageName } from '../lib/pages';

  let { name, load = loadPage }: { name: LazyPageName; load?: (name: LazyPageName) => Promise<{ default: Component }> } = $props();
  let attempt = $state(0);
  let region: HTMLElement;
  const titles: Record<LazyPageName, string> = {
    settings: 'Settings', stats: 'Stats & leaderboards', quests: 'Quests', guilds: 'Factions',
    social: 'Friends & social', commands: 'Player guide', collections: 'Collections', market: 'Market', casino: 'Casino',
  };
  const pending = $derived.by(() => {
    attempt;
    const page = name;
    const loader = load;
    return Promise.resolve().then(() => loader(page));
  });

  function retry() {
    region.focus({ preventScroll: true });
    attempt++;
  }
</script>

<div class="lazy-region" role="region" aria-label={titles[name]} tabindex="-1" bind:this={region}>
{#await pending}
  <section class="page-load" role="status" aria-live="polite" aria-busy="true">
    <h2>Loading {titles[name]}</h2>
    <p>Your account and selected instance stay unchanged.</p>
  </section>
{:then loaded}
  <loaded.default />
{:catch}
  <section class="page-load" role="alert">
    <h2>This page could not load</h2>
    <p>Retry may recover a temporary download failure. If it keeps failing, use navigation to return home. No installation files were changed.</p>
    <button class="primary" onclick={retry}>Retry page</button>
  </section>
{/await}
</div>

<style>
  .lazy-region { height: 100%; min-height: 0; }
  .page-load { padding: 2rem; max-width: 42rem; }
  .page-load p { color: var(--muted); }
</style>
