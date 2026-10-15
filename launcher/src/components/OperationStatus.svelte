<script lang="ts">
  import { app, cancel, kill, play, repair } from '../lib/store.svelte';
  import { bytes } from '../lib/format';
  import { OperationPresentation, launchShortcutAllowed, type OperationKind } from '../lib/operation-presentation';
  import { actionContrast } from '../lib/action-contrast';
  import type { Instance } from '../lib/types';

  let { inst }: { inst: Instance } = $props();
  const controller = new OperationPresentation();
  const context = () => ({ selectedId: inst.id, ownerId: app.launchingId, launch: app.launch, hasAccount: !!app.active });
  let revision = $state(0);
  const view = $derived.by(() => {
    revision;
    return controller.observe(context());
  });
  const running = $derived(app.running.filter(game => game.instance_id === inst.id));
  const primaryColors = $derived(actionContrast((app.settings?.theme_mode === 'custom' && app.manifest?.branding.features.allow_user_theme && app.settings.accent) || app.manifest?.branding.colors.accent));
  let operationElement: HTMLDivElement;
  let sessionError = $state<string | null>(null);
  let stopping = $state<string[]>([]);
  $effect(() => {
    inst.id;
    sessionError = null;
  });
  const perform = (kind: OperationKind, instanceId: string) => kind === 'repair' ? repair(instanceId) : play(instanceId);

  async function start(kind: OperationKind) {
    const request = controller.start(kind, context(), context, perform);
    revision++;
    await request;
    revision++;
  }

  async function retry() {
    const request = controller.retry(view.scope, context(), context, perform);
    revision++;
    await request;
    revision++;
  }

  async function requestCancel() {
    const request = controller.cancel(view.scope, context(), context, cancel);
    revision++;
    await request;
    revision++;
  }

  async function stop(runId: string) {
    if (stopping.includes(runId) || !running.some(game => game.run_id === runId)) return;
    const selectedId = inst.id;
    stopping = [...stopping, runId];
    sessionError = null;
    try { await kill(runId); }
    catch { if (selectedId === inst.id) sessionError = 'Could not stop this Minecraft window. Try Stop again.'; }
    finally { stopping = stopping.filter(candidate => candidate !== runId); }
  }
</script>

<svelte:window onkeydown={(event) => {
  const editable = event.target instanceof Element && !!event.target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])');
  const dialogOpen = !!document.querySelector('[role="dialog"][aria-modal="true"]');
  const inert = !!operationElement?.closest('[inert]');
  if (launchShortcutAllowed(event, editable, dialogOpen, inert) && app.view === 'home' && view.canStart && app.active) {
    event.preventDefault();
    void start('launch');
  }
}} />

