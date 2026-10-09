<script lang="ts">
  import { playPath } from '../../lib/router.svelte';
  import { enabled, pageEnabled } from '@velora/experience';
  import ExperienceWidgets from '@velora/experience/ExperienceWidgets.svelte';
  import { route } from '../../lib/router.svelte';
  import { nativeApp } from '../../lib/native';
  import { onMount } from 'svelte';
  import { Banknote, Bell, Clock, Crown, Dices, Download, Flame, Gavel, Gift, Map as MapIcon, Newspaper, Shield, Sparkles, Store, Target, Trophy, Users, Check, ChevronRight, Server as ServerIcon, UserPlus } from '@lucide/svelte';
  import type { Component } from 'svelte';
  import { get, post } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { toast, toastError } from '../../lib/toast.svelte';
  import Avatar from '../../components/Avatar.svelte';
  import { balanceChanged, currentServer, money, play } from '../store.svelte';
  import Count from '../ui/Count.svelte';
  import Empty from '../ui/Empty.svelte';
  import MoneyItem from '../lib/MoneyItem.svelte';
  import { ago, greeting, num, parseTime, stripColors, timeLeft } from '../lib/money-pages';

  type Lv = { global_level: number; global_xp: number; current_level_xp: number; next_level_xp: number; progress_pct: number; title: string | null };
  type Q = { quest: { id: string; title: string; description: string; period: string; target_count: number; xp_reward: number; icon: string }; progress: number; current_count: number; completed: boolean; claimed: boolean };
  type Friend = { uuid: string; username: string; status: 'accepted' | 'pending_incoming' | 'pending_outgoing'; online: boolean; playing_on: string | null; unread: number };
  type Listing = { id: number; item_id: string; item_name: string; amount: number; price: number; kind: 'buy_now' | 'auction'; ends_at: string | null; current_bid: number | null; mine: boolean; leading: boolean };

  let level = $state<Lv | null>(null);
  let quests = $state<Q[] | null>(null);
  let friends = $state<Friend[] | null>(null);
  let spins = $state<number | null>(null);
  let auctions = $state<Listing[]>([]);
  let claiming = $state<string | null>(null);
  let now = $state(Date.now());

  const name = $derived(session.user?.username ?? 'player');
  const instance = $derived(play.manifest?.instances.find(i => i.id === route.instanceId));
  const branding = $derived(instance?.experience?.branding ?? play.manifest?.branding);
  const news = $derived([...(branding?.news ?? [])].sort((a, b) => Number(!!b.pinned) - Number(!!a.pinned)).slice(0, 4));
  const server = $derived(currentServer());

  async function loadPersonal() {
    const [l, q, f] = await Promise.allSettled([enabled(instance?.experience, 'progression') ? get<Lv>('/api/v1/levels/me') : Promise.resolve(null), enabled(instance?.experience, 'quests') ? get<Q[]>('/api/v1/quests/my') : Promise.resolve([]), get<Friend[]>('/api/v1/friends')]);
    if (l.status === 'fulfilled') level = l.value;
    quests = q.status === 'fulfilled' ? q.value : (quests ?? []);
    friends = f.status === 'fulfilled' ? f.value : (friends ?? []);
  }
  async function loadServerBits() {
    const sid = play.serverId;
    spins = null; auctions = [];
    if (sid == null) return;
    const [c, m] = await Promise.allSettled([enabled(instance?.experience, 'casino') ? get<{ enabled: boolean; free: { left: number } }>(`/api/v1/casino/${sid}`) : Promise.resolve(null), enabled(instance?.experience, 'economy') ? get<{ listings: Listing[] }>(`/api/v1/market/${sid}`) : Promise.resolve(null)]);
    if (sid !== play.serverId) return;
    if (c.status === 'fulfilled' && c.value?.enabled) spins = c.value.free?.left ?? 0;
    if (m.status === 'fulfilled' && m.value) auctions = m.value.listings;
  }

  onMount(() => {
    void loadPersonal();
    const poll = setInterval(() => { if (!document.hidden) { void loadPersonal(); void loadServerBits(); } }, 30000);
    const tick = setInterval(() => (now = Date.now()), 1000);
    return () => { clearInterval(poll); clearInterval(tick); };
  });
  $effect(() => { void play.serverId; void loadServerBits(); });

  async function claim(q: Q) {
    if (claiming) return;
    claiming = q.quest.id;
    try {
      await post(`/api/v1/quests/${q.quest.id}/claim`);
      q.claimed = true;
      toast(`+${q.quest.xp_reward} XP claimed`, 'ok');
      balanceChanged();
      void loadPersonal();
    } catch (e) { toastError(e); } finally { claiming = null; }
  }

  const prog = (q: Q) => Math.min(q.quest.target_count, q.progress || q.current_count || 0);
  const pct = (q: Q) => (q.quest.target_count ? Math.min(100, (prog(q) / q.quest.target_count) * 100) : 0);
  const daily = $derived((quests ?? []).filter((q) => q.quest.period === 'daily'));
  const shownQ = $derived((daily.length ? daily : (quests ?? [])).slice(0, 4));
  const readyToClaim = $derived((quests ?? []).filter((q) => q.completed && !q.claimed).length);
  const friendsOn = $derived((friends ?? []).filter((f) => f.status === 'accepted' && f.online));
  const friendsAll = $derived((friends ?? []).filter((f) => f.status === 'accepted'));
  const incoming = $derived((friends ?? []).filter((f) => f.status === 'pending_incoming'));
  const unread = $derived((friends ?? []).reduce((s, f) => s + (f.unread || 0), 0));
  const watching = $derived(auctions.filter((a) => a.kind === 'auction' && (a.leading || a.mine) && a.ends_at && parseTime(a.ends_at) > now).sort((a, b) => parseTime(a.ends_at!) - parseTime(b.ends_at!)).slice(0, 3));

  const allTiles: { id: string; label: string; icon: Component<any>; hue: string }[] = [
    { id: 'market', label: 'Market', icon: Store, hue: '--accent' }, { id: 'casino', label: 'Casino', icon: Dices, hue: '--warn' }, { id: 'quests', label: 'Quests', icon: Target, hue: '--good' },
    { id: 'social', label: 'Friends', icon: Users, hue: '--accent-2' }, { id: 'wallet', label: 'Wallet', icon: Banknote, hue: '--good' }, { id: 'guilds', label: 'Guilds', icon: Shield, hue: '--accent' },
    { id: 'stats', label: 'Ranks', icon: Trophy, hue: '--warn' }, { id: 'map', label: 'Live map', icon: MapIcon, hue: '--accent-2' }, { id: 'launcher', label: 'Launcher', icon: Download, hue: '--accent' },
  ];
  const tiles = $derived(allTiles.filter(t => (!nativeApp || t.id !== 'launcher') && pageEnabled(instance?.experience, t.id)));
