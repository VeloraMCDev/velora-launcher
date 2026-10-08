<script lang="ts">
  import { Plus, Server, Users, Gauge, ShieldCheck, Rocket, ChevronRight } from '@lucide/svelte';
  import Modal from '../components/Modal.svelte';
  import ServerForm from '../components/ServerForm.svelte';
  import TokenReveal from '../components/TokenReveal.svelte';
  import { get, post, timeAgo } from '../lib/api';
  import { go } from '../lib/router.svelte';
  import { toastError } from '../lib/toast.svelte';
  import type { AuthServerInfo, GameServer, Group, ServerDraft } from '../lib/types';

  let servers = $state<GameServer[] | null>(null);
  let groups = $state<Group[]>([]);
  let info = $state<AuthServerInfo | null>(null);
  let draft = $state<ServerDraft>({ name: '', access: 'all', allowed_groups: [], require_launcher: false, map_enabled: true });
  let createOpen = $state(false);
  let creating = $state(false);
  let created = $state<{ id: number; token: string } | null>(null);
  let tokenOpen = $state(false);

  const load = () => get<GameServer[]>('/api/admin/servers').then((s) => (servers = s)).catch(toastError);
  $effect(() => {
    load();
    get<Group[]>('/api/admin/groups').then((g) => (groups = g)).catch(() => {});
    get<AuthServerInfo>('/api/admin/auth-server').then((i) => (info = i)).catch(() => {});
    const t = setInterval(load, 15000);
    return () => clearInterval(t);
  });

  const online = $derived(servers?.reduce((a, s) => a + s.players, 0) ?? 0);

  function openCreate() {
    draft = { name: '', access: 'all', allowed_groups: [], require_launcher: false, map_enabled: true };
    createOpen = true;
  }

  async function create() {
    creating = true;
    try {
      const r = await post<{ server: GameServer; token: string }>('/api/admin/servers', draft);
      created = { id: r.server.id, token: r.token };
      createOpen = false;
      tokenOpen = true;
      load();
    } catch (e) {
      toastError(e);
    } finally {
      creating = false;
    }
  }

  const accessLabel = (s: GameServer) => (s.access === 'all' ? 'Anyone' : s.access === 'members' ? 'Panel accounts' : s.allowed_groups.join(', '));
  const tpsTone = (t: number | null) => (t == null ? '' : t >= 18 ? 'good' : t >= 14 ? 'warn' : 'bad');
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Servers</h1>
      <p>Game servers running the Velora plugin or mod report players, stats and events here — and ask the panel who may join.</p>
    </div>
    <button class="primary" onclick={openCreate}><Plus size={16} /> Add server</button>
  </div>

  {#if servers && !servers.length}
    <section class="card empty">
      <Server size={32} />
      <h3>Connect your first server</h3>
      <p>Add a server to get a token, drop the Velora plugin or mod on it, and you'll see live players, playtime leaderboards and events.</p>
      <div class="features">
        <span><Users size={15} /> Live players &amp; stats</span>
        <span><ShieldCheck size={15} /> Bans &amp; access groups</span>
        <span><Rocket size={15} /> "Launcher only" joins</span>
      </div>
      <button class="primary add" onclick={openCreate}><Plus size={16} /> Add server</button>
    </section>
  {:else if servers}
    <p class="summary muted small"><span class="live"></span> {online} player{online === 1 ? '' : 's'} online across {servers.filter((s) => s.online).length} of {servers.length} server{servers.length === 1 ? '' : 's'}</p>
    <div class="list">
      {#each servers as s (s.id)}
        <a class="card hover srv" href="#/servers/{s.id}">
          <span class="status" class:on={s.online} title={s.online ? 'Online' : 'Offline'}></span>
          <div class="who">
            <strong>{s.name}</strong>
            <span class="muted tiny">
              {#if s.software}{s.software}{s.mc_version ? ` ${s.mc_version}` : ''}{:else}Waiting for the first connection…{/if}
              {#if s.last_seen && !s.online} · last seen {timeAgo(s.last_seen)}{/if}
            </span>
          </div>
          <div class="metric"><Users size={14} /><strong>{s.players}</strong><span class="muted">/ {s.max_players || '–'}</span></div>
          <div class="metric {tpsTone(s.online ? s.tps : null)}"><Gauge size={14} /><strong>{s.online && s.tps != null ? s.tps.toFixed(1) : '–'}</strong><span class="muted">TPS</span></div>
          <div class="tags">
            <span class="badge">{accessLabel(s)}</span>
            {#if s.require_launcher}<span class="badge accent">Launcher only</span>{/if}
            {#if s.online_mode === false}<span class="badge warn">Offline mode</span>{/if}
          </div>
          <ChevronRight size={16} class="chev" />
        </a>
      {/each}
    </div>
  {/if}
</div>

<Modal bind:open={createOpen} title="Add server" width={540}>
  <ServerForm bind:draft {groups} />
  {#snippet footer()}
    <button class="ghost" onclick={() => (createOpen = false)}>Cancel</button>
    <button class="primary" disabled={creating || !draft.name.trim() || (draft.access === 'groups' && !draft.allowed_groups.length)} onclick={create}>Create &amp; get token</button>
  {/snippet}
</Modal>

<Modal bind:open={tokenOpen} title="Connect {draft.name || 'your server'}" width={600}>
  {#if created}<TokenReveal token={created.token} panelUrl={info?.public_url ?? location.origin} />{/if}
  {#snippet footer()}
    <button class="primary" onclick={() => { tokenOpen = false; if (created) go(`servers/${created.id}`); }}>Done</button>
  {/snippet}
</Modal>

<style>
  .summary { display: flex; align-items: center; gap: 8px; margin: -12px 0 14px; }
  .live { width: 8px; height: 8px; border-radius: 50%; background: var(--good); }
  .list { display: flex; flex-direction: column; gap: 10px; }
  .srv { display: grid; grid-template-columns: auto minmax(0, 1.6fr) 110px 110px minmax(0, 1.2fr) auto; align-items: center; gap: 18px; padding: 16px 18px; color: var(--text); }
  .status { width: 10px; height: 10px; border-radius: 50%; background: color-mix(in srgb, var(--muted) 45%, transparent); }
  .status.on { background: var(--good); box-shadow: 0 0 0 3px color-mix(in srgb, var(--good) 22%, transparent); }
  .who { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .who strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .metric { display: flex; align-items: center; gap: 6px; font-variant-numeric: tabular-nums; font-size: 0.9rem; }
  .metric :global(svg) { color: var(--muted); }
  .metric.good strong { color: #6ee7b7; }
  .metric.warn strong { color: #fcd34d; }
  .metric.bad strong { color: #fda4af; }
  .tags { display: flex; gap: 6px; flex-wrap: wrap; justify-content: flex-end; }
  .srv :global(.chev) { color: var(--muted); }
  .empty :global(svg) { color: var(--muted); }
  .empty p { max-width: 520px; margin: 0 auto; line-height: 1.5; }
  .features { display: flex; gap: 20px; justify-content: center; margin-top: 18px; flex-wrap: wrap; font-size: 0.85rem; color: var(--text-2); }
  .features span { display: inline-flex; align-items: center; gap: 6px; }
  .features :global(svg) { color: var(--accent-2); }
  .add { margin-top: 20px; }
  @media (max-width: 900px) {
    .srv { grid-template-columns: auto 1fr auto; }
    .srv .metric:nth-of-type(2), .srv .tags, .srv :global(.chev) { display: none; }
  }
</style>