<div class="operation" bind:this={operationElement} style:--action-accent={primaryColors.background} style:--action-on-accent={primaryColors.foreground}>
  <p class="announcement" role="status" aria-live="polite" aria-atomic="true">{view.announcement}</p>
  <div class="controls">
    <button class="primary" disabled={!view.canStart || !app.active} onclick={() => start('launch')}>
      {running.length ? 'Launch another window' : 'Play'}
    </button>
    <button disabled={!view.canStart} onclick={() => start('repair')}>Verify / repair files</button>
  </div>
  {#if !app.active}<p class="muted">Sign in to play. File verification does not require a selected account.</p>{/if}
  {#if view.busy && !view.mine}
    <p class="notice">Another instance is preparing. Select it to view progress or cancel. New preparation is blocked until it ends.</p>
  {:else if view.mine && (view.busy || view.state === 'failed' || view.state === 'cancelled')}
    <section class="status-card" aria-label="Instance preparation">
      <div class="heading">
        <h2>{view.state === 'failed' ? 'Preparation failed' : view.state === 'cancelled' ? 'Preparation ended' : view.phase}</h2>
        {#if view.busy}<button class="sm" disabled={!view.canCancel} onclick={requestCancel}>{view.cancellationRequested ? 'Cancellation requested' : 'Cancel'}</button>{/if}
      </div>
      {#if view.busy}
        {#if view.progress.determinate}
          <progress max="100" value={view.progress.percent ?? 0} aria-label={view.phase}></progress>
          <p>{view.progress.percent}% · {bytes(view.progress.done)} / {bytes(view.progress.total)}</p>
        {:else}
          <progress aria-label={`${view.phase}: total not available`}></progress>
          <p>Progress total not available{view.progress.done > 0 ? ` · ${bytes(view.progress.done)} reported` : ''}</p>
        {/if}
        <p class="muted">{view.progress.description}</p>
        {#if view.progress.filesTotal > 0 || view.progress.filesDone > 0}
          <p>{view.progress.filesDone} {view.progress.filesTotal > 0 ? `/ ${view.progress.filesTotal}` : '(total unknown)'} files reported</p>
        {/if}
        {#if view.cancellationRequested}<p>Waiting for preparation to end. Cancellation does not stop an already running Minecraft window.</p>{/if}
      {:else if view.state === 'failed'}
        <p>Could not finish preparation. Check your connection, available disk space, and file permissions before trying again.</p>
      {:else}
        <p>Preparation ended after your cancellation request. The store does not expose a confirmed cancellation outcome; this is not a native acknowledgement.</p>
      {/if}
      {#if !view.busy && view.kind}
        <button disabled={!view.canRetry} onclick={retry}>{view.kind === 'repair' ? 'Retry verification / repair' : 'Retry launch'}</button>
      {/if}
      <details>
        <summary>Safe preparation details</summary>
        <dl>
          <dt>Action</dt><dd>{view.kind === 'repair' ? 'Verify / repair' : view.kind === 'launch' ? 'Launch' : 'Started outside this Home view; retry type unavailable'}</dd>
          <dt>Last observed phase</dt><dd>{view.phase}</dd>
          <dt>Outcome</dt><dd>{view.state === 'cancelled' ? 'Ended after cancellation request (unconfirmed)' : view.state}</dd>
        </dl>
        <p>Raw engine errors, logs, tokens, URLs and local paths are not shown here. Repair follows the existing managed-file policy and may update managed files. These controls add no cleanup or reset action.</p>
      </details>
    </section>
  {/if}
  {#if view.actionError}<p role="status">{view.actionError}</p>{/if}
  {#if running.length}
    <div class="controls" aria-label="Running Minecraft windows">
      {#each running as game, index (game.run_id)}
        <button disabled={stopping.includes(game.run_id)} onclick={() => stop(game.run_id)}>Window {index + 1} running · {stopping.includes(game.run_id) ? 'Stop requested' : 'Stop'}</button>
      {/each}
    </div>
  {/if}
  {#if sessionError}<p role="status">{sessionError}</p>{/if}
</div>

<style>
  .operation { display: flex; flex-direction: column; gap: 0.65rem; width: 100%; max-width: 38rem; min-width: 0; }
  .controls, .heading { display: flex; align-items: center; gap: 0.65rem; flex-wrap: wrap; }
  .primary { background: var(--action-accent); color: var(--action-on-accent); min-height: 3rem; padding: 0.7rem 2rem; font-weight: 650; }
  .primary:hover:not(:disabled) { background: var(--action-accent); }
  .status-card { display: flex; flex-direction: column; gap: 0.55rem; padding: 1rem; border: 1px solid var(--line-strong); border-radius: var(--radius); background: var(--surface); overflow-wrap: anywhere; }
  h2 { font-size: 1rem; flex: 1; }
  p, details { font-size: 0.85rem; line-height: 1.5; }
  progress { width: 100%; accent-color: var(--accent); }
  summary { cursor: pointer; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: 0.3rem 0.75rem; margin: 0.75rem 0; }
  dd { margin: 0; }
  button:focus-visible, summary:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  .announcement { position: absolute; width: 1px; height: 1px; padding: 0; overflow: hidden; clip-path: inset(50%); }
  @media (max-height: 760px) { .status-card { padding: 0.65rem; gap: 0.35rem; } }
</style>
