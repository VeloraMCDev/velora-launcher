<script lang="ts">
  import Branding from './Branding.svelte';
  import LandingBuilder from './LandingBuilder.svelte';
  import { go } from '../lib/router.svelte';
  import type { Branding as BrandingData } from '../lib/types';
  let { section, onsaved }: { section: string; onsaved: (branding: BrandingData) => void } = $props();
</script>
<header><h1>Design & website</h1><p>Shape your launcher and public website from one workspace.</p></header>
<nav aria-label="Design sections">
  <button class:on={section === 'branding'} aria-current={section === 'branding' ? 'page' : undefined} onclick={() => go('branding')}>Launcher design</button>
  <button class:on={section === 'landing-builder'} aria-current={section === 'landing-builder' ? 'page' : undefined} onclick={() => go('landing-builder')}>Public website</button>
</nav>
{#if section === 'landing-builder'}<LandingBuilder />{:else}<Branding {onsaved} />{/if}
<style>
  header { padding: 1.5rem 2rem 0; } h1 { margin: 0; } p { color: var(--muted); }
  nav { display: flex; flex-wrap: wrap; gap: .5rem; padding: 1rem 2rem; border-bottom: 1px solid var(--line); }
  nav button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--text); }
  @media (max-width: 600px) { header { padding: 1rem 1rem 0; } nav { padding: 1rem; } }
</style>
