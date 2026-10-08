<script lang="ts">
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import { MapView } from './engine';
  import { dimSlug } from './types';
  import type { LivePlayer, MapInfo, MapLayers, MapOverlay, MapSelection } from './types';

  // The SCOPENET Map: tiles, claims, pins and players on one canvas. The parent feeds it data (see feed.ts) and decides what a
  // click on a player does through the `player` snippet.
  let {
    info = null, overlay = null, tileUrl, avatarUrl, selfUuid = null, error = '', initialDimension = '',
    player, toolbar, onplayer, onblock, available = { claims: true, pins: true, players: true }
  }: {
    info: MapInfo | null;
    overlay: MapOverlay | null;
    tileUrl: (slug: string, zoom: number, x: number, y: number) => string;
    avatarUrl?: (uuid: string) => string;
    selfUuid?: string | null;
    error?: string;
    initialDimension?: string;
    /** Shown in the side card when a player is selected. */
    player?: Snippet<[LivePlayer, () => void]>;
    /** Extra buttons for the top bar. */
    toolbar?: Snippet;
    onplayer?: (p: LivePlayer | null) => void;
    onblock?: (point: { dimension: string; x: number; z: number }) => void;
    /** Which layers exist for this viewer (the public map may hide some). */
    available?: MapLayers;
  } = $props();

  let canvas: HTMLCanvasElement;
  let view: MapView | undefined;
  let layers = $state<MapLayers>({ claims: true, pins: true, players: true });
  let selection = $state<MapSelection | null>(null);
  let dim = $state('');
  let cursor = $state<{ x: number; z: number } | null>(null);
  let hint = $state<string | null>(null);
  let zoomText = $state('');
  let listOpen = $state(false);
  let following = $state<string | null>(null);
  let hiddenPlayers = $state<string[]>([]);
  let showSkins = $state(true);

  const dims = $derived((info?.dimensions ?? []).filter((d) => d.available));
  const here = $derived((overlay?.players ?? []).filter((p) => dimSlug(p.dimension) === dim && !hiddenPlayers.includes(p.uuid)));
  const everyone = $derived(overlay?.players ?? []);
  const me = $derived(selfUuid ? everyone.find((p) => p.uuid.toLowerCase() === selfUuid.toLowerCase()) : undefined);
  const waiting = $derived(!info || !info.ready);
  const claimsHere = $derived((overlay?.claims ?? []).filter((c) => dimSlug(c.dimension) === dim).length);
  const pinsHere = $derived((overlay?.pins ?? []).filter((p) => dimSlug(p.dimension) === dim).length);

  onMount(() => {
    view = new MapView(canvas, {
      tileUrl: (s, z, x, y) => tileUrl(s, z, x, y),
      avatarUrl: (u) => avatarUrl?.(u) ?? '',
      selfUuid,
      onblock: (point) => onblock?.(point),
      onselect: (s) => { selection = s; onplayer?.(s?.type === 'player' ? s.player : null); },
      onhover: (h) => { cursor = h.cursor; hint = h.label; },
      onview: (v) => { zoomText = v.scale >= 1 ? `${v.scale.toFixed(v.scale >= 10 ? 0 : 1)} px per block` : `${(1 / v.scale).toFixed(1).replace(/\.0$/, '')} blocks per px`; }
    });
    return () => view?.destroy();
  });

  $effect(() => { if (view && info) { view.setInfo(info); dim = view.currentDimension?.slug ?? ''; } });
  $effect(() => { if (view && overlay) view.setOverlay({ ...overlay, players: overlay.players.filter((p) => !hiddenPlayers.includes(p.uuid)) }); });
  $effect(() => { view?.setLayers(layers); });
  $effect(() => { view?.setSkins(showSkins); });
  $effect(() => { view?.setSelf(selfUuid); });
  $effect(() => { if (view && info && initialDimension) view.setDimension(initialDimension); });

  function chooseDimension(slug: string) { view?.setDimension(slug); dim = view?.currentDimension?.slug ?? slug; following = null; }
  function goTo(p: LivePlayer) {
    if (hiddenPlayers.includes(p.uuid)) hiddenPlayers = hiddenPlayers.filter((id) => id !== p.uuid);
    if (dimSlug(p.dimension) !== dim) chooseDimension(p.dimension);
    view?.flyTo(p.x, p.z, Math.max(view.getView().scale, 1.2));
    view?.select({ type: 'player', player: p });
    listOpen = false;
  }
  function toggleFollow(p: LivePlayer) {
    following = following === p.uuid ? null : p.uuid;
    view?.follow(following);
  }
  function findMe() { if (me) { goTo(me); } }
  function togglePlayer(uuid: string) {
    hiddenPlayers = hiddenPlayers.includes(uuid) ? hiddenPlayers.filter((id) => id !== uuid) : [...hiddenPlayers, uuid];
    if (hiddenPlayers.includes(uuid) && selection?.type === 'player' && selection.player.uuid === uuid) closeCard();
  }
  function toSpawn() {
    const spawn = view?.pinsOfKind('spawn')[0];
    if (spawn) view?.flyTo(spawn.x, spawn.z, 1);
  }
  const closeCard = () => { view?.select(null); following = null; view?.follow(null); };
  const fmt = (n: number) => Math.round(n).toLocaleString();
