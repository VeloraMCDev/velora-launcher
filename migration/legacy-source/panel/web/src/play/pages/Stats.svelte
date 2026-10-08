<script lang="ts">
  import { Box, Clock, Crown, Flame, LogIn, MessageSquare, Pickaxe, RefreshCw, Skull, Sparkles, Swords, Trophy, TriangleAlert, Server as ServerIcon, History, Medal } from '@lucide/svelte';
  import type { Component } from 'svelte';
  import { get } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import Avatar from '../../components/Avatar.svelte';
  import { compact, play } from '../store.svelte';
  import Count from '../ui/Count.svelte';
  import Empty from '../ui/Empty.svelte';
  import { ago, num, playtime } from '../lib/money-pages';

  type Row = { uuid: string; name: string; playtime_secs: number; joins: number; deaths: number; player_kills: number; mob_kills: number; blocks_broken: number; blocks_placed: number; messages: number; first_seen: string; last_seen: string };
  type Mine = { username: string; uuid: string; total: Row | null; servers: (Row & { server_id: number; server_name: string })[]; events: { id: number; server_id: number; kind: string; detail: string | null; created_at: string }[] };
  type Lvl = { rank: number; uuid: string; username: string; global_level: number; global_xp: number; progress_pct: number; title: string | null };
  type Field = keyof Pick<Row, 'playtime_secs' | 'player_kills' | 'mob_kills' | 'blocks_broken' | 'blocks_placed' | 'joins' | 'deaths' | 'messages'>;

  const SORTS: { id: Field; label: string; icon: Component<any>; fmt: (n: number) => string }[] = [
    { id: 'playtime_secs', label: 'Playtime', icon: Clock, fmt: playtime },
    { id: 'player_kills', label: 'PvP kills', icon: Swords, fmt: num },
    { id: 'mob_kills', label: 'Mob kills', icon: Flame, fmt: num },
    { id: 'blocks_broken', label: 'Mined', icon: Pickaxe, fmt: num },
    { id: 'blocks_placed', label: 'Placed', icon: Box, fmt: num },
    { id: 'joins', label: 'Joins', icon: LogIn, fmt: num },
    { id: 'deaths', label: 'Deaths', icon: Skull, fmt: num },
    { id: 'messages', label: 'Chat', icon: MessageSquare, fmt: num },
  ];

  let view = $state<'me' | 'boards' | 'levels'>('boards');
  let scope = $state<number | 'global'>('global');
  let sort = $state<Field>('playtime_secs');

  let mine = $state<Mine | null>(null);
  let mineLoading = $state(true);
  let mineError = $state('');
  let board = $state<Row[]>([]);
  let totals = $state<{ players: number; playtime_secs: number } | null>(null);
  let boardLoading = $state(true);
  let boardError = $state('');
  let lvls = $state<Lvl[]>([]);
  let lvlLoading = $state(true);
  let lvlError = $state('');
  let reqId = 0;

  const me = $derived(session.user?.uuid ?? '');
  const meta = $derived(SORTS.find((s) => s.id === sort)!);
  const MetaIcon = $derived(meta.icon);

  async function loadMine() {
    mineLoading = true;
    try { mine = await get<Mine>('/api/v1/account/stats'); mineError = ''; }
    catch (e) { mineError = e instanceof Error ? e.message : 'Could not load your stats'; }
    finally { mineLoading = false; }
  }
  async function loadBoard() {
    const id = ++reqId;
    boardLoading = true;
    const path = scope === 'global' ? `/api/v1/leaderboard?sort=${sort}&limit=50` : `/api/v1/servers/${scope}/leaderboard?sort=${sort}&limit=50`;
    try {
      const r = await get<{ leaderboard: Row[]; totals: { players: number; playtime_secs: number } }>(path);
      if (id !== reqId) return;
      board = r.leaderboard; totals = r.totals; boardError = '';
    } catch (e) { if (id === reqId) boardError = e instanceof Error ? e.message : 'Could not load the leaderboard'; }
    finally { if (id === reqId) boardLoading = false; }
  }
  async function loadLevels() {
    lvlLoading = true;
    try { lvls = await get<Lvl[]>('/api/v1/levels/leaderboard'); lvlError = ''; }
    catch (e) { lvlError = e instanceof Error ? e.message : 'Could not load levels'; }
    finally { lvlLoading = false; }
  }

  $effect(() => { void scope; void sort; void loadBoard(); });
  $effect(() => { void loadMine(); void loadLevels(); });
  // Follow the server picked in the header the first time it changes.
  let seen: number | null | undefined;
  $effect(() => {
    const s = play.serverId;
    if (seen !== undefined && s != null && s !== seen) scope = s;
    seen = s;
  });

  const podium = $derived(board.slice(0, 3));
  const rest = $derived(board.slice(3));
  const topVal = $derived(Math.max(1, board[0]?.[sort] ?? 1));
  const myIdx = $derived(board.findIndex((r) => r.uuid === me));
  const t = $derived(mine?.total ?? null);
  const kd = $derived(t ? (t.deaths ? (t.player_kills / t.deaths).toFixed(2) : String(t.player_kills)) : '—');
  const myLevel = $derived(lvls.find((l) => l.uuid === me));
  const order = [1, 0, 2]; // silver, gold, bronze visual order
  const tiles = $derived(t ? [
    { l: 'Playtime', v: playtime(t.playtime_secs), i: Clock }, { l: 'Joins', v: num(t.joins), i: LogIn }, { l: 'PvP kills', v: num(t.player_kills), i: Swords }, { l: 'Deaths', v: num(t.deaths), i: Skull },
    { l: 'Mob kills', v: num(t.mob_kills), i: Flame }, { l: 'Blocks mined', v: num(t.blocks_broken), i: Pickaxe }, { l: 'Blocks placed', v: num(t.blocks_placed), i: Box }, { l: 'Chat messages', v: num(t.messages), i: MessageSquare },
  ] : []);
  const evLabel = (k: string) => k.replace(/_/g, ' ');
  const serverName = (id: number) => play.servers.find((s) => s.id === id)?.name ?? `Server ${id}`;
