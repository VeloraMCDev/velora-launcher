<script lang="ts">
  import { onMount } from 'svelte';
  import { Rocket, Package, Boxes, RefreshCw, LoaderCircle, ExternalLink, CircleCheck, ShieldCheck, GitCommit } from '@lucide/svelte';
  import { api } from '../lib/api';
  import { app, ask, runAction, toast } from '../lib/state.svelte';
  import { ago, bytes, shortDigest, when } from '../lib/format';
  import type { Updates } from '../lib/types';

  let updates = $state<Updates | null>(null);
  let checking = $state(false);
  let showCurrent = $state(false);

  const load = async () => { updates = await api<Updates>('/api/updates'); };
  onMount(() => { load().catch(e => toast(e.message, 'error')); });
  async function check() {
    checking = true;
    try { updates = await api<Updates>('/api/updates/check', { method: 'POST' }); toast('Checked for updates'); }
    catch (e) { toast((e as Error).message, 'error'); }
    checking = false;
  }
  async function approve(tag: string, version: string) {
    const ok = await ask(`Publish launcher ${version} to players?`,
      'The Panel downloads every installer from GitHub and verifies its size, checksum and Velora release signature before publishing.\n\nInstalled launchers will offer the update on their next check, and the website download buttons switch to this version.', 'Approve & publish');
    if (ok && (await runAction('/api/actions/approve-launcher', { tag }))?.status === 'succeeded') load();
  }
  async function deploy(name: string, image: string, commit: string) {
    const self = name === 'velora-operations';
    const ok = await ask(`Deploy ${name} build ${commit.slice(0, 7)}?`,
      self ? 'The dashboard restarts itself with this build. A helper checks its health and restores the previous build on failure. Reload the page after a minute to see the result.'
        : 'A fresh backup runs first. If the new build does not become healthy, the previous build is restored automatically.', 'Deploy', !self);
    if (ok && (await runAction('/api/actions/update-image', { name, image }))?.status === 'succeeded') load();
  }
  const containerUpdates = $derived((updates?.containers ?? []).filter(c => showCurrent || c.status !== 'current').sort((a, b) => (a.status === 'update-available' ? -1 : 1) - (b.status === 'update-available' ? -1 : 1)));
  const statusBadge: Record<string, string> = { 'update-available': 'accent', current: 'ok', pinned: 'info', 'local-build': '', unknown: 'warn' };
</script>