</script>

<div class="pl-page home">
  <section class="pl-hero homehero">
    <div class="top">
      <div>
        <p class="greet">{greeting()},</p>
        <h1>{instance?.name ?? name}</h1><p>{name}</p>
      </div>
      <Avatar name={name} uuid={session.user?.uuid ?? null} size={56} />
    </div>
    {#if enabled(instance?.experience, 'economy')}<div class="bal">
      <small>{server ? `Balance on ${server.name}` : 'Balance'}</small>
      <div class="big">{#if play.serverId == null}—{:else if play.balance == null}<span class="nb">No balance yet</span>{:else}<Count value={play.balance} format={(v) => money(v)} />{/if}</div>
    </div>
    {/if}
    {#if enabled(instance?.experience, 'progression')}<div class="xp">
      {#if level}
        <div class="xrow"><span class="lv"><Sparkles size={13} /> Level {level.global_level}{level.title ? ` · ${level.title}` : ''}</span><span>{num(level.global_xp)} XP</span></div>
        <div class="pl-progress"><i style="width: {Math.max(3, level.progress_pct)}%"></i></div>
        <small>{Math.round(level.progress_pct)}% to level {level.global_level + 1}</small>
      {:else}<div class="pl-skel" style="height: 38px"></div>{/if}
    </div>
    {/if}
  </section>
  <ExperienceWidgets experience={instance?.experience} />

  <div class="tilesbar pl-scroll-x" aria-label="Quick actions">
    {#each tiles as t, i (t.id)}{@const I = t.icon}
      <a class="qt" href={playPath(t.id)} style="--h: var({t.hue}); --i: {i}">
        <span class="qi"><I size={22} /></span><span>{t.label}</span>
        {#if t.id === 'quests' && readyToClaim}<em>{readyToClaim}</em>{/if}
        {#if t.id === 'social' && (incoming.length + unread)}<em>{incoming.length + unread}</em>{/if}
        {#if t.id === 'casino' && spins}<em>{spins}</em>{/if}
      </a>
    {/each}
  </div>

  {#if spins}
    <a class="freespin pl-card" href={playPath('casino')}>
      <span class="gift"><Gift size={26} /></span>
      <div class="grow"><b>{spins} free spin{spins === 1 ? '' : 's'} today</b><span>Daily casino spin on {server?.name} — tap to play</span></div>
      <ChevronRight size={20} />
    </a>
  {/if}

  <div class="pl-grid cols" style="--min: 330px">
    <section class="pl-card">
      <div class="pl-card-head"><h2><ServerIcon size={17} /> Servers</h2></div>
      {#if !play.loaded}
        <div class="pl-skel" style="height: 62px"></div>
      {:else if play.servers.length}
        <div class="pl-list">
          {#each play.servers as s (s.id)}
            <div class="pl-item" class:cur={s.id === play.serverId}>
              <span class="pl-dot" class:on={s.online}></span>
              <div class="grow"><b>{s.name}</b><span class="sub">{s.online ? 'Online' : 'Offline'}{s.mc_version ? ` · ${s.mc_version}` : ''}{s.software ? ` · ${s.software}` : ''}</span></div>
              <div class="end"><b>{s.players_online}<small class="mx">/{s.players_max}</small></b>{#if s.online && s.tps != null}<span class="sub" style="display:block" class:warnt={s.tps < 18}>{s.tps.toFixed(1)} TPS</span>{/if}</div>
            </div>
          {/each}
        </div>
      {:else}<Empty icon={ServerIcon} title="No servers yet" text="Game servers linked to the panel will be listed here." />{/if}
    </section>

    {#if enabled(instance?.experience, 'quests')}
    <section class="pl-card">
      <div class="pl-card-head"><h2><Target size={17} /> Today's quests</h2><a class="more" href={playPath('quests')}>All quests</a></div>
      {#if quests == null}
        {#each Array(3) as _, i (i)}<div class="pl-skel" style="height: 50px; margin-bottom: 8px"></div>{/each}
      {:else if shownQ.length}
        <div class="qs">
          {#each shownQ as q (q.quest.id)}
            <div class="q" class:done={q.claimed}>
              <div class="qtop"><b>{q.quest.title}</b><span class="pl-chip accent">+{q.quest.xp_reward} XP</span></div>
              <div class="pl-progress thin"><i style="width: {pct(q)}%"></i></div>
              <div class="qbot"><small>{prog(q)} / {q.quest.target_count}</small>
                {#if q.claimed}<span class="pl-chip good"><Check size={12} /> Claimed</span>
                {:else if q.completed}<button class="pl-btn sm primary" aria-busy={claiming === q.quest.id} disabled={!!claiming} onclick={() => claim(q)}><Gift size={13} /> Claim</button>{/if}</div>
            </div>
          {/each}
        </div>
      {:else}<Empty icon={Target} title="No quests right now" text="New quests are assigned every day." />{/if}
    </section>

    {/if}

    <section class="pl-card">
      <div class="pl-card-head"><h2><Users size={17} /> Friends</h2><a class="more" href={playPath('social')}>Open</a></div>
      {#if friends == null}
        <div class="pl-skel" style="height: 56px"></div>
      {:else}
        <div class="fsum">
          <div class="pl-stat"><span>Online</span><b class="pos">{friendsOn.length}</b></div>
          <div class="pl-stat"><span>Friends</span><b>{friendsAll.length}</b></div>
          <div class="pl-stat"><span>Requests</span><b class:pos={incoming.length > 0}>{incoming.length}</b></div>
        </div>
        {#if friendsOn.length}
          <div class="pl-list" style="margin-top: 8px">
            {#each friendsOn.slice(0, 4) as f (f.uuid)}
              <a class="pl-item press" href={playPath('social')}><Avatar name={f.username} uuid={f.uuid} size={34} /><div class="grow"><b>{f.username}</b><span class="sub">{f.playing_on ? `playing on ${f.playing_on}` : 'online'}</span></div>{#if f.unread}<span class="pl-chip accent">{f.unread} new</span>{/if}</a>
            {/each}
          </div>
        {:else if !friendsAll.length}
          <Empty icon={UserPlus} title="No friends yet" text="Add a friend by username in Friends."><a class="pl-btn sm primary" href={playPath('social')}>Add friends</a></Empty>
        {:else}<p class="quiet">None of your friends are online right now.</p>{/if}
      {/if}
    </section>

    {#if watching.length}
      <section class="pl-card">
        <div class="pl-card-head"><h2><Flame size={17} /> Auctions to watch</h2><a class="more" href={playPath('market')}>Market</a></div>
        <div class="pl-list">
          {#each watching as a (a.id)}
            <a class="pl-item press" href={playPath('market')}>
              <MoneyItem id={a.item_id} amount={a.amount} size={42} />
              <div class="grow"><b>{stripColors(a.item_name)}</b><span class="sub">{a.leading ? 'You lead' : 'Your auction'} · {money(a.current_bid ?? a.price)}</span></div>
              <div class="end"><span class="pl-chip" class:warn={parseTime(a.ends_at ?? '') - now < 600e3}><Clock size={11} /> {timeLeft(a.ends_at, now)}</span></div>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    <section class="pl-card news">
      <div class="pl-card-head"><h2><Newspaper size={17} /> News</h2></div>
      {#if !play.manifest}
        <div class="pl-skel" style="height: 80px"></div>
      {:else if news.length}
        <div class="pl-list">
          {#each news as n (n.id)}
            <svelte:element this={n.link ? 'a' : 'div'} class="pl-item nw" class:press={!!n.link} href={n.link ?? undefined} target={n.link ? '_blank' : undefined} rel="noreferrer">
              {#if n.image_url}<img src={n.image_url} alt="" loading="lazy" />{/if}
              <div class="grow"><b class="wrap">{n.pinned ? '📌 ' : ''}{n.title}</b><span class="sub wrap2">{n.body}</span><span class="sub">{#if n.tag}<span class="pl-chip accent">{n.tag}</span> {/if}{n.date ? ago(n.date) : ''}</span></div>
            </svelte:element>
          {/each}
        </div>
      {:else}<Empty icon={Bell} title="No news yet" text="Announcements from the staff appear here." />{/if}
    </section>
  </div>
</div>

<style>
  .homehero { padding: 22px 20px; display: flex; flex-direction: column; gap: 14px; margin-bottom: 14px; }
  .top { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .greet { margin: 0 0 2px; font-size: 0.9rem; }
  .top h1 { font-size: 1.7rem; overflow-wrap: anywhere; }
  .bal small { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.09em; font-weight: 700; color: var(--text-2); }
  .big { font-size: clamp(2.3rem, 11vw, 3.4rem); font-weight: 800; letter-spacing: -0.03em; line-height: 1.1; background: linear-gradient(180deg, #fff, color-mix(in srgb, var(--accent-2) 65%, #fff)); -webkit-background-clip: text; background-clip: text; color: transparent; }
  .nb { font-size: 1.4rem; -webkit-text-fill-color: var(--muted); }
  .xp { display: flex; flex-direction: column; gap: 7px; padding: 12px 14px; border-radius: 16px; background: rgba(0, 0, 0, 0.22); border: 1px solid var(--pl-line); }
  .xrow { display: flex; justify-content: space-between; font-size: 0.86rem; font-variant-numeric: tabular-nums; } .xrow .lv { display: inline-flex; align-items: center; gap: 6px; font-weight: 700; }
  .xp small { color: var(--muted); font-size: 0.76rem; }
  .tilesbar { margin: 0 -14px 14px; padding: 2px 14px 6px; }
  @media (min-width: 881px) { .tilesbar { margin: 0 0 14px; padding: 2px 0 6px; flex-wrap: wrap; overflow: visible; } }
  .qt { position: relative; display: flex; flex-direction: column; align-items: center; gap: 8px; width: 82px; padding: 12px 6px 10px; border-radius: 18px; color: var(--text); font-size: 0.78rem; font-weight: 650; background: var(--pl-glass); border: 1px solid var(--pl-line); transition: transform 0.18s, border-color 0.18s; animation: pop 0.4s var(--ease, ease) both; animation-delay: calc(var(--i) * 30ms); -webkit-tap-highlight-color: transparent; }
  .qt:hover, .qt:active { transform: translateY(-3px); border-color: color-mix(in srgb, var(--h) 60%, transparent); }
  .qi { width: 44px; height: 44px; border-radius: 14px; display: grid; place-items: center; color: var(--h); background: color-mix(in srgb, var(--h) 18%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--h) 30%, transparent); }
  .qt em { position: absolute; top: 6px; right: 8px; min-width: 18px; height: 18px; padding: 0 5px; border-radius: 99px; background: var(--accent); color: #fff; font-style: normal; font-size: 0.68rem; display: grid; place-items: center; box-shadow: 0 0 10px var(--accent); }
  @keyframes pop { from { opacity: 0; transform: scale(0.9) translateY(6px); } }
  a.freespin.pl-card { display: flex; align-items: center; gap: 14px; margin-bottom: 14px; background: linear-gradient(120deg, color-mix(in srgb, var(--warn) 22%, var(--surface)), var(--pl-glass)); border-color: color-mix(in srgb, var(--warn) 40%, transparent); }
  .gift { width: 48px; height: 48px; border-radius: 15px; display: grid; place-items: center; color: var(--warn); background: color-mix(in srgb, var(--warn) 20%, transparent); animation: wob 2.4s ease-in-out infinite; }
  @keyframes wob { 0%, 100% { transform: rotate(0); } 10% { transform: rotate(-8deg); } 20% { transform: rotate(8deg); } 30% { transform: rotate(0); } }
  .grow { flex: 1; min-width: 0; } .freespin .grow b, .freespin .grow span { display: block; } .freespin .grow span { color: var(--text-2); font-size: 0.85rem; }
  .cols > * { animation: pop 0.45s var(--ease, ease) both; }
  .cols > :nth-child(2) { animation-delay: 60ms; } .cols > :nth-child(3) { animation-delay: 120ms; } .cols > :nth-child(4) { animation-delay: 180ms; }
  .cur { background: color-mix(in srgb, var(--accent) 10%, transparent); border-radius: 12px; padding-inline: 8px !important; margin-inline: -8px; }
  .mx { color: var(--muted); font-weight: 500; } .warnt { color: var(--warn) !important; }
  .qs { display: flex; flex-direction: column; gap: 12px; }
  .q { display: flex; flex-direction: column; gap: 7px; padding: 12px; border-radius: 14px; background: rgba(255, 255, 255, 0.04); border: 1px solid var(--pl-line); } .q.done { opacity: 0.6; }
  .qtop, .qbot { display: flex; justify-content: space-between; align-items: center; gap: 8px; } .qtop b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.92rem; } .qbot small { color: var(--muted); font-variant-numeric: tabular-nums; }
  .thin { height: 6px; }
  .fsum { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
  .quiet { color: var(--muted); font-size: 0.86rem; margin: 10px 0 0; }
  .nw { align-items: flex-start; color: inherit; } .nw img { width: 76px; height: 56px; border-radius: 10px; object-fit: cover; flex-shrink: 0; }
  .nw .wrap { white-space: normal !important; } .nw .wrap2 { white-space: normal !important; display: -webkit-box !important; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; margin: 2px 0; }
  .news { grid-column: 1 / -1; }
  @media (prefers-reduced-motion: reduce) { .qt, .cols > *, .gift { animation: none; } }
</style>
