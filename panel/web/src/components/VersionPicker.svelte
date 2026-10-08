<script lang="ts">
  import { get, LOADERS } from '../lib/api';

  let { mc = $bindable(''), loader = $bindable('vanilla'), loaderVersion = $bindable<string | null>(null), showLoader = true }: {
    mc: string; loader: string; loaderVersion: string | null; showLoader?: boolean;
  } = $props();

  type V = { id: string; kind: string };
  let versions = $state<V[]>([]);
  let snapshots = $state(false);
  let metaError = $state(false);
  let loaderVersions = $state<{ version: string; stable: boolean }[]>([]);
  let loadingLoaders = $state(false);

  let cache: { versions?: V[] } = (globalThis as any).__mcCache ??= {};

  $effect(() => {
    if (cache.versions) {
      versions = cache.versions;
      return;
    }
    get<{ versions: V[]; latest: { release: string } }>('/api/admin/meta/minecraft')
      .then((r) => {
        versions = cache.versions = r.versions;
        if (!mc) mc = r.latest.release;
      })
      .catch(() => (metaError = true));
  });

  $effect(() => {
    const l = loader, v = mc;
    loaderVersions = [];
    if (l === 'vanilla' || !v || !showLoader) return;
    loadingLoaders = true;
    get<{ version: string; stable: boolean }[]>(`/api/admin/meta/loaders/${l}?mc=${encodeURIComponent(v)}`)
      .then((r) => (loaderVersions = r))
      .catch(() => (loaderVersions = []))
      .finally(() => (loadingLoaders = false));
  });

  const shown = $derived(versions.filter((v) => snapshots || v.kind === 'release'));
</script>

<div class="picker">
  <label class="field">
    Minecraft version
    <div class="row">
      {#if metaError || versions.length === 0}
        <input bind:value={mc} placeholder="26.3" list="mc-versions" />
      {:else}
        <select bind:value={mc}>
          {#if mc && !shown.some((v) => v.id === mc)}<option value={mc}>{mc}</option>{/if}
          {#each shown as v}<option value={v.id}>{v.id}{v.kind !== 'release' ? ` (${v.kind})` : ''}</option>{/each}
        </select>
      {/if}
      <label class="snap" title="Show snapshots and old versions"><input type="checkbox" bind:checked={snapshots} /> All</label>
    </div>
    {#if metaError}<span class="help">Couldn't reach Mojang — type the version manually.</span>{/if}
  </label>

  {#if showLoader}
    <div class="field">
      <span class="lbl">Mod loader</span>
      <div class="segmented loaders">
        {#each LOADERS as l}
          <button type="button" class:active={loader === l.id} onclick={() => { loader = l.id; loaderVersion = null; }}>{l.label}</button>
        {/each}
      </div>
    </div>

    {#if loader !== 'vanilla'}
      <label class="field">
        Loader version
        {#if loaderVersions.length}
          <select value={loaderVersion ?? ''} onchange={(e) => (loaderVersion = (e.target as HTMLSelectElement).value || null)}>
            <option value="">Latest recommended</option>
            {#if loaderVersion && !loaderVersions.some((v) => v.version === loaderVersion)}<option value={loaderVersion}>{loaderVersion}</option>{/if}
            {#each loaderVersions as v}<option value={v.version}>{v.version}{v.stable ? ' ★' : ''}</option>{/each}
          </select>
        {:else}
          <input value={loaderVersion ?? ''} oninput={(e) => (loaderVersion = (e.target as HTMLInputElement).value || null)} placeholder={loadingLoaders ? 'Loading…' : 'Latest (leave empty)'} />
          {#if !loadingLoaders}<span class="help">No version list available — leave empty to use the latest.</span>{/if}
        {/if}
      </label>
    {/if}
  {/if}
</div>

<style>
  .picker { display: flex; flex-direction: column; gap: 14px; }
  .field { display: flex; flex-direction: column; gap: 7px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .snap { display: flex; align-items: center; gap: 6px; font-size: 0.8rem; color: var(--muted); white-space: nowrap; cursor: pointer; }
  .loaders { flex-wrap: wrap; }
  .help { color: var(--muted); font-size: 0.78rem; }
</style>
