<script lang="ts">
  import PlayerLink from '../components/PlayerLink.svelte';
  import {
    ArrowLeft, Settings2, KeyRound, Trash2, Users, Gauge, Clock, UserRound, LogIn, LogOut, Skull, Award, Sword, MessageSquare,
    CircleDot, Ban, Server,
  } from '@lucide/svelte';
  import Avatar from '../components/Avatar.svelte';
  import IntegrationsPanel from '../components/IntegrationsPanel.svelte';
  import MapPanel from '../components/MapPanel.svelte';
  import Modal from '../components/Modal.svelte';
  import ServerForm from '../components/ServerForm.svelte';
  import TokenReveal from '../components/TokenReveal.svelte';
  import { del, duration, get, post, put, timeAgo } from '../lib/api';
  import { go } from '../lib/router.svelte';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { AuthServerInfo, Group, ServerDetail, ServerDraft, ServerEvent } from '../lib/types';

  let { id }: { id: string } = $props();

  let d = $state<ServerDetail | null>(null);
  let sort = $state('playtime_secs');
  let groups = $state<Group[]>([]);
  let info = $state<AuthServerInfo | null>(null);
  let draft = $state<ServerDraft>({ name: '', access: 'all', allowed_groups: [], require_launcher: false, map_enabled: false });
  let editOpen = $state(false);
  let token = $state<string | null>(null);
  let tokenOpen = $state(false);
  let regenOpen = $state(false);
  let deleteOpen = $state(false);

  const load = () =>
    get<ServerDetail>(`/api/admin/servers/${id}?sort=${sort}`)
      .then((x) => (d = x))
      .catch((e) => {
        toastError(e);
        if (e.status === 404) go('servers');
      });

  $effect(() => {
    void sort;
    load();
  });
  $effect(() => {
    get<Group[]>('/api/admin/groups').then((g) => (groups = g)).catch(() => {});
    get<AuthServerInfo>('/api/admin/auth-server').then((i) => (info = i)).catch(() => {});
    const t = setInterval(load, 15000);
    return () => clearInterval(t);
  });

  const s = $derived(d?.server);
  const columns = [
    { id: 'playtime_secs', label: 'Playtime' },
    { id: 'joins', label: 'Joins' },
    { id: 'deaths', label: 'Deaths' },
    { id: 'player_kills', label: 'PvP kills' },
    { id: 'mob_kills', label: 'Mob kills' },
    { id: 'blocks_broken', label: 'Mined' },
    { id: 'blocks_placed', label: 'Placed' },
    { id: 'messages', label: 'Chat' },
  ] as const;

  const eventIcon: Record<string, any> = { join: LogIn, quit: LogOut, death: Skull, advancement: Award, kill: Sword, chat: MessageSquare, kick: Ban, denied: Ban, start: Server, stop: Server };
  function eventText(e: ServerEvent): string {
    switch (e.kind) {
      case 'join': return 'joined the game';
      case 'quit': return 'left the game';
      case 'denied': return e.detail ? `was refused: ${e.detail}` : 'was refused';
      case 'kick': return e.detail ? `was kicked: ${e.detail}` : 'was kicked';
      case 'advancement': return e.detail ? `made the advancement [${e.detail}]` : 'made an advancement';
      case 'start': return 'Server started';
      case 'stop': return 'Server stopped';
      default: return e.detail ?? e.kind;
    }
  }
  // Death messages already contain the player's name.
  const showName = (e: ServerEvent) => e.name && !(e.kind === 'death' && e.detail?.startsWith(e.name)) && e.kind !== 'start' && e.kind !== 'stop';

  const n = (v: number) => v.toLocaleString();
  const cell = (row: any, c: string) => (c === 'playtime_secs' ? duration(row[c]) : n(row[c]));
  const tpsTone = (t: number | null | undefined) => (t == null ? '' : t >= 18 ? 'good' : t >= 14 ? 'warn' : 'bad');

  function openEdit() {
    if (!s) return;
    draft = { instance_id: s.instance_id, name: s.name, map_enabled: s.map_enabled, economy_group: s.economy_group, access: s.access, allowed_groups: [...s.allowed_groups], require_launcher: s.require_launcher };
    editOpen = true;
  }
  async function saveEdit() {
    try {
      await put(`/api/admin/servers/${id}`, draft);
      editOpen = false;
      toast('Server updated');
      load();
    } catch (e) {
      toastError(e);
    }
  }
  async function regenerate() {
    try {
      token = (await post<{ token: string }>(`/api/admin/servers/${id}/token`)).token;
      regenOpen = false;
      tokenOpen = true;
      load();
    } catch (e) {
      toastError(e);
    }
  }
  async function remove() {
    try {
      await del(`/api/admin/servers/${id}`);
      toast('Server removed');
      go('servers');
    } catch (e) {
      toastError(e);
    }
  }
</script>

