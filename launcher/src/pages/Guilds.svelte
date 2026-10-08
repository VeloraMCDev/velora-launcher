<script lang="ts">
  import PlayerLink from '../components/PlayerLink.svelte';
  import { onMount } from 'svelte';
  import {
    Shield, Map, Users, MessageSquare, Plus, Crown, UserCheck, Trash2,
     Pin, LoaderCircle, RefreshCw, Landmark, Sparkles, Send, Search, Pencil, TriangleAlert, UserMinus, Handshake
  } from '@lucide/svelte';
  import Avatar from '../components/Avatar.svelte';
  import ChunkMap from '../components/ChunkMap.svelte';
  import GuildWallet from '../components/GuildWallet.svelte';
  import LandRules from '../components/LandRules.svelte';
  import GuildEmblem from '../components/GuildEmblem.svelte';
  import GuildBanner from '../components/GuildBanner.svelte';
  import Modal from '../components/Modal.svelte';
  import { activeAccount, app, instances, selectedInstance, toast } from '../lib/store.svelte';
  import { invoke } from '../lib/tauri';
  import type { Guild, GuildMember, GuildPost, GuildRelation } from '../lib/types';

  let selectedInstId = $state<string>(selectedInstance()?.id || (instances()[0]?.id ?? ''));
  let myGuild = $state<Guild | null>(null);
  let allGuilds = $state<Guild[]>([]);
  let members = $state<GuildMember[]>([]);
  let posts = $state<GuildPost[]>([]);
  let activeTab = $state<'territory' | 'members' | 'feed' | 'wallet' | 'settings' | 'relations'>('territory');
  type GuildRole = { id: number; name: string; can_invite: boolean; can_kick: boolean; can_claim: boolean; can_post: boolean; can_manage: boolean };
  type JoinRequest = { uuid: string; name: string; message: string; created_at: string };
  let roles = $state<GuildRole[]>([]);
  let joinRequests = $state<JoinRequest[]>([]);
  let newRoleName = $state('');
  let roleFlags = $state({ can_invite: false, can_kick: false, can_claim: false, can_post: true, can_manage: false });
  let guildEdit = $state({ description: '', motd: '', icon_url: '', banner_url: '' });
  const myRole = $derived(members.find((m) => m.uuid === activeAccount()?.uuid)?.role ?? '');
  const canManageDetails = $derived(myRole === 'leader' || myRole === 'officer' || !!roles.find((r) => r.name === myRole)?.can_manage);
  let loading = $state(false);
  let walletServers = $state<Array<{ id: number; name: string; instance_id: string }>>([]);
  type Membership = { id: string; instance_id: string; name: string; tag: string; role: string; server_count: number; primary_server_count: number };
  let memberships = $state<Membership[]>([]);
  let relations = $state<GuildRelation[]>([]);
  let relationTarget = $state('');
  let relationKind = $state<'alliance' | 'rival'>('alliance');

  // Create Guild Modal
  let createModalOpen = $state(false);
  let newGuildName = $state('');
  let newGuildTag = $state('');
  let newGuildDesc = $state('');
  let newGuildIcon = $state('');
  let newGuildBanner = $state('');
  let creating = $state(false);

  // New Post Form
  let postTitle = $state('');
  let postContent = $state('');
  let postPinned = $state(false);
  let posting = $state(false);

  const currentInstance = $derived(
    instances().find((i) => i.id === selectedInstId) ?? instances()[0] ?? null
  );

  async function loadInstanceGuildData() {
    if (!selectedInstId) return;
    loading = true;
    try {
      const [mine, list, memberList] = await Promise.all([
        invoke<Guild | null>('get_my_guild', { instanceId: selectedInstId }).catch(() => null),
        invoke<Guild[]>('get_guilds', { instanceId: selectedInstId }).catch(() => []),
        invoke<Membership[]>('get_my_guild_memberships', { instanceId: selectedInstId }).catch(() => [])
      ]);
      myGuild = mine;
      allGuilds = list;
      memberships = memberList;

      if (mine) {
        const [mList, pList] = await Promise.all([
          invoke<GuildMember[]>('get_guild_members', { guildId: mine.id }).catch(() => []),
          invoke<GuildPost[]>('get_guild_posts', { guildId: mine.id }).catch(() => [])
        ]);
        members = mList;
        posts = pList;
        roles = (await invoke<GuildRole[] | null>('get_guild_roles', { guildId: mine.id }).catch(() => [])) ?? [];
        relations = await invoke<GuildRelation[]>('get_guild_relations', { guildId: mine.id }).catch(() => []);
        joinRequests = (await invoke<JoinRequest[] | null>('get_guild_join_requests', { guildId: mine.id }).catch(() => [])) ?? [];
        guildEdit = { description: mine.description ?? '', motd: mine.motd ?? '', icon_url: mine.icon_url ?? '', banner_url: mine.banner_url ?? '' };
      } else {
        members = [];
        posts = [];
        roles = [];
        joinRequests = [];
      }
    } catch (e: any) {
      toast(e?.message ?? 'Failed to load guild data', 'error');
    } finally {
      loading = false;
    }
  }

  type Invite = { id: number; guild_id: string; guild_name: string; guild_tag: string; icon_url: string; inviter: string; inviter_uuid?: string };
  let invites = $state<Invite[]>([]);
  async function loadInvites() {
    try { invites = (await invoke<Invite[] | null>('my_guild_invites')) ?? []; } catch { invites = []; }
  }
  async function answerInvite(inv: Invite, accept: boolean) {
    try {
      await invoke('respond_guild_invite', { inviteId: inv.id, accept });
      toast(accept ? `Welcome to ${inv.guild_name}!` : 'Invitation declined', 'ok');
      await Promise.all([loadInvites(), loadInstanceGuildData()]);
    } catch (e: any) {
      toast(typeof e === 'string' ? e : e?.message ?? 'Could not answer the invitation', 'error');
      loadInvites();
    }
  }

  async function makePrimary(guildId: string, serverId: number) {
    try { await invoke('set_primary_guild', { guildId, serverId }); toast('Primary guild updated for this server', 'ok'); await loadInstanceGuildData(); }
    catch (e: any) { toast(String(e), 'error'); }
  }
  async function sendRelation() {
    if (!myGuild || !relationTarget) return;
    try {
      await invoke('create_guild_relation', { guildId: myGuild.id, otherGuildId: relationTarget, relation: relationKind });
      relationTarget = '';
      relations = await invoke<GuildRelation[]>('get_guild_relations', { guildId: myGuild.id });
      toast('Relation request sent', 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
  }
  async function decideRelation(relation: GuildRelation, accept: boolean) {
    if (!myGuild) return;
    try {
      await invoke('respond_guild_relation', { guildId: myGuild.id, relationId: relation.id, accept });
      relations = await invoke<GuildRelation[]>('get_guild_relations', { guildId: myGuild.id });
      toast(accept ? 'Relation accepted' : 'Relation declined', 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
  }

  async function requestJoin(guild: Guild) {
    try {
      await invoke('request_guild_join', { guildId: guild.id, message: '' });
      toast(`Request sent to ${guild.name}`, 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
  }

  async function answerJoin(req: JoinRequest, accept: boolean) {
    if (!myGuild) return;
    try {
      await invoke('respond_guild_join_request', { guildId: myGuild.id, uuid: req.uuid, accept });
      toast(accept ? `${req.name} joined the guild` : 'Request declined', 'ok');
      await loadInstanceGuildData();
    } catch (e: any) { toast(String(e), 'error'); }
  }

  async function createRole() {
    if (!myGuild || !newRoleName.trim()) return;
    try {
      await invoke('create_guild_role', { guildId: myGuild.id, role: { name: newRoleName.trim(), ...roleFlags } });
      newRoleName = '';
      roles = (await invoke<GuildRole[] | null>('get_guild_roles', { guildId: myGuild.id })) ?? [];
      toast('Guild role created', 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
  }

  async function assignRole(member: GuildMember, role: string) {
    if (!myGuild) return;
    try {
      await invoke('assign_guild_role', { guildId: myGuild.id, uuid: member.uuid, role });
      member.role = role as GuildMember['role'];
      toast('Guild role assigned', 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
  }

  const canKick = (m: GuildMember) =>
    m.uuid !== activeAccount()?.uuid && m.role !== 'leader' && (myRole === 'leader' || (myRole === 'officer' && m.role !== 'officer'));
  let confirmKick = $state<GuildMember | null>(null);
  let confirmLeader = $state<GuildMember | null>(null);

  async function kick(m: GuildMember) {
    if (!myGuild) return;
    try {
      await invoke('kick_guild_member', { guildId: myGuild.id, uuid: m.uuid });
      members = members.filter((x) => x.uuid !== m.uuid);
      toast(`${m.name} was removed from the guild`, 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
    confirmKick = null;
  }

  async function makeLeader(m: GuildMember) {
    if (!myGuild) return;
    try {
      await invoke('transfer_guild_leader', { guildId: myGuild.id, uuid: m.uuid });
      toast(`${m.name} now leads the guild. You are an officer.`, 'ok');
      await loadInstanceGuildData();
    } catch (e: any) { toast(String(e), 'error'); }
    confirmLeader = null;
  }

  async function deleteRole(role: GuildRole) {
    if (!myGuild) return;
    try {
      await invoke('delete_guild_role', { guildId: myGuild.id, roleId: role.id });
      await loadInstanceGuildData();
      toast('Guild role deleted', 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
  }

  // ---- browsing every guild on the instance, even while in one
  let view = $state<'mine' | 'browse'>('mine');
  let guildSearch = $state('');
  const directory = $derived(allGuilds.filter((g) => {
    const q = guildSearch.trim().toLowerCase();
    return !q || g.name.toLowerCase().includes(q) || g.tag.toLowerCase().includes(q) || (g.description ?? '').toLowerCase().includes(q);
  }));
  $effect(() => { if (!myGuild) view = 'mine'; });

  // ---- rename and disband (leader only)
  let renameName = $state(''), renameTag = $state('');
  let renaming = $state(false);
  $effect(() => { if (myGuild) { renameName = myGuild.name; renameTag = myGuild.tag; } });
  async function renameGuild() {
    if (!myGuild || renaming) return;
    renaming = true;
    try {
      await invoke('rename_guild', { guildId: myGuild.id, name: renameName.trim(), tag: renameTag.trim() });
      await loadInstanceGuildData();
      toast('Guild renamed', 'ok');
    } catch (e: any) { toast(typeof e === 'string' ? e : e?.message ?? 'Could not rename the guild', 'error'); }
    finally { renaming = false; }
  }
  let disbandOpen = $state(false), disbandConfirm = $state(''), disbanding = $state(false);
  async function disbandGuild() {
    if (!myGuild || disbanding || disbandConfirm.trim().toLowerCase() !== myGuild.name.toLowerCase()) return;
    disbanding = true;
    try {
      const r = await invoke<{ refunded?: number }>('disband_guild', { guildId: myGuild.id });
      disbandOpen = false; disbandConfirm = '';
      toast(r?.refunded ? `Guild disbanded. ${r.refunded.toLocaleString()} from the treasury was paid to you.` : 'Guild disbanded', 'ok');
      await loadInstanceGuildData();
    } catch (e: any) { toast(typeof e === 'string' ? e : e?.message ?? 'Could not disband the guild', 'error'); }
    finally { disbanding = false; }
  }

  async function saveGuild() {
    if (!myGuild) return;
    try {
      await invoke('update_guild', { guildId: myGuild.id, description: guildEdit.description, motd: guildEdit.motd, iconUrl: guildEdit.icon_url, bannerUrl: guildEdit.banner_url });
      await loadInstanceGuildData();
      toast('Guild details saved', 'ok');
    } catch (e: any) { toast(String(e), 'error'); }
  }

  onMount(async () => {
    loadInvites();
    try { walletServers = (await invoke<{ servers: typeof walletServers }>('get_public_servers')).servers; }
    catch { walletServers = []; }
  });

  async function handleCreateGuild() {
    if (!newGuildName.trim() || !newGuildTag.trim()) {
      toast('Please enter both guild name and tag', 'error');
      return;
    }
    creating = true;
    try {
      const guild = await invoke<Guild>('create_guild', {
        instanceId: selectedInstId,
        name: newGuildName.trim(),
        tag: newGuildTag.trim().toUpperCase(),
        description: newGuildDesc.trim(),
        iconUrl: newGuildIcon.trim() || null,
        bannerUrl: newGuildBanner.trim() || null
      });
      toast(`Guild [${guild.tag}] ${guild.name} founded!`, 'ok');
      createModalOpen = false;
      newGuildName = '';
      newGuildTag = '';
      newGuildDesc = '';
      newGuildIcon = '';
      newGuildBanner = '';
      await loadInstanceGuildData();
    } catch (e: any) {
      toast(e?.message ?? 'Failed to create guild', 'error');
    } finally {
      creating = false;
    }
  }

  async function handleCreatePost() {
    if (!myGuild || !postTitle.trim() || !postContent.trim()) {
      toast('Please provide a title and announcement content', 'error');
      return;
    }
    posting = true;
    try {
      const post = await invoke<GuildPost>('create_guild_post', {
        guildId: myGuild.id,
        title: postTitle.trim(),
        content: postContent.trim(),
        pinned: postPinned
      });
      posts = [post, ...posts];
      postTitle = '';
      postContent = '';
      postPinned = false;
      toast('Announcement posted to guild wall', 'ok');
    } catch (e: any) {
      toast(e?.message ?? 'Failed to create post', 'error');
    } finally {
      posting = false;
    }
  }

  $effect(() => {
    if (selectedInstId) {
      loadInstanceGuildData();
    }
  });

  onMount(() => {
    if (selectedInstance()?.id) {
      selectedInstId = selectedInstance()!.id;
    }
    loadInstanceGuildData();
  });
</script>

{#snippet guildDirectory()}
  <div class="browse-section">
    <div class="browse-header">
      <h3>Guilds on {currentInstance?.name || 'this instance'} ({allGuilds.length})</h3>
      {#if allGuilds.length > 4}
        <label class="guild-search"><Search size={14} /><input type="search" placeholder="Search guilds…" bind:value={guildSearch} aria-label="Search guilds" /></label>
      {/if}
    </div>
    {#if allGuilds.length === 0}
      <div class="empty-state glass">
        <p>No guilds have been created on this instance yet. Be the first to establish territory!</p>
      </div>
    {:else if directory.length === 0}
      <div class="empty-state glass"><p>No guild matches “{guildSearch}”.</p></div>
    {:else}
      <div class="guilds-directory-grid">
        {#each directory as g (g.id)}
          <article class="guild-dir-card glass" class:mine={g.id === myGuild?.id}>
            <GuildBanner guild={g} height={5}>
              {#if g.id === myGuild?.id}<span class="ribbon">Your guild</span>{/if}
            </GuildBanner>
            <div class="dir-body">
              <div class="dir-emblem"><GuildEmblem guild={g} size={3.6} /></div>
              <div class="dir-meta">
                <h4>{g.name}</h4>
                <span class="dir-tagline">[{g.tag}]</span>
              </div>
              {#if g.description}<p class="dir-desc">{g.description}</p>{/if}
              <div class="dir-stats">
                <span title="Members"><Users size={13} /> {g.member_count}<small>/{g.max_members}</small></span>
                <span title="Claimed chunks"><Map size={13} /> {g.claims_count}</span>
              </div>
              {#if g.id === myGuild?.id}
                <button class="sm ghost" onclick={() => (view = 'mine')}>Open your guild</button>
              {:else}
                <button class="sm primary" onclick={() => requestJoin(g)} disabled={!!myGuild} title={myGuild ? 'Leave your guild first to join another' : ''}>Request to join</button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<div class="guilds-page">
  <!-- Page Header & Instance Picker -->
  <div class="page-header">
    <div class="title-wrap">
      <h1><Shield class="guild-icon" size={26} /> Guilds & Territories</h1>
      <p class="lead">Form factions, claim land chunks protected from griefing, and collaborate with your party.</p>
    </div>
    <div class="header-actions">
      <div class="instance-select-wrap">
        <label for="instance-select" class="tiny muted uppercase">Instance</label>
        <select
          id="instance-select"
          class="inst-select"
          bind:value={selectedInstId}
        >
          {#each instances() as inst}
            <option value={inst.id}>{inst.name}</option>
          {/each}
        </select>
      </div>
      <button class="ghost icon" onclick={loadInstanceGuildData} title="Refresh" aria-label="Refresh">
        <RefreshCw size={17} class={loading ? 'spin' : ''} />
      </button>
    </div>
  </div>

  {#if myGuild}
    <div class="view-toggle" role="tablist" aria-label="Guild view">
      <button role="tab" aria-selected={view === 'mine'} class:active={view === 'mine'} onclick={() => (view = 'mine')}><Shield size={14} /> My guild</button>
      <button role="tab" aria-selected={view === 'browse'} class:active={view === 'browse'} onclick={() => (view = 'browse')}><Users size={14} /> All guilds ({allGuilds.length})</button>
    </div>
  {/if}

  {#if invites.length}
    <section class="invites glass" aria-label="Guild invitations">
      <h3>Guild invitations</h3>
      {#each invites as inv (inv.id)}
        <div class="invite">
          <GuildEmblem guild={{ id: inv.guild_id, tag: inv.guild_tag, icon_url: inv.icon_url }} size={2.4} />
          <span class="text"><strong>[{inv.guild_tag}] {inv.guild_name}</strong><span class="muted tiny">Invited by <PlayerLink uuid={inv.inviter_uuid} name={inv.inviter} /></span></span>
          <button class="sm ghost" onclick={() => answerInvite(inv, false)}>Decline</button>
          <button class="sm primary" onclick={() => answerInvite(inv, true)} disabled={!!myGuild} title={myGuild ? 'Leave your guild first' : ''}>Accept</button>
        </div>
      {/each}
    </section>
  {/if}

  {#if loading && !myGuild && !allGuilds.length}
    <div class="center-state"><LoaderCircle class="spin" size={32} /></div>
  {:else if myGuild && view === 'browse'}
    {@render guildDirectory()}
  {:else if myGuild}
    <!-- User is in a Guild -->
    <div class="guild-hub">
      <!-- Guild Banner & Meta Header -->
      <section class="guild-hero glass">
        <GuildBanner guild={myGuild} height={9.5} />
        <div class="hero-body">
          <GuildEmblem guild={myGuild} size={6} />
          <div class="hero-text">
            <div class="guild-title-line">
              <h2>{myGuild.name}</h2>
              <span class="tag-chip">[{myGuild.tag}]</span>
            </div>
            {#if myGuild.description}<p class="guild-desc">{myGuild.description}</p>{/if}
            <div class="hero-chips">
              <span class="badge member-badge"><Users size={12} /> {myGuild.member_count}/{myGuild.max_members} members</span>
              <span class="badge claim-badge"><Map size={12} /> {myGuild.claims_count} chunks</span>
              {#if myRole}<span class="badge role-chip">{myRole === 'leader' ? '👑 Leader' : myRole === 'officer' ? '⭐ Officer' : myRole}</span>{/if}
            </div>
          </div>
        </div>
        {#if myGuild.motd}<div class="motd"><MessageSquare size={14} /> <span>{myGuild.motd}</span></div>{/if}
      </section>

      <!-- Segmented Navigation Tabs -->
      <div class="guild-tabs-row">
        <div class="tabs-segmented">
          <button class:active={activeTab === 'territory'} onclick={() => (activeTab = 'territory')}>
            <Map size={15} /> Territory & Land Claims
          </button>
          <button class:active={activeTab === 'members'} onclick={() => (activeTab = 'members')}>
            <Users size={15} /> Roster ({members.length})
          </button>
          <button class:active={activeTab === 'feed'} onclick={() => (activeTab = 'feed')}>
            <MessageSquare size={15} /> Guild Feed ({posts.length})
          </button>
          <button class:active={activeTab === 'wallet'} onclick={() => (activeTab = 'wallet')}>
            <Landmark size={15} /> Treasury
          </button>
          <button class:active={activeTab === 'relations'} onclick={() => (activeTab = 'relations')}><Handshake size={15} /> Memberships & relations</button>
          {#if canManageDetails}<button class:active={activeTab === 'settings'} onclick={() => (activeTab = 'settings')}>Guild settings</button>{/if}
        </div>
      </div>

      <!-- Tab Content -->
      {#if activeTab === 'territory'}
        <div class="territory-tab">
          <div class="protection-callout glass">
            <Shield size={18} class="callout-icon" />
            <div class="callout-text">
              <strong>Grief Protection Relay Active</strong>
              <p class="tiny muted">
                Chunks claimed by your guild are synchronized with the Velora mod/plugin on the server. By default non-members cannot build, open chests or use doors on your land; you can change that in the land rules below.
              </p>
            </div>
          </div>

          <LandRules guildId={myGuild.id} />

          <ChunkMap
            instanceId={selectedInstId}
            myGuildId={myGuild.id}
            myGuildTag={myGuild.tag}
            onclaim={() => { if (myGuild) myGuild.claims_count += 1; }}
            onunclaim={() => { if (myGuild) myGuild.claims_count = Math.max(0, myGuild.claims_count - 1); }}
          />
        </div>
      {:else if activeTab === 'wallet'}
        <GuildWallet guild={myGuild} {members} servers={walletServers} instanceId={selectedInstId} />
      {:else if activeTab === 'members'}
        <div class="members-tab">
          {#if joinRequests.length}
            <div class="card"><h3>Join requests</h3>
              {#each joinRequests as req}
                <div class="row"><strong>{req.name}</strong><span>{req.message}</span><span class="spacer"></span><button class="sm" onclick={() => answerJoin(req, false)}>Decline</button><button class="sm primary" onclick={() => answerJoin(req, true)}>Accept</button></div>
              {/each}
            </div>
          {/if}
          <div class="members-grid">
            {#each members as m (m.uuid)}
              <div
                class="member-card glass"
                role="button"
                tabindex="0"
                onclick={() => (app.viewProfileUuid = m.uuid)}
                onkeydown={(e) => e.key === 'Enter' && (app.viewProfileUuid = m.uuid)}
              >
                <div class="member-avatar-wrap">
                  <Avatar uuid={m.uuid} name={m.name} size={2.5} />
                  {#if m.online}
                    <span class="online-indicator" title="Online"></span>
                  {/if}
                </div>
                <div class="member-details">
                  <div class="member-name-row">
                    <span class="member-name">{m.name}</span>
                    {#if m.role === 'leader'}
                      <span class="badge leader-badge"><Crown size={11} /> Leader</span>
                    {:else if m.role === 'officer'}
                      <span class="badge officer-badge"><UserCheck size={11} /> Officer</span>
                    {:else if m.role === 'member'}
                      <span class="badge member-role-badge">Member</span>
                    {:else}
                      <span class="badge member-role-badge">{m.role}</span>
                    {/if}
                  </div>
                  <span class="tiny muted">Joined {new Date(m.joined_at).toLocaleDateString()}</span>
                  {#if myGuild.leader_uuid === activeAccount()?.uuid && m.uuid !== activeAccount()?.uuid}
                    <select value={m.role} aria-label="Role for {m.name}" onchange={(e) => assignRole(m, e.currentTarget.value)}>
                      <option value="member">Member</option><option value="officer">Officer</option>
                      {#each roles as role}<option value={role.name}>{role.name}</option>{/each}
                    </select>
                  {/if}
                  {#if canKick(m) || (myRole === 'leader' && m.uuid !== activeAccount()?.uuid)}
                    <div class="member-actions">
                      {#if myRole === 'leader' && m.uuid !== activeAccount()?.uuid}
                        <button class="ghost tiny-btn" onclick={(e) => { e.stopPropagation(); confirmLeader = m; }} title="Make {m.name} the guild leader"><Crown size={12} /> Make leader</button>
                      {/if}
                      {#if canKick(m)}
                        <button class="ghost tiny-btn danger-text" onclick={(e) => { e.stopPropagation(); confirmKick = m; }} title="Remove {m.name}"><UserMinus size={12} /> Remove</button>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {:else if activeTab === 'relations'}
        <section class="card col relation-panel">
          <h3>Your guild memberships in this instance</h3>
          {#if memberships.length === 0}<p class="muted">No memberships in this instance.</p>{:else}
            {#each memberships as membership (membership.id)}
              <div class="row"><span><strong>[{membership.tag}] {membership.name}</strong><small>{membership.role} · primary on {membership.primary_server_count} server(s)</small></span><span class="spacer"></span>
                {#each walletServers.filter((server) => server.instance_id === selectedInstId) as server (server.id)}
                  <button class="sm" class:primary={myGuild?.id === membership.id} onclick={() => makePrimary(membership.id, server.id)}>{myGuild?.id === membership.id ? 'Primary' : 'Set primary'} · {server.name}</button>
                {/each}
              </div>
            {/each}
          {/if}
          {#if myGuild && canManageDetails}
            <h3>Alliance and rivalry requests</h3>
            <div class="relation-create"><select bind:value={relationTarget} aria-label="Other guild"><option value="">Choose a guild</option>{#each allGuilds.filter((g) => g.id !== myGuild?.id) as guild}<option value={guild.id}>[{guild.tag}] {guild.name}</option>{/each}</select><select bind:value={relationKind} aria-label="Relation type"><option value="alliance">Alliance</option><option value="rival">Rivalry</option></select><button class="primary sm" onclick={sendRelation} disabled={!relationTarget}>Send request</button></div>
            {#each relations as relation (relation.id)}
              <div class="row"><span><strong>{relation.relation}: [{relation.tag}] {relation.name}</strong><small>{relation.status}</small></span><span class="spacer"></span>
                {#if relation.status === 'pending' && relation.other_guild_id === myGuild.id}<button class="sm primary" onclick={() => decideRelation(relation, true)}>Accept</button><button class="sm ghost" onclick={() => decideRelation(relation, false)}>Decline</button>{/if}
              </div>
            {/each}
          {/if}
        </section>
      {:else if activeTab === 'settings'}
        <div class="card col">
          <h3>Guild appearance</h3>
          <label>Description<textarea bind:value={guildEdit.description} maxlength="500"></textarea></label>
          <label>Message of the day<input bind:value={guildEdit.motd} maxlength="200" /></label>
          <div class="media-preview">
            <GuildBanner guild={{ id: myGuild.id, banner_url: guildEdit.banner_url }} height={6}>
              <div class="mp-emblem"><GuildEmblem guild={{ id: myGuild.id, tag: myGuild.tag, icon_url: guildEdit.icon_url }} size={4.2} /></div>
            </GuildBanner>
            <p class="tiny muted">Live preview — this is how your guild looks in the launcher. Leave a box empty to use the generated look in your guild colour.</p>
          </div>
          <label>Icon (square image address, https://… or /uploads/…)<input bind:value={guildEdit.icon_url} placeholder="https://example.com/emblem.png" /></label>
          <label>Banner (wide image address)<input bind:value={guildEdit.banner_url} placeholder="https://example.com/banner.jpg" /></label>
          <button class="primary" onclick={saveGuild}>Save guild details</button>
        </div>
        {#if myRole === 'leader'}
          <div class="card col">
            <h3><Pencil size={15} /> Rename guild</h3>
            <p class="tiny muted">The new name and tag show up everywhere straight away: the launcher, the map and in game.</p>
            <div class="row">
              <label class="grow">Name<input bind:value={renameName} maxlength="32" /></label>
              <label class="tag-field">Tag<input bind:value={renameTag} maxlength="6" class="tag-input" /></label>
            </div>
            <button class="primary" onclick={renameGuild} disabled={renaming || !renameName.trim() || (renameName.trim() === myGuild.name && renameTag.trim().toUpperCase() === myGuild.tag.toUpperCase())}>
              {#if renaming}<LoaderCircle size={14} class="spin" />{/if} Save name
            </button>
          </div>
          <div class="card col danger-zone">
            <h3><TriangleAlert size={15} /> Disband guild</h3>
            <p class="tiny muted">Removes the guild for good: every member is released, all {myGuild.claims_count} claimed chunks are freed and the roles, posts and invites are deleted. Whatever is in the treasury is paid to you.</p>
            <button class="danger" onclick={() => { disbandConfirm = ''; disbandOpen = true; }}><Trash2 size={14} /> Disband guild…</button>
          </div>
        {/if}
        {#if myRole === 'leader'}<div class="card col">
          <h3>Guild roles</h3>
          {#each roles as role}
            <div class="row"><strong>{role.name}</strong><span class="tiny muted">{[role.can_invite && 'Invite', role.can_kick && 'Kick', role.can_claim && 'Claim', role.can_post && 'Post', role.can_manage && 'Manage'].filter(Boolean).join(', ') || 'No extra permissions'}</span><span class="spacer"></span><button class="sm danger" onclick={() => deleteRole(role)}>Delete</button></div>
          {/each}
          <label>New role name<input bind:value={newRoleName} maxlength="24" /></label>
          <div class="row"><label><input type="checkbox" bind:checked={roleFlags.can_invite} /> Invite</label><label><input type="checkbox" bind:checked={roleFlags.can_kick} /> Kick</label><label><input type="checkbox" bind:checked={roleFlags.can_claim} /> Claim</label><label><input type="checkbox" bind:checked={roleFlags.can_post} /> Post</label><label><input type="checkbox" bind:checked={roleFlags.can_manage} /> Manage</label></div>
          <button class="primary" onclick={createRole}>Create role</button>
        </div>{/if}
      {:else if activeTab === 'feed'}
        <div class="feed-tab">
          <!-- Post Announcement Form -->
          <div class="post-creator glass">
            <span class="creator-title">Post Guild Announcement</span>
            <input
              type="text"
              placeholder="Announcement Title"
              bind:value={postTitle}
              class="post-title-input"
            />
            <textarea
              placeholder="Share news, raid plans, or updates with your guild..."
              bind:value={postContent}
              rows="3"
              class="post-textarea"
            ></textarea>
            <div class="post-creator-footer">
              <label class="pin-checkbox tiny">
                <input type="checkbox" bind:checked={postPinned} />
                <span>Pin to top</span>
              </label>
              <button
                class="primary sm"
                onclick={handleCreatePost}
                disabled={posting}
              >
                {#if posting}<LoaderCircle size={13} class="spin" />{:else}<Send size={13} />{/if}
                Publish
              </button>
            </div>
          </div>

          <!-- Posts Feed -->
          <div class="posts-list">
            {#if posts.length === 0}
              <div class="empty-state glass">
                <MessageSquare size={32} class="muted" />
                <p>No guild announcements posted yet.</p>
              </div>
            {:else}
              {#each posts as p (p.id)}
                <div class="post-card glass" class:pinned={p.pinned}>
                  <div class="post-header">
                    <div class="author-meta">
                      <Avatar uuid={p.author_uuid} name={p.author_name} size={1.8} />
                      <div class="author-info">
                        <span class="author-name"><PlayerLink uuid={p.author_uuid} name={p.author_name} /></span>
                        <span class="post-date tiny muted">{new Date(p.created_at).toLocaleString()}</span>
                      </div>
                    </div>
                    {#if p.pinned}
                      <span class="badge pinned-badge"><Pin size={11} /> Pinned</span>
                    {/if}
                  </div>
                  <h4 class="post-headline">{p.title}</h4>
                  <p class="post-text">{p.content}</p>
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <!-- User is NOT in a Guild on this instance -->
    <div class="no-guild-container">
      <div class="hero-prompt glass">
        <div class="prompt-icon-box">
          <Shield size={38} class="guild-icon" />
        </div>
        <div class="prompt-text">
          <h2>No Guild on {currentInstance?.name || 'this Instance'}</h2>
          <p class="lead">
            Guilds are tied to instances. Start your own guild to claim land chunks, invite members, and build a faction base, or browse existing factions below.
          </p>
          <button class="primary create-guild-btn" onclick={() => (createModalOpen = true)}>
            <Plus size={16} /> Create a Guild
          </button>
        </div>
      </div>

      {@render guildDirectory()}
    </div>
  {/if}
</div>

<!-- Disband Guild Modal -->
{#if myGuild}
<Modal bind:open={disbandOpen} title="Disband {myGuild.name}?" width={28}>
  <div class="modal-form">
    <p>This can't be undone. Type the guild's name to confirm.</p>
    <label>
      <span class="field-label">Type “{myGuild.name}”</span>
      <input type="text" bind:value={disbandConfirm} placeholder={myGuild.name} autocomplete="off" />
    </label>
    <div class="row end">
      <button class="ghost" onclick={() => (disbandOpen = false)}>Cancel</button>
      <button class="danger" onclick={disbandGuild} disabled={disbanding || disbandConfirm.trim().toLowerCase() !== myGuild.name.toLowerCase()}>
        {#if disbanding}<LoaderCircle size={14} class="spin" />{:else}<Trash2 size={14} />{/if} Disband forever
      </button>
    </div>
  </div>
</Modal>
{/if}

<!-- Remove member / hand over leadership -->
<Modal open={!!confirmKick} onclose={() => (confirmKick = null)} title="Remove {confirmKick?.name}?" width={26}>
  <div class="modal-form">
    <p>They will leave the guild straight away and get a notification. They can ask to join again later.</p>
    <div class="row end">
      <button class="ghost" onclick={() => (confirmKick = null)}>Cancel</button>
      <button class="danger" onclick={() => confirmKick && kick(confirmKick)}><UserMinus size={14} /> Remove</button>
    </div>
  </div>
</Modal>
<Modal open={!!confirmLeader} onclose={() => (confirmLeader = null)} title="Make {confirmLeader?.name} the leader?" width={26}>
  <div class="modal-form">
    <p>{confirmLeader?.name} becomes the guild leader and you become an officer. Only they can undo it.</p>
    <div class="row end">
      <button class="ghost" onclick={() => (confirmLeader = null)}>Cancel</button>
      <button class="danger" onclick={() => confirmLeader && makeLeader(confirmLeader)}><Crown size={14} /> Hand over</button>
    </div>
  </div>
</Modal>

<!-- Create Guild Modal -->
<Modal bind:open={createModalOpen} title="Found a New Guild" width={28}>
  <div class="modal-form">
    <label>
      <span class="field-label">Guild Name</span>
      <input type="text" placeholder="e.g. Iron Fortress" bind:value={newGuildName} maxlength="32" />
    </label>
    <label>
      <span class="field-label">Guild Tag (3 - 5 characters)</span>
      <input type="text" placeholder="e.g. IRON" bind:value={newGuildTag} maxlength="5" class="tag-input" />
      <span class="tiny muted">Appears in brackets on claimed chunks and chat tags.</span>
    </label>
    <label>
      <span class="field-label">Description / Moto</span>
      <textarea placeholder="Describe your guild's playstyle, goals, or rules..." bind:value={newGuildDesc} rows="3"></textarea>
    </label>
    <label>
      <span class="field-label">Banner Image URL (Optional)</span>
      <input type="url" placeholder="https://example.com/banner.png" bind:value={newGuildBanner} />
    </label>
    <label>
      <span class="field-label">Icon Image URL (Optional)</span>
      <input type="url" placeholder="https://example.com/guild-icon.png" bind:value={newGuildIcon} />
    </label>
  </div>
  {#snippet footer()}
    <button class="ghost" onclick={() => (createModalOpen = false)}>Cancel</button>
    <button class="primary" onclick={handleCreateGuild} disabled={creating}>
      {#if creating}<LoaderCircle size={14} class="spin" />{:else}<Sparkles size={14} />{/if}
      Found Guild
    </button>
  {/snippet}
</Modal>

<style>
  .invites { display: flex; flex-direction: column; gap: 0.6rem; padding: 0.9rem 1.1rem; border-radius: var(--radius); border: 1px solid color-mix(in srgb, var(--accent) 40%, var(--line)); }
  .invites h3 { margin: 0; font-size: 0.95rem; }
  .invite { display: flex; align-items: center; gap: 0.7rem; }
  .invite .text { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .guilds-page {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 1.6rem 2.2rem 2.5rem;
    gap: 1.2rem;
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    border-bottom: 1px solid var(--line);
    padding-bottom: 1rem;
    flex-wrap: wrap;
    gap: 1rem;
  }
  .title-wrap h1 {
    font-size: 1.5rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  :global(.guild-icon) {
    color: var(--accent);
  }
  .lead {
    color: var(--muted);
    font-size: 0.88rem;
    margin-top: 0.2rem;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .instance-select-wrap {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .inst-select {
    background: color-mix(in srgb, var(--bg) 70%, transparent);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 0.35rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
  }
  .guild-hub {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .guild-hero {
    display: block;
    border-radius: var(--radius);
    border: 1px solid var(--line);
  }
  .guild-title-line {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    flex-wrap: wrap;
  }
  .guild-title-line h2 {
    font-size: 1.4rem;
    font-weight: 750;
  }
  .guild-desc {
    font-size: 0.85rem;
    color: var(--muted);
    max-width: 42rem;
  }
  .guild-tabs-row {
    display: flex;
  }
  .tabs-segmented {
    display: flex;
    background: color-mix(in srgb, var(--surface) 80%, transparent);
    padding: 0.25rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    gap: 0.2rem;
  }
  .tabs-segmented button {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: none;
    padding: 0.45rem 1rem;
    font-size: 0.85rem;
    font-weight: 560;
    color: var(--muted);
    border-radius: calc(var(--radius-sm) - 2px);
    transition: all 0.15s;
  }
  .tabs-segmented button.active {
    background: color-mix(in srgb, var(--text) 10%, transparent);
    color: var(--text);
  }
  .territory-tab {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .protection-callout {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.75rem 1rem;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--success) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
  }
  :global(.callout-icon) {
    color: var(--success);
    flex-shrink: 0;
  }
  .callout-text p {
    margin: 0;
  }
  .member-actions { display: flex; gap: 0.4rem; margin-top: 0.35rem; flex-wrap: wrap; }
  .tiny-btn { font-size: 0.72rem; padding: 0.2rem 0.55rem; display: inline-flex; align-items: center; gap: 0.3rem; }
  .danger-text { color: var(--danger); }
  .members-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
    gap: 0.9rem;
  }
  .member-card {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.8rem 1rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: var(--surface);
    cursor: pointer;
    transition: transform 0.12s, border-color 0.15s;
  }
  .member-card:hover {
    transform: translateY(-2px);
    border-color: var(--line-strong);
  }
  .member-avatar-wrap {
    position: relative;
  }
  .online-indicator {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--success);
    border: 2px solid var(--surface);
  }
  .member-details {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    flex: 1;
    min-width: 0;
  }
  .member-name-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.3rem;
  }
  .member-name {
    font-weight: 650;
    font-size: 0.88rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .leader-badge {
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #f59e0b;
  }
  .officer-badge {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
  }
  .member-role-badge {
    background: color-mix(in srgb, var(--text) 8%, transparent);
    color: var(--muted);
  }
  .feed-tab {
    display: flex;
    flex-direction: column;
    gap: 1.2rem;
  }
  .post-creator {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1rem 1.2rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
  }
  .creator-title {
    font-size: 0.88rem;
    font-weight: 650;
  }
  .post-title-input {
    background: color-mix(in srgb, var(--bg) 60%, transparent);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 0.4rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.88rem;
  }
  .post-textarea {
    background: color-mix(in srgb, var(--bg) 60%, transparent);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 0.5rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    resize: vertical;
  }
  .post-creator-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .pin-checkbox {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--muted);
    cursor: pointer;
  }
  .posts-list {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .post-card {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1.1rem 1.3rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
  }
  .post-card.pinned {
    border-color: color-mix(in srgb, #f59e0b 40%, transparent);
    background: linear-gradient(135deg, color-mix(in srgb, #f59e0b 6%, var(--surface)) 0%, var(--surface) 100%);
  }
  .post-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .author-meta {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .author-info {
    display: flex;
    flex-direction: column;
  }
  .author-name {
    font-size: 0.84rem;
    font-weight: 650;
  }
  .pinned-badge {
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #f59e0b;
  }
  .post-headline {
    font-size: 1rem;
    font-weight: 700;
  }
  .post-text {
    font-size: 0.85rem;
    color: color-mix(in srgb, var(--text) 85%, transparent);
    line-height: 1.4;
    white-space: pre-wrap;
  }
  /* No Guild View */
  .no-guild-container {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  .hero-prompt {
    display: flex;
    align-items: center;
    gap: 1.8rem;
    padding: 2rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
  }
  .prompt-icon-box {
    width: 4.5rem;
    height: 4.5rem;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }
  .prompt-text {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .prompt-text h2 {
    font-size: 1.3rem;
    font-weight: 700;
  }
  .create-guild-btn {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.5rem;
  }
  .browse-section {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .view-toggle { display: inline-flex; align-self: flex-start; width: fit-content; gap: 0.25rem; padding: 0.25rem; margin-bottom: 0.9rem; border-radius: 999px; background: color-mix(in srgb, var(--bg) 60%, transparent); border: 1px solid var(--line); }
  .view-toggle button { display: inline-flex; align-items: center; gap: 0.4rem; border: 0; border-radius: 999px; padding: 0.45rem 0.95rem; background: transparent; color: var(--muted); font-weight: 600; font-size: 0.85rem; }
  .view-toggle button:hover:not(.active) { color: var(--text); }
  .view-toggle button.active { background: color-mix(in srgb, var(--accent) 24%, transparent); color: var(--text); }
  .guild-search { display: flex; align-items: center; gap: 0.4rem; padding: 0.3rem 0.7rem; border: 1px solid var(--line); border-radius: 999px; color: var(--muted); }
  .guild-search input { border: 0; background: transparent; padding: 0.15rem 0; min-width: 12rem; box-shadow: none; }
  .guild-dir-card.mine { border-color: color-mix(in srgb, var(--accent) 60%, transparent); }
  .danger-zone { border-color: color-mix(in srgb, #ef4444 45%, var(--line)); }
  .danger-zone h3, .card h3 { display: flex; align-items: center; gap: 0.45rem; }
  .row.end { justify-content: flex-end; }
  .card { padding: 1.1rem 1.25rem; margin-bottom: 0.9rem; border: 1px solid var(--line); border-radius: var(--radius); background: var(--panel); }
  .card.col { display: flex; flex-direction: column; gap: 0.7rem; }
  .card h3 { margin: 0; font-size: 1rem; }
  .card .row { display: flex; align-items: flex-end; gap: 0.7rem; flex-wrap: wrap; }
  .card .row .grow { flex: 1; min-width: 12rem; }
  .card label { display: flex; flex-direction: column; gap: 0.3rem; font-size: 0.82rem; color: var(--muted); }
  .card label input[type='checkbox'] { width: auto; }
  .card .row > label:has(input[type='checkbox']) { flex-direction: row; align-items: center; gap: 0.35rem; }
  .danger-zone { background: color-mix(in srgb, #ef4444 6%, var(--panel)); }
  .tag-field { width: 7rem; }
  .browse-header { display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap; margin-bottom: 0.8rem; }
  .browse-header h3 {
    font-size: 1.1rem;
    font-weight: 700;
  }
  .guilds-directory-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
    gap: 1rem;
  }
  .guild-dir-card {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1.1rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
  }
  .dir-meta h4 {
    font-size: 0.95rem;
    font-weight: 700;
  }
  .dir-desc {
    font-size: 0.82rem;
    color: var(--muted);
    line-height: 1.35;
  }
  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .modal-form label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .field-label {
    font-size: 0.82rem;
    font-weight: 600;
  }
  .tag-input {
    text-transform: uppercase;
    font-family: monospace;
    font-weight: 700;
    letter-spacing: 0.05em;
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
  .member-badge {
    background: color-mix(in srgb, var(--text) 8%, transparent);
  }
  .claim-badge {
    background: color-mix(in srgb, var(--success) 16%, transparent);
    color: var(--success);
  }
  .center-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.8rem;
    padding: 3rem 1.5rem;
    text-align: center;
    border-radius: var(--radius);
  }

  /* ---- guild art: hero, directory cards, settings preview ---- */
  .guild-hero { padding: 0; overflow: hidden; border-radius: var(--radius); }
  .hero-body { display: flex; gap: 1.2rem; align-items: flex-end; padding: 0 1.5rem 1.2rem; margin-top: -3.4rem; position: relative; }
  .hero-text { flex: 1; min-width: 0; padding-bottom: 0.2rem; }
  .guild-title-line { display: flex; align-items: center; gap: 0.7rem; flex-wrap: wrap; }
  .guild-title-line h2 { font-size: 1.9rem; line-height: 1.1; text-shadow: 0 2px 10px #0008; }
  .tag-chip { font-weight: 700; font-size: 0.85rem; padding: 0.15rem 0.6rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 12%, transparent); border: 1px solid var(--line-strong); }
  .hero-chips { display: flex; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.7rem; }
  .role-chip { background: color-mix(in srgb, var(--warn) 16%, transparent); color: var(--warn); border-color: color-mix(in srgb, var(--warn) 35%, transparent); }
  .motd { display: flex; align-items: center; gap: 0.6rem; margin: 0 1.5rem 1.2rem; padding: 0.55rem 0.85rem; border-radius: var(--radius-sm); font-size: 0.85rem; background: color-mix(in srgb, var(--accent) 10%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent); }
  .guilds-directory-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr)); gap: 1rem; }
  .guild-dir-card { padding: 0; overflow: hidden; display: flex; flex-direction: column; transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s; }
  .guild-dir-card:hover { transform: translateY(-3px); box-shadow: 0 1rem 2rem -1.2rem #000b; }
  .dir-body { display: flex; flex-direction: column; gap: 0.5rem; padding: 0 1rem 1rem; margin-top: -1.8rem; position: relative; flex: 1; }
  .dir-emblem { align-self: flex-start; }
  .dir-meta { display: flex; align-items: baseline; gap: 0.5rem; flex-wrap: wrap; }
  .dir-meta h4 { font-size: 1.05rem; }
  .dir-tagline { font-size: 0.78rem; color: var(--muted); font-weight: 650; }
  .dir-desc { font-size: 0.82rem; color: var(--muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; margin: 0; }
  .dir-stats { display: flex; gap: 1rem; font-size: 0.82rem; margin-top: auto; padding-top: 0.3rem; }
  .dir-stats span { display: inline-flex; align-items: center; gap: 0.35rem; }
  .dir-stats small { color: var(--muted); }
  .guild-dir-card button { margin-top: 0.4rem; }
  .ribbon { position: absolute; top: 0.6rem; right: 0.6rem; z-index: 1; font-size: 0.66rem; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; padding: 0.2rem 0.55rem; border-radius: 99rem; background: var(--accent); color: white; }
  .media-preview { border: 1px solid var(--line); border-radius: var(--radius-sm); overflow: hidden; background: var(--surface); }
  .media-preview p { padding: 0.5rem 0.8rem; margin: 0; }
  .mp-emblem { position: absolute; left: 1rem; bottom: 0.7rem; z-index: 1; }
</style>
