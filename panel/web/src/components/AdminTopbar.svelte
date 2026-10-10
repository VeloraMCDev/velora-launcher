<script lang="ts">
  import { ChevronRight, Menu, Search, Smartphone } from '@lucide/svelte';
  import NotificationBell from './NotificationBell.svelte';
  import { experienceContext } from '../lib/experience.svelte';
  import { findAdminItem, navigationParent } from '../lib/adminNav';
  import { route } from '../lib/router.svelte';

  let { brandName = 'Velora', logo = null, onmenu, onsearch }: { brandName?: string; logo?: string | null; onmenu?: () => void; onsearch?: () => void } = $props();
  const here = $derived(findAdminItem(navigationParent(route.name)));
</script>

<header class="top">
  <button class="ghost icon burger" onclick={onmenu} aria-label="Open menu"><Menu size={20} /></button>
  <img class="logo" src={logo ?? '/favicon.svg'} alt="" />
  <nav class="crumbs" aria-label="Breadcrumb">
    {#if route.instanceId}<a href="#/instances">Instances</a><ChevronRight size={13} /><strong>{experienceContext.instance?.name ?? route.instanceId}</strong><ChevronRight size={13} />{/if}
    {#if here}
      {#if here.group.label !== here.label}<span class="group">{here.group.label}</span><ChevronRight size={13} />{/if}
      <strong>{here.label}</strong>
    {:else}
      <strong>{route.name === 'control' ? 'Overview' : route.instanceId && ['experience', 'installation', 'servers'].includes(route.name) ? 'Instance setup' : brandName}</strong>
    {/if}
  </nav>
  <div class="grow"></div>
  <button class="search" onclick={onsearch} aria-label="Search pages and commands"><Search size={15} /><span>Search</span><kbd>Ctrl K</kbd></button>
  <a class="btn ghost pp" href="#/play" title="Open the player panel"><Smartphone size={16} /><span>Player panel</span></a>
  <NotificationBell inline />
</header>

<style>
  .top { position: sticky; top: 0; z-index: 40; height: 54px; display: flex; align-items: center; gap: 10px; padding: 0 20px; background: color-mix(in srgb, var(--bg) 82%, transparent); backdrop-filter: blur(14px); border-bottom: 1px solid var(--line); }
  .burger, .logo { display: none; }
  .crumbs { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 0.88rem; min-width: 0; }
  .crumbs strong { color: var(--text); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .grow { flex: 1; }
  .search { display: inline-flex; align-items: center; gap: 9px; padding: 6px 10px 6px 12px; border-radius: 10px; background: var(--surface); color: var(--muted); font-weight: 450; font-size: 0.85rem; }
  .search:hover { color: var(--text); }
  .search kbd { font-size: 0.66rem; }
  .pp { padding: 6px 12px; font-size: 0.85rem; color: var(--text-2); }
  @media (max-width: 900px) {
    .top { padding: 0 10px 0 6px; gap: 6px; }
    .burger { display: inline-flex; }
    .logo { display: block; width: 26px; height: 26px; border-radius: 7px; }
    .group, .crumbs :global(svg) { display: none; }
    .search span, .search kbd, .pp span { display: none; }
    .search { padding: 8px; }
    .pp { padding: 8px; }
  }
</style>
