<script lang="ts">
  import { onMount } from 'svelte';
  import { Upload, Trash2, LoaderCircle, Check, X } from '@lucide/svelte';
  import PlatformIcon from './PlatformIcon.svelte';
  import { api, del, formatBytes, get } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { HostedDownload } from '../lib/types';

  // One place to publish every installer the website offers: the three desktop launchers and the two phone apps.
  const PLATFORMS = [
    { id: 'windows', label: 'Windows', accept: '.exe', hint: 'The Setup.exe from a release. Players on an older launcher are offered it as an update.', notes: true, version: true },
    { id: 'mac', label: 'macOS', accept: '.dmg,.pkg,.zip', hint: 'The .dmg from a release.', notes: false, version: false },
    { id: 'linux', label: 'Linux', accept: '.AppImage,.deb,.rpm,.tar.gz,.zip', hint: 'An AppImage and a .deb can both be offered; a new file replaces the same kind.', notes: false, version: false },
    { id: 'android', label: 'Android', accept: '.apk', hint: 'The signed .apk from a release.', notes: true, version: true },
    { id: 'ios', label: 'iOS', accept: '.ipa', hint: 'The .ipa from a release. Players install it with AltStore, SideStore or Sideloadly.', notes: true, version: true },
  ] as const;

  let { compact = false }: { compact?: boolean } = $props();
  let items = $state<HostedDownload[]>([]);
  let loading = $state(true);
  let busy = $state('');
  let draft = $state<{ platform: string; file: File; version: string; notes: string } | null>(null);
  const inputs: Record<string, HTMLInputElement | undefined> = {};

  async function refresh() {
    const cfg = await get<{ hosted_downloads: HostedDownload[] }>('/api/admin/landing');
    items = cfg.hosted_downloads;
  }
  onMount(() => { refresh().catch(toastError).finally(() => (loading = false)); });

  function pick(platform: string, e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    const m = file.name.match(/(?:^|[^A-Za-z0-9])v?(\d+\.\d+\.\d+(?:-[\w.]+)?)/);
    draft = { platform, file, version: m?.[1] ?? '', notes: '' };
  }

  async function publish() {
    if (!draft) return;
    const { platform, file, version, notes } = draft;
    busy = platform;
    try {
      const form = new FormData();
      if (platform === 'windows') {
        form.append('version', version.trim()); form.append('notes', notes); form.append('file', file);
        await api('/api/admin/launcher/update', { method: 'POST', form });
      } else if (platform === 'android' || platform === 'ios') {
        form.append('platform', platform); form.append('version', version.trim()); form.append('notes', notes); form.append('file', file);
        await api('/api/admin/mobile-apps', { method: 'POST', form });
      } else {
        form.append('platform', platform); form.append('version', version.trim()); form.append('file', file);
        await api('/api/admin/landing/upload-launcher', { method: 'POST', form });
      }
      draft = null;
      await refresh();
      toast(`Published ${file.name} (${formatBytes(file.size)})`);
    } catch (e) { toastError(e); }
    finally { busy = ''; }
  }

  async function remove(platform: string, d: HostedDownload) {
    if (!confirm(`Take ${d.display_name || d.filename} off the website?`)) return;
    busy = platform;
    try {
      if (platform === 'windows') await del('/api/admin/launcher/update');
      else if (platform === 'android' || platform === 'ios') await del(`/api/admin/mobile-apps/${platform}`);
      else await del(`/api/admin/landing/launcher/${platform}?f=${encodeURIComponent(d.filename)}`);
      await refresh();
      toast('Removed');
    } catch (e) { toastError(e); }
    finally { busy = ''; }
  }
</script>

