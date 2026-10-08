<script lang="ts">
  import { untrack } from 'svelte';
  import {
    Heart, Image as ImageIcon, Send, Pencil, Check, Clock, Swords, Trophy, Shield, Sparkles, MessageSquare, UserPlus, UserMinus, UserCheck,
    Wallet, MapPin, Calendar, Users, Star, Skull, LogIn, Award, Trash2, LoaderCircle, Palette, RefreshCw
  } from '@lucide/svelte';
  import Sheet from './Sheet.svelte';
  import AchIcon from './AchIcon.svelte';
  import Avatar from '../../components/Avatar.svelte';
  import { del, get, post, put, timeAgo } from '../../lib/api';
  import { go } from '../../lib/router.svelte';
  import { session } from '../../lib/session.svelte';
  import { toast, toastError } from '../../lib/toast.svelte';
  import { compact } from '../store.svelte';

  // A player's profile in a sheet. Tap a name anywhere, set `uuid`, and bind `open`.
  let { uuid, open = $bindable(false), onchange, onmessage }: {
    uuid: string | null;
    open: boolean;
    /** Called after a friend request was sent/answered/removed so the page behind can refresh. */
    onchange?: () => void;
    /** Overrides the default "open the conversation in Friends". */
    onmessage?: (uuid: string, name: string) => void;
  } = $props();

  type Post = { id: number; user_uuid: string; username: string; content: string; image_url: string | null; likes_count: number; liked_by_me: boolean; created_at: string };
  type Ach = { id: string; title: string; description?: string; xp_reward?: number; frame_type?: string; icon_item?: string; icon_bg?: string; icon_border?: string };
  type Profile = {
    uuid: string; username: string; bio: string; banner_url: string | null; created_at: string; friends_count: number;
    level_info?: { level: number; title?: string | null; title_image?: string | null; current_xp: number; next_level_xp: number; progress_pct: number; rank?: number | null };
    badges?: string[]; achievements?: Ach[]; achievements_count?: number; posts: Post[];
    stats?: { playtime_secs: number; deaths: number; player_kills: number; mob_kills: number; blocks_broken: number; blocks_placed: number; joins: number; messages: number } | null;
    online?: boolean; last_seen?: string | null; guild?: { id: string; name: string; tag: string; role: string } | null;
    rank?: { display: string; server_name: string } | null; favorite_server?: string | null; wealth?: number;
    recent_activity?: { kind: string; text: string; at: string }[]; mutual_friends?: { uuid: string; username: string }[];
    relationship?: string; accent_color?: string | null; featured_achievement?: Ach | null;
  };

  let cur = $state<string | null>(untrack(() => uuid));
  let profile = $state<Profile | null>(null);
  let loading = $state(false);
  let error = $state('');
  let tab = $state<'overview' | 'posts' | 'awards'>('overview');
  let busy = $state(false);
  let confirmRemove = $state(false);

  let editing = $state(false);
  let bio = $state(''), banner = $state(''), accent = $state('');
  let saving = $state(false);
  let postText = $state(''), postImage = $state('');
  let posting = $state(false);
  const swatches = ['#8b6cff', '#22d3ee', '#34d399', '#fbbf24', '#fb7185', '#f472b6', '#60a5fa'];

  const isMe = $derived(!!profile && profile.uuid === session.user?.uuid);
  const kd = $derived.by(() => {
    const k = profile?.stats?.player_kills ?? 0, d = profile?.stats?.deaths ?? 0;
    return d === 0 ? (k ? `${k}.0` : '–') : (k / d).toFixed(2);
  });
  const hours = (s: number) => (s >= 3600 ? `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m` : `${Math.floor(s / 60)}m`);
  const actIcon: Record<string, any> = { join: LogIn, death: Skull, kill: Swords, advancement: Award, achievement: Trophy };

  let seq = 0;
  async function load(id: string, spinner = true) {
    const mine = ++seq;
    if (spinner) { loading = true; profile = null; }
    error = '';
    try {
      const p = await get<Profile>(`/api/v1/profiles/${id}`);
      if (mine !== seq) return;
      profile = p;
      bio = p.bio ?? ''; banner = p.banner_url ?? ''; accent = p.accent_color ?? '';
    } catch (e) {
      if (mine === seq) error = e instanceof Error ? e.message : 'Could not load this profile';
    } finally {
      if (mine === seq) loading = false;
    }
  }

  // A new uuid from the parent resets the sheet to that player.
  $effect(() => { const u = uuid; if (open) untrack(() => { cur = u; tab = 'overview'; editing = false; confirmRemove = false; }); });
  $effect(() => { if (open && cur) untrack(() => void load(cur!)); });

  async function relate(action: 'add' | 'accept' | 'decline' | 'remove') {
    if (!profile) return;
    busy = true;
    try {
      if (action === 'add') await post('/api/v1/friends/request', { username: profile.username });
      else if (action === 'remove') await del(`/api/v1/friends/${profile.uuid}`);
      else await post('/api/v1/friends/respond', { target_uuid: profile.uuid, accept: action === 'accept' });
      toast(action === 'add' ? 'Friend request sent' : action === 'accept' ? `You and ${profile.username} are friends` : action === 'decline' ? 'Request declined' : action === 'remove' ? 'Removed' : '', 'ok');
      confirmRemove = false;
      await load(profile.uuid, false);
      onchange?.();
    } catch (e) { toastError(e); } finally { busy = false; }
  }

  function message() {
    if (!profile) return;
    if (onmessage) onmessage(profile.uuid, profile.username);
    else {
      try { sessionStorage.setItem('scopenet.play.chat', profile.uuid); } catch { /* ignore */ }
      window.dispatchEvent(new CustomEvent('play:chat', { detail: profile.uuid }));
      go('play/social');
    }
    open = false;
  }

  async function saveProfile() {
    saving = true;
    try {
      await put('/api/v1/profiles/me', { bio: bio.trim(), banner_url: banner.trim() || null, accent_color: accent || null });
      toast('Profile updated');
      editing = false;
      if (cur) await load(cur, false);
    } catch (e) { toastError(e); } finally { saving = false; }
  }

  async function publish() {
    if (!postText.trim() || !profile) return;
    posting = true;
    try {
      const p = await post<Post>('/api/v1/profiles/me/posts', { content: postText.trim(), image_url: postImage.trim() || null });
      profile.posts = [p, ...profile.posts];
      postText = ''; postImage = '';
      toast('Posted');
    } catch (e) { toastError(e); } finally { posting = false; }
  }

  async function like(p: Post) {
    if (p.liked_by_me) return;
    p.liked_by_me = true; p.likes_count += 1; // optimistic
    try { await post(`/api/v1/posts/${p.id}/like`); } catch (e) { p.liked_by_me = false; p.likes_count -= 1; toastError(e); }
  }

  async function removePost(p: Post) {
    if (!profile) return;
    const before = profile.posts;
    profile.posts = before.filter((x) => x.id !== p.id);
    try { await del(`/api/v1/profiles/me/posts/${p.id}`); toast('Post deleted'); } catch (e) { profile.posts = before; toastError(e); }
  }
