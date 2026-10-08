<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, Copy, Download, Gamepad2, HardDrive, Package, Server, ExternalLink, Smartphone } from '@lucide/svelte';
  import Empty from '../ui/Empty.svelte';
  import { get } from '../../lib/api';
  import { formatBytes } from '../../lib/api';
  import { loadManifest, play } from '../store.svelte';

  type Landing = { hosted_downloads: { platform: string; label: string; file_url: string; size: number; filename: string }[]; external_download_url: string | null; server_ip: string; server_port: number };
  type App = { version: string; size: number; url: string; altstore_url?: string; altstore_link?: string };
  type Mobile = { android: App | null; ios: App | null };
  type Installer = { version: string; notes: string; url: string; size: number } | null;

  let landing = $state<Landing | null>(null);
  let installer = $state<Installer>(null);
  let mobile = $state<Mobile | null>(null);
  let copied = $state('');
  const os = /Windows/i.test(navigator.userAgent) ? 'windows' : /Mac/i.test(navigator.userAgent) ? 'mac' : /Linux|X11/i.test(navigator.userAgent) ? 'linux' : 'windows';
  const phone = /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);

  onMount(() => {
    if (!play.manifest) void loadManifest();
    get<Landing>('/api/v1/landing').then((l) => (landing = l)).catch(() => {});
    get<Mobile>('/api/v1/mobile-apps').then((m) => (mobile = m)).catch(() => {});
    get<Installer>('/api/v1/launcher/update').then((u) => (installer = u)).catch(() => {});
  });

  const downloads = $derived.by(() => {
    const out: { label: string; href: string; note: string; primary: boolean }[] = [];
    if (installer) out.push({ label: `Windows installer ${installer.version}`, href: installer.url, note: formatBytes(installer.size), primary: os === 'windows' });
    for (const d of landing?.hosted_downloads ?? []) out.push({ label: d.label || d.filename, href: d.file_url, note: formatBytes(d.size), primary: d.platform === os });
    if (landing?.external_download_url) out.push({ label: 'Download from the website', href: landing.external_download_url, note: 'External link', primary: !out.length });
    return out;
  });

  async function copy(text: string) {
    try { await navigator.clipboard.writeText(text); } catch { /* clipboard may be unavailable */ }
    copied = text;
    setTimeout(() => (copied = ''), 1400);
  }
  const address = $derived(landing?.server_ip ? (landing.server_port && landing.server_port !== 25565 ? `${landing.server_ip}:${landing.server_port}` : landing.server_ip) : '');
  const instances = $derived(play.manifest?.instances ?? []);
  const steps = [
    ['Install', 'Download the launcher on your Windows PC and run it.'],
    ['Sign in', 'Use this same account. Your balance, guild and friends follow you.'],
    ['Play', 'Pick an instance and press Play. Java, mods and updates are automatic.'],
  ];
</script>

