<script lang="ts">
  // The frame every table shares: a controls panel and a stage side by side on desktop, stage first on a phone with the
  // main button pinned above the tab bar so it is always under your thumb.
  import type { Component, Snippet } from 'svelte';

  let { title, icon: Icon, controls, stage, action }: { title: string; icon: Component<{ size?: number }>; controls: Snippet; stage: Snippet; action: Snippet } = $props();
</script>

<div class="game">
  <section class="panel controls">
    <h2><Icon size={18} /> {title}
    </h2>
    {@render controls()}
    <div class="act">{@render action()}</div>
  </section>
  <section class="stage">{@render stage()}</section>
</div>

<style>
  .game { display: grid; grid-template-columns: 19rem minmax(0, 1fr); gap: 1.1rem; align-items: start; }
  @media (max-width: 860px) {
    .game { grid-template-columns: minmax(0, 1fr); }
    .stage { order: -1; }
    .act { position: sticky; bottom: calc(var(--pl-tab, 0px) + env(safe-area-inset-bottom, 0px) + 10px); z-index: 4; }
  }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.9rem; min-width: 0; }
  h2 { display: flex; gap: 0.5rem; align-items: center; font-size: 1.05rem; }
  .act { display: flex; flex-direction: column; gap: 0.5rem; }
  .stage { min-width: 0; }
  /* Shared bits the tables use inside their snippets. */
  :global(.act .big) { padding: 0.9rem; font-size: 1.02rem; font-weight: 800; border-radius: var(--radius); border: none; color: #fff; width: 100%; touch-action: manipulation; }
  :global(.act .big:disabled) { opacity: 0.55; }
  :global(.act .big:hover:not(:disabled)) { filter: brightness(1.1); transform: translateY(-1px); }
  :global(.act .row) { display: grid; grid-auto-flow: column; grid-auto-columns: 1fr; gap: 0.5rem; }
  :global(.note) { font-size: 0.78rem; color: var(--muted); display: flex; gap: 0.35rem; align-items: flex-start; }
  :global(.seg2) { display: grid; grid-auto-flow: column; grid-auto-columns: 1fr; gap: 0.35rem; padding: 0.25rem; border-radius: var(--radius); background: color-mix(in srgb, var(--text) 6%, transparent); }
  :global(.seg2 button) { padding: 0.55rem 0.4rem; border: none; border-radius: var(--radius-sm); background: transparent; color: var(--muted); font-weight: 700; font-size: 0.88rem; display: inline-flex; gap: 0.4rem; align-items: center; justify-content: center; }
  :global(.seg2 button.on) { background: var(--accent); color: #fff; box-shadow: 0 4px 14px -4px var(--accent); }
  :global(.chips) { display: flex; gap: 0.35rem; flex-wrap: wrap; }
  :global(.chips span) { font-size: 0.74rem; font-weight: 700; padding: 0.15rem 0.55rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 8%, transparent); font-variant-numeric: tabular-nums; }
  :global(.chips span.w) { color: #4ade80; background: #22c55e22; }
  :global(.chips span.l) { color: #f87171; background: #ef444422; }
  :global(.twist) { font-size: 0.74rem; font-weight: 800; padding: 0.15rem 0.6rem; border-radius: 99rem; letter-spacing: 0.02em; }
  :global(.twist.surge) { background: linear-gradient(135deg, #f59e0b, #ec4899); color: #fff; box-shadow: 0 0 18px -2px #f59e0b; }
  :global(.twist.curse) { background: #7f1d1d; color: #fecaca; }
</style>
