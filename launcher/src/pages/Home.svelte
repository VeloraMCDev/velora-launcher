<script lang="ts">
  import { SlidersHorizontal, SquareTerminal, Package, MessageCircle, Globe, ShoppingBag, CirclePlay, Tv, Code, AtSign, Link, Trophy, Eye, EyeOff, Map as MapIcon } from '@lucide/svelte';
  import ExperienceWidgets from '@velora/experience/ExperienceWidgets.svelte';
  import { enabled } from '@velora/experience';
  import PlayButton from '../components/PlayButton.svelte';
  import ServerStatus from '../components/ServerStatus.svelte';
  import NewsFeed from '../components/NewsFeed.svelte';
  import Console from '../components/Console.svelte';
  import MapModal from '../components/MapModal.svelte';
  import { experienceBranding, abs, activeAccount, app, instances, selectedInstance, saveSettings } from '../lib/store.svelte';
  import { loaderName, shortLoaderVersion } from '../lib/format';
  import { invoke, openUrl } from '../lib/tauri';

  const inst = $derived(selectedInstance());
  const b = $derived(experienceBranding()!);
  // Lucide ships no brand logos, so each platform gets a recognisable generic icon.
  const linkIcon: Record<string, any> = { discord: MessageCircle, website: Globe, store: ShoppingBag, youtube: CirclePlay, twitch: Tv, github: Code, twitter: AtSign };
  // A game server linked to this instance that draws the Velora Map.
  let mapServer = $state<{ id: number; name: string } | null>(null);
  let mapOpen = $state(false);
  $effect(() => {
    const id = inst?.id;
    mapServer = null;
    mapOpen = false;
    if (!id) return;
    let alive = true;
    invoke<{ servers: Array<{ id: number; name: string; instance_id: string; map?: boolean }> }>('get_public_servers')
      .then((r) => { if (alive) mapServer = r.servers.find((s) => s.instance_id === id && s.map) ?? null; })
      .catch(() => {});
    return () => { alive = false; };
  });
  const hour = new Date().getHours();
  const greet = hour < 5 ? 'Late night session' : hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';
</script>

