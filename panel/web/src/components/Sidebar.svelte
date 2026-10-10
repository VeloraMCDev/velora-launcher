<script lang="ts">
  import { ChevronDown, ExternalLink, Globe, LogOut, Search, Smartphone, X } from '@lucide/svelte';
  import { adminGroups, navigationParent } from '../lib/adminNav';
  import { experienceContext } from '../lib/experience.svelte';
  import { PLATFORM_PAGES } from '@velora/experience';
  import { route } from '../lib/router.svelte';
  import { logout, session } from '../lib/session.svelte';
  import Avatar from './Avatar.svelte';
  import McIcon from './McIcon.svelte';
  import { NAV_ITEMS } from '../lib/mcNav';

  let { brandName = 'Velora', logo = null, open = $bindable(false), onsearch }: { brandName?: string; logo?: string | null; open?: boolean; onsearch?: () => void } = $props();

  // Only the group you are working in is open at first, so the list stays short. What you open is remembered per browser.
  const KEY = 'scopenet.admin.nav.open';
  const read = (): string[] => { try { return JSON.parse(localStorage.getItem(KEY) ?? '["overview"]'); } catch { return ['overview']; } };
  let opened = $state<string[]>(read());
  const groups = $derived(adminGroups(route.instanceId, experienceContext.instance?.experience));
  const activePage = $derived(navigationParent(route.name));
  const activeGroup = $derived(groups.find((g) => g.items.some((i) => navigationParent(i.id) === activePage))?.id);
  const isOpen = (id: string) => id === activeGroup || opened.includes(id);
  function toggle(id: string) {
    opened = opened.includes(id) ? opened.filter((c) => c !== id) : [...opened, id];
    try { localStorage.setItem(KEY, JSON.stringify(opened)); } catch { /* storage unavailable */ }
  }
  $effect(() => { void route.name; open = false; });
</script>

