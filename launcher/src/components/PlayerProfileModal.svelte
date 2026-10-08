<script lang="ts">
  import RankTitle from './RankTitle.svelte';
  import AchievementIcon from './AchievementIcon.svelte';
  import { onMount } from 'svelte';
  import {
    X, Trophy, Shield, Heart, Image as ImageIcon, Send, Edit3, Check,
    Clock, Swords, Flame, Pickaxe, Box, LoaderCircle, Award, Star, MessageSquare,
    UserPlus, UserMinus, UserCheck, Wallet, MapPin, Users, Calendar, Palette, LayoutGrid, Skull, LogIn, Sparkles
  } from '@lucide/svelte';
  import Avatar from './Avatar.svelte';
  import Modal from './Modal.svelte';
  import { activeAccount, app, toast } from '../lib/store.svelte';
  import { invoke } from '../lib/tauri';
  import type { UserProfileView, UserPost } from '../lib/types';

  let {
    uuid,
    onclose
  }: {
    uuid: string;
    onclose: () => void;
  } = $props();

  let profile = $state<UserProfileView | null>(null);
  let loading = $state(true);
  let activeTab = $state<'overview' | 'feed' | 'progression' | 'achievements' | 'stats'>('overview');

  // Edit Profile State
  let editing = $state(false);
  let editBio = $state('');
  let editBanner = $state('');
  let editAccent = $state('');
  let relationBusy = $state(false);
  let confirmRemove = $state(false);
  const swatches = ['#8b6cff', '#22d3ee', '#34d399', '#fbbf24', '#fb7185', '#f472b6', '#60a5fa'];
  let savingProfile = $state(false);

  // New Post State
  let newPostContent = $state('');
  let newPostImageUrl = $state('');
  let posting = $state(false);

  const isMe = $derived(activeAccount()?.uuid === uuid);

  async function loadProfile(showSpinner = true) {
    loading = showSpinner;
    try {
      profile = await invoke<UserProfileView>('get_user_profile', { uuid });
      if (profile) {
        editBio = profile.bio || '';
        editBanner = profile.banner_url || '';
        editAccent = profile.accent_color || '';
      }
    } catch (e: any) {
      toast(e?.message ?? 'Failed to load player profile', 'error');
    } finally {
      loading = false;
    }
  }

  async function handleSaveProfile() {
    savingProfile = true;
    try {
      profile = await invoke<UserProfileView>('update_my_profile', {
        bio: editBio.trim(),
        bannerUrl: editBanner.trim() || null,
        accentColor: editAccent
      });
      editing = false;
      toast('Profile updated successfully!', 'ok');
    } catch (e: any) {
      toast(e?.message ?? 'Failed to save profile', 'error');
    } finally {
      savingProfile = false;
    }
  }

  async function handleCreatePost() {
    if (!newPostContent.trim()) return;
    posting = true;
    try {
      const post = await invoke<UserPost>('create_user_post', {
        content: newPostContent.trim(),
        imageUrl: newPostImageUrl.trim() || null
      });
      if (profile) {
        profile.posts = [post, ...profile.posts];
      }
      newPostContent = '';
      newPostImageUrl = '';
      toast('Post published!', 'ok');
    } catch (e: any) {
      toast(e?.message ?? 'Failed to create post', 'error');
    } finally {
      posting = false;
    }
  }

  async function handleLikePost(post: UserPost) {
    if (post.liked_by_me) return;
    try {
      const liked = await invoke<boolean>('like_user_post', { postId: post.id });
      if (liked) post.likes_count += 1;
      post.liked_by_me = true;
    } catch (e: any) {
      toast(e?.message ?? 'Failed to update like', 'error');
    }
  }

  /** Friend actions right from the profile. */
  async function relate(action: 'add' | 'accept' | 'decline' | 'remove') {
    if (!profile) return;
    relationBusy = true;
    try {
      if (action === 'add') await invoke('send_friend_request', { username: profile.username, friendUsername: profile.username });
      else if (action === 'accept' || action === 'decline') await invoke('respond_friend_request', { targetUuid: profile.uuid, friendUuid: profile.uuid, accept: action === 'accept' });
      else await invoke('remove_friend', { targetUuid: profile.uuid, friendUuid: profile.uuid });
      toast(
        action === 'add' ? 'Friend request sent' : action === 'accept' ? `You and ${profile.username} are friends` : action === 'decline' ? 'Request declined' : `${profile.username} removed from friends`,
        'ok'
      );
      await loadProfile(false);
    } catch (e: any) {
      toast(e?.message ?? String(e), 'error');
    } finally {
      relationBusy = false;
    }
  }

  function message() {
    if (!profile) return;
    app.chatWith = profile.uuid;
    app.view = 'social';
    onclose();
  }

  const kd = $derived.by(() => {
    const k = Number(profile?.stats?.player_kills ?? 0);
    const d = Number(profile?.stats?.deaths ?? 0);
    return d === 0 ? (k ? `${k}.0` : '–') : (k / d).toFixed(2);
  });
  const money = (n: number) => `$${Math.round(n).toLocaleString()}`;
  const activityIcon: Record<string, any> = { join: LogIn, death: Skull, kill: Swords, advancement: Award, achievement: Trophy };
  const ago = (iso?: string | null) => {
    if (!iso) return 'never';
    const secs = Math.max(0, (Date.now() - new Date(iso).getTime()) / 1000);
    if (secs < 90) return 'just now';
    if (secs < 5400) return `${Math.round(secs / 60)}m ago`;
    if (secs < 129600) return `${Math.round(secs / 3600)}h ago`;
    return `${Math.round(secs / 86400)}d ago`;
  };

  function formatPlaytime(secs: number): string {
    if (!secs || secs < 60) return `${secs || 0}s`;
    const hours = Math.floor(secs / 3600);
    const mins = Math.floor((secs % 3600) / 60);
    return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
  }

  onMount(() => {
    loadProfile();
  });
