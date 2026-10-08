<script lang="ts">
  import { onMount } from 'svelte';
  import { get, post, put, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  let { onchange, onopen }: { onchange?: () => void; onopen?: (id: string) => void } = $props();
  let files = $state<string[]>([]);
  let settings = $state({ enabled: false, required: false, pack_format: 84, allow_vanilla_overrides: false });
  let path = $state('assets/scopenet/textures/item/');
  let busy = $state(false);
  async function load() { onchange?.(); try { const r = await get<{ files: string[]; settings: typeof settings }>('/api/admin/resource-assets'); files = r.files; settings = r.settings; } catch (e) { toastError(e); } }
  onMount(load);
  async function upload(file?: File) {
    if (!file) return;
    busy = true;
    try {
      if (file.size > 2 * 1024 * 1024) throw new Error('Assets must be at most 2 MiB');
      const data = await new Promise<string>((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result).split(',')[1]); reader.onerror = reject; reader.readAsDataURL(file); });
      await post('/api/admin/resource-assets', { path: path.endsWith('/') ? path + file.name.toLowerCase() : path, data });
      toast('Asset uploaded'); await load();
    } catch (e) { toastError(e); } finally { busy = false; }
  }
  type Problem = { model: string; kind: string; reference: string };
  type Left = { path: string; reason: string };
  let health = $state<{ models: number; textures: number; problems: Problem[]; left_out: { count: number; removed: Left[] } } | null>(null);
  async function check() { try { health = await get('/api/admin/resource-assets-check'); } catch (e) { toastError(e); } }
  async function save() { try { await put('/api/admin/resource-pack', settings); toast('Resource pack settings saved'); } catch (e) { toastError(e); } }
  async function remove(file: string) { if (!confirm(`Delete ${file}?`)) return; try { await del(`/api/admin/resource-assets/${file}`); await load(); } catch (e) { toastError(e); } }
</script>
<section class="card assets">
  <h2>Server assets &amp; resource pack</h2>
  <p>Upload PNG textures, JSON models, fonts and item definitions, or Ogg sounds. Velora combines them with custom item models into a downloadable pack and offers it to players on supported Paper and Fabric servers.</p>
  <label>Asset path<input bind:value={path} placeholder="assets/scopenet/textures/item/blade.png" /></label>
  <label>Upload asset<input type="file" accept=".png,.json,.ogg" disabled={busy} onchange={e => upload(e.currentTarget.files?.[0])} /></label>
  <div class="row">
    <label><input type="checkbox" bind:checked={settings.enabled} /> Offer pack to players</label>
    <label><input type="checkbox" bind:checked={settings.required} /> Require acceptance</label>
    <label title="Imported packs sometimes replace vanilla block models, textures, colour maps or shaders. Those are what make grass solid green or leave one face of a stair untextured, so they are left out unless you turn this on."><input type="checkbox" bind:checked={settings.allow_vanilla_overrides} /> Keep vanilla block overrides</label>
    <label title="Legacy label only; the pack declares a range that covers all current versions">Pack format<input type="number" min="1" max="1000" bind:value={settings.pack_format} /></label>
    <button onclick={save}>Save pack settings</button>
    <a href="/api/v1/resource-pack.zip" download>Download resource pack</a>
    <button type="button" class="ghost" onclick={check}>Check models</button>
  </div>
  {#if health}
    <div class="health" class:bad={health.problems.length}>
      {#if !health.problems.length}
        <b>All good.</b> {health.models} models and {health.textures} textures; every texture and parent model they name is in the pack.
      {:else}
        <b>{health.problems.length} problem{health.problems.length > 1 ? 's' : ''}</b> — these models name files the pack doesn't contain, so they show as black and magenta in game:
        <ul>{#each health.problems.slice(0, 40) as pr}<li><code>{pr.model}</code> needs {pr.kind} <code>{pr.reference}</code></li>{/each}</ul>
        {#if health.problems.length > 40}<small>…and {health.problems.length - 40} more</small>{/if}
        <small>Upload the missing files, or import the whole pack folder again. References missing only a namespace are repaired automatically when the pack is built.</small>
      {/if}
    </div>
    {#if health.left_out.count}
      <div class="health">
        <b>{health.left_out.count} file{health.left_out.count > 1 ? 's' : ''} kept out of the pack</b> so the world's textures stay intact:
        <ul>{#each health.left_out.removed.slice(0, 30) as r}<li><code>{r.path}</code> — {r.reason}</li>{/each}</ul>
        {#if health.left_out.count > 30}<small>…and {health.left_out.count - 30} more</small>{/if}
      </div>
    {/if}
  {/if}
  <p>The pack loads on every Minecraft version from 1.20.2 through 26.x, so there is nothing to match to your clients; the number only labels the pack for very old versions (default 84, for 26.x). Textured items use a vanilla base item and a unique Custom Model Data number. Special bases such as bows, shields and armor need an uploaded base model to preserve their vanilla appearance and animations.</p>
  {#each files as file}<div class="row"><code>{file}</code><button class="ghost" onclick={() => remove(file)}>Delete</button></div>{/each}
</section>
<style>
  .assets { display: flex; flex-direction: column; gap: 12px; }
  .assets h2, .assets p { margin: 0; }
  .assets p { color: var(--muted); font-size: 0.85rem; }
  label { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; }
  code { overflow-wrap: anywhere; }
  .health { padding: 10px 12px; border-radius: 10px; background: color-mix(in srgb, #2ecc71 14%, transparent); font-size: 0.85rem; }
  .health.bad { background: color-mix(in srgb, #ff8a3d 16%, transparent); }
  .health ul { margin: 6px 0; padding-left: 18px; }
  .health small { display: block; color: var(--muted); }
</style>
