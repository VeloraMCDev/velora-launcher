<script lang="ts">
  import {
    ArrowLeft, Save, Trash2, Search, Download, Upload, LoaderCircle, TriangleAlert, FileArchive, Package, Globe, Lock,
    Users as UsersIcon, Server, RotateCcw, ExternalLink, FolderPlus, X,
  } from '@lucide/svelte';
  import ImageInput from '../components/ImageInput.svelte';
  import Modal from '../components/Modal.svelte';
  import Toggle from '../components/Toggle.svelte';
  import VersionPicker from '../components/VersionPicker.svelte';
  import { api, del, formatBytes, get, loaderLabel, post, put } from '../lib/api';
  import { go, route } from '../lib/router.svelte';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { FileRow, Group, Instance } from '../lib/types';

  let { id }: { id: string } = $props();

  type Tab = 'general' | 'source' | 'files' | 'server' | 'access';
  const tabs: { id: Tab; label: string }[] = [
    { id: 'general', label: 'General' },
    { id: 'source', label: 'Version & modpack' },
    { id: 'files', label: 'Files' },
    { id: 'server', label: 'Server' },
    { id: 'access', label: 'Access' },
  ];
  let tab = $state<Tab>((route.params[1] as Tab) ?? 'general');

  let inst = $state<Instance | null>(null);
  let form = $state<Instance | null>(null);
  let files = $state<FileRow[]>([]);
  let groups = $state<Group[]>([]);
  let saving = $state(false);
  let importing = $state<string | null>(null);
  let confirmDelete = $state(false);

  function load(detail: { instance: Instance; files: FileRow[] }) {
    inst = detail.instance;
    form = structuredClone($state.snapshot(detail.instance)) as Instance;
    form.server ??= { name: '', address: '', port: 25565, auto_join: false, inject: true };
    files = detail.files;
  }

  $effect(() => {
    get(`/api/admin/instances/${encodeURIComponent(id)}`).then(load).catch((e) => {
      toastError(e);
      go('instances');
    });
    get<Group[]>('/api/admin/groups').then((g) => (groups = g)).catch(() => {});
  });

  const dirty = $derived(!!inst && !!form && JSON.stringify(normalize(form)) !== JSON.stringify(normalize(inst)));

  function normalize(i: Instance) {
    const s = i.server && i.server.address.trim() ? i.server : null;
    return { ...i, server: s, revision: 0, updated_at: '' };
  }

  async function save() {
    if (!form) return;
    saving = true;
    try {
      const body = { ...$state.snapshot(form), server: form.server?.address.trim() ? form.server : null };
      load(await put(`/api/admin/instances/${encodeURIComponent(id)}`, body));
      toast('Saved — players get the update on their next launch');
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    try {
      await del(`/api/admin/instances/${encodeURIComponent(id)}`);
      toast('Instance deleted');
      go('instances');
    } catch (e) {
      toastError(e);
    }
  }

  async function runImport(label: string, fn: () => Promise<any>) {
    importing = label;
    try {
      load(await fn());
      toast('Modpack imported');
      tab = 'files';
    } catch (e) {
      toastError(e);
    } finally {
      importing = null;
    }
  }

  // ---- source pickers ----
  let sourceMode = $state<'version' | 'modrinth' | 'curseforge' | 'upload'>('version');
  let query = $state('');
  let results = $state<any[]>([]);
  let searching = $state(false);
  let selected = $state<any>(null);
  let versions = $state<any[]>([]);
  let loadingVersions = $state(false);

  $effect(() => {
    if (inst && (inst.source_kind === 'modrinth' || inst.source_kind === 'curseforge')) sourceMode = inst.source_kind;
  });

  async function search() {
    searching = true;
    selected = null;
    try {
      if (sourceMode === 'modrinth') {
        const r = await get(`/api/admin/modrinth/search?q=${encodeURIComponent(query)}`);
        results = r.hits.map((h: any) => ({ id: h.project_id, title: h.title, text: h.description, icon: h.icon_url, downloads: h.downloads, author: h.author }));
      } else {
        const r = await get<any[]>(`/api/admin/curseforge/search?q=${encodeURIComponent(query)}`);
        results = r.map((m: any) => ({ id: m.id, title: m.name, text: m.summary, icon: m.logo?.thumbnailUrl, downloads: m.downloadCount, author: m.authors?.[0]?.name }));
      }
    } catch (e) {
      toastError(e);
    } finally {
      searching = false;
    }
  }

  async function pick(p: any) {
    selected = p;
    versions = [];
    loadingVersions = true;
    try {
      if (sourceMode === 'modrinth') {
        const v = await get<any[]>(`/api/admin/modrinth/project/${p.id}/versions`);
        versions = v.map((x) => ({ id: x.id, name: x.name || x.version_number, meta: [...x.game_versions.slice(-2), ...x.loaders].join(' · '), date: x.date_published }));
      } else {
        const v = await get<any[]>(`/api/admin/curseforge/mod/${p.id}/files`);
        versions = v.map((x) => ({ id: x.id, name: x.displayName, meta: (x.gameVersions ?? []).join(' · '), date: x.fileDate }));
      }
    } catch (e) {
      toastError(e);
    } finally {
      loadingVersions = false;
    }
  }

  function importVersion(v: any) {
    const path = `/api/admin/instances/${encodeURIComponent(id)}/import/`;
    if (sourceMode === 'modrinth') runImport(`${selected.title} ${v.name}`, () => post(path + 'modrinth', { version_id: v.id, project_id: selected.id }));
    else runImport(`${selected.title} ${v.name}`, () => post(path + 'curseforge', { mod_id: selected.id, file_id: v.id }));
  }

  // ---- upload ----
  let dragging = $state(false);
  let uploadFile = $state<File | null>(null);
  let fallback = $state({ mc: '', loader: 'vanilla', lv: null as string | null });

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const f = e.dataTransfer?.files?.[0];
    if (f) uploadFile = f;
  }

  function doUpload() {
    if (!uploadFile) return;
    const form = new FormData();
    form.append('mc_version', fallback.mc);
    form.append('loader', fallback.loader);
    if (fallback.lv) form.append('loader_version', fallback.lv);
    form.append('file', uploadFile);
    runImport(uploadFile.name, () => api(`/api/admin/instances/${encodeURIComponent(id)}/import/upload`, { method: 'POST', form }));
    uploadFile = null;
  }

  async function resetSource() {
    try {
      load(await post(`/api/admin/instances/${encodeURIComponent(id)}/reset`));
      toast('Modpack removed — the instance is now a plain version');
    } catch (e) {
      toastError(e);
    }
  }

  let cleaning = $state(false);
  async function cleanUpdate() {
    if (!confirm('Push a clean update?\n\nOn their next launch every player\'s launcher will delete this instance\'s old files (every mod and config it ever received, any mod the player added by hand, and the cached server resource pack) and download everything fresh.\n\nWorlds, screenshots and settings are not touched.')) return;
    cleaning = true;
    try {
      load(await post(`/api/admin/instances/${encodeURIComponent(id)}/clean-update`));
      toast('Clean update published — players refresh everything on their next launch');
    } catch (e) {
      toastError(e);
    } finally {
      cleaning = false;
    }
  }

  // ---- files ----
  let folder = $state('mods');
  let customFolder = $state('');
  let fileInput = $state<HTMLInputElement>();
  let uploadingFiles = $state(false);
  const missing = $derived(files.filter((f) => !f.url));

  async function addFiles(e: Event) {
    const list = (e.target as HTMLInputElement).files;
    if (!list?.length) return;
    uploadingFiles = true;
    const form = new FormData();
    form.append('folder', folder === 'custom' ? customFolder : folder);
    for (const f of list) form.append('file', f);
    try {
      load(await api(`/api/admin/instances/${encodeURIComponent(id)}/files`, { method: 'POST', form }));
      toast(`Uploaded ${list.length} file${list.length > 1 ? 's' : ''}`);
    } catch (err) {
      toastError(err);
    } finally {
      uploadingFiles = false;
      if (fileInput) fileInput.value = "";
    }
  }

  async function deleteFile(path: string) {
    try {
      load(await del(`/api/admin/instances/${encodeURIComponent(id)}/files?path=${encodeURIComponent(path)}`));
    } catch (e) {
      toastError(e);
    }
  }

  const originBadge: Record<string, string> = { pack: 'accent', override: '', upload: 'good' };
  const noteLink = (n: string | null) => n?.match(/https?:\/\/\S+/)?.[0]?.replace(/\.$/, '');
