<script lang="ts">
  import { Download, Upload, Trash2, LoaderCircle } from '@lucide/svelte';
  import McItem from './McItem.svelte';
  import McSprite from './McSprite.svelte';
  import { api, del, get, post } from '../lib/api';
  import { loadMc, mc } from '../lib/mc.svelte';
  import { toast, toastError } from '../lib/toast.svelte';

  type State = { ready: boolean; count: number; version: string | null; source: string | null; auto: boolean; items: string[] };
  let st = $state<State | null>(null);
  let version = $state('');
  let busy = $state<'' | 'fetch' | 'upload' | 'clear'>('');
  let input: HTMLInputElement | undefined = $state();

  const load = () => get<State>('/api/admin/mc-textures').then((s) => (st = s)).catch(toastError);
  $effect(() => { load(); });
  const sample = ['diamond_sword', 'golden_apple', 'enchanted_golden_apple', 'ender_pearl', 'emerald', 'netherite_pickaxe', 'totem_of_undying', 'elytra'];

  async function refresh() { await load(); await loadMc(true); }

  async function fetchNow() {
    busy = 'fetch';
    try {
      const r = await post<{ version: string }>('/api/admin/mc-textures/fetch', { version: version.trim() || undefined });
      toast(`Minecraft ${r.version ?? ''} textures installed`);
      await refresh();
    } catch (e) { toastError(e); } finally { busy = ''; }
  }

  async function upload(e: Event) {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    busy = 'upload';
    try {
      const form = new FormData();
      form.append('file', file);
      await api('/api/admin/mc-textures/upload', { method: 'POST', form });
      toast('Textures installed from your client.jar');
      await refresh();
    } catch (err) { toastError(err); } finally { busy = ''; if (input) input.value = ''; }
  }

  async function clear() {
    if (!confirm('Remove the Minecraft textures? Pages go back to their built-in icons until you install them again.')) return;
    busy = 'clear';
    try { await del('/api/admin/mc-textures'); await refresh(); } catch (e) { toastError(e); } finally { busy = ''; }
  }
</script>

<section class="card col mct">
  <div class="section-title"><Download size={18} /><h2>Minecraft textures</h2></div>
  <p class="help">
    Item icons, hearts, effect icons and the title-screen panorama used across the panel, the player site and your landing page.
    They are never stored in Velora's source or images: this server downloads the official client from Mojang, or you can upload a vanilla <code>client.jar</code>
    (from <code>.minecraft/versions/&lt;version&gt;/&lt;version&gt;.jar</code>) when the host has no internet access. They remain Mojang's property; use them for your own server only.
  </p>

  {#if st}
    <div class="state" class:ok={st.ready}>
      {#if st.ready}
        <div class="sample">{#each sample as id}<McItem {id} size={32} slot />{/each}</div>
        <div class="meta">
          <strong>Installed — Minecraft {st.version ?? 'unknown version'}</strong>
          <span>{st.count.toLocaleString()} textures · {st.source === 'upload' ? 'uploaded jar' : st.source === 'mojang' ? 'downloaded from Mojang' : 'local install'}</span>
        </div>
        <div class="hud" aria-hidden="true"><McSprite path="hud/heart/full" width={9} height={9} scale={2} /><McSprite path="hud/heart/full" width={9} height={9} scale={2} /><McSprite path="hud/food_full" width={9} height={9} scale={2} /></div>
      {:else}
        <div class="meta">
          <strong>Not installed</strong>
          <span>{st.auto ? 'The panel tries to download them on start-up; use the buttons below if that has not worked.' : 'Automatic download is switched off (VELORA_MC_TEXTURES=off).'}</span>
        </div>
      {/if}
    </div>

    <div class="row wrap">
      <label class="ver">Version <input bind:value={version} placeholder="latest release" maxlength="40" spellcheck="false" /></label>
      <button class="primary" disabled={!!busy} onclick={fetchNow}>
        {#if busy === 'fetch'}<LoaderCircle size={15} class="spin" /> Downloading…{:else}<Download size={15} /> {st.ready ? 'Re-download from Mojang' : 'Download from Mojang'}{/if}
      </button>
      <button disabled={!!busy} onclick={() => input?.click()}>
        {#if busy === 'upload'}<LoaderCircle size={15} class="spin" /> Reading jar…{:else}<Upload size={15} /> Upload client.jar{/if}
      </button>
      <input bind:this={input} type="file" accept=".jar,.zip,application/java-archive" hidden onchange={upload} />
      {#if st.ready}<button class="danger" disabled={!!busy} onclick={clear}><Trash2 size={15} /> Remove</button>{/if}
    </div>
  {:else}
    <p class="help">Loading…</p>
  {/if}
</section>

<style>
  .state { display: flex; align-items: center; gap: 16px; flex-wrap: wrap; padding: 12px 14px; border-radius: var(--radius-sm); background: var(--bg-2); border: 1px solid var(--line); }
  .state.ok { border-color: color-mix(in srgb, var(--good) 45%, var(--line)); }
  .sample { display: flex; gap: 4px; flex-wrap: wrap; }
  .meta { display: grid; gap: 2px; flex: 1; min-width: 14rem; }
  .meta span { color: var(--muted); font-size: 0.8rem; }
  .hud { display: flex; gap: 1px; }
  .help { color: var(--muted); font-size: 0.8rem; line-height: 1.5; margin: 0; }
  .ver { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: var(--text-2); }
  .ver input { width: 9rem; }
  :global(.spin) { animation: mc-spin 1s linear infinite; }
  @keyframes mc-spin { to { transform: rotate(360deg); } }
</style>
