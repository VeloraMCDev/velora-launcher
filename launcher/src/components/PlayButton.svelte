<script lang="ts">
  import { Play, X, Square, LoaderCircle, TriangleAlert, RotateCcw } from '@lucide/svelte';
  import { app, cancel, kill, play } from '../lib/store.svelte';
  import { bytes } from '../lib/format';
  import type { Instance } from '../lib/types';

  let { inst }: { inst: Instance } = $props();

  const l = $derived(app.launch);
  const mine = $derived(app.launchingId === inst.id);
  const busy = $derived(mine && l.state === 'preparing');
  const running = $derived(app.running.filter((g) => g.instance_id === inst.id));
  const preparing = $derived(l.state === 'preparing');
  const pct = $derived(l.total > 0 ? Math.min(100, (l.done / l.total) * 100) : 0);
  const eta = $derived(l.speed > 0 && l.total > l.done ? Math.ceil((l.total - l.done) / l.speed) : 0);
  const stageNames: Record<string, string> = { preparing: 'Preparing', java: 'Java', game: 'Game files', loader: 'Mod loader', files: 'Instance files', launching: 'Launching' };
  const noAccount = $derived(!app.active);
</script>

<svelte:window onkeydown={(e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter' && !preparing && !noAccount && app.view === 'home') play(inst.id);
}} />

<div class="wrap">
  {#if busy}
    <div class="progress glass">
      <div class="top">
        <span class="stage">
          {#if l.stage === 'launching' || !l.total}<LoaderCircle class="spin" size={15} />{/if}
          {l.label || 'Preparing…'}
        </span>
        <button class="ghost icon sm" onclick={cancel} aria-label="Cancel" title="Cancel"><X size={15} /></button>
      </div>
      <div class="bar" class:indeterminate={!l.total}><span style:width="{pct}%"></span></div>
      <div class="meta tiny muted">
        <span>{l.stage ? stageNames[l.stage] : ''}{l.filesTotal ? ` · ${l.filesDone.toLocaleString()} / ${l.filesTotal.toLocaleString()} files` : ''}</span>
        <span>{#if l.total}{bytes(l.done)} / {bytes(l.total)}{#if l.speed > 0} · {bytes(l.speed)}/s{#if eta} · {eta < 60 ? `${eta}s` : `${Math.ceil(eta / 60)}m`} left{/if}{/if}{/if}</span>
      </div>
    </div>
  {:else}
    <button class="play" onclick={() => play(inst.id)} disabled={preparing || noAccount} title={noAccount ? 'Sign in first' : 'Ctrl + Enter'}>
      <Play size={18} fill="currentColor" />
      <span class="lbl">{running.length ? 'Launch another window' : 'Play'}</span>
    </button>
  {/if}
  {#if running.length}
    <div class="sessions">
      {#each running as game, index (game.run_id)}
        <button class="sm session" onclick={() => kill(game.run_id)} title="Stop this Minecraft window"><span class="pulse"></span> Window {index + 1} running <Square size={12} /> Stop</button>
      {/each}
    </div>
  {/if}
  {#if mine && l.state === 'error'}
    <div class="error glass selectable">
      <TriangleAlert size={17} />
      <span>{l.message}</span>
      <button class="sm" onclick={() => play(inst.id)}><RotateCcw size={13} /> Retry</button>
    </div>
  {/if}
</div>

<style>
  .wrap { display: flex; flex-direction: column; align-items: flex-start; gap: 0.6rem; }
  .sessions { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .session { gap: 0.4rem; }
  .play {
    height: 3.4rem; min-width: 14.5rem; padding: 0 2rem; border: none; border-radius: var(--radius);
    background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 80%, black));
    box-shadow: 0 4px 18px color-mix(in srgb, var(--accent) 35%, transparent);
    color: white; font-size: 1.05rem; font-weight: 650; letter-spacing: 0.02em; gap: 0.7rem;
    transition: transform 0.15s cubic-bezier(0.2, 0.8, 0.2, 1), box-shadow 0.15s, background 0.15s;
  }
  .play:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 6px 22px color-mix(in srgb, var(--accent) 45%, transparent); background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 92%, white), var(--accent)); }
  .play:active:not(:disabled) { transform: translateY(0); box-shadow: 0 2px 10px color-mix(in srgb, var(--accent) 30%, transparent); }
  .play:disabled { background: color-mix(in srgb, var(--text) 10%, transparent); color: var(--muted); box-shadow: none; transform: none; }
  .lbl { position: relative; }
  .play { position: relative; overflow: hidden; animation: glow-breathe 3.2s ease-in-out infinite; }
  .play::after { content: ""; position: absolute; inset: 0; background: linear-gradient(105deg, transparent 35%, rgba(255, 255, 255, 0.28) 50%, transparent 65%); transform: translateX(-120%); transition: transform 0.7s var(--ease); }
  .play:hover:not(:disabled)::after { transform: translateX(120%); }
  .play:disabled { animation: none; }
  @keyframes glow-breathe { 0%, 100% { box-shadow: 0 4px 18px color-mix(in srgb, var(--accent) 32%, transparent); } 50% { box-shadow: 0 6px 30px color-mix(in srgb, var(--accent) 52%, transparent); } }
  .pulse { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: var(--success); box-shadow: 0 0 8px var(--success); animation: pulse-dot 1.5s ease-in-out infinite; }
  @keyframes pulse-dot { 0%, 100% { opacity: 1; transform: scale(1); } 50% { opacity: 0.6; transform: scale(0.85); } }
  .progress { width: 26rem; max-width: 100%; padding: 0.85rem 1rem 0.8rem; display: flex; flex-direction: column; gap: 0.55rem; background: var(--surface); border-radius: var(--radius); }
  .top { display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; }
  .stage { display: flex; align-items: center; gap: 0.5rem; font-weight: 560; font-size: 0.9rem; }
  .bar { height: 0.35rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 9%, transparent); overflow: hidden; position: relative; }
  .bar span { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--accent), var(--accent-2)); box-shadow: 0 0 10px var(--glow); transition: width 0.25s var(--ease); }
  .bar.indeterminate span { width: 30% !important; animation: slide 1.3s ease-in-out infinite; }
  @keyframes slide { from { transform: translateX(-100%); } to { transform: translateX(340%); } }
  .meta { display: flex; justify-content: space-between; gap: 1rem; font-variant-numeric: tabular-nums; }
  .error { display: flex; align-items: center; gap: 0.7rem; padding: 0.65rem 0.8rem; max-width: 34rem; font-size: 0.85rem; background: color-mix(in srgb, var(--danger) 10%, var(--surface)); border-color: color-mix(in srgb, var(--danger) 30%, transparent); line-height: 1.4; border-radius: var(--radius-sm); }
  .error :global(svg) { color: var(--danger); flex-shrink: 0; }
  .error span { flex: 1; }
</style>