<div class="home" class:quiet={!b.features.news || app.settings?.show_news === false || !b.news.length}>
  {#if inst}
    {#key inst.id}
      <section class="hero">
        <p class="greet muted">{greet}, <strong>{activeAccount()?.username ?? 'player'}</strong></p>
        <div class="chips">
          <span class="chip">{inst.mc_version}</span>
          <span class="chip">{loaderName[inst.loader]} {shortLoaderVersion(inst.mc_version, inst.loader_version)}</span>
          {#if inst.file_count}<span class="chip"><Package size={12} /> {inst.file_count} mods & files</span>{/if}
        </div>
        {#if inst.logo_url}
          <div class="logo-wrap">
            <img class="inst-logo" src={abs(inst.logo_url)} alt={inst.name} />
          </div>
        {:else}
          <h1>{inst.name}</h1>
        {/if}
        <p class="desc">{inst.description || b.tagline}</p>
      </section>

      <section class="actions">
        <PlayButton {inst} />
        <div class="row tools">
          <button class="glass tool" onclick={() => (app.instanceSettings = inst.id)} title="Instance settings"><SlidersHorizontal size={17} /></button>
          <button class="glass tool" class:on={app.consoleOpen} onclick={() => (app.consoleOpen = !app.consoleOpen)} title="Game console"><SquareTerminal size={17} /></button>
          {#if enabled(inst.experience, 'progression')}<button class="glass tool" onclick={() => (app.view = 'stats')} title="Player stats & leaderboards"><Trophy size={17} /></button>{/if}
          {#if mapServer && enabled(inst.experience, 'maps')}<button class="glass tool live" onclick={() => (mapOpen = true)} title="Open the live map"><MapIcon size={17} /> <span>Live Map</span></button>{/if}
          {#if inst.server && b.features.server_status}<ServerStatus server={inst.server} />{/if}
        </div>
      </section>
    {/key}
  {:else}
    <section class="hero empty">
      <h1>{b.name}</h1>
      <p class="desc">{instances().length === 0 ? "Nothing to play yet — your server admin hasn't published any instances you can access." : ''}</p>
    </section>
  {/if}

  <aside class="side">
    <ExperienceWidgets experience={inst?.experience} />
    {#if b.features.news && b.news.length}
      <div class="news-bar">
        <h4>News</h4>
        <button class="ghost sm" aria-pressed={app.settings?.show_news !== false} onclick={() => { if (app.settings) { app.settings.show_news = app.settings.show_news === false; saveSettings(); } }}>
          {#if app.settings?.show_news === false}<Eye size={13} /> Show{:else}<EyeOff size={13} /> Hide{/if}
        </button>
      </div>
      {#if app.settings?.show_news !== false}<NewsFeed news={b.news} />{/if}
    {/if}
    {#if b.links.length}
      <div class="links">
        {#each b.links as l}
          {@const Icon = linkIcon[l.icon] ?? Link}
          <button class="glass link" onclick={() => openUrl(l.url)} title={l.label}><Icon size={16} /> <span>{l.label}</span></button>
        {/each}
      </div>
    {/if}
  </aside>

  {#if app.consoleOpen}<Console />{/if}
  {#if mapOpen && mapServer}<MapModal serverId={mapServer.id} serverName={mapServer.name} instanceId={inst?.id ?? ''} onclose={() => (mapOpen = false)} />{/if}
</div>

<style>
  .home { position: relative; height: 100%; display: grid; grid-template-columns: 1fr minmax(17rem, 22rem); grid-template-rows: 1fr auto; gap: 1.5rem 2.5rem; padding: 2rem 2rem 2rem 2.5rem; }
  .hero { grid-column: 1; grid-row: 1; align-self: end; display: flex; flex-direction: column; gap: 0.75rem; animation: fade 0.3s ease; max-width: 38rem; padding-bottom: 0.5rem; }
  .greet { font-size: 0.88rem; }
  .greet strong { color: var(--text); font-weight: 560; }
  .chips { display: flex; gap: 0.45rem; flex-wrap: wrap; }
  .logo-wrap { max-width: 100%; display: flex; align-items: flex-end; min-height: 3rem; margin: 0.25rem 0; }
  .inst-logo { max-height: clamp(3.8rem, 8vw, 6.2rem); max-width: min(100%, 30rem); object-fit: contain; object-position: left bottom; filter: drop-shadow(0 4px 16px rgba(0, 0, 0, 0.5)); }
  h1 { font-size: clamp(2.2rem, 4.4vw, 3.2rem); line-height: 1.05; font-weight: 700; letter-spacing: -0.025em; }
  .desc { font-size: 0.95rem; line-height: 1.6; color: var(--muted); max-width: 32rem; }
  .actions { grid-column: 1; grid-row: 2; display: flex; flex-direction: column; gap: 0.8rem; animation: fade 0.3s ease both; }
  .tools { flex-wrap: wrap; gap: 0.6rem; }
  .tool { width: 2.75rem; height: 2.75rem; padding: 0; border-radius: var(--radius); color: color-mix(in srgb, var(--text) 75%, transparent); background: var(--surface); }
  .tool.live { width: auto; padding: 0 1rem; display: inline-flex; align-items: center; gap: 0.5rem; font-size: 0.85rem; font-weight: 560; color: var(--text); border-color: color-mix(in srgb, var(--accent) 45%, transparent); background: color-mix(in srgb, var(--accent) 14%, var(--surface)); }
  .tool.live:hover { background: color-mix(in srgb, var(--accent) 24%, var(--surface)); }
  .tool.on { color: var(--text); border-color: var(--line-strong); background: color-mix(in srgb, var(--text) 8%, var(--surface)); }
  .side { grid-column: 2; grid-row: 1 / 3; display: flex; flex-direction: column; justify-content: space-between; gap: 1rem; min-height: 0; animation: fade 0.3s ease both; }
  .links { display: flex; flex-wrap: wrap; gap: 0.45rem; }
  .link { padding: 0.5rem 0.75rem; font-size: 0.82rem; border-radius: var(--radius-sm); flex: 1 1 auto; background: var(--surface); color: color-mix(in srgb, var(--text) 80%, transparent); }
  .link:hover { border-color: var(--line-strong); color: var(--text); }
  @media (max-width: 1060px) { .home { grid-template-columns: 1fr 16rem; } .link span { display: none; } }
  .news-bar { display: flex; align-items: center; justify-content: space-between; }
  .news-bar h4 { font-size: 0.75rem; color: var(--muted); font-weight: 560; padding-left: 0.1rem; }
  .news-bar button { display: inline-flex; align-items: center; gap: 0.3rem; }
  .home.quiet { grid-template-columns: 1fr; grid-template-rows: 1fr auto auto; }
  .quiet .side { grid-column: 1; grid-row: 3; flex-direction: row; align-items: center; justify-content: flex-start; flex-wrap: wrap; }
</style>
