<script lang="ts">
  import { playPath } from '../lib/router.svelte';
  import { pageEnabled, enabled, navigation } from '@velora/experience';
  import { onMount } from 'svelte';
  import { ChevronDown, Command, Ellipsis, LogOut, Search, ShieldCheck, Check, ExternalLink } from '@lucide/svelte';
  import { COMMAND_GROUPS } from '@velora/commands';
  import Avatar from '../components/Avatar.svelte';
  import NotificationBell from '../components/NotificationBell.svelte';
  import Palette, { type PaletteItem } from '../components/Palette.svelte';
  import Sheet from './ui/Sheet.svelte';
  import Count from './ui/Count.svelte';
  import { PAGES as ALL_PAGES } from './nav';
  import { nativeApp } from '../lib/native';
  import { currentServer, loadManifest, loadServers, money, play, selectServer } from './store.svelte';
  import { go, route } from '../lib/router.svelte';
  import { logout, session } from '../lib/session.svelte';
  import './play.css';

  // The phone app has no use for the launcher download page.
  const instance = $derived(play.manifest?.instances.find(i => i.id === route.instanceId));
  const availablePages = $derived(ALL_PAGES.filter(p => (!nativeApp || p.id !== 'launcher') && (p.id !== 'experience' || !!instance?.experience?.widgets.length) && pageEnabled(instance?.experience, p.id)));
  const PAGES = $derived(navigation(instance?.experience, availablePages).map(p => ({ ...availablePages.find(a => a.id === p.id)!, label: p.label })));
  $effect(() => { if (play.manifest && !route.instanceId && play.manifest.instances.length) go(`play/instance/${encodeURIComponent(play.manifest.instances.find(i => i.featured)?.id ?? play.manifest.instances[0].id)}/home`); });
  $effect(() => { const id = route.instanceId; if (id) { play.serverId = null; play.balance = null; play.servers = []; void loadServers(); } });
  const page = $derived(PAGES.find((p) => p.id === (route.params[0] ?? 'home')) ?? PAGES[0]);
  const brand = $derived(instance?.experience?.branding ?? play.manifest?.branding);
  const primary = $derived(PAGES.filter((p) => p.primary));
  const more = $derived(PAGES.filter((p) => !p.primary));
  const isAdmin = $derived(session.user?.role === 'admin');
  const server = $derived(currentServer());

  let paletteOpen = $state(false);
  let moreOpen = $state(false);
  let serverOpen = $state(false);
  let menuOpen = $state(false);

  onMount(() => {
    void loadManifest();
    void loadServers();
    const t = setInterval(() => void loadServers(), 60_000);
    return () => clearInterval(t);
  });

  $effect(() => { document.title = `${brand?.name ?? 'Velora'} · ${page.label}`; });
  $effect(() => { void route.params[0]; moreOpen = false; menuOpen = false; serverOpen = false; });

  // The panel borrows the launcher's brand colours, so the site and the launcher feel like one product.
  const themeVars = $derived(brand
    ? `--accent:${brand.colors.accent};--accent-2:${brand.colors.accent_2};--good:${brand.colors.success};--bad:${brand.colors.danger};`
    : '');

  const items = $derived<PaletteItem[]>([
    ...PAGES.map((p) => ({ id: `page-${p.id}`, label: p.label, group: 'Go to', hint: p.blurb, icon: p.icon, run: () => go(`play/${p.id}`) })),
    ...play.servers.map((s) => ({ id: `srv-${s.id}`, label: `Switch to ${s.name}`, group: 'Servers', hint: s.online ? `${s.players_online} online` : 'offline', run: () => selectServer(s.id) })),
    ...(enabled(instance?.experience, 'commands') ? COMMAND_GROUPS : []).flatMap((g) => g.cmds.map((c) => ({
      id: `cmd-${c.usage}`, label: c.usage, group: 'In-game commands', hint: c.does, keywords: `${g.title} ${(c.aliases ?? []).join(' ')} ${c.perm}`,
      run: () => { try { sessionStorage.setItem('scopenet.play.cmdq', c.usage.split(' ')[0]); } catch { /* ignore */ } go('play/commands'); },
    }))),
    ...(isAdmin ? [{ id: 'admin', label: 'Open admin panel', group: 'Account', icon: ShieldCheck, run: () => go(route.instanceId ? 'control' : 'instances') }] : []),
    { id: 'signout', label: 'Sign out', group: 'Account', icon: LogOut, run: () => { logout(); go('login'); } },
  ]);

  const pageLoad = $derived(page.load());
</script>

