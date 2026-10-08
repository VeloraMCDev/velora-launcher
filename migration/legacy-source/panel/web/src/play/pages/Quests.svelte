<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Target, Trophy, Clock, Check, Gift, Pickaxe, Swords, Hammer, Wheat, Users, Compass, RefreshCw, LoaderCircle, Flame, Star, Lock, Crown, Sparkles, ChevronsUp, Layers, Search
  } from '@lucide/svelte';
  import AchIcon from '../ui/AchIcon.svelte';
  import Empty from '../ui/Empty.svelte';
  import Count from '../ui/Count.svelte';
  import { get, post, timeAgo } from '../../lib/api';
  import { imageIcon, resolveIcon } from '../../lib/achievementIcons';
  import { toast, toastError } from '../../lib/toast.svelte';
  import { compact } from '../store.svelte';

  type Level = {
    level: number; current_xp: number; current_level_xp?: number; next_level_xp: number; progress_pct: number; title: string | null; title_image?: string | null; rank?: number | null; badges?: string[];
    server_levels?: { server_id: number; server_name: string; server_level?: number; level?: number; server_xp?: number; current_level_xp?: number; next_level_xp: number; progress_pct: number; rank_name?: string | null; title_image?: string | null }[];
  };
  type UQ = {
    quest: { id: string; title: string; description: string; period?: string; quest_type?: string; category: string; target_count: number; xp_reward: number; icon: string };
    progress?: number; current_count?: number; completed: boolean; claimed: boolean; expires_at?: string | null;
  };
  type Ach = { id: string; title: string; description: string; category: string; xp_reward: number; frame_type?: string; icon_item?: string; icon_bg?: string; icon_border?: string; secret?: boolean; unlocked?: boolean; unlocked_at?: string | null };
  type Reward = { id: number; level: number; level_type?: string; server_id?: number | null; reward_type: string; reward_name: string; description: string; icon: string };

  let tab = $state<'quests' | 'achievements' | 'levels'>('quests');
  let period = $state<'daily' | 'weekly'>('daily');
  let cat = $state('all');
  let level = $state<Level | null>(null);
  let quests = $state<UQ[]>([]);
  let achs = $state<Ach[]>([]);
  let rewards = $state<Reward[]>([]);
  let extras = $state<Record<string, string[]>>({});
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let claiming = $state<string | null>(null);
  let claimingAll = $state(false);
  let burst = $state<string | null>(null);
  let levelUp = $state(false);
  let now = $state(Date.now());

  const catIcons: Record<string, any> = { mining: Pickaxe, combat: Swords, building: Hammer, farming: Wheat, social: Users, exploration: Compass };
  const questIcons: Record<string, any> = { pickaxe: Pickaxe, swords: Swords, blocks: Hammer, wheat: Wheat, users: Users, compass: Compass, target: Target };
  const qid = (u: UQ) => u.quest.id;
  const qPeriod = (u: UQ) => u.quest.period ?? u.quest.quest_type ?? 'daily';
  const cnt = (u: UQ) => u.current_count ?? u.progress ?? 0;
  const pctOf = (u: UQ) => Math.min(100, Math.round((cnt(u) / Math.max(1, u.quest.target_count)) * 100));

  async function load(quiet = false) {
    if (!quiet) refreshing = true;
    try {
      const [l, q, a, r, x] = await Promise.all([
        get<Level>('/api/v1/levels/me'),
        get<UQ[]>('/api/v1/quests/my'),
        get<Ach[]>('/api/v1/achievements/my').catch(() => [] as Ach[]),
        get<Reward[]>('/api/v1/levels/rewards').catch(() => [] as Reward[]),
        get<Record<string, string[]>>('/api/v1/reward-bundles').catch(() => ({}) as Record<string, string[]>),
      ]);
      level = l; quests = q; achs = a; rewards = r; extras = x ?? {}; error = '';
    } catch (e) {
      if (!quiet || !level) error = e instanceof Error ? e.message : 'Could not load your progress';
    } finally { loading = false; refreshing = false; }
  }

  onMount(() => {
    void load();
    const poll = setInterval(() => { if (!document.hidden) void load(true); }, 30_000);
    const tick = setInterval(() => (now = Date.now()), 30_000);
    return () => { clearInterval(poll); clearInterval(tick); };
  });

  // ---------- quests ----------
  const inPeriod = $derived(quests.filter((u) => qPeriod(u) === period));
  const categories = $derived(['all', ...new Set(inPeriod.map((u) => u.quest.category))]);
  $effect(() => { if (!categories.includes(cat)) cat = 'all'; });
  const rank = (u: UQ) => (u.claimed ? 2 : u.completed ? 0 : 1);
  const shown = $derived(inPeriod.filter((u) => cat === 'all' || u.quest.category === cat).sort((a, b) => rank(a) - rank(b) || pctOf(b) - pctOf(a)));
  const doneCount = $derived(inPeriod.filter((u) => u.completed).length);
  const readyList = $derived(quests.filter((u) => u.completed && !u.claimed));
  const readyIn = (p: 'daily' | 'weekly') => readyList.filter((u) => qPeriod(u) === p).length;
  const pendingXp = $derived(readyList.reduce((n, u) => n + u.quest.xp_reward, 0));

  function resetIn(p: 'daily' | 'weekly'): string {
    const ends = quests.filter((u) => qPeriod(u) === p && u.expires_at).map((u) => new Date(u.expires_at!).getTime()).filter((t) => t > now);
    let t = ends.length ? Math.min(...ends) : 0;
    if (!t) {
      const d = new Date(now);
      t = p === 'daily' ? Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate() + 1) : Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate() + ((8 - d.getUTCDay()) % 7 || 7));
    }
    const m = Math.max(0, Math.floor((t - now) / 60000));
    const days = Math.floor(m / 1440), h = Math.floor((m % 1440) / 60);
    return days ? `${days}d ${h}h` : h ? `${h}h ${m % 60}m` : `${m % 60}m`;
  }

  async function claim(u: UQ, quiet = false): Promise<boolean> {
    const id = qid(u);
    claiming = id;
    const before = level?.level ?? 0;
    try {
      await post(`/api/v1/quests/${id}/claim`);
      quests = quests.map((x) => (qid(x) === id ? { ...x, claimed: true } : x));
      burst = id; setTimeout(() => { if (burst === id) burst = null; }, 1400);
      const l = await get<Level>('/api/v1/levels/me').catch(() => null);
      if (l) { level = l; if (l.level > before && before) { levelUp = true; setTimeout(() => (levelUp = false), 2600); toast(`Level up! You reached level ${l.level}`); } }
      if (!quiet) toast(`+${u.quest.xp_reward} XP claimed`);
      return true;
    } catch (e) { toastError(e); void load(true); return false; } finally { claiming = null; }
  }

  async function claimAll() {
    claimingAll = true;
    let n = 0, xp = 0;
    for (const u of [...readyList]) { if (await claim(u, true)) { n++; xp += u.quest.xp_reward; } else break; }
    claimingAll = false;
    if (n) toast(`Claimed ${n} reward${n === 1 ? '' : 's'} for +${xp.toLocaleString()} XP`);
  }

  // ---------- achievements ----------
  let aFilter = $state<'all' | 'unlocked' | 'locked'>('all');
  let aq = $state('');
  const unlocked = $derived(achs.filter((a) => a.unlocked).length);
  const achShown = $derived(achs.filter((a) => (aFilter === 'all' || (aFilter === 'unlocked') === !!a.unlocked) && (!aq.trim() || a.title.toLowerCase().includes(aq.trim().toLowerCase()) || a.description.toLowerCase().includes(aq.trim().toLowerCase())))
    .sort((x, y) => Number(!!y.unlocked) - Number(!!x.unlocked) || (y.unlocked_at ?? '').localeCompare(x.unlocked_at ?? '')));
  const frameName = (f?: string) => (f === 'challenge' ? 'Challenge' : f === 'goal' ? 'Goal' : 'Task');
  const hidden = (a: Ach) => !!a.secret && !a.unlocked;
  const exOf = (a: Ach) => (hidden(a) ? [] : (extras[`achievement:${a.id}`] ?? []));

  // ---------- levels ----------
  let scope = $state<'global' | number>('global');
  const globalRewards = $derived(rewards.filter((r) => (r.level_type ?? 'global') === 'global'));
  const serverRewards = $derived(rewards.filter((r) => r.level_type === 'server'));
  const scopeRewards = $derived(scope === 'global' ? globalRewards : serverRewards.filter((r) => r.server_id === scope || r.server_id == null));
  const scopeLevel = $derived(scope === 'global' ? (level?.level ?? 1) : (() => { const s = level?.server_levels?.find((x) => x.server_id === scope); return s?.server_level ?? s?.level ?? 1; })());
  const byLevel = $derived.by(() => {
    const m = new Map<number, Reward[]>();
    for (const r of scopeRewards) m.set(r.level, [...(m.get(r.level) ?? []), r]);
    return [...m.entries()].sort((a, b) => a[0] - b[0]).map(([lvl, list]) => ({ lvl, list }));
  });
  const nextMilestone = $derived(byLevel.find((x) => x.lvl > scopeLevel)?.lvl ?? null);
  const rewardIcon: Record<string, any> = { title: Crown, profile_badge: Star, badge: Star, cosmetic: Sparkles, item: Gift };
  const rewardKind: Record<string, string> = { title: 'Title', profile_badge: 'Badge', badge: 'Badge', cosmetic: 'Cosmetic', item: 'Item' };
  const isUrl = (s?: string) => !!s && imageIcon(s) !== null;

  const curXp = $derived(level?.current_level_xp ?? level?.current_xp ?? 0);
  const xpPct = $derived(Math.min(100, Math.max(0, level?.progress_pct ?? 0)));
