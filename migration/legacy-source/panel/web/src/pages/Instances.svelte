<script lang="ts">
  import { Plus, Boxes, Star, Lock, Users as UsersIcon, Globe, Server, TriangleAlert, EyeOff, Package } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import VersionPicker from '../components/VersionPicker.svelte';
  import { formatBytes, get, loaderLabel, post } from '../lib/api';
  import { go } from '../lib/router.svelte';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Instance } from '../lib/types';

  let instances = $state<Instance[] | null>(null);
  let creating = $state(false);
  let busy = $state(false);
  let draft = $state({ name: '', description: '', mc_version: '', loader: 'vanilla', loader_version: null as string | null, source: 'vanilla' });

  $effect(() => {
    get<Instance[]>('/api/admin/instances').then((i) => (instances = i)).catch(toastError);
  });

  async function create() {
    busy = true;
    try {
      const inst = await post<Instance>('/api/admin/instances', {
        name: draft.name,
        description: draft.description,
        mc_version: draft.mc_version || '26.3',
        loader: draft.source === 'vanilla' ? draft.loader : 'vanilla',
        loader_version: draft.loader_version,
      });
      toast(`Created ${inst.name}`);
      go(`instance/${encodeURIComponent(inst.id)}/installation`);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }

  const visIcon = { public: Globe, members: UsersIcon, groups: Lock };
  const visLabel = { public: 'Everyone', members: 'Signed-in players', groups: 'Specific groups' };
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Instances</h1>
      <p>Each instance is a playable profile in the launcher: a Minecraft version or a modpack, plus an optional server.</p>
    </div>
    <button class="primary" onclick={() => (creating = true)}><Plus size={18} /> New instance</button>
  </div>

  {#if instances === null}
    <div class="grid cards">{#each [1, 2, 3] as _}<div class="card sk"></div>{/each}</div>
  {:else if instances.length === 0}
    <div class="card empty">
      <Boxes size={40} />
      <h3>No instances yet</h3>
      <p>Create one to give players something to launch.</p>
      <button class="primary" style="margin-top: 16px" onclick={() => (creating = true)}><Plus size={18} /> New instance</button>
    </div>
  {:else}
    <div class="grid cards">
      {#each instances as inst (inst.id)}
        {@const Vis = visIcon[inst.visibility]}
        <a class="card hover inst" href="#/instance/{inst.id}" class:disabled={!inst.enabled}>
          <div class="banner" style:background-image={inst.banner_url ? `url("${inst.banner_url}")` : undefined}>
            <div class="badges">
              {#if inst.featured}<span class="badge accent"><Star size={11} /> Featured</span>{/if}
              {#if !inst.enabled}<span class="badge"><EyeOff size={11} /> Hidden</span>{/if}
              {#if inst.missing_count}<span class="badge warn"><TriangleAlert size={11} /> {inst.missing_count} missing</span>{/if}
            </div>
          </div>
          <div class="body">
            <div class="icon">
              {#if inst.icon_url}<img src={inst.icon_url} alt="" />{:else}<Package size={22} />{/if}
            </div>
            <h3>{inst.name}</h3>
            <p class="muted small desc">{inst.description || inst.source_label}</p>
            <div class="row wrap chips">
              <span class="badge">{inst.mc_version}</span>
              <span class="badge">{loaderLabel(inst.loader)}{inst.loader_version ? ` ${inst.loader_version.replace(`${inst.mc_version}-`, '')}` : ''}</span>
              {#if inst.file_count}<span class="badge">{inst.file_count} files · {formatBytes(inst.total_size)}</span>{/if}
            </div>
            <div class="foot">
              <span class="row small muted"><Vis size={14} /> {visLabel[inst.visibility]}</span>
              {#if inst.server?.address}<span class="row small muted"><Server size={14} /> {inst.server.address}</span>{/if}
            </div>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>

<Modal bind:open={creating} title="New instance" width={560}>
  <label class="field">Name<input bind:value={draft.name} placeholder="Survival SMP" /></label>
  <label class="field">Description <span class="help">Shown under the name in the launcher.</span>
    <input bind:value={draft.description} placeholder="Our main survival world — season 4" />
  </label>
  <div class="field">
    <span class="lbl">Start from</span>
    <div class="sources">
      {#each [
        { id: 'vanilla', title: 'Minecraft version', text: 'Vanilla or a bare mod loader' },
        { id: 'modpack', title: 'Modpack', text: 'Modrinth, CurseForge or a .zip' },
      ] as s}
        <button type="button" class="source" class:active={draft.source === s.id} onclick={() => (draft.source = s.id)}>
          <strong>{s.title}</strong><span>{s.text}</span>
        </button>
      {/each}
    </div>
  </div>
  {#if draft.source === 'vanilla'}
    <VersionPicker bind:mc={draft.mc_version} bind:loader={draft.loader} bind:loaderVersion={draft.loader_version} />
  {:else}
    <p class="muted small">You'll pick the modpack on the next screen. Its Minecraft version and loader are set automatically.</p>
  {/if}
  {#snippet footer()}
    <button class="ghost" onclick={() => (creating = false)}>Cancel</button>
    <button class="primary" disabled={!draft.name.trim() || busy} onclick={create}>Create instance</button>
  {/snippet}
</Modal>

<style>
  .cards { grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); }
  .sk { height: 280px; opacity: 0.5; }
  .inst { padding: 0; overflow: hidden; color: var(--text); display: flex; flex-direction: column; }
  .inst.disabled { opacity: 0.6; }
  .banner {
    height: 96px; background: var(--surface-2); border-bottom: 1px solid var(--line);
    background-size: cover; background-position: center; position: relative;
  }
  .badges { position: absolute; top: 10px; right: 10px; display: flex; gap: 6px; }
  .badges .badge { backdrop-filter: blur(8px); background: rgba(10, 11, 20, 0.6); }
  .body { padding: 0 18px 18px; display: flex; flex-direction: column; gap: 8px; flex: 1; }
  .icon { width: 54px; height: 54px; border-radius: 14px; margin-top: -28px; background: var(--surface-3); border: 3px solid var(--surface); display: grid; place-items: center; overflow: hidden; color: var(--muted); position: relative; }
  .icon img { width: 100%; height: 100%; object-fit: cover; }
  .desc { min-height: 2.6em; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .chips { gap: 6px; }
  .foot { display: flex; justify-content: space-between; gap: 10px; margin-top: auto; padding-top: 12px; border-top: 1px solid var(--line); }
  .foot .row { gap: 6px; min-width: 0; overflow: hidden; white-space: nowrap; }
  .field { display: flex; flex-direction: column; gap: 7px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .sources { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .source { flex-direction: column; align-items: flex-start; gap: 4px; padding: 14px; background: var(--bg-2); text-align: left; white-space: normal; }
  .source span { font-size: 0.8rem; color: var(--muted); font-weight: 400; }
  .source.active { border-color: var(--accent); background: var(--accent-soft); }
</style>
