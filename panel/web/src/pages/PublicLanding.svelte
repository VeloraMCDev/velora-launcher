<script lang="ts">
  import {
    Download, Server, Users, Trophy, Sparkles, HelpCircle, Newspaper,
    Copy, Check, ExternalLink, ChevronDown, ChevronUp, Swords, Flame,
    Pickaxe, Clock, Box, Shield, Zap, ArrowRight, LogIn, Crown, MessageSquare,
  } from '@lucide/svelte';
  import { get, formatBytes } from '../lib/api';
  import { session } from '../lib/session.svelte';
  import LandingMap from '../components/LandingMap.svelte';
  import PlatformIcon from '../components/PlatformIcon.svelte';
  import McItem from '../components/McItem.svelte';
  import McIcon from '../components/McIcon.svelte';
  import McMeter from '../components/McMeter.svelte';
  import { mc, blockUrl } from '../lib/mc.svelte';
  import type { LandingConfig, HostedDownload, BlockType, FaqItem } from '../lib/types';

  let config = $state<LandingConfig | null>(null);
  let publicServers = $state<Array<{ id: number; name: string; online: boolean; players_online: number; players_max: number; software?: string; mc_version?: string }>>([]);
  let leaderboard = $state<Array<{ uuid: string; name: string; playtime_secs: number; player_kills: number; mob_kills: number; blocks_broken: number; blocks_placed: number }>>([]);
  let globalLevels = $state<Array<{ rank: number; uuid: string; username: string; global_level: number; global_xp: number; title: string | null }>>([]);
  let publicGuilds = $state<Array<{ id: string; name: string; tag: string; description: string; member_count: number; claims_count: number; icon_url: string | null }>>([]);
  let instances = $state<Array<{ id: string; name: string; description: string; mc_version: string; loader: string; file_count: number; logo_url: string | null; icon_url: string | null }>>([]);
  let news = $state<Array<{ id: string; title: string; body: string; date: string | null; tag: string | null }>>([]);
  let totalPlaytime = $state(0);
  let totalPlayers = $state(0);
  let lbSort = $state('playtime_secs');

  let copied = $state(false);
  let mapGone = $state(false);
  const PLATFORM_NAMES: Record<string, string> = { windows: 'Windows', mac: 'macOS', linux: 'Linux', android: 'Android', ios: 'iOS' };
  const PLATFORM_NOTES: Record<string, string> = { windows: 'Windows 10 and 11', mac: 'macOS 11 or newer', linux: 'AppImage and .deb', android: 'Phone app · APK', ios: 'Phone app · via AltStore' };
  let openFaq = $state<Record<string, boolean>>({});
  const isPreview = new URLSearchParams(location.search).has('landingPreview');

  function previewMessage(event: MessageEvent) {
    if (!isPreview || event.origin !== location.origin || event.data?.type !== 'scopenet-landing-preview') return;
    config = event.data.config as LandingConfig;
  }

  // A tiled deepslate wall stands in for the hero background when no image or video is configured and textures are installed.
  const heroPano = $derived(config && !mediaUrl(config.hero_bg_url) ? blockUrl('deepslate_bricks') : null);

  function safeLink(value: string | undefined | null): string {
    if (!value) return '#';
    return /^(https?:\/\/|mailto:|\/|#)/i.test(value) ? value : '#';
  }

  function mediaUrl(value: string | undefined | null): string {
    if (!value) return '';
    return /^(https?:\/\/|\/)/i.test(value) ? value : '';
  }

  function videoEmbed(value: string): string | null {
    try {
      const url = new URL(value);
      const id = url.hostname === 'youtu.be' ? url.pathname.slice(1) : url.hostname.endsWith('youtube.com') ? url.searchParams.get('v') : null;
      return id && /^[\w-]{11}$/.test(id) ? `https://www.youtube-nocookie.com/embed/${id}` : null;
    } catch { return null; }
  }

  function sectionId(block: { id: string; options?: Record<string, any> }): string {
    const anchor = String(block.options?.anchor ?? '').replace(/[^a-zA-Z0-9_-]/g, '');
    return anchor || block.id;
  }

  function firstSection(type: BlockType): string | null {
    const block = config?.blocks.find((b) => b.enabled && b.type === type);
    return block ? sectionId(block) : null;
  }

  function sectionBackground(block: { type?: string; options?: Record<string, any> }): string {
    const kind = block.options?.background;
    if (kind === 'surface') return 'var(--landing-surface)';
    if (kind === 'accent') return 'color-mix(in srgb, var(--landing-accent) 24%, var(--landing-bg))';
    if (kind === 'gradient') return 'linear-gradient(135deg, var(--landing-bg), var(--landing-surface))';
    const image = mediaUrl(block.options?.background_image);
    if (kind === 'image' && image && !/["'()\\]/.test(image)) return `linear-gradient(#0008,#0008),url("${image}") center/cover`;
    if (block.type === 'hero' && (!kind || kind === 'default')) {
      const heroImage = mediaUrl(config?.hero_bg_url);
      if (config?.hero_bg_type === 'image' && heroImage && !/["'()\\]/.test(heroImage)) return `linear-gradient(#0008,#0008),url("${heroImage}") center/cover`;
      return 'transparent';
    }
    return 'transparent';
  }

  function color(value: string | undefined, fallback: string): string {
    return value && /^#[0-9a-fA-F]{6}$/.test(value) ? value : fallback;
  }

  function font(value: string | undefined): string {
    return ['Inter', 'Outfit', 'Space Grotesk', 'Plus Jakarta Sans', 'system-ui'].includes(value ?? '') ? value! : 'Inter';
  }

  // Detect user OS
  const userAgent = typeof navigator !== 'undefined' ? navigator.userAgent.toLowerCase() : '';
  const detectedOS = /iphone|ipad|ipod/.test(userAgent) ? 'ios' : userAgent.includes('android') ? 'android' : userAgent.includes('mac') ? 'mac' : userAgent.includes('linux') ? 'linux' : 'windows';

  function scrollToSection(id: string) {
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth' });
  }

  function copyIp(ip: string) {
    if (!ip) return;
    navigator.clipboard.writeText(ip);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  function formatTime(secs: number): string {
    if (!secs) return '0h';
    const hours = Math.floor(secs / 3600);
    const mins = Math.floor((secs % 3600) / 60);
    if (hours > 0) return `${hours}h ${mins}m`;
    return `${mins}m`;
  }

  function formatNumber(num: number): string {
    return (num ?? 0).toLocaleString();
  }

  $effect(() => {
    // 1. Load Landing Config
    get<LandingConfig>('/api/v1/landing')
      .then((cfg) => {
        config = cfg;
      })
      .catch(() => {});

    // 2. Load Public Servers
    get<{ servers: typeof publicServers }>('/api/v1/servers/public')
      .then((res) => (publicServers = res.servers || []))
      .catch(() => {});

    // 3. Load Leaderboard
    get<{ leaderboard: typeof leaderboard; totals: { players: number; playtime_secs: number } }>(`/api/v1/leaderboard?sort=${lbSort}`)
      .then((res) => {
        leaderboard = res.leaderboard || [];
        totalPlayers = res.totals?.players || 0;
        totalPlaytime = res.totals?.playtime_secs || 0;
      })
      .catch(() => {});

    // 4. Load Manifest (instances & news)
    get<any>('/api/v1/launcher/manifest')
      .then((res) => {
        instances = res.instances || [];
        news = res.branding?.news || [];
      })
      .catch(() => {});

    // 5. Load Global Levels & Factions
    get<any[]>('/api/v1/levels/leaderboard')
      .then((res) => (globalLevels = res || []))
      .catch(() => {});

    get<any[]>('/api/v1/guilds')
      .then((res) => (publicGuilds = (res || []).slice(0, 6)))
      .catch(() => {});
  });

  $effect(() => {
    if (!config?.custom_css) return;
    const style = document.createElement('style');
    style.textContent = config.custom_css;
    document.head.appendChild(style);
    return () => style.remove();
  });

  $effect(() => {
    void lbSort;
    get<{ leaderboard: typeof leaderboard }>(`/api/v1/leaderboard?sort=${lbSort}`)
      .then((res) => (leaderboard = res.leaderboard || []))
      .catch(() => {});
  });

  const primaryDownload = $derived(
    config?.hosted_downloads.find((d) => d.platform === detectedOS) ??
    config?.hosted_downloads[0] ??
    null
  );

  const desktop = $derived((config?.hosted_downloads ?? []).filter((d) => ['windows', 'mac', 'linux'].includes(d.platform)));
  const phone = $derived((config?.hosted_downloads ?? []).filter((d) => ['android', 'ios'].includes(d.platform)));
  const haveDesktop = $derived(desktop.map((d) => d.platform as string));
  const totalOnline = $derived(publicServers.reduce((sum, s) => sum + (s.online ? s.players_online : 0), 0));
  const top3 = $derived(leaderboard.slice(0, 3));
  const restLb = $derived(leaderboard.slice(3, 10));
</script>

<svelte:window onmessage={previewMessage} />

<svelte:head>
  <title>{config?.brand_name ?? 'Velora'} · Official Server & Launcher</title>
</svelte:head>

<div class="landing-wrap" style={`--landing-bg:${color(config?.theme?.background, '#090a0f')};--landing-surface:${color(config?.theme?.surface, '#151620')};--landing-accent:${color(config?.theme?.accent, '#6d6af5')};--landing-text:${color(config?.theme?.text, '#f1f2f6')};--landing-muted:${color(config?.theme?.muted, '#8b8f9a')};--landing-width:${Math.min(1800, Math.max(720, config?.theme?.max_width ?? 1140))}px;--landing-radius:${Math.min(32, Math.max(0, config?.theme?.radius ?? 14))}px;--landing-font:${font(config?.theme?.font)}` }>
  <!-- Navigation Header -->
  <header class="navbar">
    <div class="nav-container">
      <div class="brand-group">
        {#if config?.logo_url}
          <img src={config.logo_url} alt="" class="brand-img" />
        {:else if !config?.brand_name || config.brand_name === 'Velora'}
          <img src="/velora-wordmark.png" alt="Velora" class="velora-wordmark" />
        {:else}
          <img src="/favicon.svg" alt="" class="brand-img" />
        {/if}
        {#if config?.logo_url || (config?.brand_name && config.brand_name !== 'Velora')}<strong class="brand-title">{config?.brand_name ?? 'Velora'}</strong>{/if}
      </div>

      <nav class="nav-links">
        {#if firstSection('servers')}<button type="button" class="nav-link-btn" onclick={() => scrollToSection(firstSection('servers')!)}>Servers</button>{/if}
        {#if firstSection('leaderboard')}<button type="button" class="nav-link-btn" onclick={() => scrollToSection(firstSection('leaderboard')!)}>Leaderboard</button>{/if}
        {#if firstSection('download')}<button type="button" class="nav-link-btn" onclick={() => scrollToSection(firstSection('download')!)}>Downloads</button>{/if}
        {#if firstSection('faq')}<button type="button" class="nav-link-btn" onclick={() => scrollToSection(firstSection('faq')!)}>FAQ</button>{/if}
      </nav>

      <div class="nav-actions">
        {#if config?.server_ip}
          <button class="ip-pill" onclick={() => copyIp(config?.server_ip ?? '')} title="Click to copy IP">
            <span class="pulse-dot"></span>
            <span class="ip-text">{config.server_ip}</span>
            <span class="copy-badge">
              {#if copied}<Check size={13} /> Copied!{:else}<Copy size={13} /> Copy{/if}
            </span>
          </button>
        {/if}

        {#if session.user}
          <a href="#/play" class="nav-login-btn">
            Player panel &rarr;
          </a>
          {#if session.user.role === 'admin'}<a href="#/dashboard" class="nav-login-btn">Admin</a>{/if}
        {:else}
          <a href="#/login" class="nav-login-btn">
            <LogIn size={14} /> Sign in
          </a>
        {/if}
      </div>
    </div>
  </header>

  <!-- Modular Blocks -->
  <main class="blocks">
    {#if config}
      {#each config.blocks.filter((b) => b.enabled) as block (block.id)}
        <div class="section-shell" style={`--section-bg:${sectionBackground(block)};--section-width:${block.options?.width === 'full' ? '100%' : block.options?.width === 'wide' ? 'min(96vw, 1600px)' : block.options?.width === 'narrow' ? 'min(92vw, 820px)' : 'var(--landing-width)'};--section-padding:${block.options?.spacing === 'none' ? '0px' : block.options?.spacing === 'compact' ? '36px' : block.options?.spacing === 'roomy' ? '120px' : '64px'}` }>
        {#if block.type === 'hero'}
          <!-- Hero Section -->
          <section class="hero-block" id={sectionId(block)}>
            {#if config.hero_bg_type === 'video' && mediaUrl(config.hero_bg_url) && (!block.options?.background || block.options.background === 'default')}<video class="hero-video" src={mediaUrl(config.hero_bg_url)} autoplay muted loop playsinline aria-hidden="true"></video>{/if}
            {#if heroPano}<div class="hero-pano" style:background-image="url({heroPano})" aria-hidden="true"></div>{/if}
            <div class="hero-bg-glow"></div>
            <div class="hero-orb orb-a" aria-hidden="true"></div><div class="hero-orb orb-b" aria-hidden="true"></div><div class="hero-grid" aria-hidden="true"></div>
            <div class="container hero-content">
              <div class="hero-badge">
                <span class="hero-badge-dot"></span>
                <span>{block.options?.badge_text || `${totalOnline} Players Online Now`}</span>
              </div>

              <h1 class="hero-title">{block.options?.headline || config.hero_title}</h1>
              <p class="hero-sub">{block.options?.subtitle || config.hero_subtitle}</p>

              <div class="hero-cta-group">
                {#if block.options?.cta_url}
                  <a href={safeLink(block.options.cta_url)} target="_blank" rel="noopener noreferrer" class="btn-hero-primary">
                    <Download size={20} />
                    <div class="cta-text-group">
                      <span class="cta-main">{block.options?.cta_text || config.hero_cta_text}</span>
                    </div>
                  </a>
                {:else if primaryDownload}
                  <a href={primaryDownload.file_url} class="btn-hero-primary" download>
                    <Download size={20} />
                    <div class="cta-text-group">
                      <span class="cta-main">{block.options?.cta_text || config.hero_cta_text}</span>
                      <span class="cta-sub">for {primaryDownload.label} ({formatBytes(primaryDownload.size)})</span>
                    </div>
                  </a>
                {:else if config.external_download_url}
                  <a href={safeLink(config.external_download_url)} target="_blank" rel="noopener noreferrer" class="btn-hero-primary">
                    <Download size={20} />
                    <div class="cta-text-group">
                      <span class="cta-main">{block.options?.cta_text || config.hero_cta_text}</span>
                      <span class="cta-sub">Download Latest Release</span>
                    </div>
                  </a>
                {:else}
                  <button type="button" class="btn-hero-primary" onclick={() => firstSection('download') && scrollToSection(firstSection('download')!)}>
                    <Download size={20} />
                    <span class="cta-main">{block.options?.cta_text || config.hero_cta_text}</span>
                  </button>
                {/if}

                {#if block.options?.show_ip_copy !== false && config.server_ip}
                  <button class="btn-hero-secondary" onclick={() => copyIp(config?.server_ip ?? '')}>
                    {#if copied}<Check size={18} /> Copied Server IP{:else}<Copy size={18} /> Copy Server IP{/if}
                  </button>
                {/if}
              </div>
              {#if block.options?.show_stats !== false && (totalOnline || publicServers.length || totalPlayers)}
                <div class="hero-stats">
                  <div>{#if mc.ready}<div class="stat-ic"><McItem id="armor_stand" size={28} /></div>{/if}<strong>{formatNumber(totalOnline)}</strong><span>Online now</span></div>
                  <div>{#if mc.ready}<div class="stat-ic"><McItem id="grass_block" size={28} /></div>{/if}<strong>{formatNumber(publicServers.length)}</strong><span>{publicServers.length === 1 ? 'Server' : 'Servers'}</span></div>
                  <div>{#if mc.ready}<div class="stat-ic"><McItem id="name_tag" size={28} /></div>{/if}<strong>{formatNumber(totalPlayers)}</strong><span>Players</span></div>
                </div>
              {/if}
              {#if mediaUrl(block.options?.image_url)}<img class="hero-image" src={mediaUrl(block.options?.image_url)} alt="" />{/if}
            </div>
          </section>

        {:else if block.type === 'download'}
          <!-- Downloads Section -->
          <section class="section" id={sectionId(block)}>
            <div class="container">
              <div class="section-head">
                <h2>{block.title || 'Get the Launcher'}</h2>
                <p class="muted">{block.subtitle || 'Custom modpack synchronisation, fast Java auto-updates, and HD skin wardrobe.'}</p>
                {#if block.options?.badge_note}
                  <div class="badge-note-pill">{block.options.badge_note}</div>
                {/if}
              </div>

              <div class="downloads-cards-grid" class:compact-list={block.options?.layout === 'compact'}>
                {#each desktop as d (d.file_url)}
                  <div class="download-card" class:recommended={d.platform === detectedOS && desktop.find((x) => x.platform === d.platform) === d}>
                    {#if d.platform === detectedOS && desktop.find((x) => x.platform === d.platform) === d}<span class="recommended-badge">For your computer</span>{/if}
                    <div class="download-card-icon"><PlatformIcon platform={d.platform} size={30} /></div>
                    <h3>{PLATFORM_NAMES[d.platform] ?? d.label}{#if config.hosted_downloads.filter((x) => x.platform === d.platform).length > 1}<small> · {d.filename.toLowerCase().endsWith('.deb') ? '.deb' : d.filename.toLowerCase().endsWith('.appimage') ? 'AppImage' : d.filename.split('.').pop()}</small>{/if}</h3>
                    <p class="dl-note">{d.filename.toLowerCase().endsWith('.deb') ? 'Debian and Ubuntu' : d.filename.toLowerCase().endsWith('.appimage') ? 'Any distro, no install needed' : (PLATFORM_NOTES[d.platform] ?? '')}</p>
                    <div class="dl-meta">{#if d.version}<span class="ver">v{d.version}</span>{/if}<span class="fname" title={d.filename}>{d.display_name || d.filename}</span><span>{formatBytes(d.size)}</span></div>
                    <a href={d.file_url} class="btn-download" download>Download <ArrowRight size={15} /></a>
                  </div>
                {:else}
                  {#if config.external_download_url}
                    <div class="download-card full-span">
                      <h3>Download {config.brand_name} Launcher</h3>
                      <p class="muted">Get the latest launcher package directly from our release repository.</p>
                      <a href={safeLink(config.external_download_url)} target="_blank" rel="noopener noreferrer" class="btn-download">
                        Download from Repository <ExternalLink size={15} />
                      </a>
                    </div>
                  {:else}
                    <p class="muted center">No downloads currently configured. Check back soon!</p>
                  {/if}
                {/each}
                {#if desktop.length}
                  {#each [['windows', 'Windows'], ['mac', 'macOS'], ['linux', 'Linux']].filter(([id]) => !haveDesktop.includes(id)) as [id, name]}
                    <div class="download-card soon" class:recommended={id === detectedOS}>
                      <div class="download-card-icon"><PlatformIcon platform={id} size={30} /></div>
                      <h3>{name}</h3>
                      <p class="dl-note">Not available yet</p>
                      <span class="btn-download" aria-disabled="true">Coming soon</span>
                    </div>
                  {/each}
                {/if}
              </div>
              {#if phone.length}
                <h3 class="dl-group">Take it with you</h3>
                <div class="downloads-cards-grid phones">
                  {#each phone as d (d.file_url)}
                    <div class="download-card">
                      <div class="download-card-icon"><PlatformIcon platform={d.platform} size={30} /></div>
                      <h3>{PLATFORM_NAMES[d.platform] ?? d.label}</h3>
                      <p class="dl-note">{PLATFORM_NOTES[d.platform] ?? ''}</p>
                      <div class="dl-meta">{#if d.version}<span class="ver">v{d.version}</span>{/if}<span class="fname" title={d.filename}>{d.display_name || d.filename}</span><span>{formatBytes(d.size)}</span></div>
                      <a href={d.file_url} class="btn-download" download>Download <ArrowRight size={15} /></a>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </section>

        {:else if block.type === 'servers'}
          <!-- Servers Section -->
          <section class="section alt-bg" id={sectionId(block)}>
            <div class="container">
              <div class="section-head">
                <h2>{block.title || 'Live Game Servers'}</h2>
                <p class="muted">{block.subtitle || 'Connect to our high-performance Minecraft worlds.'}</p>
              </div>

              {#if publicServers.filter((s) => (block.options?.show_offline === false ? s.online : true)).length}
                <div class="servers-grid">
                  {#each publicServers.filter((s) => (block.options?.show_offline === false ? s.online : true)) as s}
                    <div class="server-card">
                      <div class="server-card-top">
                        <div class="server-status-pill" class:online={s.online}>
                          <span class="dot"></span>
                          <span>{s.online ? `${s.players_online} / ${s.players_max} Online` : 'Offline'}</span>
                        </div>
                        {#if block.options?.show_badges !== false && s.mc_version}
                          <span class="mc-ver-tag tiny">{s.mc_version}</span>
                        {/if}
                      </div>

                      <h3 class="server-card-title">{s.name}</h3>
                      {#if s.online && s.players_max}<McMeter value={s.players_online} max={s.players_max} />{/if}
                      <div class="server-card-bottom">
                        <button class="server-copy-btn" onclick={() => copyIp(config?.server_ip ?? '')}>
                          <span>{block.options?.cta_label || config?.server_ip || 'play.server.com'}</span>
                          <Copy size={13} />
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="empty-state-box">
                  <Server size={32} class="muted" />
                  <p class="muted">No public servers listed yet.</p>
                </div>
              {/if}
            </div>
          </section>

        {:else if block.type === 'leaderboard'}
          <!-- Leaderboard Section -->
          <section class="section" id={sectionId(block)}>
            <div class="container">
              <div class="section-head">
                <h2>{block.title || 'Community Leaderboards'}</h2>
                <p class="muted">{block.subtitle || 'Top players and champions across our Minecraft network.'}</p>
              </div>

              <div class="sort-chips-bar">
                {#each [
                  { id: 'playtime_secs', label: 'Playtime', icon: Clock, item: 'clock' },
                  { id: 'global_level', label: 'Global Level', icon: Crown, item: 'experience_bottle' },
                  { id: 'player_kills', label: 'Player Kills', icon: Swords, item: 'diamond_sword' },
                  { id: 'mob_kills', label: 'Mob Kills', icon: Flame, item: 'bone' },
                  { id: 'blocks_broken', label: 'Blocks Mined', icon: Pickaxe, item: 'diamond_pickaxe' },
                ] as opt}
                  <button class="sort-chip" class:active={lbSort === opt.id} onclick={() => (lbSort = opt.id)}>
                    <McIcon item={opt.item} fallback={opt.icon} size={16} /> {opt.label}
                  </button>
                {/each}
              </div>

              {#if lbSort === 'global_level' && globalLevels.length}
                <div class="lb-table-wrap">
                  <div class="lb-table-head">
                    <span>Rank</span>
                    <span>Player</span>
                    <span class="text-right">Global Level</span>
                    <span class="text-right">Total XP</span>
                    <span class="text-right">Title</span>
                  </div>
                  {#each globalLevels.slice(0, 10) as row (row.uuid)}
                    <div class="lb-table-row">
                      <span class="rank-num">#{row.rank}</span>
                      <strong class="player-name">{row.username}</strong>
                      <span class="score-highlight text-right">Lvl {row.global_level}</span>
                      <span class="muted text-right">{row.global_xp.toLocaleString()} XP</span>
                      <span class="muted text-right">{row.title || '—'}</span>
                    </div>
                  {/each}
                </div>
              {:else if leaderboard.length}
                <!-- Podium -->
                {#if top3.length > 0}
                  <div class="podium-row">
                    {#if top3[1]}
                      <div class="podium-item silver">
                        <span class="medal"><McItem id="iron_ingot" size={32} emoji="🥈" /></span>
                        <strong>{top3[1].name}</strong>
                        <span class="podium-val">
                          {lbSort === 'playtime_secs' ? formatTime(top3[1].playtime_secs) : formatNumber(top3[1][lbSort as keyof typeof top3[1]] as number)}
                        </span>
                      </div>
                    {/if}
                    {#if top3[0]}
                      <div class="podium-item gold">
                        <span class="medal"><McItem id="gold_ingot" size={40} emoji="👑" /></span>
                        <strong>{top3[0].name}</strong>
                        <span class="podium-val">
                          {lbSort === 'playtime_secs' ? formatTime(top3[0].playtime_secs) : formatNumber(top3[0][lbSort as keyof typeof top3[0]] as number)}
                        </span>
                      </div>
                    {/if}
                    {#if top3[2]}
                      <div class="podium-item bronze">
                        <span class="medal"><McItem id="copper_ingot" size={32} emoji="🥉" /></span>
                        <strong>{top3[2].name}</strong>
                        <span class="podium-val">
                          {lbSort === 'playtime_secs' ? formatTime(top3[2].playtime_secs) : formatNumber(top3[2][lbSort as keyof typeof top3[2]] as number)}
                        </span>
                      </div>
                    {/if}
                  </div>
                {/if}

                <!-- Leaderboard Table -->
                <div class="lb-table-wrap">
                  <div class="lb-table-head">
                    <span>Rank</span>
                    <span>Player</span>
                    <span class="text-right">Score</span>
                    <span class="text-right">Playtime</span>
                    <span class="text-right">Kills</span>
                  </div>
                  {#each restLb as row, i (row.uuid)}
                    <div class="lb-table-row">
                      <span class="rank-num">#{i + 4}</span>
                      <strong class="player-name">{row.name}</strong>
                      <span class="score-highlight text-right">
                        {lbSort === 'playtime_secs' ? formatTime(row.playtime_secs) : formatNumber(row[lbSort as keyof typeof row] as number)}
                      </span>
                      <span class="muted text-right">{formatTime(row.playtime_secs)}</span>
                      <span class="muted text-right">{row.player_kills}</span>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="empty-state-box">
                  <Trophy size={32} class="muted" />
                  <p class="muted">No leaderboard data recorded yet.</p>
                </div>
              {/if}

              {#if publicGuilds.length > 0}
                <div class="guilds-showcase" style="margin-top: 48px;">
                  <div class="section-head" style="margin-bottom: 20px;">
                    <h3>Top Factions & Claimed Territories</h3>
                    <p class="muted">Form parties, recruit allies, and protect your faction's chunks.</p>
                  </div>
                  <div class="guilds-grid" style="display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px;">
                    {#each publicGuilds as g}
                      <div class="guild-card" style="background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 16px; display: flex; align-items: center; gap: 14px;">
                        {#if g.icon_url}
                          <img src={g.icon_url} alt="" style="width: 44px; height: 44px; border-radius: 8px; object-fit: cover;" />
                        {:else}
                          <div style="width: 44px; height: 44px; border-radius: 8px; background: rgba(99,102,241,0.15); display: flex; align-items: center; justify-content: center; color: var(--accent);">
                            <McIcon item="shield" fallback={Shield} size={32} />
                          </div>
                        {/if}
                        <div style="flex: 1; min-width: 0;">
                          <div style="display: flex; align-items: center; gap: 6px;">
                            <strong style="font-size: 0.95rem;">{g.name}</strong>
                            <span style="font-size: 0.72rem; font-family: monospace; font-weight: 700; color: #a5b4fc; background: rgba(99,102,241,0.1); padding: 1px 5px; border-radius: 4px;">[{g.tag}]</span>
                          </div>
                          <div style="display: flex; align-items: center; gap: 12px; font-size: 0.78rem; color: var(--muted); margin-top: 4px;">
                            <span><Users size={12} style="display: inline; vertical-align: -1px;" /> {g.member_count} members</span>
                            <span style="color: #4ade80;"><Box size={12} style="display: inline; vertical-align: -1px;" /> {g.claims_count} chunks</span>
                          </div>
                        </div>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          </section>

        {:else if block.type === 'stats'}
          <!-- Stats Highlights Section -->
          <section class="section alt-bg" id={sectionId(block)}>
            <div class="container">
              <div class="section-head">
                <h2>{block.title || 'Network Highlights'}</h2>
                <p class="muted">{block.subtitle || 'Live statistics computed across all connected servers.'}</p>
              </div>

              <div class="stats-counter-grid">
                {#if block.options?.show_players !== false}
                  <div class="counter-card">
                    <span class="counter-num">{formatNumber(totalPlayers)}</span>
                    <span class="counter-label">Total Players</span>
                  </div>
                {/if}
                {#if block.options?.show_playtime !== false}
                  <div class="counter-card">
                    <span class="counter-num">{formatTime(totalPlaytime)}</span>
                    <span class="counter-label">Time Explored</span>
                  </div>
                {/if}
                {#if block.options?.show_guilds !== false && publicGuilds.length > 0}
                  <div class="counter-card">
                    <span class="counter-num">{publicGuilds.length}</span>
                    <span class="counter-label">Active Factions</span>
                  </div>
                {/if}
                <div class="counter-card">
                  <span class="counter-num">{publicServers.length}</span>
                  <span class="counter-label">Connected Servers</span>
                </div>
                {#if block.options?.custom_stat1_label && block.options?.custom_stat1_value}
                  <div class="counter-card">
                    <span class="counter-num">{block.options.custom_stat1_value}</span>
                    <span class="counter-label">{block.options.custom_stat1_label}</span>
                  </div>
                {/if}
              </div>
            </div>
          </section>

        {:else if block.type === 'instances'}
          <!-- Instances Showcase Section -->
          {#if instances.length}
            <section class="section" id={sectionId(block)}>
              <div class="container">
                <div class="section-head">
                  <h2>{block.title || 'Available Instances'}</h2>
                  <p class="muted">{block.subtitle || 'Optimized modpacks ready to play through our custom launcher.'}</p>
                </div>

                <div class="instances-cards-grid">
                  {#each instances.slice(0, Math.max(1, Number(block.options?.limit) || 6)) as inst}
                    <div class="inst-card">
                      <div class="inst-card-top">
                        <span class="badge mc-tag">{inst.mc_version}</span>
                        <span class="badge loader-tag">{inst.loader}</span>
                      </div>
                      <h3>{inst.name}</h3>
                      <p class="muted small">{inst.description || 'Pre-configured instance with automated mod updates.'}</p>
                      <div class="inst-card-footer tiny muted">
                        <span>{inst.file_count} mods & files</span>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            </section>
          {/if}

        {:else if block.type === 'news'}
          <!-- News Section -->
          {#if news.length}
            <section class="section alt-bg" id={sectionId(block)}>
              <div class="container">
                <div class="section-head">
                  <h2>{block.title || 'Latest News'}</h2>
                  <p class="muted">{block.subtitle || 'Community events, server updates, and patch notes.'}</p>
                </div>

                <div class="news-cards-grid">
                {#each news.slice(0, Math.max(1, Number(block.options?.limit) || 3)) as item}
                    <div class="news-card">
                      {#if item.tag}<span class="news-tag">{item.tag}</span>{/if}
                      <h3>{item.title}</h3>
                      <p class="muted small">{item.body}</p>
                      {#if item.date}<span class="tiny muted">{item.date}</span>{/if}
                    </div>
                  {/each}
                </div>
              </div>
            </section>
          {/if}

        {:else if block.type === 'faq'}
          <!-- FAQ Section -->
          <section class="section" id={sectionId(block)}>
            <div class="container max-w-narrow">
              <div class="section-head">
                <h2>{block.title || 'Frequently Asked Questions'}</h2>
                <p class="muted">{block.subtitle || 'Got questions? We have answers.'}</p>
              </div>

              <div class="faq-accordion">
                {#each config.faqs as faq}
                  {@const isOpen = !!openFaq[faq.id]}
                  <div class="faq-item" class:open={isOpen}>
                    <button class="faq-q-btn" onclick={() => (openFaq[faq.id] = !isOpen)}>
                      <span>{faq.question}</span>
                      {#if isOpen}<ChevronUp size={18} />{:else}<ChevronDown size={18} />{/if}
                    </button>
                    {#if isOpen}
                      <div class="faq-ans">
                        <p>{faq.answer}</p>
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          </section>

        {:else if block.type === 'socials'}
          <!-- Socials Block -->
          <section class="section" id={sectionId(block)}>
            <div class="container">
              <div class="section-head">
                <h2>{block.title || 'Join Our Community'}</h2>
                <p class="muted">{block.subtitle || 'Connect with players, join our Discord, and follow news.'}</p>
              </div>

              <div class="socials-buttons-grid">
                {#if block.options?.discord_url}
                  <a href={safeLink(block.options.discord_url)} target="_blank" rel="noopener noreferrer" class="social-btn discord">
                    <MessageSquare size={18} /> Join our Discord
                  </a>
                {/if}
                {#if block.options?.store_url}
                  <a href={safeLink(block.options.store_url)} target="_blank" rel="noopener noreferrer" class="social-btn store">
                    <Sparkles size={18} /> Visit Webstore
                  </a>
                {/if}
                {#if block.options?.video_url || block.options?.media_url}
                  <a href={safeLink(block.options.video_url || block.options.media_url)} target="_blank" rel="noreferrer" class="social-btn media">
                    <Flame size={18} /> YouTube / Twitch
                  </a>
                {/if}
                {#if block.options?.social_url}
                  <a href={safeLink(block.options.social_url)} target="_blank" rel="noopener noreferrer" class="social-btn twitter">
                    <ExternalLink size={18} /> Follow on Twitter / X
                  </a>
                {/if}
              </div>
            </div>
          </section>

        {:else if block.type === 'text'}
          <section class="section custom-text" id={sectionId(block)} style:text-align={block.options?.align === 'center' ? 'center' : 'left'}><div class="container">
            {#if block.options?.eyebrow}<span class="custom-eyebrow">{block.options.eyebrow}</span>{/if}
            <h2>{block.title}</h2>{#if block.subtitle}<p class="muted">{block.subtitle}</p>{/if}
            <p class="custom-body">{block.options?.body}</p>
          </div></section>

        {:else if block.type === 'image'}
          <section class="section" id={sectionId(block)}><div class="container custom-media">
            {#if block.title}<h2>{block.title}</h2>{/if}
            {#if mediaUrl(block.options?.url)}
              {#if block.options?.link}<a href={safeLink(block.options.link)} target="_blank" rel="noopener noreferrer"><img src={mediaUrl(block.options?.url)} alt={block.options?.alt || block.title || ''} style:object-fit={block.options?.fit === 'contain' ? 'contain' : 'cover'} /></a>
              {:else}<img src={mediaUrl(block.options?.url)} alt={block.options?.alt || block.title || ''} style:object-fit={block.options?.fit === 'contain' ? 'contain' : 'cover'} />{/if}
              {#if block.options?.caption}<p class="muted">{block.options.caption}</p>{/if}
            {/if}
          </div></section>

        {:else if block.type === 'split'}
          <section class="section" id={sectionId(block)}><div class="container custom-split" class:reverse={block.options?.image_side === 'left'}>
            <div><h2>{block.title}</h2>{#if block.subtitle}<p class="muted">{block.subtitle}</p>{/if}<p class="custom-body">{block.options?.body}</p>
              {#if block.options?.button_text && block.options?.button_url}<a class="custom-button" href={safeLink(block.options.button_url)}>{block.options.button_text}</a>{/if}
            </div>
            {#if mediaUrl(block.options?.image_url)}<img src={mediaUrl(block.options?.image_url)} alt={block.options?.image_alt || ''} />{/if}
          </div></section>

        {:else if block.type === 'features'}
          <section class="section" id={sectionId(block)}><div class="container"><div class="section-head"><h2>{block.title}</h2><p class="muted">{block.subtitle}</p></div>
            <div class="custom-grid" style:grid-template-columns={`repeat(${Math.min(4, Math.max(2, Number(block.options?.columns) || 3))}, minmax(0, 1fr))`}>
              {#each block.options?.items ?? [] as item}<article class="custom-card"><span class="custom-icon">{item.icon}</span><h3>{item.title}</h3><p class="muted">{item.body}</p></article>{/each}
            </div>
          </div></section>

        {:else if block.type === 'gallery'}
          <section class="section" id={sectionId(block)}><div class="container"><div class="section-head"><h2>{block.title}</h2><p class="muted">{block.subtitle}</p></div>
            <div class="custom-grid gallery-grid" style:grid-template-columns={`repeat(${Math.min(4, Math.max(2, Number(block.options?.columns) || 3))}, minmax(0, 1fr))`}>
              {#each block.options?.items ?? [] as item}{#if mediaUrl(item.url)}<figure><img src={mediaUrl(item.url)} alt={item.alt || item.caption || ''} loading="lazy" />{#if item.caption}<figcaption>{item.caption}</figcaption>{/if}</figure>{/if}{/each}
            </div>
          </div></section>

        {:else if block.type === 'cta'}
          <section class="section custom-cta" id={sectionId(block)}><div class="container"><h2>{block.title}</h2><p>{block.options?.body || block.subtitle}</p><div class="custom-actions">
            {#if block.options?.button_text && block.options?.button_url}<a class="custom-button" href={safeLink(block.options.button_url)}>{block.options.button_text}</a>{/if}
            {#if block.options?.secondary_text && block.options?.secondary_url}<a class="custom-button secondary" href={safeLink(block.options.secondary_url)}>{block.options.secondary_text}</a>{/if}
          </div></div></section>

        {:else if block.type === 'divider'}
          <div class="custom-divider" id={sectionId(block)} style:height={`${Math.min(240, Math.max(0, Number(block.options?.height) || 48))}px`} class:glow={block.options?.style === 'glow'}>{#if block.options?.style !== 'space'}<span></span>{/if}</div>

        {:else if block.type === 'testimonials'}
          <section class="section" id={sectionId(block)}><div class="container"><div class="section-head"><h2>{block.title}</h2><p class="muted">{block.subtitle}</p></div><div class="custom-grid" style:grid-template-columns={`repeat(${Math.min(3, Math.max(2, Number(block.options?.columns) || 3))}, minmax(0, 1fr))`}>
            {#each block.options?.items ?? [] as item}<blockquote class="custom-card"><p>“{item.quote}”</p><footer>{item.name}{#if item.role}<small> · {item.role}</small>{/if}</footer></blockquote>{/each}
          </div></div></section>

        {:else if block.type === 'video'}
          <section class="section" id={sectionId(block)}><div class="container custom-media"><div class="section-head"><h2>{block.title}</h2><p class="muted">{block.subtitle}</p></div>
            {#if videoEmbed(block.options?.url ?? '')}<iframe title={block.title || 'Video'} src={videoEmbed(block.options?.url ?? '')!} allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>
            {:else if mediaUrl(block.options?.url)}<video controls poster={mediaUrl(block.options?.poster)} src={mediaUrl(block.options?.url)}><track kind="captions" /></video>{/if}
            {#if block.options?.caption}<p class="muted">{block.options.caption}</p>{/if}
          </div></section>

        {:else if block.type === 'map'}
          <section class="section" id={sectionId(block)} hidden={mapGone}><div class="container"><div class="section-head"><h2>{block.title}</h2><p class="muted">{block.subtitle}</p></div>
            <LandingMap bind:unavailable={mapGone} serverId={Number(block.options?.server_id) || 0} height={Number(block.options?.height) || 560} />
          </div></section>

        {:else if block.type === 'buttons'}
          <section class="section" id={sectionId(block)}><div class="container"><div class="section-head"><h2>{block.title}</h2><p class="muted">{block.subtitle}</p></div><div class="custom-actions" class:align-left={block.options?.align === 'left'}>
            {#each block.options?.items ?? [] as item}{#if item.label && item.url}<a class="custom-button" class:secondary={item.style === 'secondary'} href={safeLink(item.url)}>{item.label}</a>{/if}{/each}
          </div></div></section>
        {/if}
        </div>
      {/each}
    {/if}
  </main>

  <!-- Footer -->
  <footer class="footer">
    <div class="container footer-content">
      <div class="footer-left">
        {#if config?.logo_url}
          <img src={config.logo_url} alt="" class="brand-img" />
        {/if}
        <strong>{config?.brand_name ?? 'Velora'}</strong>
        <p class="tiny muted">{config?.theme?.footer_text || config?.tagline || 'Your worlds, one click away.'}</p>
      </div>
      <div class="footer-right tiny muted">
        <span>Minecraft is a trademark of Mojang Studios. Not affiliated with Mojang or Microsoft.</span>
        <span>Powered by Velora.</span>
      </div>
    </div>
  </footer>
</div>

<style>
  .landing-wrap {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--landing-bg);
    color: var(--landing-text);
    font-family: var(--landing-font), sans-serif;
  }
  .container { max-width: var(--section-width, var(--landing-width)); margin: 0 auto; padding: 0 24px; width: 100%; box-sizing: border-box; }
  .section-shell { background: var(--section-bg); }
  .section-shell .section { padding: var(--section-padding) 0; }
  .section-shell .section.alt-bg { background: transparent; }
  .section-shell .hero-block { padding-top: var(--section-padding); padding-bottom: var(--section-padding); }
  .section-shell h2 { color: var(--landing-text); }
  .section-shell .muted { color: var(--landing-muted); }
  .section-shell .custom-text h2, .section-shell .custom-split h2, .section-shell .custom-cta h2, .section-shell .custom-media h2 { font-size: clamp(26px, 4vw, 42px); margin: 0 0 14px; }
  .custom-eyebrow { color: var(--landing-accent); text-transform: uppercase; letter-spacing: .12em; font-weight: 700; font-size: 12px; display: inline-block; margin-bottom: 12px; }
  .custom-body { white-space: pre-line; line-height: 1.75; font-size: 17px; max-width: 850px; margin: 20px auto 0 0; }
  .custom-text[style*="center"] .custom-body { margin-left: auto; margin-right: auto; }
  .custom-media { text-align: center; } .custom-media img { width: 100%; max-height: 700px; border-radius: var(--landing-radius); } .custom-media iframe, .custom-media video { width: 100%; aspect-ratio: 16/9; border: 0; border-radius: var(--landing-radius); background: #000; }
  .custom-split { display: grid; grid-template-columns: 1fr 1fr; gap: clamp(30px, 5vw, 90px); align-items: center; } .custom-split.reverse img { order: -1; } .custom-split img { width: 100%; border-radius: var(--landing-radius); object-fit: cover; max-height: 560px; }
  .custom-grid { display: grid; gap: 20px; } .custom-card { background: var(--landing-surface); border: 1px solid color-mix(in srgb, var(--landing-text) 12%, transparent); border-radius: var(--landing-radius); padding: 28px; margin: 0; }
  .custom-card h3 { margin: 12px 0 8px; font-size: 19px; } .custom-card p { line-height: 1.6; } .custom-card footer { margin-top: 18px; font-weight: 700; color: var(--landing-accent); } .custom-card footer small { color: var(--landing-muted); font-weight: 400; }
  .custom-icon { font-size: 30px; color: var(--landing-accent); } .gallery-grid figure { margin: 0; } .gallery-grid img { width: 100%; aspect-ratio: 4/3; object-fit: cover; border-radius: var(--landing-radius); } .gallery-grid figcaption { color: var(--landing-muted); font-size: 13px; padding-top: 8px; }
  .custom-cta { text-align: center; } .custom-cta p { color: var(--landing-muted); font-size: 18px; margin: 0 auto 24px; max-width: 680px; }
  .custom-actions { display: flex; gap: 12px; justify-content: center; flex-wrap: wrap; } .custom-actions.align-left { justify-content: flex-start; }
  .custom-button { display: inline-flex; align-items: center; justify-content: center; padding: 12px 22px; border-radius: var(--landing-radius); background: var(--landing-accent); color: #fff; font-weight: 700; text-decoration: none; }
  .custom-button.secondary { background: var(--landing-surface); color: var(--landing-text); border: 1px solid color-mix(in srgb, var(--landing-text) 20%, transparent); }
  .custom-divider { display: grid; place-items: center; } .custom-divider span { width: min(90%, var(--landing-width)); height: 1px; background: color-mix(in srgb, var(--landing-text) 18%, transparent); } .custom-divider.glow span { height: 2px; background: var(--landing-accent); box-shadow: 0 0 24px var(--landing-accent); }
  .max-w-narrow { max-width: 820px; }
  .navbar { position: sticky; top: 0; z-index: 100; backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); background: rgba(9, 10, 15, 0.82); border-bottom: 1px solid rgba(255, 255, 255, 0.08); }
  .nav-container { max-width: 1140px; margin: 0 auto; padding: 14px 24px; display: flex; align-items: center; justify-content: space-between; gap: 16px; }
  .brand-group { display: flex; align-items: center; gap: 10px; }
  .brand-img { width: 28px; height: 28px; border-radius: 6px; object-fit: contain; }
  .velora-wordmark { width: 108px; height: 54px; object-fit: contain; }
  .brand-title { font-size: 18px; font-weight: 700; letter-spacing: -0.01em; }
  .nav-links { display: flex; gap: 24px; }
  .nav-link-btn {
    background: transparent;
    border: none;
    color: var(--landing-muted);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    padding: 0;
    transition: color 0.15s;
    font-family: inherit;
  }
  .nav-link-btn:hover { color: var(--landing-text); }
  .nav-actions { display: flex; align-items: center; gap: 12px; }
  .ip-pill { display: inline-flex; align-items: center; gap: 8px; background: rgba(255, 255, 255, 0.06); border: 1px solid rgba(255, 255, 255, 0.12); padding: 6px 12px; border-radius: 99rem; color: var(--landing-text); cursor: pointer; font-size: 13px; font-weight: 560; font-family: monospace; transition: background 0.15s; }
  .ip-pill:hover { background: rgba(255, 255, 255, 0.1); }
  .pulse-dot { width: 7px; height: 7px; border-radius: 50%; background: #3fb97c; box-shadow: 0 0 8px #3fb97c; }
  .copy-badge { display: inline-flex; align-items: center; gap: 4px; font-size: 11px; color: var(--landing-muted); margin-left: 4px; }
  .nav-login-btn { display: inline-flex; align-items: center; gap: 6px; padding: 6px 14px; border-radius: 8px; background: transparent; border: 1px solid rgba(255, 255, 255, 0.14); color: var(--landing-muted); text-decoration: none; font-size: 13px; font-weight: 500; }
  .nav-login-btn:hover { color: var(--landing-text); border-color: rgba(255, 255, 255, 0.3); }
  .hero-block { position: relative; padding: 110px 0 90px; text-align: center; overflow: hidden; }
  .hero-video { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; opacity: .4; pointer-events: none; }
  .hero-bg-glow { position: absolute; top: -100px; left: 50%; transform: translateX(-50%); width: 700px; height: 450px; background: radial-gradient(circle, color-mix(in srgb, var(--landing-accent) 22%, transparent) 0%, transparent 70%); filter: blur(50px); pointer-events: none; }
  .hero-content { position: relative; z-index: 10; display: flex; flex-direction: column; align-items: center; gap: 20px; }
  .hero-image { width: min(100%, 1000px); max-height: 520px; object-fit: cover; border-radius: var(--landing-radius); box-shadow: 0 30px 80px #0008; margin-top: 28px; }
  .hero-badge { display: inline-flex; align-items: center; gap: 8px; padding: 6px 16px; border-radius: 99rem; background: color-mix(in srgb, var(--landing-accent) 12%, transparent); border: 1px solid color-mix(in srgb, var(--landing-accent) 35%, transparent); color: color-mix(in srgb, var(--landing-accent) 60%, white); font-size: 13px; font-weight: 600; }
  .hero-badge-dot { width: 6px; height: 6px; border-radius: 50%; background: #3fb97c; box-shadow: 0 0 6px #3fb97c; }
  .hero-title { font-size: clamp(32px, 5vw, 56px); font-weight: 800; line-height: 1.1; margin: 0; letter-spacing: -0.03em; max-width: 840px; }
  .hero-sub { font-size: 18px; line-height: 1.6; color: var(--landing-muted); max-width: 640px; margin: 0; }
  .hero-cta-group { display: flex; flex-wrap: wrap; gap: 14px; margin-top: 14px; justify-content: center; }
  .btn-hero-primary { display: inline-flex; align-items: center; gap: 12px; padding: 14px 28px; border-radius: var(--landing-radius); background: var(--landing-accent); color: white; text-decoration: none; font-weight: 650; box-shadow: 0 8px 30px color-mix(in srgb, var(--landing-accent) 40%, transparent); transition: transform 0.15s, background 0.15s; }
  .btn-hero-primary:hover { transform: translateY(-2px); background: color-mix(in srgb, var(--landing-accent) 84%, white); }
  .cta-text-group { display: flex; flex-direction: column; text-align: left; }
  .cta-main { font-size: 16px; line-height: 1.2; }
  .cta-sub { font-size: 11px; opacity: 0.85; font-weight: 400; }
  .btn-hero-secondary { display: inline-flex; align-items: center; gap: 10px; padding: 14px 24px; border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.06); border: 1px solid rgba(255, 255, 255, 0.15); color: var(--landing-text); font-size: 15px; font-weight: 600; cursor: pointer; transition: background 0.15s; }
  .btn-hero-secondary:hover { background: rgba(255, 255, 255, 0.1); }
  .section { padding: 80px 0; }
  .section.alt-bg { background: rgba(255, 255, 255, 0.02); border-top: 1px solid rgba(255, 255, 255, 0.05); border-bottom: 1px solid rgba(255, 255, 255, 0.05); }
  .section-head { text-align: center; margin-bottom: 44px; display: flex; flex-direction: column; gap: 8px; }
  .section-head h2 { font-size: 32px; font-weight: 700; margin: 0; letter-spacing: -0.02em; }
  .muted { color: var(--landing-muted); margin: 0; }
  .downloads-cards-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 20px; }
  .download-card { padding: 32px 24px; border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); display: flex; flex-direction: column; align-items: center; text-align: center; gap: 12px; position: relative; }
  .download-card.recommended { border-color: color-mix(in srgb, var(--landing-accent) 50%, transparent); background: color-mix(in srgb, var(--landing-accent) 6%, transparent); }
  .recommended-badge { position: absolute; top: 12px; font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; padding: 2px 10px; border-radius: 99rem; background: var(--landing-accent); color: white; }
  .download-card-icon { width: 56px; height: 56px; border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.06); display: grid; place-items: center; color: var(--landing-accent); margin-bottom: 4px; }
  .download-card h3 { margin: 0; font-size: 18px; font-weight: 650; }
  .btn-download { display: inline-flex; align-items: center; justify-content: center; gap: 8px; width: 100%; padding: 12px; border-radius: 10px; background: var(--landing-accent); color: white; text-decoration: none; font-weight: 600; font-size: 14px; margin-top: 8px; }
  .btn-download:hover { background: color-mix(in srgb, var(--landing-accent) 84%, white); }
  .servers-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 20px; }
  .server-card { padding: 24px; border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); display: flex; flex-direction: column; gap: 14px; }
  .server-card-top { display: flex; justify-content: space-between; align-items: center; }
  .server-status-pill { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; color: var(--landing-muted); }
  .server-status-pill .dot { width: 6px; height: 6px; border-radius: 50%; background: #e5484d; }
  .server-status-pill.online { color: #3fb97c; }
  .server-status-pill.online .dot { background: #3fb97c; box-shadow: 0 0 6px #3fb97c; }
  .mc-ver-tag { background: rgba(255, 255, 255, 0.06); padding: 2px 8px; border-radius: 6px; }
  .server-card-title { margin: 0; font-size: 20px; font-weight: 700; }
  .server-copy-btn { display: flex; justify-content: space-between; align-items: center; width: 100%; padding: 10px 14px; border-radius: 8px; background: rgba(0, 0, 0, 0.25); border: 1px solid rgba(255, 255, 255, 0.1); color: var(--landing-text); font-family: monospace; font-size: 13px; cursor: pointer; }
  .server-copy-btn:hover { border-color: var(--landing-accent); }
  .sort-chips-bar { display: flex; justify-content: center; flex-wrap: wrap; gap: 8px; margin-bottom: 28px; }
  .sort-chip { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: 99rem; background: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.09); color: var(--landing-muted); font-size: 13px; font-weight: 560; cursor: pointer; transition: all 0.15s; }
  .sort-chip.active { background: color-mix(in srgb, var(--landing-accent) 15%, transparent); border-color: var(--landing-accent); color: var(--landing-text); }
  .podium-row { display: grid; grid-template-columns: 1fr 1.2fr 1fr; gap: 16px; align-items: end; max-width: 580px; margin: 0 auto 32px; text-align: center; }
  .podium-item { padding: 24px 16px; border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.08); display: flex; flex-direction: column; gap: 6px; }
  .podium-item.gold { border-color: rgba(245, 158, 11, 0.4); background: rgba(245, 158, 11, 0.08); padding-bottom: 36px; }
  .podium-item .medal { font-size: 26px; }
  .podium-val { font-size: 18px; font-weight: 700; color: var(--landing-accent); }
  .lb-table-wrap { background: rgba(255, 255, 255, 0.02); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: var(--landing-radius); overflow: hidden; }
  .lb-table-head, .lb-table-row { display: grid; grid-template-columns: 70px 1fr 120px 120px 100px; padding: 14px 20px; align-items: center; font-size: 14px; }
  .lb-table-head { border-bottom: 1px solid rgba(255, 255, 255, 0.08); font-size: 12px; font-weight: 700; text-transform: uppercase; color: var(--landing-muted); }
  .lb-table-row { border-bottom: 1px solid rgba(255, 255, 255, 0.04); }
  .text-right { text-align: right; }
  .score-highlight { color: var(--landing-accent); font-weight: 700; }
  .stats-counter-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; }
  .counter-card { padding: 32px 20px; border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); text-align: center; display: flex; flex-direction: column; gap: 6px; }
  .counter-num { font-size: 38px; font-weight: 800; color: var(--landing-accent); letter-spacing: -0.02em; }
  .counter-label { font-size: 13px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--landing-muted); }
  .instances-cards-grid, .news-cards-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 20px; }
  .inst-card, .news-card { padding: 24px; border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); display: flex; flex-direction: column; gap: 10px; }
  .inst-card h3, .news-card h3 { margin: 0; font-size: 18px; font-weight: 700; }
  .badge { font-size: 11px; padding: 2px 8px; border-radius: 6px; background: rgba(255, 255, 255, 0.08); }
  .news-tag { align-self: flex-start; padding: 2px 8px; border-radius: 4px; background: color-mix(in srgb, var(--landing-accent) 20%, transparent); color: color-mix(in srgb, var(--landing-accent) 60%, white); font-size: 11px; font-weight: 600; text-transform: uppercase; }
  .faq-accordion { display: flex; flex-direction: column; gap: 12px; }
  .faq-item { border-radius: var(--landing-radius); background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); overflow: hidden; }
  .faq-q-btn { width: 100%; display: flex; justify-content: space-between; align-items: center; padding: 18px 20px; background: transparent; border: none; color: var(--landing-text); font-size: 16px; font-weight: 600; cursor: pointer; text-align: left; }
  .faq-ans { padding: 0 20px 18px; color: var(--landing-muted); line-height: 1.6; font-size: 14px; }
  .footer { border-top: 1px solid rgba(255, 255, 255, 0.08); padding: 40px 0; margin-top: auto; }
  .footer-content { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 16px; }
  .empty-state-box { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 40px; }
  .badge-note-pill { display: inline-flex; align-items: center; padding: 4px 12px; border-radius: 99rem; background: color-mix(in srgb, var(--landing-accent) 15%, transparent); color: color-mix(in srgb, var(--landing-accent) 60%, white); font-size: 13px; font-weight: 600; margin-top: 10px; }
  .downloads-cards-grid.compact-list { grid-template-columns: 1fr; }
  .socials-buttons-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px; margin-top: 24px; }
  .social-btn { display: flex; align-items: center; justify-content: center; gap: 10px; padding: 16px 20px; border-radius: var(--landing-radius); font-weight: 600; text-decoration: none; transition: transform 0.15s, opacity 0.15s; color: #fff; font-size: 15px; }
  .social-btn:hover { transform: translateY(-2px); opacity: 0.95; }
  .social-btn.discord { background: #5865F2; }
  .social-btn.store { background: #8b5cf6; }
  .social-btn.media { background: #ef4444; }
  .social-btn.twitter { background: #0284c7; }
  @media (max-width: 768px) {
    .custom-split { grid-template-columns: 1fr; } .custom-split.reverse img { order: 0; } .custom-grid { grid-template-columns: 1fr !important; }
    .nav-links { display: none; }
    .hero-title { font-size: 34px; }
    .hero-cta-group { flex-direction: column; width: 100%; }
    .btn-hero-primary, .btn-hero-secondary { width: 100%; justify-content: center; }
    .lb-table-head, .lb-table-row { grid-template-columns: 50px 1fr 100px; }
    .lb-table-head span:nth-child(4), .lb-table-head span:nth-child(5),
    .lb-table-row span:nth-child(4), .lb-table-row span:nth-child(5) { display: none; }
  }
  /* ---------- design layer: depth, light and motion ---------- */
  .landing-wrap {
    position: relative; isolation: isolate;
    background:
      radial-gradient(1200px 700px at 12% -8%, color-mix(in srgb, var(--landing-accent) 16%, transparent), transparent 60%),
      radial-gradient(900px 600px at 95% 18%, color-mix(in srgb, var(--landing-accent) 9%, #38bdf8 6%), transparent 62%),
      var(--landing-bg);
  }
  .landing-wrap::before { content: ''; position: fixed; inset: 0; z-index: -1; pointer-events: none; opacity: .5; background-image: radial-gradient(rgba(255,255,255,.055) 1px, transparent 1px); background-size: 28px 28px; mask-image: linear-gradient(180deg, #000 0, transparent 70%); -webkit-mask-image: linear-gradient(180deg, #000 0, transparent 70%); }
  .section-shell { position: relative; }

  .navbar { top: 12px; margin: 12px auto 0; width: min(1180px, calc(100% - 24px)); border-radius: 18px; border: 1px solid rgba(255,255,255,.1); background: rgba(14, 15, 24, .62); box-shadow: 0 18px 50px -20px rgba(0,0,0,.7), inset 0 1px 0 rgba(255,255,255,.06); }
  .nav-container { padding: 10px 14px 10px 18px; max-width: none; }
  .brand-title { font-size: 17px; letter-spacing: .01em; }
  .nav-links { gap: 6px; }
  .nav-link-btn { padding: 8px 14px; border-radius: 10px; transition: color .15s, background .15s; }
  .nav-link-btn:hover { background: rgba(255,255,255,.07); }
  .ip-pill { border-radius: 12px; background: rgba(255,255,255,.05); }
  .nav-login-btn { border-radius: 11px; padding: 8px 15px; font-weight: 600; background: rgba(255,255,255,.05); border-color: rgba(255,255,255,.14); color: var(--landing-text); transition: background .15s, border-color .15s, transform .12s; }
  .nav-login-btn:hover { background: color-mix(in srgb, var(--landing-accent) 22%, transparent); border-color: color-mix(in srgb, var(--landing-accent) 60%, transparent); transform: translateY(-1px); }

  /* hero */
  .hero-block { padding: 120px 0 110px; }
  .hero-bg-glow { display: none; }
  .hero-orb { position: absolute; border-radius: 50%; filter: blur(70px); opacity: .55; pointer-events: none; will-change: transform; }
  .orb-a { width: 520px; height: 520px; left: calc(50% - 560px); top: -160px; background: radial-gradient(circle, color-mix(in srgb, var(--landing-accent) 70%, transparent), transparent 68%); animation: drift-a 16s ease-in-out infinite alternate; }
  .orb-b { width: 440px; height: 440px; left: calc(50% + 120px); top: -40px; background: radial-gradient(circle, rgba(56,189,248,.45), transparent 68%); animation: drift-b 19s ease-in-out infinite alternate; }
  @keyframes drift-a { to { transform: translate(90px, 50px) scale(1.12); } }
  @keyframes drift-b { to { transform: translate(-80px, 70px) scale(1.08); } }
  .hero-grid { position: absolute; inset: 0; pointer-events: none; background-image: linear-gradient(rgba(255,255,255,.05) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.05) 1px, transparent 1px); background-size: 56px 56px; mask-image: radial-gradient(ellipse at 50% 30%, #000 0, transparent 70%); -webkit-mask-image: radial-gradient(ellipse at 50% 30%, #000 0, transparent 70%); }
  .hero-content { gap: 24px; }
  .hero-badge { padding: 7px 16px; backdrop-filter: blur(8px); box-shadow: 0 0 40px -8px color-mix(in srgb, var(--landing-accent) 70%, transparent); }
  .hero-title { font-size: clamp(38px, 6.2vw, 74px); line-height: 1.04; letter-spacing: -.035em; background: linear-gradient(180deg, #fff 30%, color-mix(in srgb, var(--landing-accent) 38%, #fff) 120%); -webkit-background-clip: text; background-clip: text; color: transparent; -webkit-text-fill-color: transparent; max-width: 920px; text-wrap: balance; }
  .hero-sub { font-size: clamp(16px, 1.6vw, 20px); max-width: 680px; text-wrap: pretty; }
  .hero-cta-group { margin-top: 10px; gap: 12px; }
  .btn-hero-primary { position: relative; overflow: hidden; padding: 15px 30px; border-radius: 14px; background: linear-gradient(180deg, color-mix(in srgb, var(--landing-accent) 80%, #fff), var(--landing-accent) 55%, color-mix(in srgb, var(--landing-accent) 82%, #000)); border: 1px solid color-mix(in srgb, var(--landing-accent) 60%, #fff); box-shadow: inset 0 1px 0 rgba(255,255,255,.35), 0 18px 44px -14px color-mix(in srgb, var(--landing-accent) 90%, transparent); text-shadow: 0 1px 0 rgba(0,0,0,.25); transition: transform .15s, box-shadow .15s, filter .15s; }
  .btn-hero-primary::after { content: ''; position: absolute; inset: 0; background: linear-gradient(110deg, transparent 30%, rgba(255,255,255,.35) 50%, transparent 70%); transform: translateX(-120%); transition: transform .6s; }
  .btn-hero-primary:hover { transform: translateY(-3px); filter: brightness(1.06); background: linear-gradient(180deg, color-mix(in srgb, var(--landing-accent) 80%, #fff), var(--landing-accent) 55%, color-mix(in srgb, var(--landing-accent) 82%, #000)); box-shadow: inset 0 1px 0 rgba(255,255,255,.4), 0 24px 54px -14px color-mix(in srgb, var(--landing-accent) 100%, transparent); }
  .btn-hero-primary:hover::after { transform: translateX(120%); }
  .btn-hero-secondary { padding: 15px 26px; border-radius: 14px; background: rgba(255,255,255,.05); border: 1px solid rgba(255,255,255,.16); backdrop-filter: blur(8px); box-shadow: inset 0 1px 0 rgba(255,255,255,.06); transition: background .15s, border-color .15s, transform .15s; }
  .btn-hero-secondary:hover { background: rgba(255,255,255,.1); border-color: rgba(255,255,255,.3); transform: translateY(-2px); }
  .hero-stats { display: flex; gap: 0; margin-top: 30px; border-radius: 18px; border: 1px solid rgba(255,255,255,.1); background: rgba(255,255,255,.04); backdrop-filter: blur(10px); box-shadow: inset 0 1px 0 rgba(255,255,255,.06), 0 24px 60px -30px rgba(0,0,0,.8); }
  .hero-stats > div { display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 16px 34px; min-width: 130px; }
  .hero-stats > div + div { border-left: 1px solid rgba(255,255,255,.08); }
  .stat-ic { display: grid; place-items: center; margin-bottom: 2px; }
  .hero-pano { position: absolute; inset: 0; background-size: 128px; background-repeat: repeat; image-rendering: pixelated; opacity: .22; filter: saturate(.8); mask-image: linear-gradient(#000 55%, transparent); -webkit-mask-image: linear-gradient(#000 55%, transparent); pointer-events: none; }
  .hero-stats strong { font-size: 26px; font-weight: 800; letter-spacing: -.02em; background: linear-gradient(180deg, #fff, color-mix(in srgb, var(--landing-accent) 45%, #fff)); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .hero-stats span { font-size: 11px; font-weight: 700; letter-spacing: .09em; text-transform: uppercase; color: var(--landing-muted); }

  /* sections */
  .section-head { margin-bottom: 52px; gap: 12px; }
  .section-head h2 { font-size: clamp(28px, 3.4vw, 42px); letter-spacing: -.03em; font-weight: 800; background: linear-gradient(180deg, #fff 40%, color-mix(in srgb, var(--landing-text) 62%, transparent)); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .section-head h2::after { content: ''; display: block; width: 46px; height: 4px; border-radius: 4px; margin: 16px auto 0; background: linear-gradient(90deg, var(--landing-accent), color-mix(in srgb, var(--landing-accent) 30%, #38bdf8)); box-shadow: 0 0 18px color-mix(in srgb, var(--landing-accent) 70%, transparent); }
  .section-head .muted { font-size: 16.5px; max-width: 640px; margin: 0 auto; }

  /* cards share one glassy look and lift on hover */
  .download-card, .server-card, .counter-card, .inst-card, .news-card, .faq-item, .podium-item, .lb-table-wrap, .custom-card {
    background: linear-gradient(180deg, rgba(255,255,255,.055), rgba(255,255,255,.02));
    border: 1px solid rgba(255,255,255,.1);
    box-shadow: inset 0 1px 0 rgba(255,255,255,.06), 0 18px 40px -26px rgba(0,0,0,.8);
    backdrop-filter: blur(6px);
    border-radius: calc(var(--landing-radius) + 4px);
    transition: transform .2s cubic-bezier(.2,.8,.2,1), border-color .2s, box-shadow .2s, background .2s;
  }
  .download-card:hover, .server-card:hover, .inst-card:hover, .news-card:hover, .counter-card:hover { transform: translateY(-4px); border-color: color-mix(in srgb, var(--landing-accent) 45%, rgba(255,255,255,.12)); box-shadow: inset 0 1px 0 rgba(255,255,255,.08), 0 28px 60px -26px color-mix(in srgb, var(--landing-accent) 55%, #000); }
  .faq-item { overflow: hidden; } .faq-item:hover { border-color: color-mix(in srgb, var(--landing-accent) 40%, rgba(255,255,255,.12)); }
  .counter-num { font-size: 46px; background: linear-gradient(180deg, #fff, var(--landing-accent)); -webkit-background-clip: text; background-clip: text; color: transparent; }

  /* downloads */
  .downloads-cards-grid { grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 22px; }
  .download-card { padding: 38px 24px 26px; gap: 8px; }
  .download-card.recommended { border-color: color-mix(in srgb, var(--landing-accent) 70%, transparent); background: linear-gradient(180deg, color-mix(in srgb, var(--landing-accent) 17%, transparent), rgba(255,255,255,.02)); box-shadow: 0 0 0 1px color-mix(in srgb, var(--landing-accent) 40%, transparent), 0 30px 70px -30px color-mix(in srgb, var(--landing-accent) 80%, #000); }
  .recommended-badge { top: -11px; padding: 4px 12px; font-size: 10px; letter-spacing: .08em; background: linear-gradient(180deg, color-mix(in srgb, var(--landing-accent) 80%, #fff), var(--landing-accent)); box-shadow: 0 8px 20px -8px var(--landing-accent); }
  .download-card-icon { width: 68px; height: 68px; border-radius: 20px; margin-bottom: 8px; color: color-mix(in srgb, var(--landing-accent) 55%, #fff); background: radial-gradient(circle at 30% 20%, color-mix(in srgb, var(--landing-accent) 35%, transparent), rgba(255,255,255,.04)); border: 1px solid rgba(255,255,255,.12); box-shadow: inset 0 1px 0 rgba(255,255,255,.12), 0 14px 30px -14px color-mix(in srgb, var(--landing-accent) 80%, transparent); }
  .download-card h3 { font-size: 21px; font-weight: 750; letter-spacing: -.01em; } .download-card h3 small { font-size: 13px; font-weight: 600; color: var(--landing-muted); }
  .dl-note { margin: 0; font-size: 13px; color: var(--landing-muted); }
  .dl-meta { display: flex; flex-wrap: wrap; justify-content: center; align-items: center; gap: 6px 10px; margin: 8px 0 4px; font-size: 12px; color: var(--landing-muted); max-width: 100%; }
  .dl-meta .ver { padding: 2px 9px; border-radius: 99rem; font-weight: 700; color: color-mix(in srgb, var(--landing-accent) 55%, #fff); background: color-mix(in srgb, var(--landing-accent) 16%, transparent); border: 1px solid color-mix(in srgb, var(--landing-accent) 35%, transparent); }
  .dl-meta .fname { font-family: ui-monospace, 'JetBrains Mono Variable', monospace; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: rgba(255,255,255,.62); }
  .download-card.soon { opacity: .6; } .download-card.soon:hover { transform: none; }
  .dl-group { text-align: center; margin: 54px 0 22px; font-size: 14px; font-weight: 700; letter-spacing: .14em; text-transform: uppercase; color: var(--landing-muted); }
  .downloads-cards-grid.phones { max-width: 620px; margin: 0 auto; }
  .btn-download { padding: 13px 18px; border-radius: 12px; margin-top: 10px; font-weight: 700; letter-spacing: .01em; background: linear-gradient(180deg, color-mix(in srgb, var(--landing-accent) 80%, #fff), var(--landing-accent) 55%, color-mix(in srgb, var(--landing-accent) 84%, #000)); border: 1px solid color-mix(in srgb, var(--landing-accent) 60%, #fff); box-shadow: inset 0 1px 0 rgba(255,255,255,.3), 0 12px 26px -12px color-mix(in srgb, var(--landing-accent) 90%, transparent); text-shadow: 0 1px 0 rgba(0,0,0,.22); transition: transform .12s, filter .15s, box-shadow .15s; }
  .btn-download:hover { filter: brightness(1.08); transform: translateY(-1px); background: linear-gradient(180deg, color-mix(in srgb, var(--landing-accent) 80%, #fff), var(--landing-accent) 55%, color-mix(in srgb, var(--landing-accent) 84%, #000)); }
  .btn-download[aria-disabled='true'] { background: rgba(255,255,255,.06); border-color: rgba(255,255,255,.12); box-shadow: none; color: var(--landing-muted); text-shadow: none; pointer-events: none; }
  .custom-button { border-radius: 12px; padding: 13px 24px; background: linear-gradient(180deg, color-mix(in srgb, var(--landing-accent) 80%, #fff), var(--landing-accent) 55%, color-mix(in srgb, var(--landing-accent) 84%, #000)); border: 1px solid color-mix(in srgb, var(--landing-accent) 60%, #fff); box-shadow: inset 0 1px 0 rgba(255,255,255,.3), 0 12px 26px -12px color-mix(in srgb, var(--landing-accent) 90%, transparent); transition: transform .12s, filter .15s; }
  .custom-button:hover { transform: translateY(-2px); filter: brightness(1.07); }
  .custom-button.secondary { background: rgba(255,255,255,.06); border-color: rgba(255,255,255,.18); box-shadow: inset 0 1px 0 rgba(255,255,255,.06); }
  .social-btn { border-radius: 14px; box-shadow: inset 0 1px 0 rgba(255,255,255,.22), 0 16px 34px -18px rgba(0,0,0,.8); }

  /* servers, leaderboard */
  .server-copy-btn { border-radius: 11px; transition: border-color .15s, background .15s; } .server-copy-btn:hover { background: rgba(255,255,255,.05); }
  .sort-chip { border-radius: 99rem; padding: 9px 18px; } .sort-chip:hover { border-color: rgba(255,255,255,.25); color: var(--landing-text); }
  .podium-item.gold { background: linear-gradient(180deg, rgba(245,158,11,.2), rgba(245,158,11,.04)); border-color: rgba(245,158,11,.5); box-shadow: 0 30px 70px -34px rgba(245,158,11,.7); }
  .lb-table-row { transition: background .15s; } .lb-table-row:hover { background: rgba(255,255,255,.04); }

  /* footer */
  .footer { margin-top: 40px; padding: 48px 0; border-top: 0; background: linear-gradient(180deg, transparent, rgba(0,0,0,.35)); position: relative; }
  .footer::before { content: ''; position: absolute; top: 0; left: 8%; right: 8%; height: 1px; background: linear-gradient(90deg, transparent, color-mix(in srgb, var(--landing-accent) 60%, transparent), transparent); }

  @media (max-width: 768px) {
    .navbar { top: 8px; margin-top: 8px; border-radius: 16px; }
    .hero-block { padding: 80px 0 70px; }
    .hero-stats > div { padding: 14px 18px; min-width: 0; } .hero-stats strong { font-size: 22px; }
    .orb-a, .orb-b { opacity: .4; }
  }
  @media (prefers-reduced-motion: reduce) { .orb-a, .orb-b { animation: none; } .btn-hero-primary::after { display: none; } .download-card, .server-card, .inst-card, .news-card, .counter-card { transition: none; } }
</style>
