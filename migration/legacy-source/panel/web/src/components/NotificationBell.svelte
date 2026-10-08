<script lang="ts">
  import { Bell, BellRing, Check, Trash2, Shield, Gavel, Swords, Info } from '@lucide/svelte';
  import { get, post, del } from '../lib/api';

  let { inline = false }: { inline?: boolean } = $props();

  type Note = { id: number; kind: string; title: string; body: string; link: string | null; created_at: string; read: boolean };
  let items = $state<Note[]>([]);
  let unread = $state(0);
  let open = $state(false);

  async function load() {
    try {
      const r = await get<{ unread: number; items: Note[] }>('/api/v1/notifications');
      items = r.items;
      unread = r.unread;
    } catch { /* ignore */ }
  }
  $effect(() => {
    void load();
    const t = setInterval(load, 45_000);
    return () => clearInterval(t);
  });
  async function toggle() { open = !open; if (open) await load(); }
  async function readAll() { await post('/api/v1/notifications/read', {}); await load(); }
  async function clearAll() { await del('/api/v1/notifications'); await load(); }
  async function readOne(n: Note) { if (!n.read) { await post('/api/v1/notifications/read', { ids: [n.id] }); await load(); } }
  function ago(iso: string) {
    const s = Math.max(1, (Date.now() - new Date(iso).getTime()) / 1000);
    if (s < 3600) return `${Math.round(s / 60)}m ago`;
    if (s < 86400) return `${Math.round(s / 3600)}h ago`;
    return `${Math.round(s / 86400)}d ago`;
  }
  const icon = (k: string) => (k.startsWith('guild') ? Shield : k.startsWith('auction') || k.startsWith('market') ? Gavel : k.startsWith('quest') ? Swords : Info);
</script>

<svelte:window onclick={(e) => { if (open && !(e.target as HTMLElement).closest('.bell-wrap')) open = false; }} />

<div class="bell-wrap" class:inline>
  <button class="bell" class:has={unread > 0} onclick={toggle} aria-label="Notifications" title="Notifications">
    {#if unread > 0}<BellRing size={18} />{:else}<Bell size={18} />{/if}
    {#if unread > 0}<span class="badge">{unread > 9 ? '9+' : unread}</span>{/if}
  </button>
  {#if open}
    <div class="panel" role="dialog" aria-label="Notifications">
      <header>
        <strong>Notifications</strong>
        <span class="actions">
          <button onclick={readAll} disabled={!unread}><Check size={13} /> Read all</button>
          <button onclick={clearAll} disabled={!items.length} aria-label="Clear all"><Trash2 size={13} /></button>
        </span>
      </header>
      <div class="list">
        {#each items as n (n.id)}
          {@const Icon = icon(n.kind)}
          <button class="row" class:unread={!n.read} onclick={() => readOne(n)}>
            <span class="ico"><Icon size={15} /></span>
            <span class="txt"><b>{n.title}</b><span>{n.body}</span><em>{ago(n.created_at)}</em></span>
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
  .bell-wrap { position: fixed; top: 0.9rem; right: 1.2rem; z-index: 50; }
  .bell-wrap.inline { position: relative; top: auto; right: auto; z-index: 40; }
  .inline .bell { box-shadow: none; background: color-mix(in srgb, var(--surface) 70%, transparent); }
  .inline .panel { right: -3.2rem; width: min(23rem, calc(100vw - 1.6rem)); }
  @media (max-width: 640px) { .inline .panel { position: fixed; top: 4rem; left: 0.8rem; right: 0.8rem; width: auto; } }
  .bell { position: relative; width: 2.5rem; height: 2.5rem; border-radius: 50%; display: grid; place-items: center; padding: 0; background: var(--surface, #1b1b22); border: 1px solid var(--line, #ffffff22); color: inherit; cursor: pointer; box-shadow: 0 0.4rem 1rem -0.5rem rgba(0,0,0,.6); }
  .has { color: var(--accent, #8b7bff); }
  .has :global(svg) { animation: ring 2.4s ease-in-out infinite; transform-origin: 50% 10%; }
  @keyframes ring { 0%, 70%, 100% { transform: rotate(0); } 75% { transform: rotate(14deg); } 85% { transform: rotate(-12deg); } 92% { transform: rotate(6deg); } }
  .badge { position: absolute; top: -0.2rem; right: -0.2rem; min-width: 1.1rem; height: 1.1rem; padding: 0 0.28rem; border-radius: 99rem; background: #ef4444; color: #fff; font-size: 0.65rem; font-weight: 700; display: grid; place-items: center; }
  .panel { position: absolute; top: 3rem; right: 0; width: 23rem; max-height: 28rem; display: flex; flex-direction: column; background: var(--surface, #1b1b22); border: 1px solid var(--line, #ffffff22); border-radius: 0.9rem; box-shadow: 0 1.2rem 3rem -1rem rgba(0,0,0,.7); overflow: hidden; }
  header { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 0.95rem; border-bottom: 1px solid var(--line, #ffffff1a); font-size: 0.88rem; }
  .actions { display: flex; gap: 0.35rem; }
  .actions button { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; padding: 0.25rem 0.55rem; }
  .list { overflow-y: auto; }
  .row { display: flex; gap: 0.7rem; width: 100%; padding: 0.75rem 0.95rem; border: 0; border-bottom: 1px solid var(--line, #ffffff14); border-radius: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; }
  .row:hover { background: #ffffff0d; }
  .row.unread { background: color-mix(in srgb, var(--accent, #8b7bff) 10%, transparent); }
  .ico { width: 1.9rem; height: 1.9rem; border-radius: 0.55rem; display: grid; place-items: center; flex-shrink: 0; background: color-mix(in srgb, var(--accent, #8b7bff) 18%, transparent); color: var(--accent, #8b7bff); }
  .txt { display: flex; flex-direction: column; gap: 0.15rem; font-size: 0.8rem; min-width: 0; }
  .txt span { opacity: 0.75; line-height: 1.35; }
  .txt em { font-style: normal; font-size: 0.68rem; opacity: 0.5; }
  .dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: var(--accent, #8b7bff); margin-left: auto; margin-top: 0.3rem; flex-shrink: 0; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 2rem 1rem; margin: 0; opacity: 0.6; font-size: 0.82rem; }
</style>