<div class="page">
  <a class="back" href="#/servers"><ArrowLeft size={15} /> Servers</a>
  {#if s && d}
    <div class="page-head">
      <div>
        <h1 class="row title"><span class="status" class:on={s.online}></span>{s.name}</h1>
        <p>
          {[
            s.software ? `${s.software}${s.mc_version ? ` ${s.mc_version}` : ''}` : 'Not connected yet',
            s.plugin_version ? `integration ${s.plugin_version}` : null,
            s.online ? 'online' : s.last_seen ? `last seen ${timeAgo(s.last_seen)}` : null,
          ].filter(Boolean).join(' · ')}
        </p>
      </div>
      <div class="row">
        <button onclick={openEdit}><Settings2 size={16} /> Settings</button>
        <button onclick={() => (regenOpen = true)}><KeyRound size={16} /> New token</button>
        <button class="ghost icon" aria-label="Remove server" onclick={() => (deleteOpen = true)}><Trash2 size={16} /></button>
      </div>
    </div>

    {#if !s.last_seen}
      <section class="card connect">
        <h3>Waiting for the server to connect</h3>
        <p class="muted small">Install the SCOPENET plugin or mod and put the token (ending in <code>…{s.token_hint}</code>) in its config. Lost it? Generate a new one.</p>
      </section>
    {/if}

    <div class="grid tiles">
      <div class="card tile"><span class="lbl"><Users size={14} /> Online now</span><span class="val">{s.players}<small> / {s.max_players || '–'}</small></span></div>
      <div class="card tile {tpsTone(s.online ? s.tps : null)}"><span class="lbl"><Gauge size={14} /> TPS</span><span class="val">{s.online && s.tps != null ? s.tps.toFixed(1) : '–'}</span></div>
      <div class="card tile"><span class="lbl"><UserRound size={14} /> Players seen</span><span class="val">{n(d.totals.players)}</span></div>
      <div class="card tile"><span class="lbl"><Clock size={14} /> Total playtime</span><span class="val">{duration(d.totals.playtime_secs)}</span></div>
    </div>

    <MapPanel id={s.id} enabled={s.map_enabled} />

    <IntegrationsPanel id={s.id} />

    <div class="grid layout">
      <section class="card board">
        <div class="section-title">
          <h2>Leaderboard</h2>
          <select class="sort" bind:value={sort} aria-label="Sort by">
            {#each columns as c}<option value={c.id}>{c.label}</option>{/each}
            <option value="last_seen">Recently seen</option>
          </select>
        </div>
        {#if d.leaderboard.length}
          <div class="table-wrap">
            <table class="table">
              <thead>
                <tr><th>#</th><th>Player</th>{#each columns as c}<th class="num" class:sorted={sort === c.id}>{c.label}</th>{/each}<th>Last seen</th></tr>
              </thead>
              <tbody>
                {#each d.leaderboard as row, i (row.uuid)}
                  <tr>
                    <td class="rank" class:top={i < 3 && sort !== 'last_seen'}>{i + 1}</td>
                    <td><span class="row who"><Avatar name={row.name} uuid={row.uuid} size={24} /><PlayerLink uuid={row.uuid} name={row.name} /></span></td>
                    {#each columns as c}<td class="num" class:sorted={sort === c.id}>{cell(row, c.id)}</td>{/each}
                    <td class="muted small nowrap">{d.online_players.some((p) => p.uuid === row.uuid) ? 'Online' : timeAgo(row.last_seen)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="muted small">Stats appear once players join.</p>
        {/if}
      </section>

      <div class="col side">
        <section class="card">
          <div class="section-title"><h2>Online</h2><span class="hint">{d.online_players.length}</span></div>
          {#if d.online_players.length}
            <div class="online">
              {#each d.online_players as p (p.uuid)}
                <div class="row op"><Avatar name={p.name} uuid={p.uuid} size={26} /><span class="grow"><PlayerLink uuid={p.uuid} name={p.name} /></span><span class="muted tiny">{timeAgo(p.joined_at).replace(' ago', '')}</span></div>
              {/each}
            </div>
          {:else}
            <p class="muted small">Nobody's online.</p>
          {/if}
        </section>

        <section class="card">
          <div class="section-title"><h2>Activity</h2></div>
          {#if d.events.length}
            <div class="feed">
              {#each d.events as e (e.id)}
                {@const Icon = eventIcon[e.kind] ?? CircleDot}
                <div class="ev {e.kind}">
                  <span class="ic"><Icon size={13} /></span>
                  <span class="txt">{#if showName(e)}<strong>{e.name}</strong>{' '}{/if}{eventText(e)}</span>
                  <span class="muted tiny nowrap">{timeAgo(e.created_at)}</span>
                </div>
              {/each}
            </div>
          {:else}
            <p class="muted small">Joins, deaths and advancements show up here.</p>
          {/if}
        </section>
      </div>
    </div>
  {:else}
    <div class="skeleton"></div>
  {/if}
</div>

<Modal bind:open={editOpen} title="Server settings" width={540}>
  <ServerForm bind:draft {groups} />
  {#snippet footer()}
    <button class="ghost" onclick={() => (editOpen = false)}>Cancel</button>
    <button class="primary" disabled={!draft.name.trim() || (draft.access === 'groups' && !draft.allowed_groups.length)} onclick={saveEdit}>Save</button>
  {/snippet}
</Modal>

<Modal bind:open={regenOpen} title="Generate a new token?">
  <p class="muted">The current token stops working immediately — the server disconnects from the panel until you update its config.</p>
  {#snippet footer()}
    <button class="ghost" onclick={() => (regenOpen = false)}>Cancel</button>
    <button class="primary" onclick={regenerate}><KeyRound size={16} /> Generate</button>
  {/snippet}
</Modal>

<Modal bind:open={tokenOpen} title="New token for {s?.name}" width={600}>
  {#if token}<TokenReveal {token} panelUrl={info?.public_url ?? location.origin} />{/if}
  {#snippet footer()}
    <button class="primary" onclick={() => (tokenOpen = false)}>Done</button>
  {/snippet}
</Modal>

<Modal bind:open={deleteOpen} title="Remove {s?.name}?">
  <p class="muted">Its token stops working and all stats and events collected for it are deleted.</p>
  {#snippet footer()}
    <button class="ghost" onclick={() => (deleteOpen = false)}>Cancel</button>
    <button class="danger" onclick={remove}><Trash2 size={16} /> Remove</button>
  {/snippet}
</Modal>

<style>
  .back { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 0.85rem; margin-bottom: 14px; }
  .back:hover { color: var(--text); }
  .title { gap: 12px; }
  .status { width: 11px; height: 11px; border-radius: 50%; background: color-mix(in srgb, var(--muted) 45%, transparent); flex-shrink: 0; }
  .status.on { background: var(--good); box-shadow: 0 0 0 4px color-mix(in srgb, var(--good) 20%, transparent); }
  .connect { margin-bottom: 16px; border-color: color-mix(in srgb, var(--accent) 35%, transparent); background: color-mix(in srgb, var(--accent) 6%, var(--surface)); display: flex; flex-direction: column; gap: 6px; }
  .tiles { grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); margin-bottom: 16px; }
  .tile { display: flex; flex-direction: column; gap: 8px; padding: 16px 18px; }
  .tile .lbl { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 0.8rem; }
  .tile .val { font-size: 1.6rem; font-weight: 600; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .tile .val small { font-size: 0.9rem; color: var(--muted); font-weight: 500; }
  .tile.good .val { color: #6ee7b7; }
  .tile.warn .val { color: #fcd34d; }
  .tile.bad .val { color: #fda4af; }
  .layout { grid-template-columns: minmax(0, 1fr) 320px; align-items: start; }
  @media (max-width: 1150px) { .layout { grid-template-columns: 1fr; } }
  .board { padding-bottom: 8px; }
  .sort { width: auto; margin-left: auto; padding: 7px 34px 7px 12px; font-size: 0.85rem; }
  .table-wrap { overflow-x: auto; margin: 0 -20px; }
  .table th, .table td { padding: 10px 9px; }
  .table th:first-child, .table td:first-child { padding-left: 20px; }
  .table th:last-child, .table td:last-child { padding-right: 20px; }
  .table { font-size: 0.85rem; }
  .num { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
  th.num { text-align: right; }
  .sorted { color: var(--text); }
  td.num:not(.sorted) { color: var(--text-2); }
  .rank { color: var(--muted); font-variant-numeric: tabular-nums; width: 36px; }
  .rank.top { color: #fcd34d; font-weight: 600; }
  .who { gap: 10px; white-space: nowrap; }
  .nowrap { white-space: nowrap; }
  .side { gap: 16px; }
  .online { display: flex; flex-direction: column; gap: 8px; max-height: 320px; overflow-y: auto; }
  .op { gap: 10px; font-size: 0.9rem; }
  .grow { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .feed { display: flex; flex-direction: column; gap: 10px; max-height: 520px; overflow-y: auto; }
  .ev { display: grid; grid-template-columns: 24px 1fr auto; gap: 10px; align-items: start; font-size: 0.85rem; line-height: 1.4; }
  .ic { width: 24px; height: 24px; border-radius: 6px; display: grid; place-items: center; background: var(--bg-2); color: var(--muted); }
  .ev.join .ic { color: #6ee7b7; }
  .ev.death .ic, .ev.denied .ic, .ev.kick .ic { color: #fda4af; }
  .ev.advancement .ic { color: #fcd34d; }
  .txt { color: var(--text-2); padding-top: 3px; }
  .txt strong { color: var(--text); font-weight: 560; }
  .ev .tiny { padding-top: 5px; }
  .skeleton { height: 420px; border-radius: var(--radius); background: var(--surface); }
</style>