<section class="card col downloads" class:compact>
  {#if !compact}
    <div class="head"><h2>Downloads</h2><p class="help">Upload each installer once. The website, the player panel and the launcher's update check all use these files, with tidy names.</p></div>
  {/if}
  {#if loading}<p class="help">Loading…</p>{/if}
  {#each PLATFORMS as p (p.id)}
    {@const files = items.filter((d) => d.platform === p.id)}
    <div class="plat" class:has={files.length}>
      <div class="icon"><PlatformIcon platform={p.id} size={22} /></div>
      <div class="info">
        <strong>{p.label}</strong>
        {#each files as d (d.filename + d.file_url)}
          <div class="file">
            <span class="name" title={d.filename}>{d.display_name || d.filename}</span>
            <span class="meta">{d.version ? `v${d.version} · ` : ''}{formatBytes(d.size)}{d.uploaded_at ? ` · ${new Date(d.uploaded_at).toLocaleDateString()}` : ''}</span>
            <button class="ghost sm danger-text" disabled={busy === p.id} onclick={() => remove(p.id, d)} aria-label={`Remove ${d.display_name}`}><Trash2 size={14} /></button>
          </div>
        {:else}
          <span class="meta none">Nothing uploaded yet</span>
        {/each}
        {#if draft?.platform === p.id}
          <div class="draft">
            <div class="chosen"><strong>{draft.file.name}</strong> <span class="meta">{formatBytes(draft.file.size)}</span></div>
            {#if p.version}<label>Version<input bind:value={draft.version} placeholder="1.0.1" maxlength="64" /></label>{/if}
            {#if p.notes}<label>Release notes <span class="help">(optional)</span><textarea rows="2" bind:value={draft.notes} maxlength="8000"></textarea></label>{/if}
            <div class="row">
              <button class="primary sm" disabled={busy === p.id || (p.version && !draft.version.trim())} onclick={publish}>
                {#if busy === p.id}<LoaderCircle class="spin" size={14} /> Uploading…{:else}<Check size={14} /> Publish{/if}
              </button>
              <button class="ghost sm" disabled={busy === p.id} onclick={() => (draft = null)}><X size={14} /> Cancel</button>
            </div>
          </div>
        {:else if !compact}
          <span class="help">{p.hint}</span>
        {/if}
      </div>
      <label class="btn sm upload" class:disabled={busy === p.id}>
        <Upload size={14} /> {files.length && p.id !== 'linux' && p.id !== 'mac' ? 'Replace' : files.length ? 'Add file' : 'Upload'}
        <input bind:this={inputs[p.id]} type="file" accept={p.accept} hidden disabled={busy === p.id} onchange={(e) => pick(p.id, e)} />
      </label>
    </div>
  {/each}
</section>

<style>
  .downloads { gap: 10px; }
  .head h2 { margin: 0 0 4px; }
  .help { color: var(--muted); font-size: .8rem; line-height: 1.5; font-weight: 400; }
  .plat { display: grid; grid-template-columns: 44px 1fr auto; gap: 14px; align-items: start; padding: 14px; border: 1px solid var(--line); border-radius: 12px; background: color-mix(in srgb, var(--surface-2) 70%, transparent); transition: border-color .15s, background .15s; }
  .plat.has { border-color: color-mix(in srgb, var(--accent) 28%, var(--line)); }
  .icon { width: 44px; height: 44px; border-radius: 12px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent-2); }
  .info { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
  .file { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 12px; }
  .name { font-family: var(--mono); font-size: .86rem; color: var(--text); overflow: hidden; text-overflow: ellipsis; }
  .meta { color: var(--muted); font-size: .8rem; }
  .none { font-style: italic; }
  .upload { cursor: pointer; white-space: nowrap; }
  .upload.disabled { opacity: .5; pointer-events: none; }
  .draft { display: flex; flex-direction: column; gap: 8px; margin-top: 6px; padding: 12px; border-radius: 10px; background: var(--bg-2); border: 1px dashed var(--line-strong); }
  .draft label { display: flex; flex-direction: column; gap: 5px; font-size: .82rem; color: var(--text-2); }
  .danger-text { color: #f2999c; }
  .compact .plat { padding: 10px; }
  @media (max-width: 640px) { .plat { grid-template-columns: 40px 1fr; } .upload { grid-column: 1 / -1; justify-self: start; } }
</style>