</script>

{#if form && inst}
  <div class="page">
    <a href="#/instances" class="back"><ArrowLeft size={16} /> All instances</a>
    <div class="page-head">
      <div class="row head">
        <div class="icon">{#if form.icon_url}<img src={form.icon_url} alt="" />{:else}<Package size={26} />{/if}</div>
        <div>
          <h1>{inst.name}</h1>
          <p class="row wrap">
            <span class="badge">{inst.mc_version}</span>
            <span class="badge">{loaderLabel(inst.loader)} {inst.loader_version?.replace(`${inst.mc_version}-`, '') ?? ''}</span>
            <span class="badge accent">{inst.source_label || 'Vanilla'}</span>
            <span class="muted tiny">revision {inst.revision}</span>
          </p>
        </div>
      </div>
      <button class="danger" onclick={() => (confirmDelete = true)}><Trash2 size={16} /> Delete</button>
    </div>

    <div class="tabs">
      {#each tabs as t}
        <button class:active={tab === t.id} onclick={() => (tab = t.id)}>
          {t.label}
          {#if t.id === 'files' && missing.length}<span class="badge warn">{missing.length}</span>{/if}
        </button>
      {/each}
    </div>

    {#if tab === 'general'}
      <div class="grid two">
        <section class="card col">
          <h3>Details</h3>
          <label class="field">Name<input bind:value={form.name} /></label>
          <label class="field">Description<textarea bind:value={form.description} rows="3" placeholder="What makes this instance special?"></textarea></label>
          <ImageInput bind:value={form.icon_url} label="Icon" help="Square image, shown in the launcher sidebar." />
          <ImageInput bind:value={form.logo_url} label="Logo (optional)" help="Wide logo or wordmark image used in place of the instance title text." />
          <ImageInput bind:value={form.banner_url} label="Banner / background" help="Wide image shown behind this instance. Overrides the launcher background." />
        </section>
        <section class="card col">
          <h3>Performance defaults</h3>
          <p class="muted small">Players can change these in their settings; these are the starting values.</p>
          <label class="field">
            <span class="row">Maximum memory <span class="spacer"></span><strong>{(form.memory.max_mb / 1024).toFixed(1)} GB</strong></span>
            <input type="range" min="1024" max="16384" step="256" bind:value={form.memory.max_mb} />
          </label>
          <label class="field">
            <span class="row">Minimum memory <span class="spacer"></span><strong>{(form.memory.min_mb / 1024).toFixed(1)} GB</strong></span>
            <input type="range" min="512" max={form.memory.max_mb} step="256" bind:value={form.memory.min_mb} />
          </label>
          <label class="field">Extra JVM arguments <span class="help">Added before the player's own arguments.</span>
            <input class="mono" bind:value={form.jvm_args} placeholder="-Dfml.readTimeout=180" />
          </label>
        </section>
      </div>
    {:else if tab === 'source'}
      <section class="card current">
        <FileArchive size={22} />
        <div class="grow">
          <strong>{inst.source_label || 'Vanilla'}</strong>
          <span class="muted small">Minecraft {inst.mc_version} · {loaderLabel(inst.loader)} {inst.loader_version ?? ''} · {inst.file_count} files ({formatBytes(inst.total_size)})</span>
        </div>
        {#if inst.source_kind !== 'vanilla'}
          <button class="sm" onclick={resetSource}><RotateCcw size={14} /> Remove modpack</button>
        {/if}
      </section>

      <section class="card current">
        <RotateCcw size={22} />
        <div class="grow">
          <strong>Clean update</strong>
          <span class="muted small">Make every player's launcher clear this instance's old files and download all of them again on the next launch.{inst.clean_epoch ? ` Pushed ${inst.clean_epoch} time${inst.clean_epoch === 1 ? '' : 's'} so far.` : ''}</span>
        </div>
        <button class="sm" onclick={cleanUpdate} disabled={cleaning}><RotateCcw size={14} /> {cleaning ? 'Publishing…' : 'Push clean update'}</button>
      </section>

      <div class="segmented modes">
        <button class:active={sourceMode === 'version'} onclick={() => (sourceMode = 'version')}><Package size={15} /> Version</button>
        <button class:active={sourceMode === 'modrinth'} onclick={() => { sourceMode = 'modrinth'; results = []; selected = null; }}>Modrinth</button>
        <button class:active={sourceMode === 'curseforge'} onclick={() => { sourceMode = 'curseforge'; results = []; selected = null; }}>CurseForge</button>
        <button class:active={sourceMode === 'upload'} onclick={() => (sourceMode = 'upload')}><Upload size={15} /> Upload .zip</button>
      </div>

      {#if sourceMode === 'version'}
        <section class="card col narrow">
          <p class="muted small">
            {#if inst.source_kind !== 'vanilla'}Changing versions here overrides what the modpack declared — only do this if you know the pack supports it.{:else}Pick the game version and (optionally) a mod loader. Add mods in the Files tab.{/if}
          </p>
          <VersionPicker bind:mc={form.mc_version} bind:loader={form.loader} bind:loaderVersion={form.loader_version} />
        </section>
      {:else if sourceMode === 'upload'}
        <section class="card col narrow">
          <div
            class="drop" class:dragging role="button" tabindex="0"
            ondragover={(e) => { e.preventDefault(); dragging = true; }}
            ondragleave={() => (dragging = false)}
            ondrop={onDrop}
            onclick={() => document.getElementById('pack-file')?.click()}
            onkeydown={(e) => e.key === 'Enter' && document.getElementById('pack-file')?.click()}
          >
            <Upload size={28} />
            {#if uploadFile}
              <strong>{uploadFile.name}</strong><span class="muted small">{formatBytes(uploadFile.size)}</span>
            {:else}
              <strong>Drop a modpack here</strong>
              <span class="muted small">.mrpack, CurseForge .zip, or any instance folder zipped up</span>
            {/if}
          </div>
          <input id="pack-file" type="file" accept=".zip,.mrpack" hidden onchange={(e) => (uploadFile = (e.target as HTMLInputElement).files?.[0] ?? null)} />
          <details>
            <summary class="small muted">Plain zip? Choose its Minecraft version and loader</summary>
            <div style="margin-top: 12px"><VersionPicker bind:mc={fallback.mc} bind:loader={fallback.loader} bind:loaderVersion={fallback.lv} /></div>
          </details>
          <button class="primary" disabled={!uploadFile} onclick={doUpload}><Upload size={16} /> Import</button>
        </section>
      {:else}
        <section class="card col">
          <form class="row" onsubmit={(e) => { e.preventDefault(); search(); }}>
            <input bind:value={query} placeholder="Search {sourceMode === 'modrinth' ? 'Modrinth' : 'CurseForge'} modpacks…" />
            <button class="primary" disabled={searching}>{#if searching}<LoaderCircle class="spin" size={16} />{:else}<Search size={16} />{/if} Search</button>
          </form>

          {#if selected}
            <div class="picked">
              <button class="ghost sm" onclick={() => (selected = null)}><ArrowLeft size={14} /> Results</button>
              <div class="row">
                {#if selected.icon}<img src={selected.icon} alt="" class="pi" />{/if}
                <div><h3>{selected.title}</h3><p class="muted small">{selected.text}</p></div>
              </div>
              {#if loadingVersions}
                <p class="muted row"><LoaderCircle class="spin" size={16} /> Loading versions…</p>
              {:else}
                <div class="versions">
                  {#each versions as v}
                    <div class="version">
                      <div class="grow"><strong>{v.name}</strong><span class="muted tiny">{v.meta}</span></div>
                      <span class="muted tiny">{v.date ? new Date(v.date).toLocaleDateString() : ''}</span>
                      <button class="sm primary" onclick={() => importVersion(v)}><Download size={14} /> Use</button>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {:else if results.length}
            <div class="results">
              {#each results as r}
                <button class="result" onclick={() => pick(r)}>
                  {#if r.icon}<img src={r.icon} alt="" />{:else}<span class="ph"><Package size={20} /></span>{/if}
                  <span class="grow">
                    <strong>{r.title}</strong>
                    <span class="muted tiny">{r.author ? `by ${r.author} · ` : ''}{Intl.NumberFormat(undefined, { notation: 'compact' }).format(r.downloads ?? 0)} downloads</span>
                    <span class="muted small clamp">{r.text}</span>
                  </span>
                </button>
              {/each}
            </div>
          {:else}
            <p class="muted small">
              {sourceMode === 'curseforge' ? 'CurseForge needs an API key (Settings → Integrations). ' : ''}Search for a modpack, then pick a version. Mods download straight from the CDN; configs are hosted by this panel.
            </p>
          {/if}
        </section>
      {/if}
    {:else if tab === 'files'}
      {#if missing.length}
        <section class="card warnbox">
          <div class="row"><TriangleAlert size={18} /><strong>{missing.length} file{missing.length > 1 ? 's' : ''} need a manual upload</strong></div>
          <p class="small">These CurseForge authors block third-party downloads. Download each file and upload it below into the same folder — it replaces the placeholder automatically.</p>
          {#each missing as m}
            <div class="row small miss">
              <code>{m.path}</code>
              <span class="spacer"></span>
              {#if noteLink(m.note)}<a href={noteLink(m.note)} target="_blank" rel="noreferrer" class="row"><ExternalLink size={13} /> Download page</a>{/if}
            </div>
          {/each}
        </section>
      {/if}

      <section class="card">
        <div class="section-title">
          <h3>{files.length} files</h3>
          <span class="spacer"></span>
          <select bind:value={folder} style="width: auto">
            <option value="mods">mods/</option>
            <option value="config">config/</option>
            <option value="resourcepacks">resourcepacks/</option>
            <option value="shaderpacks">shaderpacks/</option>
            <option value="">(game folder)</option>
            <option value="custom">custom…</option>
          </select>
          {#if folder === 'custom'}<input bind:value={customFolder} placeholder="kubejs/scripts" style="width: 180px" />{/if}
          <button class="primary sm" disabled={uploadingFiles} onclick={() => fileInput?.click()}>
            {#if uploadingFiles}<LoaderCircle class="spin" size={14} />{:else}<FolderPlus size={14} />{/if} Add files
          </button>
          <input bind:this={fileInput} type="file" multiple hidden onchange={addFiles} />
        </div>
        {#if files.length}
          <div class="table-wrap">
            <table class="table">
              <thead><tr><th>Path</th><th>Source</th><th>Size</th><th></th></tr></thead>
              <tbody>
                {#each files as f (f.path)}
                  <tr class:missing={!f.url}>
                    <td class="mono path">{f.path}</td>
                    <td>{#if !f.url}<span class="badge warn">missing</span>{:else}<span class="badge {originBadge[f.origin] ?? ''}">{f.origin === 'pack' ? 'CDN' : f.origin === 'upload' ? 'uploaded' : 'config'}</span>{/if}</td>
                    <td class="muted">{formatBytes(f.size)}</td>
                    <td class="actions"><button class="ghost icon" title="Remove" aria-label="Remove {f.path}" onclick={() => deleteFile(f.path)}><X size={15} /></button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="muted small">No extra files. Vanilla instances don't need any — add mods, configs or resource packs here.</p>
        {/if}
      </section>
    {:else if tab === 'server' && form.server}
      <section class="card col narrow">
        <div class="row"><Server size={20} /><h3>Built-in server</h3></div>
        <p class="muted small">Adds your server to the in-game multiplayer list and shows its live status in the launcher.</p>
        <div class="grid srv">
          <label class="field">Display name<input bind:value={form.server.name} placeholder="Velora SMP" /></label>
          <label class="field">Address<input bind:value={form.server.address} placeholder="play.example.net" /></label>
          <label class="field">Port<input type="number" min="1" max="65535" bind:value={form.server.port} /></label>
        </div>
        <Toggle bind:checked={form.server.inject} label="Add to multiplayer list" help="Written into servers.dat without touching servers players added themselves." />
        <Toggle bind:checked={form.server.auto_join} label="Join automatically" help="The game connects straight to the server after loading (Quick Play on 1.20+)." />
        {#if form.server.address}
          <button class="ghost sm" style="align-self: flex-start" onclick={() => form && (form.server = { name: '', address: '', port: 25565, auto_join: false, inject: true })}><X size={14} /> Remove server</button>
        {/if}
      </section>
    {:else if tab === 'access'}
      <div class="grid two">
        <section class="card col">
          <h3>Who can see this instance?</h3>
          <div class="vis">
            {#each [
              { id: 'public', icon: Globe, title: 'Everyone', text: 'Including local offline accounts' },
              { id: 'members', icon: UsersIcon, title: 'Signed-in players', text: 'Anyone with a panel account' },
              { id: 'groups', icon: Lock, title: 'Specific groups', text: 'Only members of the groups below' },
            ] as v}
              <button type="button" class="vopt" class:active={form.visibility === v.id} onclick={() => form && (form.visibility = v.id as Instance['visibility'])}>
                <v.icon size={18} /><span><strong>{v.title}</strong><span class="muted tiny">{v.text}</span></span>
              </button>
            {/each}
          </div>
          {#if form.visibility === 'groups'}
            <div class="row wrap">
              {#each groups as g}
                {@const on = form.allowed_groups.includes(g.name)}
                <button type="button" class="chip" class:on style:--c={g.color}
                  onclick={() => form && (form.allowed_groups = on ? form.allowed_groups.filter((x) => x !== g.name) : [...form.allowed_groups, g.name])}>
                  <span class="dot"></span>{g.name}
                </button>
              {:else}
                <p class="muted small">No groups yet — create them on the <a href="#/users">Players</a> page.</p>
              {/each}
            </div>
          {/if}
        </section>
        <section class="card col">
          <h3>Listing</h3>
          <Toggle bind:checked={form.enabled} label="Enabled" help="Hidden instances disappear from every launcher." />
          <Toggle bind:checked={form.featured} label="Featured" help="Shown first and selected by default." />
          <label class="field">Sort order <span class="help">Lower numbers come first.</span><input type="number" bind:value={form.sort} /></label>
        </section>
      </div>
    {/if}
  </div>

  {#if dirty}
    <div class="savebar">
      <span>You have unsaved changes</span>
      <button class="ghost" onclick={() => inst && (form = { ...structuredClone($state.snapshot(inst)), server: inst.server ?? { name: '', address: '', port: 25565, auto_join: false, inject: true } } as Instance)}>Discard</button>
      <button class="primary" disabled={saving} onclick={save}>{#if saving}<LoaderCircle class="spin" size={16} />{:else}<Save size={16} />{/if} Save changes</button>
    </div>
  {/if}

  {#if importing}
    <div class="importing">
      <div class="card col">
        <LoaderCircle class="spin" size={32} />
        <h3>Importing {importing}</h3>
        <p class="muted small">Downloading and unpacking the pack. Large packs can take a minute.</p>
      </div>
    </div>
  {/if}

  <Modal bind:open={confirmDelete} title="Delete {inst.name}?">
    <p class="muted">The instance disappears from every launcher and its hosted files are deleted. Players keep their local worlds.</p>
    {#snippet footer()}
      <button class="ghost" onclick={() => (confirmDelete = false)}>Cancel</button>
      <button class="danger" onclick={remove}><Trash2 size={16} /> Delete instance</button>
    {/snippet}
  </Modal>
{:else}
  <div class="page"><p class="muted row"><LoaderCircle class="spin" size={16} /> Loading…</p></div>
{/if}

<style>
  .back { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 0.88rem; margin-bottom: 16px; }
  .back:hover { color: var(--text); }
  .head { gap: 16px; }
  .head p { gap: 6px; margin-top: 8px; }
  .icon { width: 64px; height: 64px; border-radius: 16px; background: var(--surface-2); border: 1px solid var(--line-strong); display: grid; place-items: center; overflow: hidden; color: var(--muted); flex-shrink: 0; }
  .icon img { width: 100%; height: 100%; object-fit: cover; }
  .two { grid-template-columns: 1fr 1fr; align-items: start; }
  @media (max-width: 1000px) { .two { grid-template-columns: 1fr; } }
  .narrow { max-width: 680px; }
  .grow { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .current { display: flex; align-items: center; gap: 14px; margin-bottom: 18px; }
  .current :global(svg:first-child) { color: var(--accent); }
  .modes { margin-bottom: 18px; flex-wrap: wrap; }
  .drop { border: 2px dashed var(--line-strong); border-radius: 14px; padding: 36px; display: flex; flex-direction: column; align-items: center; gap: 6px; cursor: pointer; transition: border-color 0.15s, background 0.15s; color: var(--muted); }
  .drop strong { color: var(--text); }
  .drop:hover, .drop.dragging { border-color: var(--accent); background: var(--accent-soft); }
  details summary { cursor: pointer; }
  .results { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 10px; }
  .result { justify-content: flex-start; align-items: flex-start; gap: 12px; padding: 12px; text-align: left; white-space: normal; background: var(--bg-2); font-weight: 400; }
  .result img, .ph { width: 48px; height: 48px; border-radius: 10px; object-fit: cover; flex-shrink: 0; background: var(--surface-3); display: grid; place-items: center; }
  .clamp { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .picked { display: flex; flex-direction: column; gap: 14px; }
  .picked > button { align-self: flex-start; }
  .pi { width: 56px; height: 56px; border-radius: 12px; }
  .versions { display: flex; flex-direction: column; gap: 6px; max-height: 460px; overflow: auto; }
  .version { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border-radius: 10px; background: var(--bg-2); border: 1px solid var(--line); }
  .warnbox { border-color: rgba(251, 191, 36, 0.3); background: rgba(251, 191, 36, 0.06); display: flex; flex-direction: column; gap: 10px; margin-bottom: 16px; }
  .warnbox :global(svg) { color: var(--warn); }
  .miss { padding: 6px 0; border-top: 1px solid rgba(251, 191, 36, 0.15); }
  .table-wrap { overflow-x: auto; }
  .path { word-break: break-all; }
  tr.missing td { background: rgba(251, 191, 36, 0.04); }
  .actions { width: 40px; }
  .srv { grid-template-columns: 1fr 1.4fr 110px; }
  .vis { display: flex; flex-direction: column; gap: 8px; }
  .vopt { justify-content: flex-start; gap: 14px; padding: 14px; background: var(--bg-2); text-align: left; }
  .vopt span { display: flex; flex-direction: column; gap: 2px; font-weight: 400; }
  .vopt.active { border-color: var(--accent); background: var(--accent-soft); }
  .vopt.active :global(svg) { color: var(--accent); }
  .chip { padding: 6px 12px; border-radius: 99px; font-size: 0.85rem; background: var(--bg-2); }
  .chip .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--c); }
  .chip.on { border-color: var(--c); background: color-mix(in srgb, var(--c) 18%, transparent); }
  .savebar {
    position: fixed; bottom: 20px; left: 50%; transform: translateX(calc(-50% + 124px)); display: flex; align-items: center; gap: 12px;
    padding: 10px 10px 10px 20px; border-radius: 10px; background: var(--surface-2);
    border: 1px solid var(--line-strong); box-shadow: var(--shadow); z-index: 20; animation: up 0.2s ease;
  }
  @media (max-width: 900px) { .savebar { transform: translateX(-50%); } }
  @keyframes up { from { opacity: 0; transform: translate(calc(-50% + 124px), 10px); } }
  .importing { position: fixed; inset: 0; background: rgba(5, 6, 9, 0.75); display: grid; place-items: center; z-index: 60; }
  .importing .card { align-items: center; text-align: center; max-width: 380px; padding: 32px; }
  .importing :global(svg) { color: var(--accent); }
</style>