</script>

<div class="pl-page qst">
  <div class="pl-head">
    <div><h1><Target size={26} /> Quests</h1><p>Complete quests, unlock achievements and climb the levels.</p></div>
    <div class="pl-actions"><button class="pl-iconbtn" aria-label="Refresh" aria-busy={refreshing} onclick={() => load()}><RefreshCw size={16} class={refreshing ? 'spin' : ''} /></button></div>
  </div>

  {#if error && !level}
    <div class="pl-alert err">{error} <button class="pl-btn sm" onclick={() => load()}>Retry</button></div>
  {:else if loading}
    <div class="pl-stack" aria-busy="true">
      <div class="pl-skel" style="height:150px;border-radius:24px"></div>
      <div class="pl-skel" style="height:46px"></div>
      {#each [0, 1, 2] as i}<div class="pl-skel" style="height:118px;border-radius:18px;animation-delay:{i * 90}ms"></div>{/each}
    </div>
  {:else if level}
    <!-- Level hero -->
    <section class="pl-hero lvl" class:up={levelUp}>
      <div class="ring" style="--p:{xpPct}">
        <div class="ring-in"><small>LVL</small><b><Count value={level.level} format={(v) => String(Math.round(v))} /></b></div>
        {#if level.rank}<span class="rk"><Trophy size={11} /> #{level.rank}</span>{/if}
      </div>
      <div class="lv-main">
        <div class="lv-top">
          <div class="lv-name">
            {#if level.title}
              {#if level.title_image}<img class="ttl" src={level.title_image} alt={level.title} />{:else}<h2>{level.title}</h2>{/if}
            {:else}<h2>Adventurer</h2>{/if}
            <span class="sub">SCOPENET account level{#if level.rank} · global rank #{level.rank}{/if}</span>
          </div>
          <div class="xp"><b><Count value={curXp} format={(v) => Math.round(v).toLocaleString()} /></b> / {level.next_level_xp.toLocaleString()} XP</div>
        </div>
        <div class="pl-progress big"><i style="width:{xpPct}%"></i></div>
        <div class="lv-foot">
          <span class="sub">{Math.max(0, level.next_level_xp - curXp).toLocaleString()} XP to level {level.level + 1}</span>
          {#if level.badges?.length}<div class="chips">{#each level.badges as b}<span class="pl-chip warn"><Star size={11} /> {b}</span>{/each}</div>{/if}
        </div>
      </div>
      {#if levelUp}<div class="levelup" aria-hidden="true"><ChevronsUp size={20} /> LEVEL UP</div>{/if}
    </section>

    {#if readyList.length}
      <button class="readybar" onclick={() => { tab = 'quests'; }} aria-label="Rewards ready">
        <Gift size={18} /> <span><b>{readyList.length}</b> reward{readyList.length === 1 ? '' : 's'} ready to claim <span class="sub">· +{pendingXp.toLocaleString()} XP</span></span>
      </button>
    {/if}

    <div class="pl-tabs tabs" role="tablist">
      <button role="tab" class:on={tab === 'quests'} onclick={() => (tab = 'quests')}><Target size={15} /> Quests {#if readyList.length}<span class="count">{readyList.length}</span>{/if}</button>
      <button role="tab" class:on={tab === 'achievements'} onclick={() => (tab = 'achievements')}><Trophy size={15} /> Achievements <span class="count">{unlocked}/{achs.length}</span></button>
      <button role="tab" class:on={tab === 'levels'} onclick={() => (tab = 'levels')}><Layers size={15} /> Levels</button>
    </div>

    {#key tab}
    <div class="pane">
      {#if tab === 'quests'}
        <div class="bar">
          <div class="pl-tabs mini">
            <button class:on={period === 'daily'} onclick={() => (period = 'daily')}><Clock size={14} /> Daily {#if readyIn('daily')}<span class="count">{readyIn('daily')}</span>{/if}</button>
            <button class:on={period === 'weekly'} onclick={() => (period = 'weekly')}><Flame size={14} /> Weekly {#if readyIn('weekly')}<span class="count">{readyIn('weekly')}</span>{/if}</button>
          </div>
          <div class="reset"><Clock size={13} /> Resets in <b>{resetIn(period)}</b><span class="dot">·</span><b>{doneCount}/{inPeriod.length}</b> done</div>
          {#if readyList.length > 1}
            <button class="pl-btn primary sm" disabled={claimingAll} aria-busy={claimingAll} onclick={claimAll}>{#if claimingAll}<LoaderCircle size={14} class="spin" />{:else}<Gift size={14} />{/if} Claim all ({readyList.length})</button>
          {/if}
        </div>
        {#if inPeriod.length > 0}
          <div class="pl-progress overall" title="{doneCount} of {inPeriod.length} completed"><i style="width:{inPeriod.length ? (doneCount / inPeriod.length) * 100 : 0}%"></i></div>
        {/if}
        {#if categories.length > 2}
          <div class="pl-scroll-x cats">
            {#each categories as c}
              {@const CI = catIcons[c]}
              <button class="cat" class:on={cat === c} onclick={() => (cat = c)}>{#if CI}<CI size={14} />{/if} <span>{c}</span></button>
            {/each}
          </div>
        {/if}

        {#if shown.length === 0}
          <div class="pl-card"><Empty icon={Target} title={`No ${period} quests here`} text={cat === 'all' ? 'New quests appear after the next reset.' : 'Try another category.'} /></div>
        {:else}
          <div class="pl-grid" style="--min: 330px">
            {#each shown as u, i (qid(u))}
              {@const q = u.quest}
              {@const QI = questIcons[q.icon] ?? catIcons[q.category] ?? Target}
              {@const pct = pctOf(u)}
              <article class="pl-card quest rise" class:ready={u.completed && !u.claimed} class:claimed={u.claimed} class:burst={burst === qid(u)} style:--i={Math.min(i, 10)}>
                {#if burst === qid(u)}
                  <div class="confetti" aria-hidden="true">{#each Array(14) as _, k}<i style="--a:{k * 25.7}deg;--d:{40 + (k % 4) * 14}px;--c:{k % 3}"></i>{/each}<b class="xpf">+{q.xp_reward} XP</b></div>
                {/if}
                <div class="qtop">
                  <div class="qic" class:done={u.completed}>
                    {#if imageIcon(q.icon)}<img src={imageIcon(q.icon)} alt="" />{:else if q.icon && !questIcons[q.icon] && !catIcons[q.icon]}<span class="emoji">{resolveIcon({ icon_item: q.icon }).emoji}</span>{:else}<QI size={22} />{/if}
                  </div>
                  <div class="grow">
                    <div class="qtitle"><b>{q.title}</b><span class="pl-chip">{q.category}</span></div>
                    <p>{q.description}</p>
                  </div>
                </div>
                <div class="prog">
                  <div class="pl-progress" class:full={pct >= 100}><i style="width:{pct}%"></i></div>
                  <div class="plabels"><span><b>{Math.min(cnt(u), q.target_count).toLocaleString()}</b> / {q.target_count.toLocaleString()}</span><span class="pc">{pct}%</span></div>
                </div>
                <div class="qfoot">
                  <div class="rw"><span class="pl-chip accent"><Sparkles size={11} /> +{q.xp_reward} XP</span>{#each extras[`quest:${q.id}`] ?? [] as ex}<span class="pl-chip warn"><Gift size={11} /> {ex}</span>{/each}</div>
                  {#if u.claimed}
                    <span class="pl-chip good done-chip"><Check size={13} /> Claimed</span>
                  {:else if u.completed}
                    <button class="pl-btn primary claim" disabled={claiming === qid(u) || claimingAll} aria-busy={claiming === qid(u)} onclick={() => claim(u)}>{#if claiming === qid(u)}<LoaderCircle size={15} class="spin" />{:else}<Gift size={15} />{/if} Claim</button>
                  {:else}
                    <span class="sub">In progress</span>
                  {/if}
                </div>
              </article>
            {/each}
          </div>
        {/if}

      {:else if tab === 'achievements'}
        <div class="pl-card tight asum">
          <div class="pl-row-flex"><Trophy size={22} class="gold" /><div class="grow"><b>{unlocked} of {achs.length} unlocked</b><div class="pl-progress" style="margin-top:6px"><i style="width:{achs.length ? (unlocked / achs.length) * 100 : 0}%"></i></div></div></div>
        </div>
        <div class="bar">
          <div class="pl-tabs mini">
            {#each ['all', 'unlocked', 'locked'] as f}<button class:on={aFilter === f} onclick={() => (aFilter = f as typeof aFilter)} style="text-transform:capitalize">{f}</button>{/each}
          </div>
          <div class="pl-search grow"><Search size={16} /><input class="pl-input" type="search" placeholder="Search achievements" bind:value={aq} aria-label="Search achievements" /></div>
        </div>
        {#if achShown.length === 0}
          <div class="pl-card"><Empty icon={Trophy} title="Nothing here" text="No achievements match this filter." /></div>
        {:else}
          <div class="pl-grid" style="--min: 300px">
            {#each achShown as a, i (a.id)}
              <article class="pl-card tight ach rise" class:on={a.unlocked} style:--i={Math.min(i, 14)}>
                <AchIcon ach={hidden(a) ? { frame_type: a.frame_type, icon_item: 'skull', icon_bg: 'deepslate', icon_border: 'netherite' } : a} size={54} locked={!a.unlocked} />
                <div class="grow">
                  <div class="atop"><b>{hidden(a) ? 'Secret achievement' : a.title}</b></div>
                  <p>{hidden(a) ? 'Keep playing to discover this one.' : a.description}</p>
                  <div class="rw">
                    <span class="pl-chip" class:accent={a.frame_type === 'goal'} class:warn={a.frame_type === 'challenge'}>{frameName(a.frame_type)}</span>
                    {#if !hidden(a)}<span class="pl-chip accent">+{a.xp_reward} XP</span>{/if}
                    {#each exOf(a) as ex}<span class="pl-chip warn"><Gift size={11} /> {ex}</span>{/each}
                    {#if a.unlocked}<span class="pl-chip good"><Check size={11} /> {a.unlocked_at ? timeAgo(a.unlocked_at) : 'Unlocked'}</span>{:else}<span class="pl-chip"><Lock size={11} /> Locked</span>{/if}
                  </div>
                </div>
              </article>
            {/each}
          </div>
        {/if}

      {:else}
        {#if level.server_levels?.length}
          <div class="pl-tabs mini scopes">
            <button class:on={scope === 'global'} onclick={() => (scope = 'global')}>Account</button>
            {#each level.server_levels as s (s.server_id)}<button class:on={scope === s.server_id} onclick={() => (scope = s.server_id)}>{s.server_name}</button>{/each}
          </div>
        {/if}
        {#if scope !== 'global'}
          {@const s = level.server_levels?.find((x) => x.server_id === scope)}
          {#if s}
            <div class="pl-card">
              <div class="pl-row-flex"><div class="smallring"><b>{s.server_level ?? s.level ?? 1}</b></div>
                <div class="grow"><b>{s.server_name}</b><span class="sub">{s.rank_name ?? 'Server rank'} · {Math.round(s.progress_pct)}% to next level</span><div class="pl-progress" style="margin-top:8px"><i style="width:{Math.min(100, s.progress_pct)}%"></i></div></div></div>
            </div>
          {/if}
        {:else if nextMilestone}
          <div class="pl-card tight next"><ChevronsUp size={18} /> Next reward at <b>level {nextMilestone}</b> <span class="sub">· {nextMilestone - scopeLevel} level{nextMilestone - scopeLevel === 1 ? '' : 's'} to go</span></div>
        {/if}

        {#if byLevel.length === 0}
          <div class="pl-card"><Empty icon={Layers} title="No level rewards yet" text="Titles, badges and cosmetics unlocked at milestone levels will appear here." /></div>
        {:else}
          <div class="tl">
            {#each byLevel as g, i (g.lvl)}
              {@const got = g.lvl <= scopeLevel}
              <div class="tl-row rise" class:got class:next={g.lvl === nextMilestone} style:--i={Math.min(i, 12)}>
                <div class="node"><span>{g.lvl}</span></div>
                <div class="pl-card tight tl-card">
                  <div class="tl-h"><b>Level {g.lvl}</b>{#if got}<span class="pl-chip good"><Check size={11} /> Unlocked</span>{:else if g.lvl === nextMilestone}<span class="pl-chip accent">Next up</span>{:else}<span class="pl-chip"><Lock size={11} /> Locked</span>{/if}</div>
                  <div class="pl-list">
                    {#each g.list as r (r.id)}
                      {@const RI = rewardIcon[r.reward_type] ?? Gift}
                      <div class="pl-item">
                        <span class="ric">{#if isUrl(r.icon)}<img src={imageIcon(r.icon)} alt="" />{:else if r.icon && [...r.icon].length <= 3}{r.icon}{:else}<RI size={16} />{/if}</span>
                        <div class="grow"><b>{r.reward_name}</b><span class="sub">{rewardKind[r.reward_type] ?? r.reward_type}{#if r.description} · {r.description}{/if}</span></div>
                      </div>
                    {/each}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
    {/key}
  {/if}
</div>

<style>
  .qst > :global(* + *) { margin-top: 14px; }
  .qst > :global(.pl-head) { margin-bottom: 20px; }
  .grow { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .sub { font-size: 0.8rem; color: var(--muted); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .tabs { width: 100%; } .tabs button { flex: 1; justify-content: center; }
  .pane { animation: rise 0.3s var(--ease, ease) both; }
  .pane > :global(* + *) { margin-top: 14px; }
  .rise { animation: rise 0.4s var(--ease, ease) both; animation-delay: calc(var(--i, 0) * 45ms); }
  @keyframes rise { from { opacity: 0; transform: translateY(10px); } }
  .mini { width: fit-content; }
  .mini button { padding: 7px 12px; }

  /* hero */
  .lvl { display: flex; gap: 20px; align-items: center; }
  .lvl.up { animation: glow 1.4s ease 2; }
  @keyframes glow { 50% { box-shadow: 0 0 70px -4px var(--accent-2); } }
  .ring { position: relative; flex-shrink: 0; width: 104px; height: 104px; border-radius: 50%; display: grid; place-items: center; background: conic-gradient(var(--accent-2) calc(var(--p) * 1%), rgba(255, 255, 255, 0.1) 0); transition: background 0.6s; box-shadow: 0 0 36px -8px var(--accent); }
  .ring-in { width: 86px; height: 86px; border-radius: 50%; background: var(--surface); display: flex; flex-direction: column; align-items: center; justify-content: center; line-height: 1; }
  .ring-in small { font-size: 0.62rem; letter-spacing: 0.14em; color: var(--accent-2); font-weight: 700; }
  .ring-in b { font-size: 2rem; font-weight: 800; margin-top: 3px; font-variant-numeric: tabular-nums; }
  .rk { position: absolute; bottom: -8px; left: 50%; translate: -50% 0; display: inline-flex; gap: 3px; align-items: center; padding: 2px 9px; border-radius: 99px; font-size: 0.72rem; font-weight: 700; background: color-mix(in srgb, #f59e0b 28%, var(--surface)); color: #fbbf24; border: 1px solid color-mix(in srgb, #f59e0b 55%, transparent); white-space: nowrap; }
  .lv-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 10px; }
  .lv-top { display: flex; justify-content: space-between; gap: 10px; align-items: flex-end; flex-wrap: wrap; }
  .lv-name { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .lv-name h2 { font-size: 1.35rem; letter-spacing: -0.01em; }
  .ttl { height: 28px; max-width: 200px; object-fit: contain; image-rendering: pixelated; align-self: flex-start; }
  .xp { font-variant-numeric: tabular-nums; color: var(--text-2); font-size: 0.9rem; }
  .xp b { color: var(--text); font-size: 1.1rem; }
  .big { height: 12px; }
  .lv-foot { display: flex; justify-content: space-between; gap: 10px; flex-wrap: wrap; align-items: center; }
  .levelup { position: absolute; top: 12px; right: 14px; display: inline-flex; gap: 6px; align-items: center; padding: 6px 14px; border-radius: 99px; background: var(--accent); color: #fff; font-weight: 800; letter-spacing: 0.1em; font-size: 0.78rem; animation: lu 2.6s ease both; }
  @keyframes lu { 0% { opacity: 0; transform: translateY(10px) scale(0.8); } 12%, 80% { opacity: 1; transform: none; } 100% { opacity: 0; transform: translateY(-8px); } }
  .readybar { width: 100%; display: flex; align-items: center; gap: 12px; padding: 13px 16px; border-radius: 16px; text-align: left; color: var(--text); background: linear-gradient(100deg, color-mix(in srgb, var(--good) 22%, var(--surface)), var(--surface)); border: 1px solid color-mix(in srgb, var(--good) 45%, transparent); animation: pulse 2.4s ease-in-out infinite; }
  .readybar :global(svg) { color: var(--good); }
  @keyframes pulse { 50% { box-shadow: 0 0 0 4px color-mix(in srgb, var(--good) 14%, transparent); } }

  /* quests */
  .bar { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .bar .pl-search { min-width: 160px; }
  .reset { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 0.84rem; flex: 1; min-width: 0; flex-wrap: wrap; }
  .reset b { color: var(--text); font-variant-numeric: tabular-nums; }
  .dot { opacity: 0.5; }
  .overall { height: 5px; }
  .cats { gap: 8px; }
  .cat { display: inline-flex; gap: 6px; align-items: center; padding: 7px 14px; border-radius: 99px; background: var(--surface-2); border: 1px solid var(--pl-line); color: var(--text-2); font-size: 0.85rem; min-height: 0 !important; }
  .cat span { text-transform: capitalize; }
  .cat.on { background: var(--accent); border-color: transparent; color: #fff; }
  .quest { position: relative; display: flex; flex-direction: column; gap: 14px; transition: border-color 0.3s, box-shadow 0.3s, transform 0.3s; }
  .quest.ready { border-color: color-mix(in srgb, var(--good) 55%, transparent); box-shadow: 0 0 0 1px color-mix(in srgb, var(--good) 22%, transparent), 0 14px 38px -22px var(--good); }
  .quest.claimed { opacity: 0.62; }
  .quest.burst { animation: bump 0.5s var(--ease, ease); }
  @keyframes bump { 35% { transform: scale(1.025); } }
  .qtop { display: flex; gap: 12px; }
  .qic { width: 46px; height: 46px; border-radius: 14px; flex-shrink: 0; display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 20%, transparent); color: var(--accent-2); }
  .qic.done { background: color-mix(in srgb, var(--good) 22%, transparent); color: var(--good); }
  .qic img { width: 30px; height: 30px; object-fit: contain; }
  .emoji { font-size: 1.4rem; }
  .qtitle { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .qtitle .pl-chip { text-transform: capitalize; font-size: 0.68rem; }
  .qtop p, .ach p { color: var(--muted); font-size: 0.86rem; margin-top: 3px; line-height: 1.4; }
  .plabels { display: flex; justify-content: space-between; margin-top: 6px; font-size: 0.82rem; color: var(--muted); font-variant-numeric: tabular-nums; }
  .plabels b { color: var(--text); }
  .pl-progress.full > i { background: linear-gradient(90deg, var(--good), color-mix(in srgb, var(--good) 60%, var(--accent-2))); }
  .qfoot { display: flex; justify-content: space-between; gap: 10px; align-items: center; flex-wrap: wrap; }
  .rw { display: flex; gap: 6px; flex-wrap: wrap; }
  .claim { min-width: 120px; animation: pop 1.8s ease-in-out infinite; }
  @keyframes pop { 50% { transform: scale(1.04); } }
  .done-chip { padding: 6px 14px; font-size: 0.82rem; animation: stamp 0.5s var(--ease, ease); }
  @keyframes stamp { from { transform: scale(1.7) rotate(-6deg); opacity: 0; } }
  .confetti { position: absolute; inset: 0; pointer-events: none; z-index: 3; display: grid; place-items: center; overflow: visible; }
  .confetti i { position: absolute; left: 50%; bottom: 22px; width: 8px; height: 8px; border-radius: 2px; background: var(--accent-2); animation: fly 1.1s cubic-bezier(0.1, 0.8, 0.3, 1) forwards; }
  .confetti i[style*='--c:1'] { background: var(--good); } .confetti i[style*='--c:2'] { background: var(--warn); }
  @keyframes fly { from { transform: translate(-50%, 0) rotate(0); opacity: 1; } to { transform: translate(calc(-50% + cos(var(--a)) * var(--d) * 2.2), calc(sin(var(--a)) * var(--d) * -2)) rotate(300deg); opacity: 0; } }
  .xpf { position: absolute; right: 18px; bottom: 30px; color: var(--good); font-size: 1.1rem; animation: xpf 1.3s ease forwards; text-shadow: 0 2px 10px #000; }
  @keyframes xpf { from { opacity: 0; transform: translateY(8px); } 20% { opacity: 1; } to { opacity: 0; transform: translateY(-40px); } }

  /* achievements */
  :global(.gold) { color: #fbbf24; }
  .ach { display: flex; gap: 14px; align-items: flex-start; opacity: 0.72; transition: opacity 0.2s, transform 0.2s; }
  .ach.on { opacity: 1; border-color: color-mix(in srgb, #fbbf24 28%, transparent); }
  .ach:hover { transform: translateY(-2px); opacity: 1; }
  .ach .rw { margin-top: 10px; }

  /* levels */
  .scopes { max-width: 100%; }
  .smallring { width: 54px; height: 54px; border-radius: 50%; display: grid; place-items: center; border: 3px solid var(--accent); background: color-mix(in srgb, var(--accent) 20%, transparent); font-size: 1.3rem; flex-shrink: 0; }
  .next { display: flex; gap: 8px; align-items: center; color: var(--text-2); }
  .next :global(svg) { color: var(--accent-2); }
  .tl { position: relative; display: flex; flex-direction: column; gap: 12px; padding-left: 4px; }
  .tl::before { content: ''; position: absolute; left: 25px; top: 10px; bottom: 10px; width: 2px; background: linear-gradient(var(--accent), var(--pl-line)); opacity: 0.55; }
  .tl-row { display: flex; gap: 14px; align-items: flex-start; position: relative; }
  .node { width: 44px; height: 44px; flex-shrink: 0; border-radius: 50%; display: grid; place-items: center; font-weight: 800; background: var(--surface-2); border: 2px solid var(--pl-line); z-index: 1; font-variant-numeric: tabular-nums; }
  .tl-row.got .node { background: var(--accent); border-color: transparent; color: #fff; box-shadow: 0 0 18px -4px var(--accent); }
  .tl-row.next .node { border-color: var(--accent-2); box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 20%, transparent); }
  .tl-card { flex: 1; min-width: 0; }
  .tl-row:not(.got) .tl-card { opacity: 0.85; }
  .tl-h { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
  .ric { width: 34px; height: 34px; border-radius: 11px; display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 18%, transparent); color: var(--accent-2); flex-shrink: 0; font-size: 1.1rem; }
  .ric img { width: 24px; height: 24px; object-fit: contain; }

  @media (max-width: 560px) {
    .lvl { flex-direction: column; text-align: center; padding: 20px 16px; gap: 22px; }
    .lv-top { flex-direction: column; align-items: center; text-align: center; } .lv-name { align-items: center; } .ttl { align-self: center; }
    .lv-foot { justify-content: center; }
    .bar .pl-btn { width: 100%; }
    .tabs button { padding: 8px 6px; font-size: 0.8rem; gap: 4px; }
  }
  @media (prefers-reduced-motion: reduce) { .rise, .pane, .readybar, .claim, .lvl.up { animation: none; } }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