</script>

<div class="pl-page stats">
  <div class="pl-head">
    <div><h1><Trophy size={26} /> Leaderboards</h1><p>Rankings, your stats and levels</p></div>
    <div class="pl-tabs" role="tablist">
      <button role="tab" class:on={view === 'boards'} aria-selected={view === 'boards'} onclick={() => (view = 'boards')}>Rankings</button>
      <button role="tab" class:on={view === 'me'} aria-selected={view === 'me'} onclick={() => (view = 'me')}>My stats</button>
      <button role="tab" class:on={view === 'levels'} aria-selected={view === 'levels'} onclick={() => (view = 'levels')}>Levels</button>
    </div>
  </div>

  {#if view === 'boards'}
    <div class="pl-stack">
      <div class="pl-scroll-x scope">
        <button class="fchip" class:on={scope === 'global'} onclick={() => (scope = 'global')}><Sparkles size={14} /> All servers</button>
        {#each play.servers as s (s.id)}<button class="fchip" class:on={scope === s.id} onclick={() => (scope = s.id)}><ServerIcon size={14} /> {s.name}</button>{/each}
      </div>
      <div class="pl-scroll-x scope">
        {#each SORTS as s (s.id)}{@const I = s.icon}<button class="fchip sm" class:on={sort === s.id} onclick={() => (sort = s.id)}><I size={14} /> {s.label}</button>{/each}
      </div>

      {#if boardError}
        <div class="pl-alert err row" role="alert"><TriangleAlert size={16} /><span class="grow">{boardError}</span><button class="pl-btn sm" onclick={loadBoard}>Retry</button></div>
      {/if}

      {#if boardLoading && !board.length}
        <div class="pl-skel" style="height: 220px; border-radius: 24px"></div>
        {#each Array(5) as _, i (i)}<div class="pl-skel" style="height: 56px"></div>{/each}
      {:else if !board.length}
        <div class="pl-card"><Empty icon={Trophy} title="No rankings yet" text="Play on a server and your stats start counting." /></div>
      {:else}
        <section class="pl-hero podium" aria-label="Top three">
          <div class="ptitle"><span><MetaIcon size={15} /> Top {meta.label.toLowerCase()}</span>{#if totals}<small>{num(totals.players)} players · {playtime(totals.playtime_secs)} played</small>{/if}</div>
          <div class="pods">
            {#each order as idx (idx)}
              {@const r = podium[idx]}
              {#if r}
                <div class="pod p{idx + 1}" class:me={r.uuid === me} style="--d: {idx * 90}ms">
                  {#if idx === 0}<Crown class="crown" size={22} />{/if}
                  <div class="av"><Avatar name={r.name} uuid={r.uuid} size={idx === 0 ? 68 : 54} /><span class="medal">{idx + 1}</span></div>
                  <b class="nm">{r.name}</b>
                  <span class="val">{meta.fmt(r[sort])}</span>
                  <div class="step"></div>
                </div>
              {:else}<div class="pod empty"></div>{/if}
            {/each}
          </div>
        </section>

        <section class="pl-card tight list" class:busy={boardLoading}>
          {#each rest as r, i (r.uuid)}
            <div class="pl-item row-r" class:me={r.uuid === me} style="--i: {Math.min(i, 14)}">
              <span class="rk">{i + 4}</span>
              <Avatar name={r.name} uuid={r.uuid} size={36} />
              <div class="grow"><b>{r.name}{#if r.uuid === me} <span class="pl-chip accent">You</span>{/if}</b>
                <div class="pl-progress thin"><i style="width: {Math.max(2, (r[sort] / topVal) * 100)}%"></i></div></div>
              <div class="end"><b>{meta.fmt(r[sort])}</b><span class="sub" style="display:block">{ago(r.last_seen)}</span></div>
            </div>
          {/each}
          {#if !rest.length}<p class="few">Only {board.length} player{board.length === 1 ? '' : 's'} ranked so far.</p>{/if}
        </section>
        {#if myIdx < 0 && mine && !boardLoading}<p class="few">You're not in the top 50 for this ranking yet. Keep playing!</p>{/if}
      {/if}
    </div>

  {:else if view === 'me'}
    <div class="pl-stack">
      {#if mineError}
        <div class="pl-alert err row" role="alert"><TriangleAlert size={16} /><span class="grow">{mineError}</span><button class="pl-btn sm" onclick={loadMine}>Retry</button></div>
      {/if}
      {#if mineLoading && !mine}
        <div class="pl-skel" style="height: 150px; border-radius: 24px"></div>
        <div class="pl-grid" style="--min: 150px">{#each Array(8) as _, i (i)}<div class="pl-skel" style="height: 84px"></div>{/each}</div>
      {:else if mine && !t}
        <div class="pl-card"><Empty icon={Trophy} title="No stats yet" text="Join a server and your playtime, kills and blocks are tracked automatically." /></div>
      {:else if mine && t}
        <section class="pl-hero me-hero">
          <Avatar name={mine.username} uuid={mine.uuid} size={72} />
          <div class="who">
            <h1>{mine.username}</h1>
            <p>{myLevel ? `Level ${myLevel.global_level}${myLevel.title ? ' · ' + myLevel.title : ''} · ` : ''}last seen {ago(t.last_seen)}</p>
            <div class="chips"><span class="pl-chip accent"><Clock size={12} /> <Count value={t.playtime_secs} format={(v) => playtime(Math.round(v))} /></span><span class="pl-chip">K/D {kd}</span>{#if myIdx >= 0}<span class="pl-chip good"><Medal size={12} /> #{myIdx + 1} · {meta.label}</span>{/if}</div>
          </div>
        </section>
        <div class="tiles">
          {#each tiles as x, i (x.l)}{@const I = x.i}
            <div class="tile pl-card tight" style="--i: {i}"><span><I size={14} /> {x.l}</span><b>{x.v}</b></div>
          {/each}
        </div>

        {#if mine.servers.length > 1}
          <section class="pl-card">
            <div class="pl-card-head"><h2><ServerIcon size={17} /> By server</h2></div>
            <div class="pl-grid" style="--min: 230px">
              {#each mine.servers as s (s.server_id)}
                <div class="srv"><b>{s.server_name}</b><div class="mini"><span><Clock size={12} /> {playtime(s.playtime_secs)}</span><span><Swords size={12} /> {compact(s.player_kills)}</span><span><Skull size={12} /> {compact(s.deaths)}</span><span><Pickaxe size={12} /> {compact(s.blocks_broken)}</span></div>
                  <small>since {new Date(s.first_seen).toLocaleDateString()}</small></div>
              {/each}
            </div>
          </section>
        {/if}

        <section class="pl-card">
          <div class="pl-card-head"><h2><History size={17} /> Recent events</h2></div>
          {#if mine.events.length}
            <div class="pl-list">{#each mine.events as e (e.id)}
              <div class="pl-item"><span class="dot"></span><div class="grow"><b style="text-transform: capitalize">{evLabel(e.kind)}</b>{#if e.detail}<span class="sub">{e.detail}</span>{/if}</div><div class="end"><span class="sub">{serverName(e.server_id)}</span><span class="sub" style="display:block">{ago(e.created_at)}</span></div></div>
            {/each}</div>
          {:else}<Empty title="Nothing yet" text="Joins, deaths and milestones show up here." />{/if}
        </section>
      {/if}
    </div>

  {:else}
    <div class="pl-stack">
      {#if lvlError}
        <div class="pl-alert err row" role="alert"><TriangleAlert size={16} /><span class="grow">{lvlError}</span><button class="pl-btn sm" onclick={loadLevels}>Retry</button></div>
      {/if}
      {#if lvlLoading && !lvls.length}
        {#each Array(6) as _, i (i)}<div class="pl-skel" style="height: 64px"></div>{/each}
      {:else if !lvls.length}
        <div class="pl-card"><Empty icon={Sparkles} title="No levels yet" text="Earn XP from quests and playtime to climb the ladder." /></div>
      {:else}
        <section class="pl-card tight list">
          {#each lvls as l, i (l.uuid)}
            <div class="pl-item row-r" class:me={l.uuid === me} style="--i: {Math.min(i, 14)}">
              <span class="rk" class:g={l.rank === 1} class:s={l.rank === 2} class:b={l.rank === 3}>{l.rank}</span>
              <Avatar name={l.username} uuid={l.uuid} size={38} />
              <div class="grow"><b>{l.username}{#if l.uuid === me} <span class="pl-chip accent">You</span>{/if}</b>
                <span class="sub">{l.title ?? `${num(l.global_xp)} XP`}</span>
                <div class="pl-progress thin"><i style="width: {Math.max(2, l.progress_pct)}%"></i></div></div>
              <div class="lvl"><small>LVL</small><b>{l.global_level}</b></div>
            </div>
          {/each}
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  h1 :global(svg) { color: var(--accent-2); }
  .row { display: flex; align-items: center; gap: 10px; } .grow { flex: 1; min-width: 0; }
  .scope { padding: 2px 0 4px; }
  .fchip { display: inline-flex; align-items: center; gap: 7px; padding: 9px 15px; border-radius: 99px; background: var(--pl-glass); border: 1px solid var(--pl-line); color: var(--text-2); font-weight: 600; font-size: 0.86rem; white-space: nowrap; }
  .fchip.sm { padding: 7px 12px; font-size: 0.8rem; }
  .fchip.on { background: var(--accent); border-color: transparent; color: #fff; box-shadow: 0 6px 16px -8px var(--accent); }
  .podium { padding: 18px 14px 0; }
  .ptitle { display: flex; justify-content: space-between; align-items: baseline; gap: 10px; flex-wrap: wrap; padding: 0 6px 6px; }
  .ptitle span { display: inline-flex; gap: 7px; align-items: center; font-weight: 700; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.09em; color: var(--text-2); }
  .ptitle small { color: var(--muted); font-size: 0.78rem; }
  .pods { display: grid; grid-template-columns: 1fr 1.15fr 1fr; align-items: end; gap: 8px; }
  .pod { position: relative; display: flex; flex-direction: column; align-items: center; gap: 4px; min-width: 0; animation: up 0.55s var(--ease, ease) both; animation-delay: var(--d); text-align: center; }
  .pod :global(.crown) { color: #f5d97a; filter: drop-shadow(0 0 8px #f5d97a88); margin-bottom: -4px; }
  .av { position: relative; padding: 3px; border-radius: 28%; background: var(--ring, rgba(255, 255, 255, 0.2)); }
  .p1 { --ring: linear-gradient(135deg, #f5d97a, #c98d1c); } .p2 { --ring: linear-gradient(135deg, #e5eaf2, #8a94a6); } .p3 { --ring: linear-gradient(135deg, #e2a56b, #8a4f1f); }
  .medal { position: absolute; right: -6px; bottom: -6px; width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; font-size: 0.72rem; font-weight: 800; color: #1a1a1a; background: var(--ring); box-shadow: 0 2px 6px #0008; }
  .nm { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.92rem; }
  .val { font-weight: 800; font-size: 1.05rem; font-variant-numeric: tabular-nums; color: var(--accent-2); }
  .step { width: 100%; border-radius: 12px 12px 0 0; background: linear-gradient(180deg, rgba(255, 255, 255, 0.16), rgba(255, 255, 255, 0.03)); margin-top: 6px; }
  .p1 .step { height: 52px; } .p2 .step { height: 34px; } .p3 .step { height: 22px; }
  .pod.me .nm { color: var(--accent-2); }
  @keyframes up { from { opacity: 0; transform: translateY(18px); } }
  .list { padding: 6px 12px; transition: opacity 0.2s; } .list.busy { opacity: 0.55; }
  .row-r { animation: up 0.35s var(--ease, ease) both; animation-delay: calc(var(--i) * 25ms); }
  .rk { width: 28px; text-align: center; font-weight: 800; color: var(--muted); font-variant-numeric: tabular-nums; flex-shrink: 0; }
  .rk.g { color: #f5d97a; } .rk.s { color: #cfd6e0; } .rk.b { color: #d9965b; }
  .pl-item.me { background: color-mix(in srgb, var(--accent) 15%, transparent); border-radius: 12px; padding-inline: 8px; margin-inline: -8px; border-bottom-color: transparent; }
  .grow b :global(.pl-chip) { margin-left: 6px; }
  .thin { height: 4px; margin-top: 6px; }
  .few { text-align: center; color: var(--muted); font-size: 0.85rem; padding: 10px; margin: 0; }
  .lvl { text-align: center; padding: 6px 12px; border-radius: 12px; background: color-mix(in srgb, var(--accent) 20%, transparent); }
  .lvl small { display: block; font-size: 0.6rem; letter-spacing: 0.1em; color: var(--accent-2); font-weight: 700; } .lvl b { font-size: 1.3rem; line-height: 1; }
  .me-hero { display: flex; gap: 18px; align-items: center; }
  .me-hero .who { min-width: 0; } .me-hero h1 { overflow: hidden; text-overflow: ellipsis; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 10px; }
  .tiles { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  @media (min-width: 640px) { .tiles { grid-template-columns: repeat(4, minmax(0, 1fr)); } }
  .tile { display: flex; flex-direction: column; gap: 4px; padding: 14px; animation: up 0.4s var(--ease, ease) both; animation-delay: calc(var(--i) * 40ms); }
  .tile :global(svg) { color: var(--accent-2); }
  .tile span { display: inline-flex; align-items: center; gap: 6px; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.07em; color: var(--muted); font-weight: 600; }
  .tile b { font-size: 1.45rem; font-variant-numeric: tabular-nums; letter-spacing: -0.02em; }
  .srv { padding: 12px 14px; border-radius: 14px; background: rgba(255, 255, 255, 0.04); border: 1px solid var(--pl-line); display: flex; flex-direction: column; gap: 8px; }
  .mini { display: flex; gap: 12px; flex-wrap: wrap; font-size: 0.82rem; color: var(--text-2); } .mini span { display: inline-flex; gap: 4px; align-items: center; }
  .srv small { color: var(--muted); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent-2); flex-shrink: 0; }
  @media (prefers-reduced-motion: reduce) { .pod, .row-r, .tile { animation: none; } }
</style>