<div class="head row wrap">
  <div><h1>Updates</h1><p class="muted">Approve launcher releases, deploy new Velora builds and track container image updates. Checked {ago(updates?.checked)}.</p></div>
  <span class="spacer"></span>
  <button onclick={check} disabled={checking}>{#if checking}<LoaderCircle size={16} class="spin" />{:else}<RefreshCw size={16} />{/if} Check now</button>
</div>

{#each updates?.errors ?? [] as error}<div class="warnline small">{error}</div>{/each}

{#if updates?.dashboard_update}
  <div class="card small" role="status">
    {#if updates.dashboard_update === 'running'}Dashboard self-update is running. Reload this page after a minute.
    {:else if updates.dashboard_update === 'succeeded'}The latest dashboard self-update passed its health check.
    {:else if updates.dashboard_update === 'rolled-back'}The latest dashboard self-update failed; the previous build was restored and is healthy.
    {:else}Dashboard rollback failed. Inspect the deployment host and recover using <code>.env.ops-rollback</code> before attempting another update.{/if}
  </div>
{/if}

<div class="card">
  <div class="card-head"><Rocket size={18} /><h2>Launcher releases</h2><span class="spacer"></span>
    <span class="tiny muted">Players currently receive <b>{updates?.launcher_served ? `v${updates.launcher_served}` : 'no signed release'}</b></span></div>
    <p class="muted small intro">Desktop installers and Android/iOS apps built by the <code>Launcher release</code> workflow wait here. Nothing reaches players until you approve it.</p>
  <div class="col">
    {#each updates?.launcher?.releases ?? [] as r}
      <div class="release" class:current={r.approved}>
        <div class="grow">
          <div class="row wrap"><b class="ver">v{r.version}</b>
            {#if r.approved}<span class="badge ok"><CircleCheck size={12} /> Live for players</span>{/if}
            {#if r.prerelease}<span class="badge warn">pre-release</span>{/if}
            {#if !r.has_manifest}<span class="badge bad">unsigned</span>{:else}<span class="badge info"><ShieldCheck size={12} /> signed</span>{/if}
          </div>
          <div class="tiny muted">Published {when(r.published_at)} · {r.assets.filter(a => !a.name.endsWith('.json') && a.name !== 'SHA256SUMS').map(a => `${a.name.split('_').at(-1)} ${bytes(a.size)}`).join(' · ')}</div>
        </div>
        <a class="btn sm ghost" href={r.html_url} target="_blank" rel="noreferrer"><ExternalLink size={14} /> GitHub</a>
        {#if !r.approved && r.has_manifest && !r.prerelease}
          <button class="sm primary" onclick={() => approve(r.tag, r.version)}><Rocket size={14} /> Approve & publish</button>
        {/if}
      </div>
    {:else}<div class="empty">No launcher releases yet. Run the <code>Launcher release</code> workflow on GitHub to build one.</div>{/each}
  </div>
</div>

<div class="grid g2 section">
  {#each Object.entries(updates?.images ?? {}) as [name, image]}
    <div class="card">
      <div class="card-head"><Package size={18} /><h2>{name}</h2><span class="spacer"></span><span class="tiny muted mono">running {shortDigest(image.current)}</span></div>
      <div class="col">
        {#each image.releases as r, i}
          {@const live = r.digest === image.current}
          <div class="build" class:current={live}>
            <GitCommit size={16} class="faint" />
            <div class="grow">
              <div class="row"><b class="mono small">{r.commit.slice(0, 7)}</b>{#if live}<span class="badge ok">running</span>{:else if i === 0}<span class="badge accent">newest</span>{/if}<span class="tiny faint">run {r.run}</span></div>
              <div class="small msg">{r.commitInfo?.message ?? 'Release build'}</div>
              <div class="tiny faint">{r.commitInfo?.date ? when(r.commitInfo.date) : ''} · {shortDigest(r.digest)}</div>
            </div>
            {#if !live}<button class="sm {i === 0 ? 'primary' : ''}" onclick={() => deploy(name, r.image, r.commit)}>{i === 0 ? 'Deploy' : 'Roll back'}</button>{/if}
          </div>
        {:else}<div class="empty">No release builds found on GHCR.</div>{/each}
      </div>
    </div>
  {/each}
</div>

<div class="card flush section">
  <div class="card-head pad"><Boxes size={18} /><h2>Container images</h2><span class="spacer"></span>
    <label class="row tiny muted"><input type="checkbox" bind:checked={showCurrent} style="width:auto" /> Show up-to-date</label></div>
  <p class="muted small pad2">Compares each running container with its registry tag. Velora services update from this page; other stacks are listed for awareness.</p>
  <div class="table-scroll">
    <table>
      <thead><tr><th>Container</th><th>Project</th><th>Image</th><th>Status</th><th></th></tr></thead>
      <tbody>
        {#each containerUpdates as c}
          <tr>
            <td>{c.name}</td>
            <td class="muted">{c.project ?? '—'}</td>
            <td class="mono tiny muted">{c.image}</td>
            <td><span class="badge {statusBadge[c.status]}" title={c.error ?? ''}>{c.status.replace('-', ' ')}</span></td>
            <td class="right">{#if c.status === 'update-available' && c.project === app.overview?.project && c.service}
              <button class="sm" onclick={async () => { if (await ask(`Update ${c.service}?`, 'Pulls the newer image and recreates the container.', 'Update')) runAction('/api/actions/pull', { service: c.service }); }}>Update</button>
            {/if}</td>
          </tr>
        {:else}<tr><td colspan="5" class="empty">{updates?.containers ? 'Every container is up to date.' : 'Checked every six hours.'}</td></tr>{/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .head { margin-bottom: 22px; }
  .head p { margin: 4px 0 0; }
  .section { margin-top: 16px; }
  .intro { margin: -6px 0 14px; }
  .warnline { background: #f5b04112; border: 1px solid #f5b04140; color: #ffe0ad; border-radius: 10px; padding: 8px 12px; margin-bottom: 12px; }
  .release, .build { display: flex; align-items: center; gap: 12px; padding: 12px 14px; border: 1px solid var(--line); border-radius: 12px; background: var(--bg-2); }
  .release.current, .build.current { border-color: #3ccf9150; background: #3ccf910a; }
  .ver { color: var(--head); font-size: 16px; }
  .grow { flex: 1; min-width: 0; }
  .msg { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pad { padding: 16px 18px 0; margin-bottom: 4px; }
  .pad2 { padding: 0 18px; margin: 0 0 12px; }
  .right { text-align: right; }
</style>