{#if open}<button class="scrim" onclick={() => (open = false)} aria-label="Close menu" tabindex="-1"></button>{/if}

<aside class:open aria-label="Admin navigation">
  <div class="brand">
    {#if logo}<img src={logo} alt="" />{:else}<img src="/favicon.svg" alt="" />{/if}
    <div>
      <strong>{brandName}</strong>
      <span>{experienceContext.instance?.name ?? "Platform administration"}</span>
    </div>
    <button class="ghost icon close" onclick={() => (open = false)} aria-label="Close menu"><X size={18} /></button>
  </div>

  {#if route.instanceId}<a class="find" href="#/instances">← Switch instance</a>{/if}

  <button class="find" onclick={() => { open = false; onsearch?.(); }}>
    <Search size={15} /> <span>Search…</span> <kbd>Ctrl K</kbd>
  </button>

  <nav>
    {#each groups as g (g.id)}
      {@const expanded = isOpen(g.id)}
      <div class="group">
        <button class="gh" onclick={() => toggle(g.id)} aria-expanded={expanded} disabled={g.id === activeGroup}>
          <span>{g.label}</span>
          <ChevronDown size={13} class={expanded ? '' : 'turned'} />
        </button>
        {#if expanded}
          <div class="items">
            {#each g.items as item (item.id)}
              <a href={route.instanceId && !PLATFORM_PAGES.has(item.id) ? `#/instance/${encodeURIComponent(route.instanceId)}/${item.id}` : `#/${item.id}`} class:active={activePage === navigationParent(item.id)} aria-current={activePage === item.id ? 'page' : undefined}>
                <McIcon item={NAV_ITEMS[item.id]} fallback={item.icon} size={20} />
                <span>{item.label}</span>
              </a>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </nav>

  <div class="foot">
    <a class="player" href="#/play"><Smartphone size={15} /> Player panel <span class="soft">members &amp; mobile</span></a>
    <div class="links">
      <a href="#/landing" target="_blank" rel="noreferrer"><Globe size={13} /> Landing page</a>
      <a href="https://github.com/VeloraMCDev/velora-launcher#readme" target="_blank" rel="noreferrer"><ExternalLink size={13} /> Docs</a>
    </div>
    {#if session.user}
      <div class="me">
        <Avatar name={session.user.username} uuid={session.user.uuid} size={32} />
        <div class="who">
          <strong>{session.user.username}</strong>
          <span>Administrator</span>
        </div>
        <button class="ghost icon" onclick={logout} title="Sign out" aria-label="Sign out"><LogOut size={16} /></button>
      </div>
    {/if}
  </div>
</aside>

<style>
  aside {
    position: sticky; top: 0; height: 100vh; height: 100dvh; width: 244px; flex-shrink: 0; display: flex; flex-direction: column;
    padding: 16px 12px 12px; border-right: 1px solid var(--line); background: var(--bg-2); z-index: 70;
  }
  .brand { display: flex; align-items: center; gap: 11px; padding: 2px 8px 14px; flex-shrink: 0; }
  .brand img { width: 32px; height: 32px; border-radius: 9px; object-fit: cover; }
  .brand div { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .brand strong { font-size: 0.95rem; letter-spacing: 0.06em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .brand span { font-size: 0.74rem; color: var(--muted); }
  .close { display: none; }
  .find { display: flex; align-items: center; gap: 9px; justify-content: flex-start; width: 100%; padding: 8px 11px; margin-bottom: 10px; border-radius: 10px; background: var(--surface); color: var(--muted); font-weight: 450; flex-shrink: 0; }
  .find span { flex: 1; text-align: left; }
  .find:hover { color: var(--text); border-color: var(--line-strong); }
  .find kbd { font-size: 0.68rem; }
  nav { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; overscroll-behavior: contain; scrollbar-width: thin; display: flex; flex-direction: column; gap: 6px; padding-right: 2px; }
  .gh { display: flex; width: 100%; align-items: center; justify-content: space-between; gap: 6px; padding: 7px 10px 5px; border: none; background: none; border-radius: 8px; color: var(--muted); font-size: 0.68rem; letter-spacing: 0.09em; text-transform: uppercase; font-weight: 650; }
  .gh:hover:not(:disabled) { color: var(--text-2); background: none; }
  .gh:disabled { opacity: 1; cursor: default; }
  .gh :global(svg) { transition: transform 0.2s var(--ease, ease); }
  .gh :global(svg.turned) { transform: rotate(-90deg); }
  .items { display: flex; flex-direction: column; gap: 1px; }
  .items a {
    display: flex; align-items: center; gap: 11px; padding: 7px 11px; border-radius: 9px; color: var(--text-2);
    font-weight: 500; font-size: 0.9rem; transition: background 0.15s, color 0.15s; position: relative;
  }
  .items a:hover { background: rgba(255, 255, 255, 0.04); color: var(--text); }
  .items a.active { background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--text); }
  .items a.active::before { content: ''; position: absolute; left: -12px; top: 8px; bottom: 8px; width: 3px; border-radius: 0 3px 3px 0; background: var(--accent); }
  .items a.active :global(svg) { color: var(--accent-2); }
  .foot { flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; padding-top: 10px; border-top: 1px solid var(--line); margin-top: 8px; }
  .player { display: flex; align-items: center; gap: 9px; padding: 9px 11px; border-radius: 10px; background: color-mix(in srgb, var(--accent) 14%, var(--surface)); border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent); color: var(--text); font-weight: 550; font-size: 0.86rem; }
  .player:hover { border-color: var(--accent); }
  .player .soft { margin-left: auto; font-size: 0.68rem; color: var(--muted); font-weight: 450; }
  .links { display: flex; gap: 14px; padding: 0 6px; }
  .links a { display: inline-flex; align-items: center; gap: 6px; font-size: 0.78rem; color: var(--muted); }
  .links a:hover { color: var(--text); }
  .me { display: flex; align-items: center; gap: 10px; padding: 8px 8px 8px 10px; border-radius: 12px; background: var(--surface); border: 1px solid var(--line); }
  .who { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .who strong { font-size: 0.88rem; overflow: hidden; text-overflow: ellipsis; }
  .who span { font-size: 0.72rem; color: var(--muted); }
  .scrim { display: none; }

  /* Phones and small tablets: the menu is a drawer, opened from the top bar. */
  @media (max-width: 900px) {
    aside { position: fixed; left: 0; top: 0; bottom: 0; width: min(300px, 86vw); transform: translateX(-102%); transition: transform 0.26s var(--ease, ease); box-shadow: 0 0 60px #000a; }
    aside.open { transform: none; }
    .scrim { display: block; position: fixed; inset: 0; z-index: 65; background: rgba(4, 5, 9, 0.6); backdrop-filter: blur(3px); border: none; border-radius: 0; padding: 0; animation: fade 0.2s ease; }
    .close { display: inline-flex; }
    .find kbd { display: none; }
  }
  @keyframes fade { from { opacity: 0; } }
</style>
