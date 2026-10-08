<script lang="ts">
  import { Bell, BellRing, Check, Trash2, Shield, Gavel, Swords, Info, Crosshair } from '@lucide/svelte';
  import { invoke } from '../lib/tauri';
  import { onMount } from 'svelte';

  type Note = { id: number; kind: string; title: string; body: string; link: string | null; created_at: string; read: boolean };
  let items = $state<Note[]>([]);
  let unread = $state(0);
  let open = $state(false);

  async function load() {
    try {
      const r = await invoke<{ unread: number; items: Note[] }>('notifications_list');
      items = r.items;
      unread = r.unread;
    } catch { /* offline: keep what we have */ }
  }
  onMount(() => {
    void load();
    const t = setInterval(load, 45_000);
    return () => clearInterval(t);
  });

  async function toggle() {
    open = !open;
    if (open) await load();
  }
  async function readAll() {
    await invoke('notifications_mark', { ids: null, clear: false });
    await load();
  }
  async function clearAll() {
    await invoke('notifications_mark', { ids: null, clear: true });
    await load();
  }
  async function readOne(n: Note) {
    if (n.read) return;
    await invoke('notifications_mark', { ids: [n.id], clear: false });
    await load();
  }
  function ago(iso: string) {
    const s = Math.max(1, (Date.now() - new Date(iso).getTime()) / 1000);
    if (s < 3600) return `${Math.round(s / 60)}m ago`;
    if (s < 86400) return `${Math.round(s / 3600)}h ago`;
    return `${Math.round(s / 86400)}d ago`;
  }
  const icon = (k: string) => (k.startsWith('bounty') ? Crosshair : k.startsWith('guild') ? Shield : k.startsWith('auction') || k.startsWith('market') ? Gavel : k.startsWith('quest') ? Swords : Info);
</script>

<svelte:window onclick={(e) => { if (open && !(e.target as HTMLElement).closest('.bell-wrap')) open = false; }} />

<div class="bell-wrap">
  <button class="ctl bell" class:has={unread > 0} onclick={toggle} aria-label="Notifications" title="Notifications">
    {#if unread > 0}<BellRing size={15} />{:else}<Bell size={15} />{/if}
    {#if unread > 0}<span class="badge">{unread > 9 ? '9+' : unread}</span>{/if}
  </button>
  {#if open}
    <div class="panel glass" role="dialog" aria-label="Notifications">
      <header>
        <strong>Notifications</strong>
        <span class="actions">
          <button class="mini" onclick={readAll} disabled={!unread} title="Mark all as read"><Check size={13} /> Read all</button>
          <button class="mini" onclick={clearAll} disabled={!items.length} title="Clear all"><Trash2 size={13} /></button>
        </span>
      </header>
      <div class="list">
        {#each items as n (n.id)}
          {@const Icon = icon(n.kind)}
          <button class="row" class:unread={!n.read} onclick={() => readOne(n)}>
            <span class="ico"><Icon size={15} /></span>
            <span class="txt">
              <b>{n.title}</b>
              <span>{n.body}</span>
              <em>{ago(n.created_at)}</em>
            </span>
            {#if !n.read}<i class="dot"></i>{/if}
          </button>
        {:else}
          <p class="empty"><Bell size={22} />You're all caught up.</p>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .bell-wrap { position: relative; height: 100%; display: flex; }
  .ctl { height: 100%; width: 2.9rem; border: none; border-radius: 0; background: transparent; color: color-mix(in srgb, var(--text) 65%, transparent); padding: 0; position: relative; transition: background 0.15s, color 0.15s; }
  .ctl:hover { background: color-mix(in srgb, var(--text) 10%, transparent); color: var(--text); }
  .ctl:active { transform: none; }
  .has { color: var(--accent); }
  .has :global(svg) { animation: ring 2.4s ease-in-out infinite; transform-origin: 50% 10%; }
  @keyframes ring { 0%, 70%, 100% { transform: rotate(0); } 75% { transform: rotate(14deg); } 85% { transform: rotate(-12deg); } 92% { transform: rotate(6deg); } }
  .badge { position: absolute; top: 0.45rem; right: 0.5rem; min-width: 1rem; height: 1rem; padding: 0 0.25rem; border-radius: 99rem; background: var(--danger); color: white; font-size: 0.62rem; font-weight: 700; display: grid; place-items: center; line-height: 1; }
  .panel { position: absolute; top: calc(100% + 0.35rem); right: 0.4rem; width: 22rem; max-height: 26rem; display: flex; flex-direction: column; z-index: 60; background: var(--surface); border-color: var(--line-strong); box-shadow: 0 1rem 2.5rem -0.8rem rgba(0, 0, 0, 0.6); animation: pop 0.14s ease; overflow: hidden; }
  @keyframes pop { from { opacity: 0; transform: translateY(-4px) scale(0.98); } }
  header { all: unset; display: flex; align-items: center; justify-content: space-between; padding: 0.7rem 0.9rem; border-bottom: 1px solid var(--line); font-size: 0.85rem; box-sizing: border-box; }
  .actions { display: flex; gap: 0.35rem; }
  .mini { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; padding: 0.25rem 0.5rem; height: auto; }
  .list { overflow-y: auto; }
  .row { all: unset; box-sizing: border-box; width: 100%; display: flex; gap: 0.7rem; padding: 0.7rem 0.9rem; cursor: pointer; border-bottom: 1px solid var(--line); text-align: left; position: relative; }
  .row:hover { background: color-mix(in srgb, var(--text) 6%, transparent); }
  .row.unread { background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .ico { width: 1.9rem; height: 1.9rem; border-radius: 0.55rem; display: grid; place-items: center; flex-shrink: 0; background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--accent); }
  .txt { display: flex; flex-direction: column; gap: 0.15rem; font-size: 0.78rem; min-width: 0; }
  .txt b { font-size: 0.82rem; }
  .txt span { color: color-mix(in srgb, var(--text) 75%, transparent); line-height: 1.35; }
  .txt em { font-style: normal; font-size: 0.68rem; color: color-mix(in srgb, var(--text) 50%, transparent); }
  .dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: var(--accent); margin-left: auto; margin-top: 0.3rem; flex-shrink: 0; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 2rem 1rem; margin: 0; color: color-mix(in srgb, var(--text) 55%, transparent); font-size: 0.82rem; }
</style>
