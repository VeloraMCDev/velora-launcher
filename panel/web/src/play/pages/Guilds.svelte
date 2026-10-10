<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import {
    Shield, Map as MapIcon, Users, MessageSquare, Plus, Crown, UserCheck, Trash2, LoaderCircle, RefreshCw, Landmark, Send, Search, Pencil, TriangleAlert,
    UserMinus, UserPlus, ArrowDownToLine, ArrowUpFromLine, LogOut, Settings2, Inbox, Check, X, Compass, Wallet, Sparkles, ShieldCheck, Mail, Lock, History, Ellipsis
  } from '@lucide/svelte';
  import Avatar from '../../components/Avatar.svelte';
  import Sheet from '../ui/Sheet.svelte';
  import Empty from '../ui/Empty.svelte';
  import Count from '../ui/Count.svelte';
  import ProfileSheet from '../ui/ProfileSheet.svelte';
  import LandRules from '../ui/LandRules.svelte';
  import FactionMarket from '@velora/board/FactionMarket.svelte';
  import { del, get, post, put, timeAgo } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { toast, toastError } from '../../lib/toast.svelte';
  import { balanceChanged, compact, currentServer, play } from '../store.svelte';

  type Guild = { id: string; instance_id: string; name: string; tag: string; description: string; motd: string; leader_uuid: string; icon_url: string | null; banner_url: string | null; level: number; xp: number; max_claims: number; member_count: number; claims_count: number; created_at: string };
  type Member = { uuid: string; name: string; role: string; joined_at: string; online?: boolean };
  type GPost = { id: number; author_uuid: string; author_name: string; title: string; content: string; created_at: string };
  type Claim = { id: number; dimension: string; chunk_x: number; chunk_z: number; claimed_by_uuid: string; claimed_at: string };
  type Detail = Guild & { members: Member[]; posts: GPost[]; claims: Claim[] };
  type Role = { id: number; name: string; can_invite: boolean; can_kick: boolean; can_claim: boolean; can_post: boolean; can_manage: boolean; can_vault: boolean };
  type JoinReq = { uuid: string; name: string; message: string; created_at: string };
  type GInvite = { id: number; guild_id: string; guild_name: string; guild_tag: string; icon_url: string; inviter: string; inviter_uuid?: string };
  type Tx = { id: number; actor_uuid: string; kind: string; amount: number; created_at: string; note?: string };
  type Wallet = { balance: number; my_balance?: number; role?: string; currency_symbol?: string; transactions: Tx[]; upkeep?: {daily_cents:number;arrears_cents:number;grace_days:number;freezes_at:string|null;claims_frozen:boolean}|null };
  type Relation = {id:number;guild_id:string;other_guild_id:string;name:string;tag:string;relation:string;status:string;reward_cents:number;reward_bps:number;expires_at:string|null;terms_revision:string};
  let relations=$state<Relation[]>([]), relationTarget=$state(''), relationKind=$state('alliance'), rewardKind=$state('percent'), reward=$state(5), relationHours=$state(0);
  async function proposeRelation(){if(!g||!relationTarget)return;await act('relation',()=>post(`/api/v1/guilds/${g!.id}/relations`,{other_guild_id:relationTarget,relation:relationKind,reward_cents:relationKind==='rival'&&rewardKind==='fixed'?Math.round(reward*100):0,reward_bps:relationKind==='rival'&&rewardKind==='percent'?Math.round(reward*100):0,duration_hours:relationKind==='rival'&&relationHours>0?relationHours:null}),'Request sent');}

  const me = $derived(session.user?.uuid ?? '');
  const instanceId = $derived(currentServer()?.instance_id ?? '');
  const inst = $derived(instanceId ? `?instance_id=${encodeURIComponent(instanceId)}` : '');

  let tab = $state<'overview' | 'members' | 'bank' | 'land' | 'posts' | 'discover' | 'relations'>('overview');
  let g = $state<Detail | null>(null);
  let all = $state<Guild[]>([]);
  let roles = $state<Role[]>([]);
  let requests = $state<JoinReq[]>([]);
  let invites = $state<GInvite[]>([]);
  let loading = $state(true);
  let refreshing = $state(false);
  let error = $state('');
  let busy = $state<string | null>(null);

  const myRole = $derived(g?.members.find((m) => m.uuid === me)?.role ?? '');
  const isLeader = $derived(myRole === 'leader');
  const isOfficer = $derived(myRole === 'officer');
  const myPerms = $derived(roles.find((r) => r.name === myRole));
  const can = (a: 'invite' | 'kick' | 'claim' | 'post' | 'manage') =>
    isLeader || isOfficer || (myRole === 'member' && (a === 'claim' || a === 'post')) || (!!myPerms && myPerms[`can_${a}` as const]);
  const canManage = $derived(isLeader || isOfficer || !!myPerms?.can_manage);
  const online = $derived(g?.members.filter((m) => m.online).length ?? 0);

  // Same colour the live map gives a guild (hash of its id), so a guild looks the same everywhere.
  const hue = (id: string) => { let h = 0; for (let i = 0; i < id.length; i++) h = (Math.imul(31, h) + id.charCodeAt(i)) | 0; return ((h % 360) + 360) % 360; };
  const roleLabel = (r: string) => (r === 'leader' ? 'Leader' : r === 'officer' ? 'Officer' : r === 'member' ? 'Member' : r);

  let seq = 0;
  async function load(quiet = false) {
    const mine = ++seq;
    if (!quiet) { refreshing = true; if (!g && !all.length) loading = true; }
    try {
      const [my, list, inv] = await Promise.all([
        get<Detail | null>(`/api/v1/guilds/my${inst}`),
        get<Guild[]>(`/api/v1/guilds${inst}`),
        get<GInvite[]>('/api/v1/guilds/invites').catch(() => [] as GInvite[]),
      ]);
      if (mine !== seq) return;
      g = my; all = list; invites = inv; error = '';
      if (my) {
        const [r, jr, rel] = await Promise.all([
          get<Role[]>(`/api/v1/guilds/${my.id}/roles`).catch(() => [] as Role[]),
          get<JoinReq[]>(`/api/v1/guilds/${my.id}/requests`).catch(() => [] as JoinReq[]),
          get<Relation[]>(`/api/v1/guilds/${my.id}/relations`),
        ]);
        if (mine !== seq) return;
        roles = r; requests = jr; relations = rel;
      } else { roles = []; requests = []; relations = []; if (tab !== 'discover') tab = 'overview'; }
    } catch (e) {
      if (mine === seq && (!quiet || !g)) error = e instanceof Error ? e.message : 'Could not load guilds';
    } finally {
      if (mine === seq) { loading = false; refreshing = false; }
    }
  }

  $effect(() => { void play.serverId; void instanceId; untrack(() => { wallet = null; void load(); }); });
  onMount(() => { const t = setInterval(() => { if (!document.hidden) void load(true); }, 30_000); return () => clearInterval(t); });

  async function act<T>(key: string, fn: () => Promise<T>, ok?: string | ((r: T) => string), reload = true): Promise<T | undefined> {
    busy = key;
    try {
      const r = await fn();
      if (ok) toast(typeof ok === 'function' ? ok(r) : ok);
      if (reload) await load(true);
      return r;
    } catch (e) { toastError(e); return undefined; } finally { busy = null; }
  }

  // ---------- profile sheet ----------
  let profileOpen = $state(false), profileUuid = $state<string | null>(null);
  const openProfile = (u: string) => { profileUuid = u; profileOpen = true; };

  // ---------- invitations received ----------
  const answerInvite = (inv: GInvite, accept: boolean) =>
    act(`inv${inv.id}`, () => post(`/api/v1/guilds/invites/${inv.id}/${accept ? 'accept' : 'decline'}`), accept ? `Welcome to ${inv.guild_name}!` : 'Invitation declined');

  // ---------- members ----------
  let memberSheet = $state<Member | null>(null), memberOpen = $state(false);
  let confirm = $state<{ kind: 'kick' | 'leader' | 'leave' | 'release' | 'disband'; member?: Member; claim?: Claim } | null>(null), confirmOpen = $state(false);
  let typed = $state('');
  const canKick = (m: Member) => m.uuid !== me && m.role !== 'leader' && can('kick') && (isLeader || m.role !== 'officer');
  function ask(kind: NonNullable<typeof confirm>['kind'], extra: { member?: Member; claim?: Claim } = {}) { confirm = { kind, ...extra }; typed = ''; memberOpen = false; confirmOpen = true; }

  async function runConfirm() {
    if (!g || !confirm) return;
    const c = confirm, id = g.id;
    let ok = false;
    if (c.kind === 'kick' && c.member) ok = !!(await act('kick', () => del(`/api/v1/guilds/${id}/members/${c.member!.uuid}`), `${c.member.name} was removed`));
    else if (c.kind === 'leader' && c.member) ok = !!(await act('leader', () => put(`/api/v1/guilds/${id}/leader`, { uuid: c.member!.uuid }), `${c.member.name} now leads the guild`));
    else if (c.kind === 'leave') ok = !!(await act('leave', () => del(`/api/v1/guilds/${id}/members/${me}`), 'You left the guild'));
    else if (c.kind === 'release' && c.claim) ok = !!(await act('release', () => del(`/api/v1/guilds/claims/${c.claim!.id}`), 'Chunk released'));
    else if (c.kind === 'disband') {
      const r = await act('disband', () => del<{ refunded?: number }>(`/api/v1/guilds/${id}`), (x) => (x?.refunded ? `Guild disbanded. ${x.refunded.toLocaleString()} from the treasury was paid to you.` : 'Guild disbanded'));
      ok = !!r || r === null; if (r) balanceChanged();
    }
    if (ok) confirmOpen = false;
  }

  async function setRole(m: Member, role: string) {
    if (!g || m.role === role) return;
    const prev = m.role;
    m.role = role; // optimistic
    try { await put(`/api/v1/guilds/${g.id}/members/${m.uuid}/role`, { role }); toast(`${m.name} is now ${roleLabel(role)}`); await load(true); }
    catch (e) { m.role = prev; toastError(e); }
  }

  // ---------- join requests ----------
  const answerJoin = (r: JoinReq, accept: boolean) =>
    g && act(`jr${r.uuid}`, () => post(`/api/v1/guilds/${g!.id}/requests/${r.uuid}/respond`, { accept }), accept ? `${r.name} joined the guild` : 'Request declined');

  // ---------- invite a player ----------
  let inviteOpen = $state(false), iq = $state(''), found = $state<{ uuid: string; username: string; online: boolean; global_level: number }[]>([]), searching = $state(false), invited = $state(new Set<string>());
  let iTimer: ReturnType<typeof setTimeout> | undefined, iSeq = 0;
  async function isearch() {
    const mine = ++iSeq; searching = true;
    try { const r = await get<typeof found>(`/api/v1/members/search?q=${encodeURIComponent(iq.trim())}`); if (mine === iSeq) found = r.filter((x) => !g?.members.some((m) => m.uuid === x.uuid)); }
    catch (e) { if (mine === iSeq) toastError(e); } finally { if (mine === iSeq) searching = false; }
  }
  const onIq = () => { clearTimeout(iTimer); iTimer = setTimeout(isearch, 260); };
  async function invite(u: { uuid: string; username: string }) {
    if (!g) return;
    const r = await act(`i${u.uuid}`, () => post(`/api/v1/guilds/${g!.id}/invites`, { uuid: u.uuid }), `Invited ${u.username}`, false);
    if (r !== undefined) invited = new Set([...invited, u.uuid]);
  }
  $effect(() => { if (inviteOpen) untrack(() => { iq = ''; void isearch(); }); });

  // ---------- roles ----------
  let rolesOpen = $state(false), rName = $state(''), rFlags = $state({ can_invite: false, can_kick: false, can_claim: false, can_post: true, can_manage: false, can_vault: false });
  async function createRole() {
    if (!g || rName.trim().length < 2) return;
    const r = await act('role', () => post(`/api/v1/guilds/${g!.id}/roles`, { name: rName.trim(), ...rFlags }), 'Role created');
    if (r !== undefined) rName = '';
  }
  const deleteRole = (r: Role) => g && act(`rd${r.id}`, () => del(`/api/v1/guilds/${g!.id}/roles/${r.id}`), 'Role deleted');
  const permList = (r: Role) => [r.can_invite && 'Invite', r.can_kick && 'Kick', r.can_claim && 'Claim', r.can_post && 'Post', r.can_manage && 'Manage', r.can_vault && 'Faction vault'].filter(Boolean) as string[];

  // ---------- edit / rename ----------
  let editOpen = $state(false), eDesc = $state(''), eMotd = $state(''), eIcon = $state(''), eBanner = $state(''), eName = $state(''), eTag = $state('');
  function openEdit() { if (!g) return; eDesc = g.description ?? ''; eMotd = g.motd ?? ''; eIcon = g.icon_url ?? ''; eBanner = g.banner_url ?? ''; eName = g.name; eTag = g.tag; editOpen = true; }
  async function saveEdit() {
    if (!g) return;
    const ok = await act('edit', async () => {
      await put(`/api/v1/guilds/${g!.id}`, { description: eDesc, motd: eMotd, icon_url: eIcon, banner_url: eBanner });
      if (isLeader && (eName.trim() !== g!.name || eTag.trim().toUpperCase() !== g!.tag.toUpperCase())) await put(`/api/v1/guilds/${g!.id}/name`, { name: eName.trim(), tag: eTag.trim() });
      return true;
    }, 'Guild saved');
    if (ok) editOpen = false;
  }

  // ---------- posts ----------
  let pTitle = $state(''), pBody = $state('');
  async function publish() {
    if (!g || !pTitle.trim() || !pBody.trim()) return;
    const ok = await act('post', () => post<GPost>(`/api/v1/guilds/${g!.id}/posts`, { title: pTitle.trim(), content: pBody.trim() }), 'Announcement posted');
    if (ok) { pTitle = ''; pBody = ''; }
  }

  // ---------- bank ----------
  let wallet = $state<Wallet | null>(null), wLoading = $state(false), wError = $state('');
  let mode = $state<'deposit' | 'withdraw'>('deposit'), amount = $state('');
  const walletOk = $derived(!!g && !!currentServer() && currentServer()!.instance_id === g.instance_id);
  const sym = $derived(wallet?.currency_symbol ?? '$');
  const fmt = (n: number) => `${sym}${n.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  const canWithdraw = $derived(isLeader || isOfficer);
  const val = $derived(Number(amount));
  const valid = $derived(Number.isFinite(val) && val >= 0.01 && Math.abs(val * 100 - Math.round(val * 100)) < 1e-6);
  const limit = $derived(mode === 'deposit' ? wallet?.my_balance : wallet?.balance);
  const over = $derived(valid && limit != null && val > limit + 1e-9);
  const isOut = (k: string) => k === 'withdraw' || k === 'purchase' || k === 'transfer_out' || k === 'upkeep';
  const verbs: Record<string, string> = { deposit: 'deposited', withdraw: 'withdrew', sale: 'sold items', purchase: 'bought something', transfer_in: 'received a payment', transfer_out: 'paid a guild' };
  const totals = $derived({ in: wallet?.transactions.filter((t) => !isOut(t.kind)).reduce((n, t) => n + t.amount, 0) ?? 0, out: wallet?.transactions.filter((t) => isOut(t.kind)).reduce((n, t) => n + t.amount, 0) ?? 0 });
  const who = (uuid: string) => g?.members.find((m) => m.uuid === uuid);

  let wSeq = 0;
  async function loadWallet(quiet = false) {
    if (!g || !walletOk || play.serverId == null) { wallet = null; return; }
    const mine = ++wSeq;
    if (!quiet) wLoading = true;
    try {
      const w = await get<Wallet>(`/api/v1/guilds/${g.id}/wallet?server_id=${play.serverId}`);
      if (mine === wSeq) { wallet = w; wError = ''; if (!canWithdraw) mode = 'deposit'; }
    } catch (e) { if (mine === wSeq) wError = e instanceof Error ? e.message : 'Could not load the treasury'; } finally { if (mine === wSeq) wLoading = false; }
  }
  $effect(() => { if (tab === 'bank' && g?.id) { void play.serverId; untrack(() => void loadWallet()); } });

  async function transfer() {
    if (!g || !valid || over || play.serverId == null) return;
    const w = mode, n = val;
    const r = await act('xfer', () => post<{ balance?: number }>(`/api/v1/guilds/${g!.id}/wallet/${w}`, { server_id: play.serverId, amount: n }), w === 'withdraw' ? `Withdrew ${fmt(n)} from the treasury` : `Deposited ${fmt(n)} into the treasury`, false);
    if (r !== undefined) { amount = ''; balanceChanged(); await loadWallet(true); }
  }

  // ---------- land ----------
  const dims = $derived([...new Set(g?.claims.map((c) => c.dimension) ?? [])]);
  let dim = $state('');
  $effect(() => { if (!dims.includes(dim)) dim = dims.includes('minecraft:overworld') ? 'minecraft:overworld' : (dims[0] ?? ''); });
  const dimClaims = $derived(g?.claims.filter((c) => c.dimension === dim) ?? []);
  const dimName = (d: string) => d.replace('minecraft:', '').replace(/_/g, ' ');
  const bounds = $derived.by(() => {
    if (!dimClaims.length) return null;
    const xs = dimClaims.map((c) => c.chunk_x), zs = dimClaims.map((c) => c.chunk_z);
    const x0 = Math.min(...xs) - 1, x1 = Math.max(...xs) + 1, z0 = Math.min(...zs) - 1, z1 = Math.max(...zs) + 1;
    return { x0, z0, w: x1 - x0 + 1, h: z1 - z0 + 1 };
  });
  const claimPct = $derived(g && g.max_claims > 0 ? Math.min(100, Math.round((g.claims_count / g.max_claims) * 100)) : 0);
  const canRelease = $derived(isLeader || isOfficer || !!myPerms?.can_claim);

  // ---------- discover ----------
  let gq = $state('');
  const directory = $derived(all.filter((x) => { const q = gq.trim().toLowerCase(); return !q || x.name.toLowerCase().includes(q) || x.tag.toLowerCase().includes(q) || (x.description ?? '').toLowerCase().includes(q); }));
  let joinTarget = $state<Guild | null>(null), joinOpen = $state(false), joinMsg = $state(''), asked = $state(new Set<string>());
  function askJoin(x: Guild) { joinTarget = x; joinMsg = ''; joinOpen = true; }
  async function sendJoin() {
    if (!joinTarget) return;
    const t = joinTarget;
    const r = await act('join', () => post(`/api/v1/guilds/${t.id}/requests`, { message: joinMsg.trim() }), `Request sent to ${t.name}`, false);
    if (r !== undefined) { asked = new Set([...asked, t.id]); joinOpen = false; }
  }

  // ---------- create ----------
  let createOpen = $state(false), cName = $state(''), cTag = $state(''), cDesc = $state(''), cIcon = $state(''), cBanner = $state('');
  async function createGuild() {
    const instance = instanceId || play.manifest?.instances[0]?.id;
    if (!instance) { toast('Pick a server first', 'error'); return; }
    const r = await act('create', () => post<Guild>('/api/v1/guilds', { instance_id: instance, name: cName.trim(), tag: cTag.trim().toUpperCase(), description: cDesc.trim(), icon_url: cIcon.trim() || null, banner_url: cBanner.trim() || null }), (x) => `[${x.tag}] ${x.name} founded!`);
    if (r) { createOpen = false; cName = cTag = cDesc = cIcon = cBanner = ''; tab = 'overview'; }
  }
</script>

{#snippet emblem(x: { id: string; tag: string; icon_url: string | null }, size: number)}
  <span class="emblem" style="--s:{size}px;--h:{hue(x.id)}">
    {#if x.icon_url}<img src={x.icon_url} alt="" onerror={(e) => ((e.currentTarget as HTMLElement).style.display = 'none')} />{/if}
    <span class="etag">{x.tag.slice(0, 4)}</span>
  </span>
{/snippet}

{#snippet banner(x: { id: string; banner_url: string | null }, h: number)}
  <div class="banner" style="--h:{hue(x.id)};height:{h}px">
    {#if x.banner_url}<img src={x.banner_url} alt="" onerror={(e) => ((e.currentTarget as HTMLElement).style.display = 'none')} />{/if}
    <div class="shade"></div>
  </div>
{/snippet}

{#snippet guildCard(x: Guild, i: number)}
  <article class="pl-card flush dir rise" class:mine={x.id === g?.id} style:--i={Math.min(i, 10)}>
    {@render banner(x, 74)}
    <div class="dir-body">
      <div class="dir-top">{@render emblem(x, 54)}<div class="grow"><b>{x.name}</b><span class="sub">[{x.tag}] · Level {x.level}</span></div></div>
      {#if x.description}<p class="desc">{x.description}</p>{/if}
      <div class="chips"><span class="pl-chip"><Users size={12} /> {x.member_count}</span><span class="pl-chip"><MapIcon size={12} /> {x.claims_count}{#if x.max_claims}/{x.max_claims}{/if}</span></div>
      {#if x.id === g?.id}
        <button class="pl-btn block" onclick={() => (tab = 'overview')}>Open your guild</button>
      {:else if asked.has(x.id)}
        <button class="pl-btn block" disabled><Check size={15} /> Request sent</button>
      {:else}
        <button class="pl-btn primary block" disabled={!!g} title={g ? 'Leave your guild first to join another' : ''} onclick={() => askJoin(x)}><UserPlus size={15} /> {g ? 'Already in a guild' : 'Request to join'}</button>
      {/if}
    </div>
  </article>
{/snippet}

<div class="pl-page gld">
  <div class="pl-head">
    <div>
      <h1><Shield size={26} /> Guilds</h1>
      <p>{currentServer() ? `Your faction on ${currentServer()!.name}` : 'Factions, land and treasuries'}</p>
    </div>
    <div class="pl-actions">
      <button class="pl-iconbtn" aria-label="Refresh" aria-busy={refreshing} onclick={() => load()}><RefreshCw size={16} class={refreshing ? 'spin' : ''} /></button>
    </div>
  </div>

  {#if invites.length}
    <section class="pl-card invs">
      <div class="pl-card-head"><h2><Mail size={17} /> Guild invitations <span class="pl-chip accent">{invites.length}</span></h2></div>
      <div class="pl-list">
        {#each invites as inv (inv.id)}
          <div class="pl-item">
            {@render emblem({ id: inv.guild_id, tag: inv.guild_tag, icon_url: inv.icon_url || null }, 42)}
            <div class="grow"><b>[{inv.guild_tag}] {inv.guild_name}</b><span class="sub">Invited by {inv.inviter}</span></div>
            <button class="pl-btn primary sm" disabled={!!g || busy === `inv${inv.id}`} title={g ? 'Leave your guild first' : ''} onclick={() => answerInvite(inv, true)}>Accept</button>
            <button class="pl-iconbtn" aria-label="Decline" disabled={busy === `inv${inv.id}`} onclick={() => answerInvite(inv, false)}><X size={16} /></button>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if error && !g && !all.length}
    <div class="pl-alert err">{error} <button class="pl-btn sm" onclick={() => load()}>Retry</button></div>
  {:else if loading}
    <div class="pl-stack" aria-busy="true">
      <div class="pl-skel" style="height:190px;border-radius:24px"></div>
      <div class="pl-skel" style="height:46px"></div>
      <div class="pl-skel" style="height:120px;border-radius:18px"></div>
    </div>

  {:else if !g}
    <!-- Not in a guild -->
    <section class="pl-hero nog">
      <div class="nog-ic"><Shield size={34} /></div>
      <div>
        <h1>No guild yet</h1>
        <p>Found a guild to claim land, share a treasury and play as a faction. Or browse the guilds below and ask to join.</p>
        <button class="pl-btn primary lg" onclick={() => (createOpen = true)}><Plus size={17} /> Create a guild</button>
      </div>
    </section>
    <div class="dirhead">
      <h2>Guilds {#if all.length}<span class="pl-chip">{all.length}</span>{/if}</h2>
      {#if all.length > 3}<div class="pl-search"><Search size={16} /><input class="pl-input" type="search" placeholder="Search guilds" bind:value={gq} aria-label="Search guilds" /></div>{/if}
    </div>
    {#if all.length === 0}
      <div class="pl-card"><Empty icon={Compass} title="No guilds here yet" text="Be the first to found one on this server." /></div>
    {:else if directory.length === 0}
      <div class="pl-card"><Empty icon={Search} title="No match" text={`Nothing matches “${gq}”.`} /></div>
    {:else}
      <div class="pl-grid" style="--min: 280px">{#each directory as x, i (x.id)}{@render guildCard(x, i)}{/each}</div>
    {/if}

  {:else}
    <section class="pl-card flush hero">
      {@render banner(g, 128)}
      <div class="hero-body">
        <div class="hero-emblem">{@render emblem(g, 84)}</div>
        <div class="grow">
          <div class="title"><h2>{g.name}</h2><span class="tagchip">[{g.tag}]</span></div>
          <div class="chips">
            <span class="pl-chip accent"><Sparkles size={12} /> Level {g.level}</span>
            <span class="pl-chip"><Users size={12} /> {g.member_count} members</span>
            <span class="pl-chip"><MapIcon size={12} /> {g.claims_count}{#if g.max_claims}/{g.max_claims}{/if} chunks</span>
            {#if myRole}<span class="pl-chip" class:warn={isLeader} class:accent={isOfficer}>{#if isLeader}<Crown size={12} />{:else if isOfficer}<UserCheck size={12} />{/if} {roleLabel(myRole)}</span>{/if}
          </div>
        </div>
      </div>
      {#if g.motd}<div class="motd"><MessageSquare size={14} /> <span>{g.motd}</span></div>{/if}
    </section>

    <div class="pl-tabs tabs" role="tablist">
      <button role="tab" class:on={tab === 'overview'} onclick={() => (tab = 'overview')}>Overview</button>
      <button role="tab" class:on={tab === 'members'} onclick={() => (tab = 'members')}>Members {#if requests.length}<span class="count">{requests.length}</span>{/if}</button>
      <button role="tab" class:on={tab === 'bank'} onclick={() => (tab = 'bank')}>Bank</button>
      <button role="tab" class:on={tab === 'land'} onclick={() => (tab = 'land')}>Land</button>
      <button role="tab" class:on={tab === 'posts'} onclick={() => (tab = 'posts')}>Posts</button>
      <button role="tab" class:on={tab === 'relations'} onclick={() => (tab = 'relations')}>Allies & rivals</button>
      <button role="tab" class:on={tab === 'discover'} onclick={() => (tab = 'discover')}>Discover</button>
    </div>

    {#key tab}
    <div class="pane">
    {#if tab === 'overview'}
      <div class="pl-grid" style="--min: 300px">
        <div class="pl-stack">
          <div class="pl-card">
            <div class="pl-stats">
              <div class="pl-stat"><span>Members</span><b><Count value={g.member_count} /></b><small>{online} online</small></div>
              <div class="pl-stat"><span>Land</span><b><Count value={g.claims_count} /></b><small>{g.max_claims ? `of ${g.max_claims} chunks` : 'chunks'}</small></div>
              <div class="pl-stat"><span>Founded</span><b class="sm">{new Date(g.created_at).toLocaleDateString()}</b><small>{timeAgo(g.created_at)}</small></div>
            </div>
            {#if g.max_claims}<div class="pl-progress" style="margin-top:14px" title="{claimPct}% of the claim limit"><i style="width:{claimPct}%"></i></div>{/if}
          </div>
          {#if g.description}<div class="pl-card"><div class="pl-card-head"><h2>About</h2></div><p class="body">{g.description}</p></div>{/if}
          {#if g.posts[0]}
            <div class="pl-card">
              <div class="pl-card-head"><h2><MessageSquare size={16} /> Latest announcement</h2><button class="more" onclick={() => (tab = 'posts')}>All posts</button></div>
              <b>{g.posts[0].title}</b>
              <p class="body clamp">{g.posts[0].content}</p>
              <span class="sub">{g.posts[0].author_name} · {timeAgo(g.posts[0].created_at)}</span>
            </div>
          {/if}
        </div>
        <div class="pl-stack">
          {#if requests.length && can('invite')}
            <div class="pl-card alert-card">
              <div class="pl-card-head"><h2><Inbox size={16} /> {requests.length} join request{requests.length === 1 ? '' : 's'}</h2></div>
              <button class="pl-btn block" onclick={() => (tab = 'members')}>Review requests</button>
            </div>
          {/if}
          <div class="pl-card">
            <div class="pl-card-head"><h2><Settings2 size={16} /> Manage</h2></div>
            <div class="pl-stack tight">
              {#if can('invite')}<button class="pl-btn block" onclick={() => (inviteOpen = true)}><UserPlus size={16} /> Invite a player</button>{/if}
              {#if canManage}<button class="pl-btn block" onclick={openEdit}><Pencil size={16} /> Edit guild details</button>{/if}
              {#if isLeader}<button class="pl-btn block" onclick={() => (rolesOpen = true)}><ShieldCheck size={16} /> Custom roles <span class="pl-chip">{roles.length}</span></button>{/if}
              {#if !isLeader}
                <button class="pl-btn block danger" onclick={() => ask('leave')}><LogOut size={16} /> Leave guild</button>
              {:else}
                <button class="pl-btn block danger" onclick={() => ask('disband')}><Trash2 size={16} /> Disband guild</button>
              {/if}
            </div>
            {#if !can('invite') && !canManage && isLeader === false}<p class="sub" style="margin-top:8px">Officers and the leader manage the guild.</p>{/if}
          </div>
        </div>
      </div>

    {:else if tab === 'members'}
      <div class="pl-stack">
        {#if requests.length && can('invite')}
          <section class="pl-card alert-card">
            <div class="pl-card-head"><h2><Inbox size={16} /> Join requests <span class="pl-chip accent">{requests.length}</span></h2></div>
            <div class="pl-list">
              {#each requests as r (r.uuid)}
                <div class="pl-item">
                  <button class="who" onclick={() => openProfile(r.uuid)}><Avatar name={r.name} uuid={r.uuid} size={40} /><div class="grow"><b>{r.name}</b><span class="sub">{r.message || 'No message'} · {timeAgo(r.created_at)}</span></div></button>
                  <button class="pl-btn primary sm" disabled={busy === `jr${r.uuid}`} aria-busy={busy === `jr${r.uuid}`} onclick={() => answerJoin(r, true)}><Check size={14} /> Accept</button>
                  <button class="pl-iconbtn" aria-label="Decline" disabled={busy === `jr${r.uuid}`} onclick={() => answerJoin(r, false)}><X size={16} /></button>
                </div>
              {/each}
            </div>
          </section>
        {/if}
        <div class="rowhead">
          <h2>Roster <span class="pl-chip">{g.members.length}</span></h2>
          {#if can('invite')}<button class="pl-btn primary sm" onclick={() => (inviteOpen = true)}><UserPlus size={15} /> Invite</button>{/if}
        </div>
        <div class="pl-grid" style="--min: 300px">
          {#each g.members as m, i (m.uuid)}
            <div class="pl-card tight member rise" style:--i={Math.min(i, 12)}>
              <button class="who" onclick={() => openProfile(m.uuid)}>
                <span class="av"><Avatar name={m.name} uuid={m.uuid} size={46} /><i class="pl-dot" class:on={m.online}></i></span>
                <div class="grow">
                  <b>{m.name}{#if m.uuid === me} <span class="you">you</span>{/if}</b>
                  <span class="sub">Joined {new Date(m.joined_at).toLocaleDateString()}</span>
                </div>
              </button>
              <span class="pl-chip" class:warn={m.role === 'leader'} class:accent={m.role === 'officer'}>{#if m.role === 'leader'}<Crown size={11} />{:else if m.role === 'officer'}<UserCheck size={11} />{/if} {roleLabel(m.role)}</span>
              {#if m.uuid !== me && (isLeader || canKick(m))}
                <button class="pl-iconbtn" aria-label="Manage {m.name}" onclick={() => { memberSheet = m; memberOpen = true; }}><Ellipsis size={17} /></button>
              {/if}
            </div>
          {/each}
        </div>
      </div>

    {:else if tab === 'relations'}
      <section class="pl-card pl-stack">
        <h2>Allies & rivals</h2>
        <p>Both factions must accept a rivalry. Kill rewards transfer the agreed amount from the defeated player's balance, with the server's repeated-kill cooldown.</p>
        {#if isLeader}<form class="pl-stack" onsubmit={(e)=>{e.preventDefault();void proposeRelation();}}>
          <label>Faction<select bind:value={relationTarget}><option value="">Choose a faction</option>{#each all.filter(f=>f.id!==g?.id) as faction}<option value={faction.id}>{faction.name} [{faction.tag}]</option>{/each}</select></label>
          <label>Relationship<select bind:value={relationKind}><option value="alliance">Alliance</option><option value="rival">Rivalry</option></select></label>
          {#if relationKind==='rival'}<label>Kill reward<select bind:value={rewardKind}><option value="percent">Percentage of victim's balance</option><option value="fixed">Fixed dollars</option></select></label>
            <label>Reward<input type="number" min="0" max={rewardKind==='percent'?100:1000000000} step="0.01" bind:value={reward}/></label>
            <label>Duration in hours (0 for permanent)<input type="number" min="0" max="8760" step="1" bind:value={relationHours}/></label>{/if}
          <button class="pl-btn primary" disabled={!!busy||!relationTarget}>Send request</button>
        </form>{/if}
        {#each relations as relation}<article class="pl-card pl-stack tight"><b>{relation.name} [{relation.tag}] · {relation.relation}</b><span>{relation.status}</span>
          {#if relation.relation==='rival'}<p>Kill reward: {relation.reward_bps>0?`${relation.reward_bps/100}%`:fmt(relation.reward_cents/100)}. {relation.expires_at?`Ends ${new Date(relation.expires_at).toLocaleString()}`:'Permanent'}</p>{/if}
          {#if isLeader&&relation.status==='pending'&&relation.other_guild_id===g?.id}<div class="pl-actions"><button class="pl-btn primary" disabled={!!busy} onclick={()=>act('relation',()=>post(`/api/v1/guilds/${g!.id}/relations/${relation.id}/respond`,{accept:true,expected_revision:relation.terms_revision}),'Request accepted')}>Accept terms</button><button class="pl-btn" disabled={!!busy} onclick={()=>act('relation',()=>post(`/api/v1/guilds/${g!.id}/relations/${relation.id}/respond`,{accept:false,expected_revision:relation.terms_revision}),'Request declined')}>Decline</button></div>{/if}
        </article>{/each}
      </section>
    {:else if tab === 'bank'}
      {#if play.manifest?.instances.find(instance => instance.id === instanceId)?.experience?.kind === 'velora-smp'}
        <FactionMarket guildId={g.id} {canManage} get={(id) => get(`/api/v1/guilds/${id}/upgrades`)} buy={(id, track, tiers, price) => post(`/api/v1/guilds/${id}/upgrades`, { track, expected_tiers: tiers, expected_price_cents: price })} onchanged={() => { void loadWallet(); }} />
      {/if}
      {#if !walletOk}
        <div class="pl-card"><Empty icon={Landmark} title="Switch server to see the treasury" text={`The guild treasury lives on the ${g.name} server's economy. Pick a server from the same modpack in the top bar.`} /></div>
      {:else if wLoading && !wallet}
        <div class="pl-skel" style="height:200px;border-radius:24px"></div>
      {:else if wError && !wallet}
        <div class="pl-alert err">{wError} <button class="pl-btn sm" onclick={() => loadWallet()}>Retry</button></div>
      {:else if wallet}
        <div class="pl-grid" style="--min: 320px">
          <div class="pl-stack">
            <section class="pl-hero treasury">
              <span class="tl"><Landmark size={15} /> Guild treasury · [{g.tag}]</span>
              <div class="big"><Count value={wallet.balance} format={fmt} /></div>
              {#if wallet.upkeep}<p>Daily upkeep {fmt(wallet.upkeep.daily_cents/100)} · {wallet.upkeep.grace_days}-day grace period.</p>
                {#if wallet.upkeep.arrears_cents>0}<p class="neg">Unpaid: {fmt(wallet.upkeep.arrears_cents/100)}. {wallet.upkeep.claims_frozen?'New claims are frozen.':`New claims freeze on ${wallet.upkeep.freezes_at} UTC.`} Deposit funds to settle outstanding bills within a minute. Existing claims remain protected.</p>{/if}
              {/if}
              <div class="flow"><span class="pos"><ArrowDownToLine size={13} /> {fmt(totals.in)} in</span><span class="neg"><ArrowUpFromLine size={13} /> {fmt(totals.out)} out</span><small>last {wallet.transactions.length} transactions</small></div>
            </section>
            <div class="pl-card">
              <div class="seg" role="tablist">
                <button class:on={mode === 'deposit'} onclick={() => (mode = 'deposit')}><ArrowDownToLine size={15} /> Deposit</button>
                <button class:on={mode === 'withdraw'} disabled={!canWithdraw} onclick={() => (mode = 'withdraw')} title={canWithdraw ? '' : 'Only leaders and officers can withdraw'}>{#if !canWithdraw}<Lock size={13} />{:else}<ArrowUpFromLine size={15} />{/if} Withdraw</button>
              </div>
              <div class="avail"><Wallet size={14} /> {mode === 'deposit' ? `You have ${fmt(wallet.my_balance ?? 0)}` : `Treasury has ${fmt(wallet.balance)}`}</div>
              <form onsubmit={(e) => { e.preventDefault(); void transfer(); }} class="pl-stack tight">
                <input class="pl-input amt" inputmode="decimal" placeholder="Amount" bind:value={amount} aria-label="Amount" autocomplete="off" />
                <div class="quick">
                  {#each [10, 50, 100, 500, 1000] as q}<button type="button" class="pl-chip" onclick={() => (amount = String(q))}>{compact(q)}</button>{/each}
                  <button type="button" class="pl-chip accent" onclick={() => limit != null && (amount = String(Math.floor(limit * 100) / 100))}>Max</button>
                </div>
                {#if over}<div class="pl-alert err">That is more than you can {mode === 'deposit' ? 'afford' : 'take out'}.</div>{/if}
                {#if valid && !over}<div class="preview">Treasury after: <b>{fmt(wallet.balance + (mode === 'deposit' ? val : -val))}</b></div>{/if}
                <button class="pl-btn primary lg block" disabled={!valid || over || busy === 'xfer'} aria-busy={busy === 'xfer'}>{#if busy === 'xfer'}<LoaderCircle size={16} class="spin" />{/if} {mode === 'deposit' ? 'Deposit' : 'Withdraw'} {valid ? fmt(val) : ''}</button>
              </form>
            </div>
          </div>
          <div class="pl-card">
            <div class="pl-card-head"><h2><History size={16} /> Activity</h2></div>
            {#if wallet.transactions.length === 0}
              <Empty icon={History} title="No transactions yet" text="Deposits and withdrawals will show up here." />
            {:else}
              <div class="pl-list">
                {#each wallet.transactions as t (t.id)}
                  <div class="pl-item">
                    <span class="txi" class:out={isOut(t.kind)}>{#if isOut(t.kind)}<ArrowUpFromLine size={14} />{:else}<ArrowDownToLine size={14} />{/if}</span>
                    <div class="grow"><b>{who(t.actor_uuid)?.name ?? 'Someone'}</b><span class="sub">{verbs[t.kind] ?? t.kind} · {timeAgo(t.created_at)}</span></div>
                    <b class="end" class:pos={!isOut(t.kind)} class:neg={isOut(t.kind)}>{isOut(t.kind) ? '−' : '+'}{fmt(t.amount)}</b>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/if}

    {:else if tab === 'land'}
      <div class="pl-grid" style="--min: 320px">
        <div class="pl-stack">
          <div class="pl-card">
            <div class="pl-stats">
              <div class="pl-stat"><span>Claimed</span><b><Count value={g.claims_count} /></b><small>chunks</small></div>
              <div class="pl-stat"><span>Limit</span><b>{g.max_claims || '–'}</b><small>grows with members and level</small></div>
              <div class="pl-stat"><span>Area</span><b>{compact(g.claims_count * 256)}</b><small>blocks²</small></div>
            </div>
            {#if g.max_claims}<div class="pl-progress" style="margin-top:14px"><i style="width:{claimPct}%"></i></div>{/if}
            <p class="sub" style="margin-top:10px">Claim and release chunks in game with the guild commands. Chunks you own are protected from griefing.</p>
          </div>
          <LandRules guildId={g.id} />
          {#if bounds}
            <div class="pl-card">
              <div class="pl-card-head"><h2><MapIcon size={16} /> Territory</h2>
                {#if dims.length > 1}<div class="pl-tabs mini">{#each dims as d}<button class:on={dim === d} onclick={() => (dim = d)}>{dimName(d)}</button>{/each}</div>{/if}
              </div>
              <svg class="tmap" viewBox="0 0 {bounds.w} {bounds.h}" preserveAspectRatio="xMidYMid meet" style="--h:{hue(g.id)};aspect-ratio:{Math.min(2.2, Math.max(0.6, bounds.w / bounds.h))}" role="img" aria-label="Claimed chunks">
                <rect width={bounds.w} height={bounds.h} class="bg" />
                {#each dimClaims as c (c.id)}<rect x={c.chunk_x - bounds.x0} y={c.chunk_z - bounds.z0} width="0.94" height="0.94" rx="0.15" class="chunk" />{/each}
              </svg>
              <p class="sub">{dimClaims.length} chunk{dimClaims.length === 1 ? '' : 's'} in {dimName(dim)} · x {bounds.x0 * 16} to {(bounds.x0 + bounds.w) * 16}, z {bounds.z0 * 16} to {(bounds.z0 + bounds.h) * 16}</p>
            </div>
          {/if}
        </div>
        <div class="pl-card">
          <div class="pl-card-head"><h2>Claimed chunks</h2></div>
          {#if g.claims.length === 0}
            <Empty icon={MapIcon} title="No land claimed" text="Stand in a chunk in game and use the guild claim command." />
          {:else}
            <div class="pl-list claims">
              {#each g.claims as c (c.id)}
                <div class="pl-item">
                  <span class="cx">{c.chunk_x}, {c.chunk_z}</span>
                  <div class="grow"><b>{dimName(c.dimension)}</b><span class="sub">Block {c.chunk_x * 16}, {c.chunk_z * 16} · {who(c.claimed_by_uuid)?.name ?? 'unknown'} · {timeAgo(c.claimed_at)}</span></div>
                  {#if canRelease}<button class="pl-iconbtn" aria-label="Release chunk" onclick={() => ask('release', { claim: c })}><Trash2 size={15} /></button>{/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>

    {:else if tab === 'posts'}
      <div class="pl-stack">
        {#if can('post')}
          <form class="pl-card compose" onsubmit={(e) => { e.preventDefault(); void publish(); }}>
            <div class="pl-card-head"><h2><Pencil size={16} /> New announcement</h2></div>
            <input class="pl-input" placeholder="Title" maxlength="80" bind:value={pTitle} aria-label="Title" />
            <textarea class="pl-input" rows="3" placeholder="Share news, plans or updates with your guild" bind:value={pBody} aria-label="Content"></textarea>
            <button class="pl-btn primary" disabled={!pTitle.trim() || !pBody.trim() || busy === 'post'} aria-busy={busy === 'post'}>{#if busy === 'post'}<LoaderCircle size={15} class="spin" />{:else}<Send size={15} />{/if} Publish</button>
          </form>
        {/if}
        {#if g.posts.length === 0}
          <div class="pl-card"><Empty icon={MessageSquare} title="No announcements yet" text="Posts from your officers and members show up here." /></div>
        {:else}
          {#each g.posts as p, i (p.id)}
            <article class="pl-card post rise" style:--i={Math.min(i, 8)}>
              <div class="pl-row-flex"><Avatar name={p.author_name} uuid={p.author_uuid} size={36} /><div class="grow"><b>{p.author_name}</b><span class="sub">{timeAgo(p.created_at)}</span></div></div>
              <h3>{p.title}</h3>
              <p class="body">{p.content}</p>
            </article>
          {/each}
        {/if}
      </div>

    {:else}
      <div class="dirhead">
        <h2>All guilds <span class="pl-chip">{all.length}</span></h2>
        {#if all.length > 3}<div class="pl-search"><Search size={16} /><input class="pl-input" type="search" placeholder="Search guilds" bind:value={gq} aria-label="Search guilds" /></div>{/if}
      </div>
      {#if directory.length === 0}
        <div class="pl-card"><Empty icon={Search} title="No match" text={`Nothing matches “${gq}”.`} /></div>
      {:else}
        <div class="pl-grid" style="--min: 280px">{#each directory as x, i (x.id)}{@render guildCard(x, i)}{/each}</div>
      {/if}
    {/if}
    </div>
    {/key}
  {/if}
</div>

<!-- member actions -->
<Sheet bind:open={memberOpen} title={memberSheet?.name ?? 'Member'}>
  {#if memberSheet}
    {@const m = memberSheet}
    <div class="pl-row-flex"><Avatar name={m.name} uuid={m.uuid} size={56} /><div class="grow"><b>{m.name}</b><span class="sub">{roleLabel(m.role)} · joined {new Date(m.joined_at).toLocaleDateString()}</span></div></div>
    <button class="pl-btn block" onclick={() => { memberOpen = false; openProfile(m.uuid); }}>View profile</button>
    {#if isLeader}
      <div>
        <h4 class="lbl">Role</h4>
        <div class="roles">
          {#each [{ name: 'member' }, { name: 'officer' }, ...roles] as r (r.name)}
            <button class="pl-chip rolebtn" class:accent={m.role === r.name} onclick={() => setRole(m, r.name)}>{#if m.role === r.name}<Check size={12} />{/if} {roleLabel(r.name)}</button>
          {/each}
        </div>
      </div>
      <button class="pl-btn block" onclick={() => ask('leader', { member: m })}><Crown size={16} /> Make guild leader</button>
    {/if}
    {#if canKick(m)}<button class="pl-btn block danger" onclick={() => ask('kick', { member: m })}><UserMinus size={16} /> Remove from guild</button>{/if}
  {/if}
</Sheet>

<!-- confirmations -->
<Sheet bind:open={confirmOpen} title={confirm?.kind === 'kick' ? `Remove ${confirm.member?.name}?` : confirm?.kind === 'leader' ? `Make ${confirm.member?.name} the leader?` : confirm?.kind === 'leave' ? 'Leave this guild?' : confirm?.kind === 'release' ? 'Release this chunk?' : `Disband ${g?.name ?? 'guild'}?`}>
  {#if confirm?.kind === 'kick'}<p class="body">They leave the guild straight away and get a notification. They can ask to join again later.</p>
  {:else if confirm?.kind === 'leader'}<p class="body">{confirm.member?.name} becomes the guild leader and you become an officer. Only they can undo it.</p>
  {:else if confirm?.kind === 'leave'}<p class="body">You will lose access to the guild land and treasury. You can ask to join again later.</p>
  {:else if confirm?.kind === 'release'}<p class="body">Chunk {confirm.claim?.chunk_x}, {confirm.claim?.chunk_z} stops being protected and anyone can claim it.</p>
  {:else if confirm?.kind === 'disband' && g}
    <div class="pl-alert err"><TriangleAlert size={15} style="vertical-align:-3px" /> This removes the guild for good: every member is released, all {g.claims_count} claimed chunks are freed and roles, posts and invites are deleted. The treasury is paid to you.</div>
    <label class="field">Type “{g.name}” to confirm<input class="pl-input" bind:value={typed} placeholder={g.name} autocomplete="off" /></label>
  {/if}
  {#snippet footer()}
    <button class="pl-btn" onclick={() => (confirmOpen = false)}>Cancel</button>
    <button class="pl-btn danger-solid" disabled={!!busy || (confirm?.kind === 'disband' && typed.trim().toLowerCase() !== g?.name.toLowerCase())} aria-busy={!!busy} onclick={runConfirm}>
      {confirm?.kind === 'kick' ? 'Remove' : confirm?.kind === 'leader' ? 'Hand over' : confirm?.kind === 'leave' ? 'Leave' : confirm?.kind === 'release' ? 'Release' : 'Disband forever'}
    </button>
  {/snippet}
</Sheet>

<!-- invite -->
<Sheet bind:open={inviteOpen} title="Invite a player">
  <div class="pl-search"><Search size={16} /><input class="pl-input" type="search" placeholder="Search players" bind:value={iq} oninput={onIq} aria-label="Search players" /></div>
  {#if searching && !found.length}
    <div class="pl-skel" style="height:56px"></div>
  {:else if found.length === 0}
    <Empty icon={Search} title="No players found" />
  {:else}
    <div class="pl-list">
      {#each found as u (u.uuid)}
        <div class="pl-item">
          <Avatar name={u.username} uuid={u.uuid} size={40} />
          <div class="grow"><b>{u.username}</b><span class="sub">Lv {u.global_level} · {u.online ? 'Online' : 'Offline'}</span></div>
          {#if invited.has(u.uuid)}<span class="pl-chip good"><Check size={12} /> Invited</span>
          {:else}<button class="pl-btn primary sm" disabled={busy === `i${u.uuid}`} aria-busy={busy === `i${u.uuid}`} onclick={() => invite(u)}>Invite</button>{/if}
        </div>
      {/each}
    </div>
  {/if}
</Sheet>

<!-- roles -->
<Sheet bind:open={rolesOpen} title="Custom roles">
  <p class="sub" style="margin:0">Leaders and officers always can do everything. Members can claim and post. Create roles to hand out specific powers.</p>
  {#if roles.length === 0}<p class="sub">No custom roles yet.</p>{/if}
  <div class="pl-list">
    {#each roles as r (r.id)}
      <div class="pl-item"><div class="grow"><b>{r.name}</b><span class="sub">{permList(r).join(', ') || 'No extra permissions'}</span></div>
        <button class="pl-iconbtn" aria-label="Delete role {r.name}" disabled={busy === `rd${r.id}`} onclick={() => deleteRole(r)}><Trash2 size={15} /></button></div>
    {/each}
  </div>
  <form class="pl-card tight pl-stack" onsubmit={(e) => { e.preventDefault(); void createRole(); }}>
    <b>New role</b>
    <input class="pl-input" placeholder="Role name" maxlength="24" bind:value={rName} aria-label="Role name" />
    <div class="flags">
      {#each [['can_invite', 'Invite'], ['can_kick', 'Kick'], ['can_claim', 'Claim'], ['can_post', 'Post'], ['can_manage', 'Manage'], ['can_vault', 'Faction vault']] as [k, l]}
        <label class="flag"><input type="checkbox" bind:checked={rFlags[k as keyof typeof rFlags]} /> {l}</label>
      {/each}
    </div>
    <button class="pl-btn primary" disabled={rName.trim().length < 2 || busy === 'role'} aria-busy={busy === 'role'}>Create role</button>
  </form>
</Sheet>

<!-- edit -->
<Sheet bind:open={editOpen} title="Guild details" width={560}>
  {#if isLeader}
    <div class="two">
      <label class="field">Name<input class="pl-input" bind:value={eName} maxlength="32" /></label>
      <label class="field tagf">Tag<input class="pl-input" bind:value={eTag} maxlength="6" style="text-transform:uppercase" /></label>
    </div>
  {/if}
  <label class="field">Description<textarea class="pl-input" rows="3" maxlength="500" bind:value={eDesc}></textarea></label>
  <label class="field">Message of the day<input class="pl-input" maxlength="200" bind:value={eMotd} /></label>
  <label class="field">Icon (square image URL)<input class="pl-input" bind:value={eIcon} placeholder="https://…" /></label>
  <label class="field">Banner (wide image URL)<input class="pl-input" bind:value={eBanner} placeholder="https://…" /></label>
  {#if g}<div class="prev">{@render banner({ id: g.id, banner_url: eBanner || null }, 92)}<div class="prev-e">{@render emblem({ id: g.id, tag: eTag || g.tag, icon_url: eIcon || null }, 60)}</div></div>{/if}
  {#snippet footer()}
    <button class="pl-btn" onclick={() => (editOpen = false)}>Cancel</button>
    <button class="pl-btn primary" disabled={busy === 'edit'} aria-busy={busy === 'edit'} onclick={saveEdit}>Save</button>
  {/snippet}
</Sheet>

<!-- request to join -->
<Sheet bind:open={joinOpen} title={joinTarget ? `Join ${joinTarget.name}` : 'Join'}>
  <p class="sub" style="margin:0">The leaders will see your request and can accept it.</p>
  <label class="field">Message (optional)<textarea class="pl-input" rows="3" maxlength="300" bind:value={joinMsg} placeholder="Tell them a bit about you"></textarea></label>
  {#snippet footer()}
    <button class="pl-btn" onclick={() => (joinOpen = false)}>Cancel</button>
    <button class="pl-btn primary" disabled={busy === 'join'} aria-busy={busy === 'join'} onclick={sendJoin}><Send size={15} /> Send request</button>
  {/snippet}
</Sheet>

<!-- create -->
<Sheet bind:open={createOpen} title="Found a guild" width={560}>
  <label class="field">Guild name<input class="pl-input" placeholder="e.g. Iron Fortress" maxlength="32" bind:value={cName} /></label>
  <label class="field">Tag (2 to 6 letters)<input class="pl-input" placeholder="IRON" maxlength="6" bind:value={cTag} style="text-transform:uppercase" /></label>
  <label class="field">Description<textarea class="pl-input" rows="3" maxlength="500" bind:value={cDesc} placeholder="Your playstyle, goals and rules"></textarea></label>
  <label class="field">Icon URL (optional)<input class="pl-input" bind:value={cIcon} placeholder="https://…" /></label>
  <label class="field">Banner URL (optional)<input class="pl-input" bind:value={cBanner} placeholder="https://…" /></label>
  {#snippet footer()}
    <button class="pl-btn" onclick={() => (createOpen = false)}>Cancel</button>
    <button class="pl-btn primary" disabled={cName.trim().length < 3 || cTag.trim().length < 2 || busy === 'create'} aria-busy={busy === 'create'} onclick={createGuild}><Shield size={15} /> Found guild</button>
  {/snippet}
</Sheet>

<ProfileSheet bind:open={profileOpen} uuid={profileUuid} />

<style>
  .gld { --gap: 14px; }
  .gld > :global(* + *) { margin-top: 14px; }
  .gld > :global(.pl-head) { margin-bottom: 20px; }
  .grow { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .grow b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { font-size: 0.8rem; color: var(--muted); }
  .body { color: var(--text-2); line-height: 1.5; white-space: pre-wrap; overflow-wrap: anywhere; }
  .clamp { display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .tight { gap: 10px; }
  .rise { animation: rise 0.4s var(--ease, ease) both; animation-delay: calc(var(--i, 0) * 45ms); }
  @keyframes rise { from { opacity: 0; transform: translateY(10px); } }
  .pane { animation: rise 0.3s var(--ease, ease) both; }
  .pane > :global(* + *) { margin-top: 14px; }
  .tabs { width: 100%; }
  .tabs button { flex: 1; justify-content: center; }
  .pl-stat :global(b span) { font-size: inherit; text-transform: none; letter-spacing: inherit; color: inherit; font-weight: inherit; }
  .pl-stat b.sm { font-size: 1.05rem; }

  /* emblem + banner */
  .emblem { width: var(--s); height: var(--s); flex-shrink: 0; display: grid; place-items: center; overflow: hidden; border-radius: 24%; position: relative;
    background: linear-gradient(145deg, hsl(var(--h) 60% 42%), hsl(calc(var(--h) + 35) 65% 22%)); border: 2px solid hsl(var(--h) 70% 65% / 0.55);
    box-shadow: 0 8px 22px -8px hsl(var(--h) 80% 50% / 0.6), inset 0 0 14px hsl(var(--h) 80% 70% / 0.15); }
  .emblem img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .etag { font-weight: 800; font-size: calc(var(--s) * 0.28); color: #fff; text-shadow: 0 2px 6px #0007; letter-spacing: 0.02em; }
  .banner { position: relative; overflow: hidden; background: radial-gradient(120% 140% at 0% 0%, hsl(var(--h) 65% 38%), transparent 60%), radial-gradient(100% 120% at 100% 100%, hsl(calc(var(--h) + 50) 60% 28%), transparent 55%), hsl(var(--h) 40% 12%); }
  .banner::before { content: ''; position: absolute; inset: 0; background: repeating-linear-gradient(45deg, hsl(var(--h) 60% 70% / 0.05) 0 12px, transparent 12px 24px); }
  .banner img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .shade { position: absolute; inset: 0; background: linear-gradient(180deg, transparent 30%, color-mix(in srgb, var(--surface) 92%, transparent)); }
  .hero { overflow: hidden; }
  .hero-body { display: flex; gap: 16px; align-items: flex-end; padding: 0 18px 16px; margin-top: -42px; position: relative; min-width: 0; }
  .hero-emblem { flex-shrink: 0; }
  .hero-body .grow { gap: 8px; padding-bottom: 2px; }
  .title { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
  .title h2 { font-size: 1.5rem; letter-spacing: -0.02em; overflow-wrap: anywhere; }
  .tagchip { color: var(--accent-2); font-weight: 700; }
  .motd { display: flex; gap: 10px; align-items: center; padding: 11px 18px; border-top: 1px solid var(--pl-line); background: color-mix(in srgb, var(--accent) 10%, transparent); color: var(--text-2); font-size: 0.9rem; }
  .motd :global(svg) { flex-shrink: 0; color: var(--accent-2); }

  .nog { display: flex; gap: 18px; align-items: center; }
  .nog h1 { margin-bottom: 6px; }
  .nog p { margin-bottom: 16px; max-width: 52ch; }
  .nog-ic { width: 72px; height: 72px; border-radius: 22px; display: grid; place-items: center; flex-shrink: 0; background: color-mix(in srgb, var(--accent) 24%, transparent); color: var(--accent-2); box-shadow: 0 0 40px -8px var(--accent); }
  @media (max-width: 560px) { .nog { flex-direction: column; align-items: flex-start; } }
  .dirhead, .rowhead { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .dirhead h2, .rowhead h2 { font-size: 1.05rem; display: flex; gap: 8px; align-items: center; }
  .dirhead .pl-search { flex: 1; max-width: 320px; min-width: 180px; }

  .dir { display: flex; flex-direction: column; }
  .dir.mine { border-color: color-mix(in srgb, var(--accent) 60%, transparent); }
  .dir-body { padding: 0 14px 14px; margin-top: -26px; display: flex; flex-direction: column; gap: 10px; position: relative; }
  .dir-top { display: flex; gap: 12px; align-items: flex-end; }
  .dir-top .grow { padding-bottom: 2px; }
  .desc { color: var(--text-2); font-size: 0.88rem; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; min-height: 2.6em; }

  .who { display: flex; align-items: center; gap: 12px; min-width: 0; flex: 1; text-align: left; background: none; border: none; padding: 0; color: inherit; min-height: 0 !important; }
  .av { position: relative; display: inline-flex; flex-shrink: 0; }
  .av :global(img), .av :global(.fallback) { border-radius: 14px; }
  .av .pl-dot { position: absolute; right: -2px; bottom: -2px; width: 12px; height: 12px; border: 2px solid var(--surface); }
  .member { display: flex; align-items: center; gap: 10px; }
  .you { font-size: 0.68rem; color: var(--accent-2); font-weight: 600; margin-left: 4px; }
  .alert-card { border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
  .invs { border-color: color-mix(in srgb, var(--warn) 45%, transparent); }
  .lbl { font-size: 0.78rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 8px; }
  .roles { display: flex; flex-wrap: wrap; gap: 8px; }
  .rolebtn { min-height: 0 !important; border: 1px solid var(--pl-line); padding: 7px 14px; }
  .field { display: flex; flex-direction: column; gap: 6px; font-size: 0.82rem; color: var(--muted); }
  .two { display: flex; gap: 10px; } .two .field { flex: 1; min-width: 0; } .two .tagf { flex: 0 0 110px; }
  .flags { display: flex; flex-wrap: wrap; gap: 8px; }
  .flag { display: inline-flex; gap: 6px; align-items: center; padding: 7px 12px; border-radius: 10px; background: var(--surface-2); font-size: 0.86rem; }
  .prev { position: relative; border-radius: 16px; overflow: hidden; }
  .prev-e { position: absolute; left: 14px; bottom: 6px; }
  :global(.play) .pl-btn.danger { color: #f59ca0; border-color: color-mix(in srgb, var(--bad) 40%, transparent); }
  :global(.play) .pl-btn.danger-solid { background: var(--bad); border-color: transparent; color: #fff; }

  /* bank */
  .treasury { padding: 22px; }
  .tl { display: inline-flex; align-items: center; gap: 7px; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-2); font-weight: 600; }
  .big { font-size: 2.4rem; font-weight: 800; letter-spacing: -0.02em; margin: 8px 0 12px; font-variant-numeric: tabular-nums; }
  .flow { display: flex; gap: 14px; flex-wrap: wrap; font-size: 0.86rem; align-items: center; }
  .flow span { display: inline-flex; gap: 5px; align-items: center; font-weight: 600; }
  .flow small { color: var(--muted); margin-left: auto; }
  .seg { display: grid; grid-template-columns: 1fr 1fr; gap: 4px; padding: 4px; border-radius: 14px; background: var(--surface-2); }
  .seg button { border: none; background: none; border-radius: 10px; padding: 10px; display: inline-flex; gap: 7px; align-items: center; justify-content: center; font-weight: 600; color: var(--muted); }
  .seg button.on { background: var(--accent); color: #fff; }
  .seg button:disabled { opacity: 0.45; }
  .avail { display: flex; gap: 7px; align-items: center; color: var(--muted); font-size: 0.84rem; margin: 12px 0; }
  .amt { font-size: 1.4rem !important; font-weight: 700; text-align: center; font-variant-numeric: tabular-nums; }
  .quick { display: flex; gap: 6px; flex-wrap: wrap; justify-content: center; }
  .quick .pl-chip { border: none; min-height: 0 !important; padding: 6px 14px; }
  .preview { text-align: center; color: var(--muted); font-size: 0.86rem; }
  .preview b { color: var(--text); }
  .txi { width: 32px; height: 32px; border-radius: 10px; display: grid; place-items: center; flex-shrink: 0; background: color-mix(in srgb, var(--good) 18%, transparent); color: var(--good); }
  .txi.out { background: color-mix(in srgb, var(--bad) 18%, transparent); color: var(--bad); }

  /* land */
  .tmap { width: 100%; max-height: 340px; border-radius: 14px; display: block; background: hsl(var(--h) 30% 9%); }
  .tmap .bg { fill: hsl(var(--h) 30% 9%); }
  .tmap .chunk { fill: hsl(var(--h) 70% 58%); stroke: hsl(var(--h) 80% 75% / 0.6); stroke-width: 0.04; }
  .mini { width: fit-content; }
  .mini button { text-transform: capitalize; padding: 5px 10px; }
  .claims { max-height: 480px; overflow-y: auto; }
  .cx { font-variant-numeric: tabular-nums; font-weight: 700; font-size: 0.82rem; min-width: 74px; padding: 6px 8px; text-align: center; border-radius: 10px; background: var(--surface-2); }
  .compose { display: flex; flex-direction: column; gap: 10px; }
  .compose .pl-btn { align-self: flex-end; }
  .post h3 { margin: 10px 0 6px; font-size: 1.05rem; }

  @media (max-width: 880px) {
    .hero-body { gap: 12px; padding: 0 14px 14px; }
    .title h2 { font-size: 1.25rem; }
    .big { font-size: 2rem; }
    .tabs button { padding: 8px 12px; }
    .tabs { width: 100%; }
    .tabs button { flex: 0 0 auto; }
  }
  @media (prefers-reduced-motion: reduce) { .rise, .pane { animation: none; } }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