</script>

<div class="map">
  <canvas bind:this={canvas} aria-label="World map"></canvas>

  <div class="top">
    <div class="bar">
      {#if dims.length > 1}
        <div class="seg" role="tablist" aria-label="Dimension">
          {#each dims as d}
            <button role="tab" aria-selected={dim === d.slug} class:on={dim === d.slug} onclick={() => chooseDimension(d.slug)}>{d.label}</button>
          {/each}
        </div>
      {:else if dims.length === 1}
        <span class="chip solo">{dims[0].label}</span>
      {/if}
      <div class="chips" aria-label="Layers">
        {#if available.claims}<button class="chip" class:on={layers.claims} onclick={() => (layers.claims = !layers.claims)} title="Guild land">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M4 4h16v16H4z" /><path d="M4 12h16M12 4v16" /></svg> Claims <b>{claimsHere}</b>
        </button>{/if}
        {#if available.pins}<button class="chip" class:on={layers.pins} onclick={() => (layers.pins = !layers.pins)} title="Spawn, warps, markets and shops">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M12 21s7-6.2 7-11.5A7 7 0 0 0 5 9.5C5 14.8 12 21 12 21z" /><circle cx="12" cy="9.5" r="2.4" /></svg> Places <b>{pinsHere}</b>
        </button>{/if}
        {#if available.players}<button class="chip" class:on={layers.players} onclick={() => (layers.players = !layers.players)} title="Players online">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="3.5" /><path d="M5 20c.8-4 3.5-6 7-6s6.2 2 7 6" /></svg> Players <b>{here.length}</b>
        </button>{/if}
      </div>
      {#if toolbar}{@render toolbar()}{/if}
    </div>

    <div class="right">
      {#if available.players}<button class="chip" class:on={listOpen} onclick={() => (listOpen = !listOpen)} disabled={!everyone.length}>
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01" /></svg> Online <b>{everyone.length}</b>
      </button>{/if}
      {#if listOpen && everyone.length}
        <ul class="plist" aria-label="Players online">
          <li class="options">{#if avatarUrl}<label><input type="checkbox" bind:checked={showSkins} /> Show skins</label>{/if}<button onclick={() => (hiddenPlayers = hiddenPlayers.length ? [] : everyone.map((p) => p.uuid))}>{hiddenPlayers.length ? 'Show all' : 'Hide all'}</button></li>
          {#each everyone as p (p.uuid)}
            <li>
              <input type="checkbox" aria-label={`Show ${p.name} on map`} checked={!hiddenPlayers.includes(p.uuid)} onchange={() => togglePlayer(p.uuid)} />
              <button onclick={() => goTo(p)}>
                {#if avatarUrl}<img src={avatarUrl(p.uuid)} alt="" width="22" height="22" />{/if}
                <span class="nm">{p.name}</span>
                <span class="where">{dimSlug(p.dimension).replace('the_', '').replace('_', ' ')}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>

  <div class="bottom-left">
    {#if hint}<span class="hint">{hint}</span>{/if}
    <span class="coords">{#if cursor}X {fmt(cursor.x)} · Z {fmt(cursor.z)}{:else}&nbsp;{/if}<i>{zoomText}</i></span>
  </div>

  <div class="zoom">
    {#if me}<button class="round" onclick={findMe} title="Find me" aria-label="Find me"><svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="3.5" /><path d="M12 2v4M12 18v4M2 12h4M18 12h4" /></svg></button>{/if}
    <button class="round" onclick={toSpawn} title="Go to spawn" aria-label="Go to spawn"><svg viewBox="0 0 24 24" width="17" height="17" fill="currentColor"><path d="M12 2l2.9 6.3 6.9.7-5.2 4.6 1.5 6.8L12 17l-6.1 3.4 1.5-6.8L2.2 9l6.9-.7z" /></svg></button>
    <div class="stack">
      <button onclick={() => view?.zoomBy(1.6)} aria-label="Zoom in">+</button>
      <button onclick={() => view?.zoomBy(1 / 1.6)} aria-label="Zoom out">−</button>
    </div>
  </div>

  {#if selection}
    <aside class="card" class:wide={selection.type === 'player' && !!player}>
      <button class="x" onclick={closeCard} aria-label="Close">×</button>
      {#if selection.type === 'player'}
        {#if player}
          {@render player(selection.player, closeCard)}
        {:else}
          <div class="head">
            {#if avatarUrl}<img src={avatarUrl(selection.player.uuid)} alt="" width="44" height="44" />{/if}
            <div><h3>{selection.player.name}</h3><span class="muted">{dimSlug(selection.player.dimension).replace('the_', '')}</span></div>
          </div>
          <p class="muted">X {fmt(selection.player.x)} · Y {fmt(selection.player.y)} · Z {fmt(selection.player.z)}</p>
          <button class="primary" onclick={() => toggleFollow((selection as { player: LivePlayer }).player)}>{following === selection.player.uuid ? 'Stop following' : 'Follow on map'}</button>
        {/if}
      {:else if selection.type === 'pin'}
        <h3>{selection.pin.label}</h3>
        {#each selection.pin.lines as line}<p>{line}</p>{/each}
        <p class="muted">X {fmt(selection.pin.x)} · Y {fmt(selection.pin.y)} · Z {fmt(selection.pin.z)}</p>
      {:else}
        <div class="head">
          {#if selection.claim.icon}<img class="icon" src={selection.claim.icon} alt="" width="40" height="40" />{/if}
          <div><h3 style:color={selection.claim.color}>{selection.claim.tag ? `[${selection.claim.tag}] ` : ''}{selection.claim.name}</h3>
          <span class="muted">{selection.claim.chunks} chunk{selection.claim.chunks === 1 ? '' : 's'} here</span></div>
        </div>
        {#each selection.claim.lines.slice(1) as line}<p>{line}</p>{/each}
      {/if}
    </aside>
  {/if}

  {#if waiting}
    <div class="veil">
      <div class="spinner"></div>
      <strong>{info?.enabled === false ? 'The map is off for this server' : 'Drawing the world…'}</strong>
      <span>{error || info?.message || 'Connecting to the map.'}</span>
    </div>
  {:else if error}
    <div class="toast" role="status">{error}</div>
  {/if}
</div>

<style>
  .map { position: relative; width: 100%; height: 100%; min-height: 320px; overflow: hidden; border-radius: inherit; background: #0d1220; color: #f4f6ff; font-family: inherit; }
  canvas { position: absolute; inset: 0; width: 100%; height: 100%; display: block; cursor: grab; }
  .top { position: absolute; top: 12px; left: 12px; right: 12px; display: flex; justify-content: space-between; align-items: flex-start; gap: 10px; pointer-events: none; }
  .top > * { pointer-events: auto; }
  .bar { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .seg { display: inline-flex; padding: 3px; border-radius: 999px; background: var(--map-glass, rgba(14, 19, 34, 0.82)); border: 1px solid var(--map-line, rgba(255,255,255,0.12)); backdrop-filter: blur(12px); }
  .seg button { border: 0; background: transparent; color: #aab3d0; padding: 6px 14px; border-radius: 999px; font: 600 12.5px/1 inherit; cursor: pointer; }
  .seg button.on { background: var(--map-accent, #8b6cff); color: #fff; box-shadow: 0 2px 10px rgba(139, 108, 255, 0.45); }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; }
  .chip { display: inline-flex; align-items: center; gap: 6px; padding: 7px 12px; border-radius: 999px; border: 1px solid var(--map-line, rgba(255,255,255,0.12)); background: var(--map-glass, rgba(14, 19, 34, 0.82)); color: #aab3d0; font: 600 12.5px/1 inherit; cursor: pointer; backdrop-filter: blur(12px); transition: color .12s, border-color .12s, background .12s; }
  .chip b { font-weight: 700; opacity: 0.8; }
  .chip.on { color: #fff; border-color: color-mix(in srgb, var(--map-accent, #8b6cff) 70%, transparent); background: color-mix(in srgb, var(--map-accent, #8b6cff) 26%, rgba(14, 19, 34, 0.85)); }
  .chip:disabled { opacity: 0.5; cursor: default; }
  .chip.solo { color: #fff; cursor: default; }
  .right { position: relative; display: flex; flex-direction: column; align-items: flex-end; gap: 6px; }
  .plist { list-style: none; margin: 0; padding: 6px; width: 230px; max-height: 320px; overflow: auto; border-radius: 16px; background: var(--map-glass, rgba(14, 19, 34, 0.92)); border: 1px solid var(--map-line, rgba(255,255,255,0.12)); backdrop-filter: blur(14px); box-shadow: 0 14px 40px rgba(0,0,0,0.45); }
  .plist button { display: flex; align-items: center; gap: 9px; width: 100%; padding: 7px 9px; border: 0; border-radius: 11px; background: transparent; color: inherit; font: 600 13px/1 inherit; cursor: pointer; text-align: left; }
  .plist li { display: flex; align-items: center; gap: 3px; }
  .plist input { accent-color: var(--map-accent, #8b6cff); flex: 0 0 auto; }
  .plist .options { justify-content: space-between; padding: 6px 9px; border-bottom: 1px solid var(--map-line, rgba(255,255,255,0.12)); margin-bottom: 3px; font-size: 12px; }
  .plist .options label { display: flex; align-items: center; gap: 4px; white-space: nowrap; }
  .plist .options button { width: auto; padding: 4px 6px; font-size: 12px; }
  .plist button:hover { background: rgba(255,255,255,0.08); }
  .plist img { border-radius: 6px; image-rendering: pixelated; }
  .nm { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .where { font-size: 11px; color: #8d97b8; text-transform: capitalize; }
  .bottom-left { position: absolute; left: 12px; bottom: 12px; display: flex; flex-direction: column; gap: 6px; align-items: flex-start; pointer-events: none; }
  .hint, .coords { padding: 6px 11px; border-radius: 999px; background: var(--map-glass, rgba(14, 19, 34, 0.82)); border: 1px solid var(--map-line, rgba(255,255,255,0.1)); font: 600 12px/1 inherit; backdrop-filter: blur(10px); }
  .coords { color: #c3cbe6; font-variant-numeric: tabular-nums; }
  .coords i { font-style: normal; color: #7f89ab; margin-left: 10px; }
  .zoom { position: absolute; right: 12px; bottom: 12px; display: flex; flex-direction: column; align-items: flex-end; gap: 8px; }
  .round, .stack button { width: 38px; height: 38px; display: grid; place-items: center; border: 1px solid var(--map-line, rgba(255,255,255,0.12)); background: var(--map-glass, rgba(14, 19, 34, 0.86)); color: #e7ebff; cursor: pointer; backdrop-filter: blur(12px); font: 600 20px/1 inherit; }
  .round { border-radius: 50%; }
  .stack { display: flex; flex-direction: column; border-radius: 14px; overflow: hidden; border: 1px solid var(--map-line, rgba(255,255,255,0.12)); }
  .stack button { border: 0; border-radius: 0; }
  .stack button + button { border-top: 1px solid rgba(255,255,255,0.1); }
  .round:hover, .stack button:hover { background: rgba(60, 70, 110, 0.9); }
  .card { position: absolute; left: 12px; bottom: 62px; width: min(320px, calc(100% - 24px)); max-height: 55%; overflow: auto; padding: 14px 16px 16px; border-radius: 18px; background: var(--map-glass, rgba(14, 19, 34, 0.92)); border: 1px solid var(--map-line, rgba(255,255,255,0.12)); backdrop-filter: blur(16px); box-shadow: 0 18px 50px rgba(0,0,0,0.5); animation: rise .16s ease; }
  .card.wide { left: auto; right: 12px; bottom: 12px; top: 64px; width: min(340px, calc(100% - 24px)); max-height: none; padding: 0; background: transparent; border: 0; box-shadow: none; backdrop-filter: none; }
  .card h3 { margin: 0 0 6px; font-size: 15px; }
  .card p { margin: 3px 0; font-size: 13px; color: #d4daf0; }
  .card .muted { color: #8d97b8; }
  .head { display: flex; gap: 12px; align-items: center; margin-bottom: 8px; }
  .head img { border-radius: 10px; image-rendering: pixelated; }
  .head img.icon { image-rendering: auto; border-radius: 50%; object-fit: cover; }
  .x { position: absolute; top: 8px; right: 10px; width: 26px; height: 26px; border-radius: 50%; border: 0; background: rgba(255,255,255,0.08); color: #fff; font-size: 18px; line-height: 1; cursor: pointer; z-index: 2; }
  .card.wide .x { display: none; }
  .primary { margin-top: 10px; width: 100%; padding: 9px 14px; border: 0; border-radius: 12px; background: var(--map-accent, #8b6cff); color: #fff; font: 700 13px/1 inherit; cursor: pointer; }
  .veil { position: absolute; inset: 0; display: flex; flex-direction: column; gap: 8px; align-items: center; justify-content: center; text-align: center; padding: 24px; background: radial-gradient(circle at 50% 40%, rgba(139,108,255,0.14), rgba(9,12,22,0.92) 60%); }
  .veil span { color: #98a2c4; max-width: 34ch; font-size: 13px; line-height: 1.5; }
  .spinner { width: 34px; height: 34px; border-radius: 50%; border: 3px solid rgba(255,255,255,0.14); border-top-color: var(--map-accent, #8b6cff); animation: spin .9s linear infinite; margin-bottom: 6px; }
  .toast { position: absolute; left: 50%; bottom: 14px; transform: translateX(-50%); padding: 8px 14px; border-radius: 999px; background: rgba(120, 30, 50, 0.9); font-size: 12.5px; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes rise { from { transform: translateY(6px); opacity: 0; } }
  @media (max-width: 640px) { .top { flex-direction: column; } .right { align-items: flex-start; } }
</style>
