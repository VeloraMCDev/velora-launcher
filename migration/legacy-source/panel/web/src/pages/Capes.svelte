<script lang="ts">
  import { Upload, Pencil, Trash2, Globe, Tag, Lock, Flag, Users as UsersIcon, ImagePlus } from '@lucide/svelte';
  import CapePreview from '../components/CapePreview.svelte';
  import Modal from '../components/Modal.svelte';
  import { api, del, get, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Cape, Group } from '../lib/types';

  type Draft = { id?: number; name: string; visibility: Cape['visibility']; allowed_groups: string[]; file: File | null; preview: string | null };

  let capes = $state<Cape[] | null>(null);
  let groups = $state<Group[]>([]);
  let draft = $state<Draft | null>(null);
  let draftOpen = $state(false);
  let saving = $state(false);
  let confirm = $state<Cape | null>(null);
  let confirmOpen = $state(false);

  $effect(() => {
    get<Cape[]>('/api/admin/capes').then((c) => (capes = c)).catch(toastError);
    get<Group[]>('/api/admin/groups').then((g) => (groups = g)).catch(() => {});
  });

  const visibilities = [
    { id: 'public', label: 'Everyone', help: 'Any player can pick it in the launcher.', icon: Globe },
    { id: 'groups', label: 'Groups', help: 'Players in the chosen groups can pick it.', icon: Tag },
    { id: 'private', label: 'Given only', help: 'Only admins can give it out, from the Players page.', icon: Lock },
  ] as const;

  function openNew() {
    draft = { name: '', visibility: 'public', allowed_groups: [], file: null, preview: null };
    draftOpen = true;
  }
  function openEdit(c: Cape) {
    draft = { id: c.id, name: c.name, visibility: c.visibility, allowed_groups: [...c.allowed_groups], file: null, preview: c.url };
    draftOpen = true;
  }
  function pick(e: Event & { currentTarget: HTMLInputElement }) {
    const f = e.currentTarget.files?.[0];
    if (!f || !draft) return;
    draft.file = f;
    draft.preview = URL.createObjectURL(f);
    if (!draft.name) draft.name = f.name.replace(/\.png$/i, '').replace(/[-_]+/g, ' ').slice(0, 40);
  }

  async function save() {
    if (!draft) return;
    saving = true;
    try {
      if (draft.id) {
        capes = await put<Cape[]>(`/api/admin/capes/${draft.id}`, { name: draft.name, visibility: draft.visibility, allowed_groups: draft.allowed_groups });
        toast('Cape updated');
      } else {
        const form = new FormData();
        form.append('name', draft.name);
        form.append('visibility', draft.visibility);
        form.append('allowed_groups', JSON.stringify(draft.allowed_groups));
        form.append('file', draft.file!);
        capes = await api<Cape[]>('/api/admin/capes', { method: 'POST', form });
        toast(`${draft.name} added`);
      }
      draftOpen = false;
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!confirm) return;
    try {
      capes = await del<Cape[]>(`/api/admin/capes/${confirm.id}`);
      toast('Cape deleted');
      confirmOpen = false;
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Capes</h1>
      <p>Upload capes and decide who can wear them. They show up in game on every server that uses the panel's sign-in.</p>
    </div>
    <button class="primary" onclick={openNew}><Upload size={16} /> Upload cape</button>
  </div>

  {#if capes && !capes.length}
    <section class="card empty">
      <Flag size={32} />
      <h3>No capes yet</h3>
      <p>Upload a 64×32 cape texture (or an HD multiple like 128×64) to get started.</p>
      <button class="primary add" onclick={openNew}><Upload size={16} /> Upload cape</button>
    </section>
  {:else if capes}
    <div class="grid capes">
      {#each capes as c (c.id)}
        {@const vis = visibilities.find((v) => v.id === c.visibility)!}
        <article class="card cape">
          <div class="stage"><CapePreview src={c.url} scale={7} /></div>
          <div class="meta">
            <strong>{c.name}</strong>
            <div class="row wrap tags">
              <span class="badge" class:accent={c.visibility === 'public'}><vis.icon size={11} /> {c.visibility === 'groups' ? c.allowed_groups.join(', ') || 'No groups' : vis.label}</span>
              <span class="muted tiny row gap"><UsersIcon size={12} /> {c.wearers} wearing</span>
            </div>
          </div>
          <div class="actions">
            <button class="ghost icon" aria-label="Edit {c.name}" onclick={() => openEdit(c)}><Pencil size={15} /></button>
            <button class="ghost icon" aria-label="Delete {c.name}" onclick={() => { confirm = c; confirmOpen = true; }}><Trash2 size={15} /></button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<Modal bind:open={draftOpen} title={draft?.id ? `Edit ${draft.name}` : 'Upload cape'} width={560}>
  {#if draft}
    <div class="editor">
      <label class="drop" class:has={draft.preview}>
        {#if draft.preview}<CapePreview src={draft.preview} scale={8} />{:else}<ImagePlus size={24} /><span class="tiny">Choose PNG</span>{/if}
        {#if !draft.id}<input type="file" accept="image/png" onchange={pick} />{/if}
      </label>
      <div class="col grow">
        <label class="field">Name<input bind:value={draft.name} maxlength="40" placeholder="Founder" /></label>
        <div class="field">
          <span class="lbl">Who can wear it</span>
          <div class="vis">
            {#each visibilities as v}
              <button type="button" class="opt" class:on={draft.visibility === v.id} onclick={() => draft && (draft.visibility = v.id)}>
                <v.icon size={16} /><span><strong>{v.label}</strong><span class="tiny muted">{v.help}</span></span>
              </button>
            {/each}
          </div>
        </div>
        {#if draft.visibility === 'groups'}
          <div class="row wrap">
            {#each groups as g}
              {@const on = draft.allowed_groups.includes(g.name)}
              <button type="button" class="gpick" class:on style:--c={g.color} onclick={() => draft && (draft.allowed_groups = on ? draft.allowed_groups.filter((x) => x !== g.name) : [...draft.allowed_groups, g.name])}>
                <span class="dot" style:background={g.color}></span>{g.name}
              </button>
            {:else}
              <span class="muted small">Create groups on the Players page first.</span>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
  {#snippet footer()}
    <button class="ghost" onclick={() => (draftOpen = false)}>Cancel</button>
    <button class="primary" disabled={saving || !draft?.name.trim() || (!draft?.id && !draft?.file)} onclick={save}>{draft?.id ? 'Save' : 'Upload'}</button>
  {/snippet}
</Modal>

<Modal bind:open={confirmOpen} title="Delete {confirm?.name}?">
  <p class="muted">{confirm?.wearers ? `${confirm.wearers} player${confirm.wearers === 1 ? '' : 's'} will lose it.` : 'Nobody is wearing it.'} This can't be undone.</p>
  {#snippet footer()}
    <button class="ghost" onclick={() => (confirmOpen = false)}>Cancel</button>
    <button class="danger" onclick={remove}><Trash2 size={16} /> Delete</button>
  {/snippet}
</Modal>

<style>
  .capes { grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); }
  .cape { padding: 0; overflow: hidden; display: flex; flex-direction: column; position: relative; }
  .stage { display: grid; place-items: center; padding: 22px; background: var(--bg-2); border-bottom: 1px solid var(--line); }
  .meta { padding: 14px 16px 16px; display: flex; flex-direction: column; gap: 8px; }
  .tags { gap: 10px; }
  .gap { gap: 5px; }
  .actions { position: absolute; top: 8px; right: 8px; display: flex; gap: 2px; opacity: 0; transition: opacity 0.15s; }
  .cape:hover .actions, .cape:focus-within .actions { opacity: 1; }
  .actions button { background: color-mix(in srgb, var(--surface) 85%, transparent); }
  .empty :global(svg) { color: var(--muted); }
  .empty .add { margin-top: 18px; }
  .editor { display: flex; gap: 20px; align-items: flex-start; }
  .grow { flex: 1; min-width: 0; }
  .drop { position: relative; width: 120px; height: 168px; flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; align-items: center; justify-content: center; border: 1px dashed var(--line-strong); border-radius: var(--radius); color: var(--muted); background: var(--bg-2); cursor: pointer; }
  .drop.has { border-style: solid; }
  .drop input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .field { display: flex; flex-direction: column; gap: 7px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .vis { display: flex; flex-direction: column; gap: 6px; }
  .opt { justify-content: flex-start; gap: 12px; padding: 10px 12px; text-align: left; white-space: normal; background: var(--bg-2); }
  .opt > span { display: flex; flex-direction: column; gap: 2px; }
  .opt :global(svg) { color: var(--muted); flex-shrink: 0; }
  .opt.on { border-color: color-mix(in srgb, var(--accent) 60%, transparent); background: var(--accent-soft); }
  .opt.on :global(svg) { color: var(--accent-2); }
  .gpick { padding: 6px 12px; border-radius: 99px; font-size: 0.85rem; background: var(--bg-2); }
  .gpick.on { border-color: var(--c); background: color-mix(in srgb, var(--c) 18%, transparent); }
  .dot { width: 10px; height: 10px; border-radius: 50%; }
  @media (max-width: 560px) { .editor { flex-direction: column; align-items: center; } }
</style>
