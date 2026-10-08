<script lang="ts">
  import { Minus, Square, X, WifiOff, Users, Search } from '@lucide/svelte';
  import { experienceBranding, abs, activeAccount, app, saveSettings } from '../lib/store.svelte';
  import { windowAction } from '../lib/tauri';
  import NotificationBell from './NotificationBell.svelte';

  const b = $derived(experienceBranding());
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    {#if b?.logo_url}
      <img src={abs(b.logo_url)} alt="" />
    {:else}
      <span class="mark"></span>
    {/if}
    <span data-tauri-drag-region>{b?.name ?? 'Launcher'}</span>
    {#if app.offline}<span class="offline" title="Can't reach the server — using saved data"><WifiOff size={12} /> Offline</span>{/if}
  </div>
  <div class="controls">
    {#if activeAccount()}<button class="ctl search" title="Quick search (Ctrl+K)" aria-label="Quick search" onclick={() => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }))}><Search size={15} /></button><button class="ctl" class:on={app.settings?.show_social_sidebar} title="Friends" aria-label="Toggle social sidebar" aria-pressed={app.settings?.show_social_sidebar ?? false} onclick={() => { if (app.settings) { app.settings.show_social_sidebar = !app.settings.show_social_sidebar; saveSettings(); } }}><Users size={16} /></button><NotificationBell />{/if}
    <button class="ctl" onclick={() => windowAction('minimize')} aria-label="Minimise"><Minus size={15} /></button>
    <button class="ctl" onclick={() => windowAction('maximize')} aria-label="Maximise"><Square size={12} /></button>
    <button class="ctl close" onclick={() => windowAction('close')} aria-label="Close"><X size={16} /></button>
  </div>
</header>

<style>
  header {
    height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 1.1rem;
    position: relative;
    z-index: 30;
    flex-shrink: 0;
    border-bottom: 1px solid var(--line);
    background: color-mix(in srgb, var(--surface) 50%, transparent);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }
  .brand { display: flex; align-items: center; gap: 0.65rem; font-weight: 650; font-size: 0.82rem; letter-spacing: 0.06em; pointer-events: none; }
  .brand img { height: 1.35rem; border-radius: 0.35rem; }
  .mark { width: 0.85rem; height: 0.85rem; border-radius: 50%; border: 2px solid var(--accent); position: relative; box-shadow: 0 0 8px color-mix(in srgb, var(--accent) 50%, transparent); }
  .mark::after { content: ''; position: absolute; inset: 0.15rem; border-radius: 50%; background: var(--accent); }
  .offline { display: inline-flex; align-items: center; gap: 0.35rem; font-size: 0.72rem; font-weight: 600; color: var(--warn); background: color-mix(in srgb, var(--warn) 14%, transparent); border: 1px solid color-mix(in srgb, var(--warn) 30%, transparent); padding: 0.15rem 0.55rem; border-radius: 99rem; pointer-events: auto; }
  .controls { display: flex; height: 100%; }
  .ctl { height: 100%; width: 2.9rem; border: none; border-radius: 0; background: transparent; color: color-mix(in srgb, var(--text) 65%, transparent); padding: 0; transition: background 0.15s, color 0.15s; }
  .ctl:hover { background: color-mix(in srgb, var(--text) 10%, transparent); color: var(--text); }
  .ctl:active { transform: none; }
  .ctl.on { color: var(--accent); background: color-mix(in srgb, var(--accent) 12%, transparent); }
  .close:hover { background: var(--danger); color: white; }
</style>