</script>

<Sheet bind:open title={profile?.username ?? 'Player profile'} width={600}>
  {#if loading}
    <div class="skel-wrap" aria-busy="true">
      <div class="pl-skel" style="height:96px;border-radius:16px"></div>
      <div class="pl-skel" style="height:22px;width:55%"></div>
      <div class="pl-skel" style="height:64px"></div>
      <div class="pl-skel" style="height:140px"></div>
    </div>
  {:else if error}
    <div class="pl-alert err">{error}</div>
    <button class="pl-btn" onclick={() => cur && load(cur)}><RefreshCw size={15} /> Try again</button>
  {:else if profile}
    <div class="prof" style:--pa={profile.accent_color || 'var(--accent)'}>
      <div class="banner" style:background-image={profile.banner_url ? `linear-gradient(rgba(0,0,0,.15), rgba(0,0,0,.7)), url(${profile.banner_url})` : undefined}></div>
      <div class="head">
        <div class="ring"><Avatar uuid={profile.uuid} name={profile.username} size={72} /></div>
        <div class="who">
          <h3>{profile.username}</h3>
          <div class="meta">
            <span class="pl-dot" class:on={profile.online}></span>
            <span>{profile.online ? 'Online now' : `Seen ${timeAgo(profile.last_seen)}`}</span>
            <span class="sep">•</span><span>{profile.friends_count} friends</span>
          </div>
          <div class="chips">
            <span class="pl-chip accent"><Star size={12} /> Level {profile.level_info?.level ?? 1}</span>
            {#if profile.level_info?.title}<span class="pl-chip">{profile.level_info.title}</span>{/if}
            {#if profile.guild}<span class="pl-chip good"><Shield size={12} /> [{profile.guild.tag}] {profile.guild.name}</span>{/if}
            {#if profile.rank}<span class="pl-chip warn"><Sparkles size={12} /> {profile.rank.display}</span>{/if}
          </div>
        </div>
      </div>

      {#if isMe}
        <button class="pl-btn sm" onclick={() => (editing = !editing)}><Pencil size={14} /> {editing ? 'Cancel editing' : 'Edit profile'}</button>
      {:else if profile.relationship}
        <div class="rel">
          {#if profile.relationship === 'accepted'}
            <button class="pl-btn primary" onclick={message}><MessageSquare size={15} /> Message</button>
            {#if confirmRemove}
              <button class="pl-btn" style="color:var(--bad)" disabled={busy} onclick={() => relate('remove')}>Yes, remove</button>
              <button class="pl-btn" onclick={() => (confirmRemove = false)}>Keep</button>
            {:else}
              <button class="pl-btn" disabled={busy} onclick={() => (confirmRemove = true)}><UserMinus size={15} /> Unfriend</button>
            {/if}
          {:else if profile.relationship === 'pending_incoming'}
            <button class="pl-btn primary" disabled={busy} aria-busy={busy} onclick={() => relate('accept')}><UserCheck size={15} /> Accept request</button>
            <button class="pl-btn" disabled={busy} onclick={() => relate('decline')}>Decline</button>
          {:else if profile.relationship === 'pending_outgoing'}
            <span class="pl-chip">Request sent</span>
            <button class="pl-btn sm" disabled={busy} onclick={() => relate('remove')}>Cancel</button>
          {:else}
            <button class="pl-btn primary" disabled={busy} aria-busy={busy} onclick={() => relate('add')}><UserPlus size={15} /> Add friend</button>
          {/if}
        </div>
      {/if}

      {#if editing}
        <div class="pl-card tight edit">
          <label>Bio<textarea class="pl-input" rows="3" maxlength="300" bind:value={bio} placeholder="Tell other players about yourself"></textarea></label>
          <label>Banner image URL<input class="pl-input" type="url" bind:value={banner} placeholder="https://…" /></label>
          <div class="sw"><span><Palette size={13} /> Accent</span>
            {#each swatches as c}<button type="button" class="swatch" class:on={accent === c} style:background={c} aria-label="Use {c}" onclick={() => (accent = c)}></button>{/each}
            <input type="color" value={accent || '#8b6cff'} oninput={(e) => (accent = e.currentTarget.value)} aria-label="Custom colour" />
            <button type="button" class="pl-btn sm" onclick={() => (accent = '')}>Default</button>
          </div>
          <button class="pl-btn primary" disabled={saving} aria-busy={saving} onclick={saveProfile}>{#if saving}<LoaderCircle size={15} class="spin" />{:else}<Check size={15} />{/if} Save profile</button>
        </div>
      {:else if profile.bio}
        <p class="bio">{profile.bio}</p>
      {/if}

      {#if profile.badges?.length}
        <div class="chips">{#each profile.badges as b}<span class="pl-chip warn"><Star size={11} /> {b}</span>{/each}</div>
      {/if}

      <div class="pl-tabs" role="tablist">
        <button class:on={tab === 'overview'} onclick={() => (tab = 'overview')}>Overview</button>
        <button class:on={tab === 'posts'} onclick={() => (tab = 'posts')}>Posts <span class="count">{profile.posts.length}</span></button>
        <button class:on={tab === 'awards'} onclick={() => (tab = 'awards')}>Awards <span class="count">{profile.achievements_count ?? profile.achievements?.length ?? 0}</span></button>
      </div>

      {#if tab === 'overview'}
        <div class="facts">
          <div><Star size={15} /><span>Level</span><b>{profile.level_info?.level ?? 1}</b></div>
          <div><Clock size={15} /><span>Playtime</span><b>{hours(profile.stats?.playtime_secs ?? 0)}</b></div>
          <div><Swords size={15} /><span>K/D</span><b>{kd}</b></div>
          <div><Wallet size={15} /><span>Wealth</span><b>{profile.wealth != null ? '$' + compact(profile.wealth) : '–'}</b></div>
          <div><MapPin size={15} /><span>Favourite</span><b class="sm">{profile.favorite_server ?? '–'}</b></div>
          <div><Calendar size={15} /><span>Joined</span><b class="sm">{new Date(profile.created_at).toLocaleDateString()}</b></div>
        </div>
        {#if profile.stats}
          <div class="pl-stats stats">
            <div class="pl-stat"><span>Blocks broken</span><b>{compact(profile.stats.blocks_broken)}</b></div>
            <div class="pl-stat"><span>Blocks placed</span><b>{compact(profile.stats.blocks_placed)}</b></div>
            <div class="pl-stat"><span>Mob kills</span><b>{compact(profile.stats.mob_kills)}</b></div>
            <div class="pl-stat"><span>Deaths</span><b>{compact(profile.stats.deaths)}</b></div>
          </div>
        {/if}
        {#if profile.mutual_friends?.length}
          <h4><Users size={14} /> {profile.mutual_friends.length} mutual friend{profile.mutual_friends.length === 1 ? '' : 's'}</h4>
          <div class="pl-scroll-x">
            {#each profile.mutual_friends as m (m.uuid)}
              <button class="mutual" onclick={() => (cur = m.uuid)}><Avatar uuid={m.uuid} name={m.username} size={22} /> {m.username}</button>
            {/each}
          </div>
        {/if}
        <h4>Recent activity</h4>
        {#if profile.recent_activity?.length}
          <div class="pl-list">
            {#each profile.recent_activity as a}
              {@const Ic = actIcon[a.kind] ?? Sparkles}
              <div class="pl-item"><span class="ic"><Ic size={14} /></span><div class="grow"><span>{a.text}</span></div><span class="sub end">{timeAgo(a.at)}</span></div>
            {/each}
          </div>
        {:else}
          <p class="muted">Nothing yet. Activity shows up as they play.</p>
        {/if}
      {:else if tab === 'posts'}
        {#if isMe}
          <div class="pl-card tight compose">
            <textarea class="pl-input" rows="2" maxlength="500" bind:value={postText} placeholder="Share a milestone or a status update"></textarea>
            <div class="row">
              <label class="img"><ImageIcon size={15} /><input class="pl-input" type="url" bind:value={postImage} placeholder="Photo URL (optional)" /></label>
              <button class="pl-btn primary" disabled={posting || !postText.trim()} aria-busy={posting} onclick={publish}>{#if posting}<LoaderCircle size={15} class="spin" />{:else}<Send size={15} />{/if} Post</button>
            </div>
          </div>
        {/if}
        {#each profile.posts as p (p.id)}
          <article class="pl-card tight post">
            <div class="ptop"><span class="sub">{timeAgo(p.created_at)}</span>
              {#if isMe}<button class="x" aria-label="Delete post" onclick={() => removePost(p)}><Trash2 size={14} /></button>{/if}
            </div>
            <p>{p.content}</p>
            {#if p.image_url}<img class="photo" src={p.image_url} alt="" loading="lazy" />{/if}
            <button class="like" class:liked={p.liked_by_me} onclick={() => like(p)} aria-label="Like"><Heart size={15} fill={p.liked_by_me ? 'currentColor' : 'none'} /> {p.likes_count}</button>
          </article>
        {:else}
          <p class="muted">No posts yet.</p>
        {/each}
      {:else}
        {#if profile.achievements?.length}
          <div class="pl-list">
            {#each profile.achievements as a (a.id)}
              <div class="pl-item"><AchIcon ach={a} size={42} /><div class="grow"><b>{a.title}</b><span class="sub">{a.description ?? ''}</span></div>{#if a.xp_reward}<span class="pl-chip accent end">+{a.xp_reward} XP</span>{/if}</div>
            {/each}
          </div>
        {:else}
          <p class="muted">No achievements unlocked yet.</p>
        {/if}
      {/if}
    </div>
  {/if}
</Sheet>

<style>
  .skel-wrap, .prof { display: flex; flex-direction: column; gap: 12px; }
  .prof { --pa: var(--accent); }
  .banner { height: 88px; margin: -14px -18px 0; background: radial-gradient(120% 140% at 0% 0%, color-mix(in srgb, var(--pa) 70%, #000), transparent 65%), linear-gradient(135deg, color-mix(in srgb, var(--pa) 40%, var(--surface)), var(--surface-2)); background-size: cover; background-position: center; }
  .head { display: flex; gap: 14px; align-items: flex-start; margin-top: -40px; position: relative; min-width: 0; }
  .ring { padding: 4px; border-radius: 22px; background: var(--surface); border: 2px solid var(--pa); box-shadow: 0 0 22px -4px var(--pa); flex-shrink: 0; line-height: 0; }
  .ring :global(img) { border-radius: 16px; }
  .who { min-width: 0; display: flex; flex-direction: column; gap: 5px; padding-top: 44px; }
  h3 { font-size: 1.25rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { display: flex; align-items: center; gap: 7px; font-size: 0.8rem; color: var(--muted); flex-wrap: wrap; }
  .sep { opacity: 0.5; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .rel { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .bio { color: var(--text-2); line-height: 1.5; white-space: pre-wrap; overflow-wrap: anywhere; }
  .edit { display: flex; flex-direction: column; gap: 10px; }
  .edit label { display: flex; flex-direction: column; gap: 5px; font-size: 0.8rem; color: var(--muted); }
  .sw { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-size: 0.8rem; color: var(--muted); }
  .sw span { display: inline-flex; gap: 5px; align-items: center; }
  .swatch { width: 28px; height: 28px; min-height: 0 !important; padding: 0; border-radius: 50%; border: 2px solid transparent; }
  .swatch.on { border-color: #fff; }
  .sw input[type='color'] { width: 34px; height: 30px; padding: 0; border: none; background: none; }
  .facts { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .facts > div { display: flex; flex-direction: column; gap: 2px; padding: 10px; border-radius: 14px; background: var(--surface-2); border: 1px solid var(--pl-line); min-width: 0; }
  .facts :global(svg) { color: var(--pa); }
  .facts span { font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); }
  .facts b { font-size: 1.05rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .facts b.sm { font-size: 0.86rem; }
  .stats { grid-template-columns: repeat(2, 1fr); gap: 10px; }
  h4 { display: flex; align-items: center; gap: 6px; font-size: 0.88rem; color: var(--text-2); margin-top: 4px; }
  .muted { color: var(--muted); font-size: 0.86rem; }
  .mutual { display: inline-flex; align-items: center; gap: 7px; padding: 5px 12px 5px 6px; border-radius: 99px; background: var(--surface-2); border: 1px solid var(--pl-line); font-size: 0.84rem; }
  .ic { width: 28px; height: 28px; border-radius: 9px; display: grid; place-items: center; background: color-mix(in srgb, var(--pa) 20%, transparent); color: var(--pa); flex-shrink: 0; }
  .compose { display: flex; flex-direction: column; gap: 8px; }
  .compose .row { display: flex; gap: 8px; align-items: center; }
  .img { flex: 1; min-width: 0; display: flex; align-items: center; gap: 6px; color: var(--muted); }
  .img input { flex: 1; min-width: 0; }
  .post { display: flex; flex-direction: column; gap: 8px; }
  .post p { overflow-wrap: anywhere; white-space: pre-wrap; }
  .ptop { display: flex; justify-content: space-between; align-items: center; }
  .sub { font-size: 0.78rem; color: var(--muted); }
  .x { border: none; background: none; color: var(--muted); padding: 4px; min-height: 0 !important; }
  .x:hover { color: var(--bad); }
  .photo { width: 100%; max-height: 260px; object-fit: cover; border-radius: 12px; }
  .like { align-self: flex-start; display: inline-flex; gap: 6px; align-items: center; padding: 6px 12px; border-radius: 99px; background: var(--surface-2); border-color: var(--pl-line); color: var(--text-2); min-height: 0 !important; transition: transform 0.15s; }
  .like.liked { color: #fb7185; }
  .like:active { transform: scale(1.15); }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