<div class="play" style={themeVars}>
  <div class="pl-shell">
    <aside class="pl-rail" aria-label="Player panel">
      <a class="pl-brand" href={playPath('home')}>
        <img src={brand?.logo_url ?? brand?.icon_url ?? '/favicon.svg'} alt="" />
        <div><strong>{brand?.name ?? 'Velora'}</strong><span>Player panel</span></div>
      </a>
      <nav class="pl-links">
        {#each PAGES as p (p.id)}
          <a class="pl-link" class:on={page.id === p.id} href={`#/play/instance/${encodeURIComponent(route.instanceId ?? '')}/${p.id}`} aria-current={page.id === p.id ? 'page' : undefined}><p.icon size={18} /> <span>{p.label}</span></a>
        {/each}
      </nav>
      <div class="pl-rail-foot">
        <button class="pl-kbd-hint" onclick={() => (paletteOpen = true)}><span class="pl-row-flex"><Search size={14} /> Quick search</span><kbd>Ctrl K</kbd></button>
        {#if isAdmin}<a class="pl-link" href={route.instanceId ? `#/instance/${encodeURIComponent(route.instanceId)}/control` : '#/instances'}><ShieldCheck size={18} /> <span>Admin panel</span></a>{/if}
      </div>
    </aside>

    <div class="pl-main">
      <header class="pl-top">
        <a class="pl-brand-sm" href={playPath('home')}><img src={brand?.logo_url ?? brand?.icon_url ?? '/favicon.svg'} alt="" /><span>{brand?.name ?? 'Velora'}</span></a>
        <div class="pl-grow"></div>

        <label class="pl-pill">Experience
          <select value={route.instanceId ?? ''} onchange={e => go(`play/instance/${encodeURIComponent(e.currentTarget.value)}/home`)}>
            {#each play.manifest?.instances ?? [] as i}<option value={i.id}>{i.name}</option>{/each}
          </select>
        </label>
        {#if play.servers.length}
          <button class="pl-pill" onclick={() => (serverOpen = true)} aria-label="Choose server" title="Choose server">
            <i class="pl-dot" class:on={server?.online}></i>
            <span class="srv-name">{server?.name ?? 'Server'}</span>
            <ChevronDown size={14} />
          </button>
        {/if}
        {#if enabled(instance?.experience, 'economy')}<button class="pl-pill gold" onclick={() => go('play/wallet')} title="Your balance" aria-label="Wallet"><Count value={play.balance ?? (play.loaded && play.serverId != null ? 0 : null)} format={(v) => money(v)} /></button>{/if}
        <button class="pl-iconbtn pl-hide-sm" onclick={() => (paletteOpen = true)} aria-label="Quick search" title="Quick search (Ctrl K)"><Command size={17} /></button>
        <NotificationBell inline />
        <button class="me" onclick={() => (menuOpen = !menuOpen)} aria-label="Account menu" aria-expanded={menuOpen}>
          <Avatar name={session.user?.username ?? '?'} uuid={session.user?.uuid} size={38} />
        </button>
        {#if menuOpen}
          <button class="scrim" onclick={() => (menuOpen = false)} aria-label="Close menu" tabindex="-1"></button>
          <div class="menu" role="menu">
            <div class="who"><Avatar name={session.user?.username ?? '?'} uuid={session.user?.uuid} size={40} /><div><b>{session.user?.username}</b><span>{isAdmin ? 'Administrator' : 'Player'}</span></div></div>
            <a href={playPath('profile')} role="menuitem">Account & skin</a>
            {#if enabled(instance?.experience, 'commands')}<a href={playPath('commands')} role="menuitem">Command guide</a>{/if}
            {#if isAdmin}<a href={route.instanceId ? `#/instance/${encodeURIComponent(route.instanceId)}/control` : '#/instances'} role="menuitem"><ShieldCheck size={15} /> Admin panel</a>{/if}
            <button onclick={() => { logout(); go('login'); }} role="menuitem"><LogOut size={15} /> Sign out</button>
          </div>
        {/if}
      </header>

      <main class="pl-content">
        {#key (route.instanceId ?? '') + page.id}
          {#await pageLoad}
            <div class="pl-page"><div class="pl-skel" style="height: 38px; width: 220px; margin-bottom: 18px"></div><div class="pl-grid"><div class="pl-skel" style="height: 150px"></div><div class="pl-skel" style="height: 150px"></div><div class="pl-skel" style="height: 150px"></div></div></div>
          {:then mod}
            {@const Page = mod.default}
            <Page />
          {:catch}
            <div class="pl-page"><div class="pl-alert err">This page couldn't load. Check your connection and refresh.</div></div>
          {/await}
        {/key}
      </main>
    </div>
  </div>

  <nav class="pl-tabbar" aria-label="Main">
    {#each primary as p (p.id)}
      <a class="pl-tab" class:on={page.id === p.id} href={`#/play/instance/${encodeURIComponent(route.instanceId ?? '')}/${p.id}`} aria-current={page.id === p.id ? 'page' : undefined}><p.icon size={22} /><span>{p.label}</span></a>
    {/each}
    <button class="pl-tab" class:on={more.some((p) => p.id === page.id)} onclick={() => (moreOpen = true)} aria-label="More pages"><Ellipsis size={22} /><span>More</span></button>
  </nav>

  <Sheet bind:open={moreOpen} title="More">
    <div class="more-grid">
      {#each more as p (p.id)}
        <a class="more-item" class:on={page.id === p.id} href={`#/play/instance/${encodeURIComponent(route.instanceId ?? '')}/${p.id}`}><span class="ic"><p.icon size={22} /></span><b>{p.label}</b><small>{p.blurb}</small></a>
      {/each}
    </div>
    <button class="pl-btn block" onclick={() => { moreOpen = false; paletteOpen = true; }}><Search size={16} /> Quick search</button>
    {#if isAdmin}<a class="btn pl-btn block" href={route.instanceId ? `#/instance/${encodeURIComponent(route.instanceId)}/control` : '#/instances'}><ShieldCheck size={16} /> Open admin panel</a>{/if}
  </Sheet>

  <Sheet bind:open={serverOpen} title="Choose a server" width={440}>
    <div class="pl-list">
      {#each play.servers as s (s.id)}
        <button class="pl-item press srv" onclick={() => { selectServer(s.id); serverOpen = false; }}>
          <i class="pl-dot" class:on={s.online}></i>
          <div class="grow"><b>{s.name}</b><span class="sub">{s.online ? `${s.players_online}${s.players_max ? ' / ' + s.players_max : ''} online` : 'Offline'}{s.mc_version ? ` · ${s.mc_version}` : ''}</span></div>
          {#if s.id === play.serverId}<Check size={18} />{/if}
        </button>
      {/each}
    </div>
    <p class="hint"><ExternalLink size={12} /> Servers that share an economy share one balance.</p>
  </Sheet>

  <Palette bind:open={paletteOpen} {items} placeholder="Jump to a page, server or command…" />
</div>

<style>
  .srv-name { max-width: 9.5rem; overflow: hidden; text-overflow: ellipsis; }
  .me { padding: 0; border: 2px solid var(--pl-line, #ffffff14); border-radius: 14px; background: none; display: inline-grid; line-height: 0; overflow: hidden; flex: none; min-height: 0; }
  .me:hover { border-color: var(--accent); background: none; }
  .scrim { position: fixed; inset: 0; z-index: 60; background: transparent; border: none; cursor: default; padding: 0; border-radius: 0; }
  .menu { position: absolute; top: calc(100% - 6px); right: 14px; z-index: 61; width: 240px; padding: 8px; border-radius: 16px; background: var(--surface); border: 1px solid var(--line-strong); box-shadow: 0 24px 60px -20px #000; display: flex; flex-direction: column; gap: 2px; animation: pop 0.16s ease; }
  .menu .who { display: flex; align-items: center; gap: 10px; padding: 8px 8px 12px; border-bottom: 1px solid var(--line); margin-bottom: 4px; }
  .menu .who div { display: flex; flex-direction: column; min-width: 0; }
  .menu .who span { font-size: 0.76rem; color: var(--muted); }
  .menu a, .menu button { display: flex; align-items: center; gap: 10px; padding: 9px 10px; border-radius: 10px; color: var(--text-2); font-weight: 520; border: none; background: none; text-align: left; justify-content: flex-start; }
  .menu a:hover, .menu button:hover { background: rgba(255, 255, 255, 0.06); color: var(--text); }
  @keyframes pop { from { opacity: 0; transform: translateY(-6px) scale(0.98); } }
  .more-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; }
  .more-item { display: flex; flex-direction: column; gap: 2px; padding: 14px; border-radius: 16px; background: var(--surface-2); border: 1px solid var(--line); color: var(--text); }
  .more-item.on { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, var(--surface-2)); }
  .more-item .ic { width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 20%, transparent); color: var(--accent-2); margin-bottom: 6px; }
  .more-item small { color: var(--muted); font-size: 0.74rem; line-height: 1.3; }
  .srv { width: 100%; background: none; border-radius: 12px; border: none; text-align: left; color: var(--text); }
  .hint { display: flex; gap: 6px; align-items: center; color: var(--muted); font-size: 0.78rem; }
  :global(.play .pl-top) { position: sticky; }
  @media (max-width: 880px) {
    .srv-name { max-width: 5.2rem; }
    :global(.play .pl-top .pl-pill) { padding: 6px 11px; font-size: 0.84rem; gap: 6px; min-width: 0; }
    :global(.play .pl-top .pl-brand-sm span) { display: none; }
    :global(.play .pl-top > :not(.pl-grow)) { flex-shrink: 0; }
    :global(.play .pl-top > .pl-pill:not(.gold)) { flex-shrink: 1; min-width: 0; }
  }
</style>