</script>

<div class="profile-backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="profile-modal glass" role="dialog" aria-modal="true" aria-label="Player Profile" style:--pa={profile?.accent_color || 'var(--accent)'}>
    <button class="ghost icon close-btn" onclick={onclose} aria-label="Close Profile">
      <X size={18} />
    </button>

    {#if loading || !profile}
      <div class="center-state"><LoaderCircle class="spin" size={32} /></div>
    {:else}
      <!-- Banner & Profile Header -->
      <div
        class="profile-banner"
        style:background-image={profile.banner_url ? `linear-gradient(rgba(0,0,0,0.4), rgba(0,0,0,0.8)), url(${profile.banner_url})` : undefined}
      >
        <div class="header-overlay">
          <div class="user-main">
            <div class="avatar-ring">
              <Avatar uuid={profile.uuid} name={profile.username} size={4.2} />
            </div>
            <div class="user-titles">
              <div class="user-row">
                <h2>{profile.username}</h2>
                {#if profile.level_info?.title}
                  <span class="badge title-badge"><RankTitle text={profile.level_info.title} image={profile.level_info.title_image} /></span>
                {/if}
              </div>
              <div class="meta-row tiny muted">
                <span class="presence" class:on={profile.online}></span>
                <span>{profile.online ? 'Online now' : `Seen ${ago(profile.last_seen)}`}</span>
                <span>•</span>
                <span>{profile.friends_count} Friends</span>
                <span>•</span>
                <span class="global-lvl-tag">Level {profile.level_info?.level ?? 1}</span>
              </div>
              {#if profile.guild || profile.rank}
                <div class="chips-row">
                  {#if profile.guild}<button class="chip guild" onclick={() => { app.view = 'guilds'; onclose(); }} title="{profile.guild.role} of {profile.guild.name}"><Shield size={11} /> [{profile.guild.tag}] {profile.guild.name}</button>{/if}
                  {#if profile.rank}<span class="chip rank" title="Reported by {profile.rank.server_name}"><Sparkles size={11} /> {profile.rank.display}</span>{/if}
                </div>
              {/if}
            </div>
          </div>

          {#if isMe}
            <button class="ghost sm edit-btn" onclick={() => (editing = !editing)}>
              <Edit3 size={14} /> {editing ? 'Cancel' : 'Edit Profile'}
            </button>
          {/if}
        </div>
      </div>

      {#if !isMe && profile.relationship}
        <div class="relation-bar">
          {#if profile.relationship === 'accepted'}
            <button class="sm primary" onclick={message}><MessageSquare size={14} /> Message</button>
            {#if confirmRemove}
              <span class="tiny muted">Remove {profile.username} from your friends?</span>
              <button class="sm danger" disabled={relationBusy} onclick={() => { confirmRemove = false; relate('remove'); }}>Yes, remove</button>
              <button class="sm ghost" onclick={() => (confirmRemove = false)}>Keep</button>
            {:else}
              <button class="sm ghost" disabled={relationBusy} onclick={() => (confirmRemove = true)}><UserMinus size={14} /> Remove friend</button>
            {/if}
          {:else if profile.relationship === 'pending_incoming'}
            <span class="tiny muted">{profile.username} wants to be friends</span>
            <button class="sm primary" disabled={relationBusy} onclick={() => relate('accept')}><UserCheck size={14} /> Accept</button>
            <button class="sm ghost" disabled={relationBusy} onclick={() => relate('decline')}>Decline</button>
          {:else if profile.relationship === 'pending_outgoing'}
            <span class="tiny muted">Friend request sent</span>
            <button class="sm ghost" disabled={relationBusy} onclick={() => relate('remove')}>Cancel request</button>
          {:else}
            <button class="sm primary" disabled={relationBusy} onclick={() => relate('add')}><UserPlus size={14} /> Add friend</button>
          {/if}
        </div>
      {/if}

      <!-- Bio / Edit Form -->
      <div class="bio-section">
        {#if editing}
          <div class="edit-profile-box glass">
            <label>
              <span class="tiny bold">Bio</span>
              <textarea bind:value={editBio} placeholder="Tell other players about yourself..." rows="2"></textarea>
            </label>
            <label>
              <span class="tiny bold">Banner Image URL</span>
              <input type="url" bind:value={editBanner} placeholder="https://example.com/banner.jpg" />
            </label>
            <div class="accent-edit">
              <span class="tiny bold"><Palette size={12} /> Accent colour</span>
              <div class="swatches">
                {#each swatches as c}
                  <button type="button" class="swatch" class:on={editAccent === c} style:background={c} aria-label="Use {c}" onclick={() => (editAccent = c)}></button>
                {/each}
                <input type="color" value={editAccent || '#8b6cff'} oninput={(e) => (editAccent = e.currentTarget.value)} aria-label="Custom colour" />
                <button type="button" class="ghost sm" onclick={() => (editAccent = '')}>Default</button>
              </div>
            </div>
            <div class="edit-actions">
              <button class="ghost sm" onclick={() => (editing = false)}>Cancel</button>
              <button class="primary sm" onclick={handleSaveProfile} disabled={savingProfile}>
                {#if savingProfile}<LoaderCircle size={13} class="spin" />{:else}<Check size={13} />{/if}
                Save
              </button>
            </div>
          </div>
        {:else if profile.bio}
          <p class="bio-text">{profile.bio}</p>
        {/if}

        {#if profile.badges && profile.badges.length > 0}
          <div class="badges-list">
            {#each profile.badges as badge}
              <span class="badge custom-badge"><Star size={11} /> {badge}</span>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Segmented Profile Tabs -->
      <div class="profile-tabs">
        <button class:active={activeTab === 'overview'} onclick={() => (activeTab = 'overview')}>
          <LayoutGrid size={14} /> <span>Overview</span>
        </button>
        <button class:active={activeTab === 'feed'} onclick={() => (activeTab = 'feed')}>
          <MessageSquare size={14} /> <span>Posts</span>
        </button>
        <button class:active={activeTab === 'progression'} onclick={() => (activeTab = 'progression')}>
          <Award size={14} /> <span>Levels</span>
        </button>
        <button class:active={activeTab === 'achievements'} onclick={() => (activeTab = 'achievements')}>
          <Trophy size={14} /> <span>Awards {profile.achievements_count ?? profile.achievements?.length ?? 0}</span>
        </button>
        <button class:active={activeTab === 'stats'} onclick={() => (activeTab = 'stats')}>
          <Swords size={14} /> <span>Stats</span>
        </button>
      </div>

      <!-- Tab Content Area -->
      <div class="modal-tab-content">
        {#if activeTab === 'overview'}
          <div class="overview">
            <div class="facts">
              <div class="fact glass"><Star size={15} class="accent" /><span class="tiny muted">Level</span><strong>{profile.level_info?.level ?? 1}</strong></div>
              <div class="fact glass"><Clock size={15} class="muted" /><span class="tiny muted">Playtime</span><strong>{formatPlaytime(profile.stats?.playtime_secs ?? 0)}</strong></div>
              <div class="fact glass"><Swords size={15} class="accent" /><span class="tiny muted">K/D</span><strong>{kd}</strong></div>
              <div class="fact glass"><Wallet size={15} class="success" /><span class="tiny muted">Wealth</span><strong>{money(profile.wealth ?? 0)}</strong></div>
              <div class="fact glass"><MapPin size={15} class="warn" /><span class="tiny muted">Favourite server</span><strong class="small-val">{profile.favorite_server ?? '–'}</strong></div>
              <div class="fact glass"><Calendar size={15} class="muted" /><span class="tiny muted">Joined</span><strong class="small-val">{new Date(profile.created_at).toLocaleDateString()}</strong></div>
            </div>

            {#if profile.featured_achievement}
              <div class="featured glass"><Trophy size={15} class="warn" /><span class="tiny muted">Featured</span><strong>{profile.featured_achievement.title}</strong></div>
            {/if}

            {#if profile.mutual_friends && profile.mutual_friends.length}
              <div>
                <h4 class="subhead"><Users size={14} /> {profile.mutual_friends.length} mutual friend{profile.mutual_friends.length === 1 ? '' : 's'}</h4>
                <div class="mutuals">
                  {#each profile.mutual_friends as m (m.uuid)}
                    <button class="mutual" onclick={() => (app.viewProfileUuid = m.uuid)}><Avatar uuid={m.uuid} name={m.username} size={1.6} /> {m.username}</button>
                  {/each}
                </div>
              </div>
            {/if}

            <div>
              <h4 class="subhead">Recent activity</h4>
              {#if profile.recent_activity && profile.recent_activity.length}
                <div class="activity">
                  {#each profile.recent_activity as a}
                    {@const Icon = activityIcon[a.kind] ?? Sparkles}
                    <div class="act"><span class="act-ic {a.kind}"><Icon size={12} /></span><span>{a.text}</span><span class="tiny muted">{ago(a.at)}</span></div>
                  {/each}
                </div>
              {:else}
                <p class="muted tiny">Nothing yet. Activity shows up as they play.</p>
              {/if}
            </div>
          </div>
        {:else if activeTab === 'feed'}
          <div class="feed-container">
            <!-- Create Post (if viewing self) -->
            {#if isMe}
              <div class="create-post-box glass">
                <textarea
                  bind:value={newPostContent}
                  placeholder="Share a milestone, screenshot, or status update..."
                  rows="2"
                  class="post-input"
                ></textarea>
                <div class="post-controls">
                  <div class="image-input-wrap">
                    <ImageIcon size={14} class="muted" />
                    <input
                      type="url"
                      placeholder="Photo URL (optional)"
                      bind:value={newPostImageUrl}
                      class="img-url-input"
                    />
                  </div>
                  <button
                    class="primary sm"
                    onclick={handleCreatePost}
                    disabled={posting || !newPostContent.trim()}
                  >
                    {#if posting}<LoaderCircle size={13} class="spin" />{:else}<Send size={13} />{/if}
                    Post
                  </button>
                </div>
              </div>
            {/if}

            <!-- User Posts List -->
            {#if profile.posts.length === 0}
              <div class="empty-state">
                <p class="muted">No posts yet from {profile.username}.</p>
              </div>
            {:else}
              <div class="posts-stream">
                {#each profile.posts as post (post.id)}
                  <div class="post-card glass">
                    <div class="post-top">
                      <Avatar uuid={profile.uuid} name={profile.username} size={2} />
                      <div class="post-author-meta">
                        <span class="author-name">{profile.username}</span>
                        <span class="post-time tiny muted">{new Date(post.created_at).toLocaleString()}</span>
                      </div>
                    </div>
                    <p class="post-body">{post.content}</p>
                    {#if post.image_url}
                      <div class="post-photo-wrap">
                        <img src={post.image_url} alt="Post Attachment" class="post-photo" />
                      </div>
                    {/if}
                    <div class="post-footer">
                      <button
                        class="ghost sm like-btn"
                        class:liked={post.liked_by_me}
                        onclick={() => handleLikePost(post)}
                      >
                        <Heart size={14} fill={post.liked_by_me ? 'currentColor' : 'none'} />
                        <span>{post.likes_count}</span>
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {:else if activeTab === 'progression'}
          <!-- Progression Tab -->
          <div class="progression-view">
            <!-- Global Level Card -->
            <div class="prog-card glass">
              <div class="prog-header">
                <div>
                  <span class="tiny bold uppercase muted">Account Progression</span>
                  <h3>Velora Global Level</h3>
                </div>
                <div class="level-pill">
                  Level {profile.level_info.level}
                </div>
              </div>
              <div class="xp-bar-wrap">
                <div class="xp-numbers tiny">
                  <span>{profile.level_info.current_xp.toLocaleString()} XP</span>
                  <span>{profile.level_info.next_level_xp.toLocaleString()} XP</span>
                </div>
                <div class="xp-bar">
                  <div class="xp-fill" style:width="{profile.level_info.progress_pct}%"></div>
                </div>
              </div>
            </div>

            <!-- Server Levels List -->
            {#if profile.server_levels && profile.server_levels.length > 0}
              <h4 class="subhead">Server Specific Levels</h4>
              <div class="servers-levels-grid">
                {#each profile.server_levels as sl}
                  <div class="server-lvl-card glass">
                    <div class="sl-top">
                      <div>
                        <strong>{sl.server_name}</strong>
                        {#if sl.rank_title || sl.rank_name}
                          <span class="badge rank-badge"><RankTitle text={sl.rank_title || sl.rank_name || ''} image={sl.title_image} /></span>
                        {/if}
                      </div>
                      <span class="lvl-badge">Lv. {sl.level}</span>
                    </div>
                    <div class="xp-bar">
                      <div class="xp-fill" style:width="{sl.progress_pct}%"></div>
                    </div>
                    <span class="tiny muted">{sl.current_xp} / {sl.next_level_xp} XP</span>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="muted tiny">No server levels recorded yet.</p>
            {/if}
          </div>
        {:else if activeTab === 'achievements'}
          <!-- Achievements Tab -->
          <div class="achievements-view">
            {#if profile.achievements.length === 0}
              <div class="empty-state">
                <Trophy size={32} class="muted" />
                <p class="muted">No achievements unlocked yet.</p>
              </div>
            {:else}
              <div class="ach-grid">
                {#each profile.achievements as ach (ach.id)}
                  <div class="ach-item glass" class:frame-challenge={ach.frame_type === 'challenge'}>
                    <AchievementIcon {ach} size={2.2} />
                    <div class="ach-meta">
                      <span class="ach-name">{ach.title}</span>
                      <p class="ach-sub tiny muted">{ach.description}</p>
                      <span class="tiny xp-label">+{ach.xp_reward} XP</span>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {:else if activeTab === 'stats'}
          <!-- Stats Tab -->
          <div class="stats-view">
            {#if profile.stats}
              <div class="stats-cards-grid">
                <div class="stat-mini-card glass">
                  <Clock size={16} class="muted" />
                  <span class="tiny muted">Playtime</span>
                  <strong>{formatPlaytime(profile.stats.playtime_secs)}</strong>
                </div>
                <div class="stat-mini-card glass">
                  <Swords size={16} class="accent" />
                  <span class="tiny muted">Player Kills</span>
                  <strong>{profile.stats.player_kills.toLocaleString()}</strong>
                </div>
                <div class="stat-mini-card glass">
                  <Flame size={16} class="warn" />
                  <span class="tiny muted">Mob Kills</span>
                  <strong>{profile.stats.mob_kills.toLocaleString()}</strong>
                </div>
                <div class="stat-mini-card glass">
                  <Pickaxe size={16} class="success" />
                  <span class="tiny muted">Blocks Mined</span>
                  <strong>{profile.stats.blocks_broken.toLocaleString()}</strong>
                </div>
                <div class="stat-mini-card glass">
                  <Box size={16} class="muted" />
                  <span class="tiny muted">Blocks Placed</span>
                  <strong>{profile.stats.blocks_placed.toLocaleString()}</strong>
                </div>
                <div class="stat-mini-card glass">
                  <Award size={16} class="accent" />
                  <span class="tiny muted">Server Joins</span>
                  <strong>{profile.stats.joins.toLocaleString()}</strong>
                </div>
              </div>
            {:else}
              <div class="empty-state">
                <p class="muted">No statistics recorded yet for this player.</p>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .profile-backdrop {
    position: fixed;
    inset: 0;
    z-index: 85;
    display: grid;
    place-items: center;
    padding: 1.5rem;
    background: color-mix(in srgb, var(--bg) 75%, transparent);
    animation: fade 0.15s ease;
  }
  .profile-modal {
    width: 100%;
    max-width: 38rem;
    max-height: calc(100vh - 4rem);
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--line-strong);
    border-radius: var(--radius);
    position: relative;
    box-shadow: 0 1rem 3rem -1rem rgba(0, 0, 0, 0.6);
  }
  .close-btn {
    position: absolute;
    top: 0.8rem;
    right: 0.8rem;
    z-index: 10;
    background: rgba(0, 0, 0, 0.5);
    border-radius: 50%;
  }
  .profile-banner {
    height: 9rem;
    background-color: color-mix(in srgb, var(--accent) 25%, black);
    background-size: cover;
    background-position: center;
    position: relative;
    display: flex;
    align-items: flex-end;
  }
  .header-overlay {
    width: 100%;
    padding: 1rem 1.4rem;
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.85) 0%, transparent 100%);
  }
  .user-main {
    display: flex;
    align-items: flex-end;
    gap: 1rem;
  }
  .avatar-ring {
    border: 3px solid var(--pa, var(--surface));
    border-radius: 28%;
    overflow: hidden;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    background: var(--surface);
  }
  .user-titles {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .user-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .user-row h2 {
    font-size: 1.3rem;
    font-weight: 750;
    margin: 0;
  }
  .title-badge {
    background: color-mix(in srgb, #f59e0b 25%, transparent);
    color: #f59e0b;
  }
  .meta-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .global-lvl-tag {
    color: var(--pa, var(--accent));
    font-weight: 650;
  }
  .edit-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: rgba(0, 0, 0, 0.5);
  }
  .bio-section {
    padding: 0.9rem 1.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    border-bottom: 1px solid var(--line);
  }
  .bio-text {
    font-size: 0.88rem;
    color: color-mix(in srgb, var(--text) 90%, transparent);
    line-height: 1.4;
    margin: 0;
  }
  .edit-profile-box {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.9rem;
    border-radius: var(--radius-sm);
  }
  .edit-profile-box textarea, .edit-profile-box input {
    background: color-mix(in srgb, var(--bg) 60%, transparent);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius-sm);
    font-size: 0.82rem;
  }
  .edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.4rem;
  }
  .badges-list {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .custom-badge {
    background: color-mix(in srgb, var(--text) 8%, transparent);
    font-size: 0.72rem;
  }
  .profile-tabs {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    border-bottom: 1px solid var(--line);
    background: color-mix(in srgb, var(--surface) 90%, black);
    padding: 0 0.5rem;
    gap: 0.15rem;
  }
  .profile-tabs button {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    gap: 0.3rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 0.65rem 0.25rem;
    font-size: 0.8rem;
    font-weight: 560;
    color: var(--muted);
    border-radius: 0;
    transition: all 0.15s;
  }
  .profile-tabs button.active {
    color: var(--text);
    border-bottom-color: var(--pa, var(--accent));
  }
  .profile-tabs button { white-space: nowrap; }
  .profile-tabs button span { overflow: hidden; text-overflow: ellipsis; }
  .profile-tabs button :global(svg) { flex-shrink: 0; }
  @media (max-width: 34rem) { .profile-tabs button :global(svg) { display: none; } }
  .presence { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: color-mix(in srgb, var(--muted) 60%, transparent); }
  .presence.on { background: #34d399; box-shadow: 0 0 0 3px color-mix(in srgb, #34d399 25%, transparent); }
  .chips-row { display: flex; gap: 0.4rem; flex-wrap: wrap; margin-top: 0.25rem; }
  .chip { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.7rem; font-weight: 650; padding: 0.15rem 0.55rem; border-radius: 99rem; background: rgba(0, 0, 0, 0.45); border: 1px solid color-mix(in srgb, var(--text) 18%, transparent); color: var(--text); }
  .chip.guild { cursor: pointer; }
  .chip.guild:hover { border-color: var(--pa, var(--accent)); }
  .chip.rank { color: #fbbf24; }
  .relation-bar { display: flex; align-items: center; gap: 0.5rem; padding: 0.7rem 1.4rem 0; flex-wrap: wrap; }
  .relation-bar .sm { display: inline-flex; align-items: center; gap: 0.35rem; }
  .accent-edit { display: flex; flex-direction: column; gap: 0.4rem; }
  .swatches { display: flex; align-items: center; gap: 0.4rem; flex-wrap: wrap; }
  .swatch { width: 1.4rem; height: 1.4rem; padding: 0; border-radius: 50%; border: 2px solid transparent; }
  .swatch.on { border-color: var(--text); }
  .swatches input[type='color'] { width: 1.8rem; height: 1.6rem; padding: 0; border: none; background: none; }
  .overview { display: flex; flex-direction: column; gap: 1rem; }
  .facts { display: grid; grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr)); gap: 0.7rem; }
  .fact { display: flex; flex-direction: column; gap: 0.2rem; padding: 0.75rem; border-radius: var(--radius-sm); border: 1px solid var(--line); }
  .fact strong { font-size: 1.1rem; }
  .fact strong.small-val { font-size: 0.9rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .featured { display: flex; align-items: center; gap: 0.5rem; padding: 0.6rem 0.8rem; border-radius: var(--radius-sm); border: 1px solid var(--line); }
  .mutuals { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.4rem; }
  .mutual { display: inline-flex; align-items: center; gap: 0.4rem; padding: 0.2rem 0.6rem 0.2rem 0.25rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 7%, transparent); font-size: 0.78rem; }
  .activity { display: flex; flex-direction: column; gap: 0.45rem; margin-top: 0.4rem; }
  .act { display: grid; grid-template-columns: 1.6rem 1fr auto; gap: 0.6rem; align-items: center; font-size: 0.82rem; }
  .act-ic { width: 1.6rem; height: 1.6rem; display: grid; place-items: center; border-radius: 0.45rem; background: color-mix(in srgb, var(--text) 8%, transparent); color: var(--muted); }
  .act-ic.achievement, .act-ic.advancement { color: #fbbf24; }
  .act-ic.death { color: #fb7185; }
  .act-ic.join { color: #34d399; }
  .subhead { display: flex; align-items: center; gap: 0.4rem; }
  .modal-tab-content {
    padding: 1.2rem 1.4rem;
  }
  /* Feed Tab */
  .feed-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .create-post-box {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.9rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
  }
  .post-input {
    background: color-mix(in srgb, var(--bg) 50%, transparent);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 0.5rem 0.7rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    resize: vertical;
  }
  .post-controls {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
  }
  .image-input-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: color-mix(in srgb, var(--bg) 50%, transparent);
    border: 1px solid var(--line);
    padding: 0.25rem 0.6rem;
    border-radius: var(--radius-sm);
  }
  .img-url-input {
    flex: 1;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 0.8rem;
    outline: none;
  }
  .posts-stream {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .post-card {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
  }
  .post-top {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .author-name {
    font-size: 0.85rem;
    font-weight: 650;
    display: block;
  }
  .post-body {
    font-size: 0.86rem;
    line-height: 1.4;
    margin: 0;
  }
  .post-photo-wrap {
    border-radius: var(--radius-sm);
    overflow: hidden;
    max-height: 16rem;
  }
  .post-photo {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .post-footer {
    display: flex;
    justify-content: flex-start;
  }
  .like-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .like-btn.liked {
    color: #e11d48;
  }
  /* Progression */
  .progression-view {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .prog-card {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1.1rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
  }
  .prog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .prog-header h3 {
    margin: 0.1rem 0 0;
    font-size: 1.1rem;
  }
  .level-pill {
    padding: 0.3rem 0.7rem;
    border-radius: 99rem;
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
    font-weight: 700;
    font-size: 0.88rem;
  }
  .xp-bar-wrap {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .xp-numbers {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
  }
  .xp-bar {
    width: 100%;
    height: 0.45rem;
    background: color-mix(in srgb, var(--text) 10%, transparent);
    border-radius: 99rem;
    overflow: hidden;
  }
  .xp-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 99rem;
  }
  .subhead {
    font-size: 0.88rem;
    font-weight: 700;
  }
  .servers-levels-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr));
    gap: 0.8rem;
  }
  .server-lvl-card {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.8rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
  }
  .sl-top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }
  .lvl-badge {
    font-weight: 700;
    color: var(--accent);
    font-size: 0.82rem;
  }
  .rank-badge {
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #f59e0b;
    font-size: 0.65rem;
  }
  /* Achievements */
  .ach-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
    gap: 0.8rem;
  }
  .ach-item {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.75rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
  }
  .ach-meta {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .ach-name {
    font-size: 0.84rem;
    font-weight: 700;
  }
  .ach-sub {
    margin: 0.1rem 0;
    line-height: 1.25;
  }
  .xp-label {
    color: var(--accent);
    font-weight: 700;
  }
  /* Stats Grid */
  .stats-cards-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
    gap: 0.8rem;
  }
  .stat-mini-card {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.8rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
  }
  .stat-mini-card strong {
    font-size: 1.1rem;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.72rem;
    font-weight: 600;
    padding: 0.16rem 0.5rem;
    border-radius: 99rem;
  }
  .center-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.8rem;
    padding: 2.5rem 1rem;
    text-align: center;
  }
</style>