<div class="pl-page">
  <div class="pl-head">
    <div><h1><Gamepad2 size={26} /> Launcher</h1><p>Get the game on your PC. Everything else in this panel works from your phone.</p></div>
  </div>

  <div class="pl-hero hero">
    <div>
      <h2>Ready to play?</h2>
      <p>{phone ? 'The launcher runs on a computer. Send yourself the link and grab it there:' : 'One installer, and every instance, mod and update is handled for you.'}</p>
      <div class="cta">
        {#each downloads as d}
          <a class="btn pl-btn lg" class:primary={d.primary} href={d.href} target="_blank" rel="noopener noreferrer"><Download size={18} /> {d.label}<small>{d.note}</small></a>
        {:else}
          <span class="pl-chip warn">The launcher download hasn't been published yet.</span>
        {/each}
      </div>
      {#if phone}<button class="pl-btn" onclick={() => copy(location.origin + '/#/play/launcher')}>{#if copied}<Check size={16} /> Link copied{:else}<Copy size={16} /> Copy this page's link{/if}</button>{/if}
    </div>
    <ol class="steps">
      {#each steps as [t, d], i}<li><b>{i + 1}</b><div><strong>{t}</strong><span>{d}</span></div></li>{/each}
    </ol>
  </div>

  {#if mobile?.android || mobile?.ios}
    <section class="pl-card apps">
      <div class="pl-card-head"><h2><Smartphone size={17} /> Get the phone app</h2></div>
      <div class="appgrid">
        {#if mobile.android}
          <div class="app">
            <b>Android</b><span class="sub">v{mobile.android.version} · {formatBytes(mobile.android.size)}</span>
            <a class="btn pl-btn primary" href={mobile.android.url} download><Download size={16} /> Download .apk</a>
            <small>Open the file after it downloads and allow installs from your browser when asked.</small>
          </div>
        {/if}
        {#if mobile.ios}
          <div class="app">
            <b>iPhone / iPad</b><span class="sub">v{mobile.ios.version} · {formatBytes(mobile.ios.size)}</span>
            <a class="btn pl-btn primary" href={mobile.ios.altstore_link}><Download size={16} /> Add to AltStore</a>
            <button class="pl-btn" onclick={() => copy(mobile?.ios?.altstore_url ?? '')}><Copy size={15} /> Copy source URL</button>
            <a class="btn pl-btn" href={mobile.ios.url} download>Download .ipa (Sideloadly)</a>
            <small>Apple only runs apps signed by a developer account, so use AltStore, SideStore or Sideloadly. Or open this site in Safari and use <b>Share → Add to Home Screen</b>.</small>
          </div>
        {/if}
      </div>
    </section>
  {/if}

  {#if address}
    <button class="pl-card ip" onclick={() => copy(address)}>
      <Server size={20} />
      <div><span>Server address</span><b>{address}</b></div>
      <span class="pl-chip accent">{#if copied === address}<Check size={13} /> Copied{:else}<Copy size={13} /> Copy{/if}</span>
    </button>
  {/if}

  <div class="pl-head sub"><div><h2>Instances</h2><p>Everything you can launch with your account.</p></div></div>
  {#if !play.manifest}
    <div class="pl-grid"><div class="pl-skel" style="height: 170px"></div><div class="pl-skel" style="height: 170px"></div></div>
  {:else if !instances.length}
    <div class="pl-card"><Empty icon={Package} title="No instances yet" text="Your admin hasn't published a version or modpack." /></div>
  {:else}
    <div class="pl-grid" style="--min: 300px">
      {#each instances as i, n (i.id)}
        <article class="pl-card inst" style="animation-delay:{n * 50}ms">
          <div class="art" style:background-image={i.banner_url ? `url(${i.banner_url})` : undefined}>
            {#if i.icon_url}<img src={i.icon_url} alt="" />{:else}<Package size={30} />{/if}
            {#if i.featured}<span class="pl-chip accent star">Featured</span>{/if}
          </div>
          <h3>{i.name}</h3>
          <p>{i.description || i.source_label}</p>
          <div class="tags">
            <span class="pl-chip">{i.mc_version}</span>
            <span class="pl-chip accent">{i.loader}</span>
            {#if i.file_count}<span class="pl-chip"><HardDrive size={12} /> {formatBytes(i.total_size)}</span>{/if}
            {#if i.server}<button class="pl-chip good" onclick={() => copy(i.server?.address ?? '')}><Server size={12} /> {i.server.address}</button>{/if}
          </div>
        </article>
      {/each}
    </div>
  {/if}

  {#if play.manifest?.branding.links.length}
    <div class="links">{#each play.manifest.branding.links as l}<a class="pl-chip" href={l.url} target="_blank" rel="noopener noreferrer">{l.label} <ExternalLink size={11} /></a>{/each}</div>
  {/if}
</div>

<style>
  .hero { display: grid; grid-template-columns: 1.2fr 1fr; gap: 24px; margin-bottom: 14px; align-items: center; }
  .hero h2 { font-size: 1.5rem; margin-bottom: 6px; }
  .cta { display: flex; flex-wrap: wrap; gap: 10px; margin: 16px 0 6px; }
  .cta a { gap: 8px; }
  .cta small { opacity: 0.75; font-weight: 450; margin-left: 4px; }
  .steps { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 10px; }
  .steps li { display: flex; gap: 14px; align-items: flex-start; padding: 12px 14px; border-radius: 16px; background: rgba(0, 0, 0, 0.22); border: 1px solid var(--pl-line); }
  .steps b { width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; background: var(--accent); color: #fff; flex-shrink: 0; font-size: 0.85rem; }
  .steps div { display: flex; flex-direction: column; font-size: 0.86rem; color: var(--text-2); }
  .steps strong { color: var(--text); }
  .apps { margin-bottom: 14px; }
  .appgrid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 14px; }
  .app { display: flex; flex-direction: column; gap: 8px; padding: 14px; border-radius: 16px; background: rgba(0, 0, 0, 0.2); border: 1px solid var(--pl-line); }
  .app small { color: var(--muted); line-height: 1.4; }
  .ip { display: flex !important; align-items: center; gap: 14px; margin-bottom: 8px; }
  .ip div { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .ip div span { font-size: 0.7rem; letter-spacing: 0.07em; text-transform: uppercase; color: var(--muted); }
  .ip b { font-family: var(--mono); font-size: 1.1rem; overflow: hidden; text-overflow: ellipsis; }
  .sub { margin: 26px 0 12px; }
  .sub h2 { font-size: 1.2rem; }
  .inst { display: flex; flex-direction: column; gap: 8px; animation: up 0.4s var(--ease, ease) backwards; }
  @keyframes up { from { opacity: 0; transform: translateY(10px); } }
  .art { position: relative; height: 96px; border-radius: 14px; background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 40%, var(--surface-2)), var(--surface-2)); background-size: cover; background-position: center; display: grid; place-items: center; color: var(--accent-2); overflow: hidden; }
  .art img { width: 56px; height: 56px; border-radius: 14px; object-fit: cover; box-shadow: 0 8px 24px -8px #000; }
  .star { position: absolute; top: 8px; left: 8px; }
  .inst h3 { font-size: 1.05rem; }
  .inst p { color: var(--muted); font-size: 0.85rem; line-height: 1.4; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .tags button.pl-chip { border: none; cursor: pointer; }
  .links { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 22px; }
  .links a { gap: 5px; }
  @media (max-width: 880px) { .hero { grid-template-columns: 1fr; padding: 18px; } }
</style>
