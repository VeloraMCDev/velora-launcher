<script lang="ts">
  // Velora Core (the Minecraft mod) releases: approve a signed GitHub release and the Panel serves its jars to servers.
  import { Boxes, Check, Download, RefreshCw, TriangleAlert } from '@lucide/svelte';
  import { get, post, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Asset = { role: string; name: string; size: number; sha256: string };
  type Latest = { version: string; minecraft: string; approved_at: string; server: Asset; client: Asset } | null;
  type Release = { tag: string; version: string; name: string; notes: string; published_at: string; prerelease: boolean; html_url: string; has_manifest: boolean; approved: boolean };
  type View = { repository: string; releases: Release[] };

  let { versions = [] }: { versions?: Array<string | null> } = $props();

  let latest = $state<Latest | undefined>(undefined);
  let view = $state<View | null>(null);
  let checking = $state(false);
  let confirming = $state<string | null>(null);
  let approving = $state<string | null>(null);

  const loadLatest = () => get<Latest>('/api/v1/core/latest').then((l) => (latest = l)).catch(() => (latest = null));
  $effect(() => { loadLatest(); });

  const behind = $derived(latest ? versions.filter((v) => v && v !== latest!.version).length : 0);
  const size = (n: number) => `${(n / 1024 / 1024).toFixed(1)} MB`;

  async function check() {
    checking = true;
    try { view = await get<View>('/api/admin/core/releases'); } catch (e) { toastError(e); } finally { checking = false; }
  }
  async function approve(tag: string) {
    approving = tag;
    try {
      await post(`/api/admin/core/releases/${encodeURIComponent(tag)}/approve`, {});
      toast('Velora Core release approved');
      confirming = null;
      await loadLatest();
      await check();
    } catch (e) { toastError(e); } finally { approving = null; }
  }
</script>

<section class="card core">
  <div class="head">
    <div class="title"><Boxes size={18} /><strong>Velora Core</strong>
      {#if latest}<span class="badge good">Serving {latest.version} · Minecraft {latest.minecraft}</span>
      {:else if latest === null}<span class="badge">No release approved yet</span>{/if}
    </div>
    <button class="ghost sm" onclick={check} disabled={checking}><RefreshCw size={14} class={checking ? 'spin' : ''} /> Check for releases</button>
  </div>
  <p class="muted small">
    The Velora Core mod is built and signed by GitHub Actions. Approving a release makes the Panel serve its jars, so deployment tools (like the Calagopus extension) can install and update it on your servers without touching GitHub.
  </p>
  {#if latest}
    <div class="files">
      <span class="file"><Download size={13} /> {latest.server.name} <span class="muted tiny">{size(latest.server.size)}</span></span>
      <span class="file"><Download size={13} /> {latest.client.name} <span class="muted tiny">{size(latest.client.size)}</span></span>
    </div>
    {#if behind}<p class="warn small"><TriangleAlert size={14} /> {behind} connected {behind === 1 ? 'server reports' : 'servers report'} a different version. They update on their next restart when the Calagopus extension is installed.</p>{/if}
  {/if}
  {#if view}
    {#if !view.releases.length}
      <p class="muted small">No signed releases found in {view.repository}. Run the “Velora Core release” workflow from the main branch.</p>
    {:else}
      <ul class="releases">
        {#each view.releases as r (r.tag)}
          <li>
            <div class="who"><strong>{r.version}</strong> <span class="muted tiny">{r.published_at ? timeAgo(r.published_at) : ''}</span>
              {#if r.prerelease}<span class="badge warn">Pre-release</span>{/if}
              {#if !r.has_manifest}<span class="badge warn">No manifest</span>{/if}
            </div>
            <div class="act">
              <a class="tiny" href={r.html_url} target="_blank" rel="noreferrer">Notes</a>
              {#if r.approved}<span class="badge good"><Check size={12} /> Serving</span>
              {:else if r.prerelease || !r.has_manifest}<span class="muted tiny">Can't be approved</span>
              {:else if confirming === r.tag}
                <button class="primary sm" disabled={approving === r.tag} onclick={() => approve(r.tag)}>{approving === r.tag ? 'Verifying…' : 'Confirm approve'}</button>
                <button class="ghost sm" onclick={() => (confirming = null)}>Cancel</button>
              {:else}<button class="sm" onclick={() => (confirming = r.tag)}>Approve</button>{/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .core { display: flex; flex-direction: column; gap: 10px; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .title { display: flex; align-items: center; gap: 8px; }
  .files { display: flex; flex-wrap: wrap; gap: 8px; }
  .file { display: inline-flex; align-items: center; gap: 6px; font-family: var(--mono); font-size: 0.78rem; padding: 4px 10px; border-radius: var(--radius-sm); background: var(--bg-2); border: 1px solid var(--line); }
  .warn { display: flex; align-items: center; gap: 6px; color: #fcd34d; margin: 0; }
  .releases { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .releases li { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 12px; border: 1px solid var(--line); border-radius: var(--radius-sm); background: var(--bg-2); }
  .who { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .act { display: flex; align-items: center; gap: 8px; }
</style>
