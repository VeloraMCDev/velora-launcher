<script lang="ts">
  // A scaled-down replica of the launcher's home screen, driven by the
  // branding being edited, so admins see changes instantly.
  import { Play, Settings } from '@lucide/svelte';
  import type { Branding } from '../lib/types';

  let { b, instanceName = 'Survival SMP', instanceLogo = null }: { b: Branding; instanceName?: string; instanceLogo?: string | null } = $props();

  const fontFamily = $derived(`'${b.font} Variable', '${b.font}', 'Inter Variable', sans-serif`);
  const media = $derived(b.background.kind !== 'gradient' && b.background.url ? b.background.url : null);
  const news = $derived(b.news.slice(0, 2));
  const surf = $derived(b.glass && media ? `color-mix(in srgb, ${b.colors.surface} 75%, transparent)` : b.colors.surface);
</script>

<div
  class="frame"
  style:--acc={b.colors.accent}
  style:--bg={b.colors.background}
  style:--surf={surf}
  style:--txt={b.colors.text}
  style:--mut={b.colors.muted}
  style:--ok={b.colors.success}
  style:--r="{Math.max(2, b.radius * 0.55)}px"
  style:font-family={fontFamily}
>
  <div class="inner">
    {#if media}
      <div class="bg" style:filter="blur({b.background.blur * 0.5}px)">
        {#if b.background.kind === 'video'}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video src={media} autoplay muted loop playsinline></video>
        {:else}
          <div class="img" style:background-image="url(&quot;{media}&quot;)"></div>
        {/if}
      </div>
      <div class="dim" style:--d={b.background.dim / 100}></div>
    {/if}

    <div class="titlebar">
      {#if b.logo_url}<img src={b.logo_url} alt="" />{:else}<span class="mark"></span>{/if}
      <span>{b.name}</span>
    </div>

    <div class="rail">
      <span class="inst active">{instanceName.slice(0, 2).toUpperCase()}</span>
      <span class="inst">CB</span>
      <span class="inst">CA</span>
      <span class="spacer"></span>
      <Settings size={10} />
      <span class="avatar"></span>
    </div>

    <div class="main">
      <div class="hero">
        <div class="chips"><span>26.3</span><span>Fabric</span></div>
        {#if instanceLogo}
          <img class="hero-logo" src={instanceLogo} alt={instanceName} />
        {:else}
          <h1>{instanceName}</h1>
        {/if}
        <p>{b.tagline}</p>
        <div class="row">
          <span class="play"><Play size={9} fill="currentColor" /> Play</span>
          {#if b.features.server_status}<span class="status"><i></i> 42/100 online</span>{/if}
        </div>
      </div>
      {#if b.features.news && news.length}
        <div class="news">
          <span class="news-h">News</span>
          {#each news as n}
            <div class="card">
              {#if n.image_url}<img src={n.image_url} alt="" />{/if}
              <div>
                {#if n.tag}<em>{n.tag}</em>{/if}
                <strong>{n.title || 'Untitled'}</strong>
                <small>{n.body}</small>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .frame {
    position: relative; aspect-ratio: 1120 / 700; width: 100%; border-radius: 10px; overflow: hidden; color: var(--txt);
    background: var(--bg); border: 1px solid var(--line-strong); container-type: inline-size;
  }
  /* cqw must be used inside the container, not on it */
  .inner { position: absolute; inset: 0; font-size: 1.75cqw; }
  .bg { position: absolute; inset: -8px; }
  .bg video, .img { width: 100%; height: 100%; object-fit: cover; background-size: cover; background-position: center; }
  .dim { position: absolute; inset: 0; background: linear-gradient(90deg, var(--bg) 0%, color-mix(in srgb, var(--bg) calc(var(--d) * 100%), transparent) 55%, color-mix(in srgb, var(--bg) calc(var(--d) * 70%), transparent) 100%); }
  .titlebar { position: absolute; top: 0; left: 0; right: 0; height: 5.6%; display: flex; align-items: center; gap: 0.6em; padding: 0 1.3%; font-size: 0.8em; font-weight: 650; letter-spacing: 0.08em; z-index: 2; }
  .titlebar img { height: 60%; border-radius: 3px; }
  .mark { width: 0.9em; height: 0.9em; border-radius: 50%; border: 0.15em solid var(--acc); box-sizing: border-box; }
  .rail {
    position: absolute; left: 0; top: 5.6%; bottom: 0; width: 6.2%; display: flex; flex-direction: column; align-items: center;
    gap: 0.6em; padding: 0.9em 0; border-right: 1px solid rgba(255, 255, 255, 0.07); z-index: 2; color: var(--mut);
    background: color-mix(in srgb, var(--surf) 70%, transparent);
  }
  .inst { width: 64%; aspect-ratio: 1; border-radius: var(--r); background: rgba(255, 255, 255, 0.06); display: grid; place-items: center; font-weight: 600; font-size: 0.75em; color: var(--mut); }
  .inst.active { background: color-mix(in srgb, var(--acc) 18%, transparent); color: var(--txt); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--acc) 55%, transparent); }
  .spacer { flex: 1; }
  .avatar { width: 55%; aspect-ratio: 1; border-radius: 25%; background: #3fb97c; }
  .main { position: absolute; left: 6.2%; top: 5.6%; right: 0; bottom: 0; padding: 4% 4%; z-index: 1; display: flex; gap: 6%; }
  .hero { flex: 1; align-self: flex-end; display: flex; flex-direction: column; gap: 0.6em; }
  .hero-logo { max-height: 2.8em; max-width: 80%; object-fit: contain; object-position: left bottom; }
  .hero h1 { font-size: 3.1em; line-height: 1.05; margin: 0; letter-spacing: -0.025em; font-weight: 700; }
  .hero p { color: var(--mut); font-size: 0.95em; margin: 0; }
  .chips { display: flex; gap: 0.4em; }
  .chips span { font-size: 0.7em; padding: 0.2em 0.55em; border-radius: 0.3em; background: rgba(255, 255, 255, 0.07); }
  .row { display: flex; align-items: center; gap: 0.8em; margin-top: 0.8em; }
  .play { display: inline-flex; align-items: center; gap: 0.5em; padding: 0.85em 2.2em; border-radius: var(--r); font-weight: 600; font-size: 1em; color: white; background: var(--acc); }
  .status { display: inline-flex; align-items: center; gap: 0.4em; font-size: 0.75em; padding: 0.7em 0.9em; border-radius: var(--r); background: var(--surf); border: 1px solid rgba(255, 255, 255, 0.07); color: var(--mut); }
  .status i { width: 0.45em; height: 0.45em; border-radius: 50%; background: var(--ok); }
  .news { width: 30%; display: flex; flex-direction: column; gap: 0.6em; }
  .news-h { font-size: 0.72em; color: var(--mut); }
  .card { display: flex; flex-direction: column; border-radius: var(--r); overflow: hidden; background: var(--surf); border: 1px solid rgba(255, 255, 255, 0.07); }
  .card img { width: 100%; aspect-ratio: 16/7; object-fit: cover; }
  .card div { padding: 0.7em 0.8em; display: flex; flex-direction: column; gap: 0.25em; }
  .card em { font-style: normal; font-size: 0.65em; color: var(--acc); font-weight: 560; }
  .card strong { font-size: 0.85em; font-weight: 600; }
  .card small { font-size: 0.68em; color: var(--mut); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
</style>
