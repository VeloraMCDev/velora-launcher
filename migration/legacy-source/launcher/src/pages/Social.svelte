<script lang="ts">
  import PlayerLink from '../components/PlayerLink.svelte';
  import { onMount } from 'svelte';
  import {
    Users, UserPlus, MessageSquare, Gamepad2, Send, Check, X, Trash2,
    Clock, ExternalLink, LoaderCircle, RefreshCw, Circle, UserX, Sparkles,
    Search, Trophy, Shield
  } from '@lucide/svelte';
  import Avatar from '../components/Avatar.svelte';
  import Modal from '../components/Modal.svelte';
  import { activeAccount, app, instances, toast } from '../lib/store.svelte';
  import { invoke } from '../lib/tauri';
  import type { FriendInfo, DirectMessage, GameInvite, MemberProfile } from '../lib/types';

  let activeSection = $state<'friends' | 'members' | 'messages' | 'invites'>('friends');
  let friendsFilter = $state<'all' | 'online' | 'pending'>('all');
  let loading = $state(false);

  let friends = $state<FriendInfo[]>([]);
  let invites = $state<GameInvite[]>([]);
  let activeChatFriend = $state<FriendInfo | null>(null);
  let messages = $state<DirectMessage[]>([]);
  // The sidebar can request another conversation while this page is already mounted.
  $effect(() => {
    const friend = friends.find((f) => f.uuid === app.chatWith);
    if (app.chatWith && friend) {
      app.chatWith = null;
      void selectChatFriend(friend);
    }
  });
  let messageInput = $state('');
  let sendingMessage = $state(false);

  // Add Friend Modal / Input
  let addFriendUsername = $state('');
  let sendingFriendReq = $state(false);

  // Members Directory / Search
  let memberSearchQuery = $state('');
  let membersList = $state<MemberProfile[]>([]);
  let loadingMembers = $state(false);
  let addingFriendUuid = $state<string | null>(null);

  // Send Game Invite Modal
  let inviteModalOpen = $state(false);
  let removeTarget = $state<FriendInfo | null>(null);
  let removeModalOpen = $state(false);
  const askRemove = (friend: FriendInfo) => { removeTarget = friend; removeModalOpen = true; };
  let inviteTargetFriend = $state<FriendInfo | null>(null);
  let inviteInstanceId = $state(instances()[0]?.id ?? '');
  let sendingInvite = $state(false);

  async function loadData() {
    loading = true;
    try {
      const [fList, iList] = await Promise.all([
        invoke<FriendInfo[]>('get_friends').catch(() => []),
        invoke<GameInvite[]>('get_game_invites').catch(() => [])
      ]);
      friends = fList;
      invites = iList;
      if (app.chatWith) {
        const target = fList.find((f) => f.uuid === app.chatWith);
        app.chatWith = null;
        if (target) void selectChatFriend(target);
      }
    } catch (e: any) {
      toast(e?.message ?? 'Failed to load social data', 'error');
    } finally {
      loading = false;
    }
  }

  async function searchMembers() {
    loadingMembers = true;
    try {
      membersList = await invoke<MemberProfile[]>('search_members', { query: memberSearchQuery.trim() });
    } catch (e: any) {
      toast(typeof e === 'string' ? e : e?.message ?? 'Failed to search members', 'error');
    } finally {
      loadingMembers = false;
    }
  }

  async function handleAddMemberAsFriend(username: string, uuid: string) {
    addingFriendUuid = uuid;
    try {
      await invoke('send_friend_request', { username, friendUsername: username });
      toast(`Friend request sent to ${username}!`, 'ok');
      await searchMembers();
      await loadData();
    } catch (e: any) {
      toast(e?.message ?? 'Failed to send friend request', 'error');
    } finally {
      addingFriendUuid = null;
    }
  }

  async function handleSendFriendRequest() {
    if (!addFriendUsername.trim()) return;
    sendingFriendReq = true;
    const target = addFriendUsername.trim();
    try {
      await invoke('send_friend_request', { username: target, friendUsername: target });
      toast(`Friend request sent to ${target}!`, 'ok');
      addFriendUsername = '';
      await loadData();
      if (activeSection === 'members') {
        await searchMembers();
      }
    } catch (e: any) {
      toast(e?.message ?? 'Failed to send friend request', 'error');
    } finally {
      sendingFriendReq = false;
    }
  }

  async function handleRespondRequest(friendUuid: string, accept: boolean) {
    try {
      await invoke('respond_friend_request', { targetUuid: friendUuid, friendUuid, accept });
      toast(accept ? 'Friend request accepted!' : 'Request declined.', 'ok');
      await loadData();
    } catch (e: any) {
      toast(e?.message ?? 'Action failed', 'error');
    }
  }

  async function handleRemoveFriend(friendUuid: string) {
    try {
      await invoke('remove_friend', { targetUuid: friendUuid, friendUuid });
      friends = friends.filter((f) => f.uuid !== friendUuid);
      if (activeChatFriend?.uuid === friendUuid) {
        activeChatFriend = null;
        messages = [];
      }
      toast('Friend removed', 'ok');
    } catch (e: any) {
      toast(e?.message ?? 'Failed to remove friend', 'error');
    }
  }

  async function selectChatFriend(friend: FriendInfo) {
    activeChatFriend = friend;
    activeSection = 'messages';
    try {
      messages = await invoke<DirectMessage[]>('get_direct_messages', {
        friendUuid: friend.uuid,
        limit: 50
      });
    } catch (e: any) {
      toast(e?.message ?? 'Failed to load messages', 'error');
    }
  }

  async function handleSendMessage() {
    if (!activeChatFriend || !messageInput.trim()) return;
    sendingMessage = true;
    const text = messageInput.trim();
    try {
      const msg = await invoke<DirectMessage>('send_direct_message', {
        friendUuid: activeChatFriend.uuid,
        content: text
      });
      messages = [...messages, msg];
      messageInput = '';
    } catch (e: any) {
      toast(e?.message ?? 'Failed to send message', 'error');
    } finally {
      sendingMessage = false;
    }
  }

  function openInviteModal(friend: FriendInfo) {
    inviteTargetFriend = friend;
    inviteModalOpen = true;
  }

  async function handleSendInvite() {
    if (!inviteTargetFriend) return;
    sendingInvite = true;
    try {
      // FIX #14: The Rust command takes recipient_uuid, instance_id, server_id (optional i64).
      // serverAddress/serverName were wrong fields that the command doesn't accept.
      await invoke('send_game_invite', {
        recipientUuid: inviteTargetFriend.uuid,
        instanceId: inviteInstanceId,
        serverId: null  // Optional; pass a server_id i64 here if the instance has a known server.
      });
      toast(`Game invite sent to ${inviteTargetFriend.username}!`, 'ok');
      inviteModalOpen = false;
    } catch (e: any) {
      toast(e?.message ?? 'Failed to send game invite', 'error');
    } finally {
      sendingInvite = false;
    }
  }

  async function handleRespondInvite(inviteId: string, accept: boolean) {
    try {
      await invoke('respond_game_invite', { inviteId, accept });
      invites = invites.map((inv) =>
        inv.id === inviteId ? { ...inv, status: accept ? 'accepted' : 'declined' } : inv
      );
      toast(accept ? 'Game invite accepted!' : 'Invite declined.', 'ok');
    } catch (e: any) {
      toast(e?.message ?? 'Failed to respond to invite', 'error');
    }
  }

  const acceptedFriends = $derived(friends.filter((f) => f.status === 'accepted'));
  const pendingReceived = $derived(friends.filter((f) => f.status === 'pending_incoming'));
  const pendingSent = $derived(friends.filter((f) => f.status === 'pending_outgoing'));

  const displayedFriends = $derived(
    friends.filter((f) => {
      if (friendsFilter === 'online') return f.status === 'accepted' && f.online;
      if (friendsFilter === 'pending') return f.status !== 'accepted';
      return f.status === 'accepted';
    })
  );

  onMount(() => {
    loadData();
  });
</script>

<div class="social-page">
  <!-- Page Header -->
  <div class="page-header">
    <div class="title-wrap">
      <h1><Users class="users-icon" size={26} /> Friends & Social</h1>
      <p class="lead">Connect with friends, send direct messages, and team up with game invites.</p>
    </div>
    <div class="header-actions">
      <button class="ghost icon" onclick={loadData} title="Refresh" aria-label="Refresh">
        <RefreshCw size={17} class={loading ? 'spin' : ''} />
      </button>
    </div>
  </div>

  <!-- Social Navigation Tabs -->
  <div class="tabs-row">
    <div class="tabs-segmented">
      <button class:active={activeSection === 'friends'} onclick={() => (activeSection = 'friends')}>
        <Users size={15} /> Friends ({acceptedFriends.length})
      </button>
      <button class:active={activeSection === 'members'} onclick={() => { activeSection = 'members'; if (!membersList.length) searchMembers(); }}>
        <Sparkles size={15} /> Find Players
      </button>
      <button class:active={activeSection === 'messages'} onclick={() => (activeSection = 'messages')}>
        <MessageSquare size={15} /> Direct Messages
      </button>
      <button class:active={activeSection === 'invites'} onclick={() => (activeSection = 'invites')}>
        <Gamepad2 size={15} /> Invites ({invites.filter((i) => i.status === 'pending').length})
      </button>
    </div>
  </div>

  <!-- Content Sections -->
  {#if activeSection === 'friends'}
    <div class="friends-layout">
      <!-- Add Friend Bar -->
      <div class="add-friend-bar glass">
        <UserPlus size={18} class="muted" />
        <input
          type="text"
          placeholder="Enter a player's username to add them..."
          bind:value={addFriendUsername}
          onkeydown={(e) => e.key === 'Enter' && handleSendFriendRequest()}
          class="add-friend-input"
        />
        <button
          class="primary sm"
          onclick={handleSendFriendRequest}
          disabled={sendingFriendReq || !addFriendUsername.trim()}
        >
          {#if sendingFriendReq}<LoaderCircle size={14} class="spin" />{:else}Send Request{/if}
        </button>
      </div>

      <!-- Filter Sub-tabs -->
      <div class="filter-pills">
        <button class="pill" class:active={friendsFilter === 'all'} onclick={() => (friendsFilter = 'all')}>
          All ({acceptedFriends.length})
        </button>
        <button class="pill" class:active={friendsFilter === 'online'} onclick={() => (friendsFilter = 'online')}>
          Online ({acceptedFriends.filter((f) => f.online).length})
        </button>
        <button class="pill" class:active={friendsFilter === 'pending'} onclick={() => (friendsFilter = 'pending')}>
          Pending ({pendingReceived.length + pendingSent.length})
        </button>
      </div>

      <!-- Pending Incoming Requests Alert -->
      {#if pendingReceived.length > 0 && friendsFilter !== 'pending'}
        <div class="pending-alert glass">
          <span class="tiny bold">You have {pendingReceived.length} pending friend request(s)</span>
          <button class="ghost sm" onclick={() => (friendsFilter = 'pending')}>Review</button>
        </div>
      {/if}

      <!-- Friends List -->
      {#if friendsFilter === 'pending'}
        <div class="pending-lists">
          {#if pendingReceived.length > 0}
            <h4 class="subheading">Incoming Requests ({pendingReceived.length})</h4>
            <div class="friends-grid">
              {#each pendingReceived as req (req.uuid)}
                <div class="friend-card glass">
                  <div class="card-left" role="button" tabindex="0" onclick={() => (app.viewProfileUuid = req.uuid)} onkeydown={(e) => e.key === 'Enter' && (app.viewProfileUuid = req.uuid)}>
                    <Avatar uuid={req.uuid} name={req.username} size={2.5} />
                    <div class="name-block">
                      <span class="user-title"><PlayerLink uuid={req.uuid} name={req.username} /></span>
                      <span class="tiny muted">Wants to be friends</span>
                    </div>
                  </div>
                  <div class="req-actions">
                    <button class="primary icon sm" onclick={() => handleRespondRequest(req.uuid, true)} title="Accept">
                      <Check size={14} />
                    </button>
                    <button class="ghost icon sm" onclick={() => handleRespondRequest(req.uuid, false)} title="Decline">
                      <X size={14} />
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          {#if pendingSent.length > 0}
            <h4 class="subheading" style:margin-top="1rem">Outgoing Requests ({pendingSent.length})</h4>
            <div class="friends-grid">
              {#each pendingSent as req (req.uuid)}
                <div class="friend-card glass">
                  <div class="card-left">
                    <Avatar uuid={req.uuid} name={req.username} size={2.5} />
                    <div class="name-block">
                      <span class="user-title"><PlayerLink uuid={req.uuid} name={req.username} /></span>
                      <span class="tiny muted">Request pending...</span>
                    </div>
                  </div>
                  <button class="ghost icon sm" onclick={() => handleRemoveFriend(req.uuid)} title="Cancel Request">
                    <Trash2 size={14} />
                  </button>
                </div>
              {/each}
            </div>
          {/if}

          {#if pendingReceived.length === 0 && pendingSent.length === 0}
            <div class="empty-state glass">
              <p class="muted">No pending friend requests.</p>
            </div>
          {/if}
        </div>
      {:else if displayedFriends.length === 0}
        <div class="empty-state glass">
          <Users size={32} class="muted" />
          <p class="muted">No friends found in this view. Add friends by typing their username above!</p>
        </div>
      {:else}
        <div class="friends-grid">
          {#each displayedFriends as friend (friend.uuid)}
            <div class="friend-card glass">
              <div
                class="card-left"
                role="button"
                tabindex="0"
                onclick={() => (app.viewProfileUuid = friend.uuid)}
                onkeydown={(e) => e.key === 'Enter' && (app.viewProfileUuid = friend.uuid)}
                title="View Profile"
              >
                <div class="avatar-holder">
                  <Avatar uuid={friend.uuid} name={friend.username} size={2.6} />
                  <span class="status-dot" class:online={friend.online}></span>
                </div>
                <div class="name-block">
                  <span class="user-title"><PlayerLink uuid={friend.uuid} name={friend.username} /></span>
                  {#if friend.online && friend.playing_on}
                    <span class="tiny playing-on">Playing on {friend.playing_on}</span>
                  {:else if friend.online}
                    <span class="tiny online-label">Online in Launcher</span>
                  {:else}
                    <span class="tiny muted">Offline</span>
                  {/if}
                </div>
              </div>

              <div class="friend-actions">
                <button class="ghost icon sm" onclick={() => selectChatFriend(friend)} title="Message">
                  <MessageSquare size={15} />
                </button>
                <button class="ghost icon sm" onclick={() => openInviteModal(friend)} title="Invite to Game">
                  <Gamepad2 size={15} />
                </button>
                <button class="ghost icon sm danger-hover" onclick={() => askRemove(friend)} title="Remove Friend">
                  <UserX size={15} />
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else if activeSection === 'members'}
    <!-- Members Directory Section -->
    <div class="members-layout">
      <div class="members-search-bar glass">
        <Search size={18} class="muted" />
        <input
          type="text"
          placeholder="Search all SCOPENET network players by username..."
          bind:value={memberSearchQuery}
          onkeydown={(e) => e.key === 'Enter' && searchMembers()}
          class="members-search-input"
        />
        <button
          class="primary sm"
          onclick={searchMembers}
          disabled={loadingMembers}
        >
          {#if loadingMembers}<LoaderCircle size={14} class="spin" />{:else}<Search size={14} /> Search{/if}
        </button>
      </div>

      {#if loadingMembers && !membersList.length}
        <div class="center-state"><LoaderCircle class="spin" size={32} /></div>
      {:else if membersList.length === 0}
        <div class="empty-state glass">
          <Users size={36} class="muted" />
          <p class="muted">No players found matching "{memberSearchQuery}". Try another username!</p>
        </div>
      {:else}
        <div class="members-grid">
          {#each membersList as member (member.uuid)}
            {@const isMe = member.uuid === activeAccount()?.uuid}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div class="member-card glass clickable" class:is-me={isMe} role="button" tabindex="0" title="Open {member.username}'s profile"
              onclick={(e) => { if (!(e.target as HTMLElement).closest('button')) app.viewProfileUuid = member.uuid; }}
              onkeydown={(e) => e.key === 'Enter' && e.target === e.currentTarget && (app.viewProfileUuid = member.uuid)}>
              <div class="member-card-top">
                <div class="avatar-wrap" role="button" tabindex="0" onclick={() => (app.viewProfileUuid = member.uuid)} onkeydown={(e) => e.key === 'Enter' && (app.viewProfileUuid = member.uuid)}>
                  <Avatar uuid={member.uuid} name={member.username} size={2.8} />
                  <span class="status-dot" class:online={member.online}></span>
                </div>
                <div class="member-meta">
                  <div class="name-row">
                    <button type="button" class="member-name-btn" onclick={() => (app.viewProfileUuid = member.uuid)}>{member.username}</button>
                    {#if isMe}
                      <span class="badge me-tag">You</span>
                    {/if}
                  </div>
                  <div class="member-sub-info">
                    <span class="badge level-pill">Lvl {member.global_level}</span>
                    {#if member.title}
                      <span class="badge title-pill">{member.title}</span>
                    {/if}
                  </div>
                  <span class="online-status tiny muted">
                    {member.online ? 'Online on Network' : member.last_seen ? `Active ${new Date(member.last_seen).toLocaleDateString()}` : 'Offline'}
                  </span>
                </div>
              </div>

              <div class="member-card-actions">
                {#if isMe}
                  <button class="ghost sm w-full" onclick={() => (app.viewProfileUuid = member.uuid)}>
                    View My Profile
                  </button>
                {:else if member.is_friend}
                  <button class="secondary sm" onclick={() => {
                    const friend = friends.find(f => f.uuid === member.uuid) || { uuid: member.uuid, username: member.username, status: 'accepted', is_online: member.online, playing_on: null };
                    selectChatFriend(friend as FriendInfo);
                  }}>
                    <MessageSquare size={13} /> Message
                  </button>
                  <button class="ghost sm icon" onclick={() => (app.viewProfileUuid = member.uuid)} title="View Profile">
                    <ExternalLink size={13} />
                  </button>
                {:else}
                  <button
                    class="primary sm"
                    onclick={() => handleAddMemberAsFriend(member.username, member.uuid)}
                    disabled={addingFriendUuid === member.uuid}
                  >
                    {#if addingFriendUuid === member.uuid}
                      <LoaderCircle size={13} class="spin" />
                    {:else}
                      <UserPlus size={13} /> Add Friend
                    {/if}
                  </button>
                  <button class="ghost sm icon" onclick={() => (app.viewProfileUuid = member.uuid)} title="View Profile">
                    <ExternalLink size={13} />
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else if activeSection === 'messages'}
    <!-- Direct Messages Split View -->
    <div class="messages-split glass">
      <!-- Conversation Sidebar -->
      <div class="conv-sidebar">
        <span class="conv-head tiny uppercase muted">Conversations</span>
        <div class="conv-list">
          {#each acceptedFriends as friend (friend.uuid)}
            <button
              class="conv-item"
              class:active={activeChatFriend?.uuid === friend.uuid}
              onclick={() => selectChatFriend(friend)}
            >
              <div class="avatar-holder">
                <Avatar uuid={friend.uuid} name={friend.username} size={2} />
                <span class="status-dot" class:online={friend.online}></span>
              </div>
              <div class="conv-meta">
                <span class="conv-name"><PlayerLink uuid={friend.uuid} name={friend.username} /></span>
                {#if friend.playing_on}
                  <span class="tiny muted">{friend.playing_on}</span>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      </div>

      <!-- Chat Thread Main -->
      <div class="chat-thread">
        {#if activeChatFriend}
          <div class="thread-header">
            <div
              class="thread-user"
              role="button"
              tabindex="0"
              onclick={() => (app.viewProfileUuid = activeChatFriend!.uuid)}
              onkeydown={(e) => e.key === 'Enter' && (app.viewProfileUuid = activeChatFriend!.uuid)}
            >
              <Avatar uuid={activeChatFriend.uuid} name={activeChatFriend.username} size={2.2} />
              <div>
                <strong><PlayerLink uuid={activeChatFriend.uuid} name={activeChatFriend.username} /></strong>
                <span class="tiny muted block">{activeChatFriend.online ? 'Active now' : 'Offline'}</span>
              </div>
            </div>
            <button class="primary sm" onclick={() => openInviteModal(activeChatFriend!)}>
              <Gamepad2 size={13} /> Invite
            </button>
          </div>

          <div class="messages-scroll">
            {#if messages.length === 0}
              <div class="empty-state">
                <MessageSquare size={28} class="muted" />
                <p class="tiny muted">No messages yet with {activeChatFriend.username}. Say hello!</p>
              </div>
            {:else}
              {#each messages as msg (msg.id)}
                {@const isMine = msg.sender_uuid === activeAccount()?.uuid}
                <div class="bubble-row" class:is-mine={isMine}>
                  <div class="bubble" class:is-mine={isMine}>
                    <p class="bubble-text">{msg.content}</p>
                    <span class="bubble-time tiny">{new Date(msg.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                  </div>
                </div>
              {/each}
            {/if}
          </div>

          <div class="message-composer">
            <input
              type="text"
              placeholder="Type a message..."
              bind:value={messageInput}
              onkeydown={(e) => e.key === 'Enter' && handleSendMessage()}
              class="composer-input"
            />
            <button
              class="primary icon"
              onclick={handleSendMessage}
              disabled={sendingMessage || !messageInput.trim()}
              aria-label="Send Message"
            >
              {#if sendingMessage}<LoaderCircle size={15} class="spin" />{:else}<Send size={15} />{/if}
            </button>
          </div>
        {:else}
          <div class="center-state">
            <MessageSquare size={36} class="muted" />
            <p class="muted">Select a friend to start chatting</p>
          </div>
        {/if}
      </div>
    </div>
  {:else if activeSection === 'invites'}
    <!-- Game Invites View -->
    <div class="invites-view">
      {#if invites.length === 0}
        <div class="empty-state glass">
          <Gamepad2 size={36} class="muted" />
          <p class="muted">No game invites received. When friends invite you to join their server, they will appear here!</p>
        </div>
      {:else}
        <div class="invites-grid">
          {#each invites as inv (inv.id)}
            <div class="invite-card glass">
              <div class="invite-top">
                <Avatar uuid={inv.sender_uuid} name={inv.sender_name} size={2.5} />
                <div class="invite-meta">
                  <span class="inviter-name"><strong><PlayerLink uuid={inv.sender_uuid} name={inv.sender_name} /></strong> invited you to play</span>
                  <span class="server-title">{inv.server_name}</span>
                  <span class="tiny muted server-addr">{inv.instance_name}</span>
                </div>
              </div>
              <div class="invite-footer">
                <span class="tiny muted">{new Date(inv.created_at).toLocaleDateString()}</span>
                {#if inv.status === 'pending'}
                  <div class="invite-btns">
                    <button class="primary sm" onclick={() => handleRespondInvite(inv.id, true)}>
                      <Check size={13} /> Accept
                    </button>
                    <button class="ghost sm" onclick={() => handleRespondInvite(inv.id, false)}>
                      Decline
                    </button>
                  </div>
                {:else}
                  <span class="badge status-badge {inv.status}">{inv.status}</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Send Game Invite Modal -->
<Modal bind:open={removeModalOpen} title="Remove {removeTarget?.username}?" width={24}>
  <p class="muted">{removeTarget?.username} will be removed from your friends. You can send a new request later.</p>
  {#snippet footer()}
    <button class="ghost" onclick={() => (removeModalOpen = false)}>Keep friend</button>
    <button class="danger" onclick={() => { const f = removeTarget; removeModalOpen = false; if (f) handleRemoveFriend(f.uuid); }}>
      <UserX size={14} /> Remove
    </button>
  {/snippet}
</Modal>

<Modal bind:open={inviteModalOpen} title="Invite {inviteTargetFriend?.username} to Play" width={24}>
  <div class="invite-modal-body">
    <p class="tiny muted">
      Choose which instance / server you want to invite {inviteTargetFriend?.username} to join:
    </p>
    <label>
      <span class="field-label">Instance</span>
      <select bind:value={inviteInstanceId} class="modal-select">
        {#each instances() as inst}
          <option value={inst.id}>{inst.name}</option>
        {/each}
      </select>
    </label>
  </div>
  {#snippet footer()}
    <button class="ghost" onclick={() => (inviteModalOpen = false)}>Cancel</button>
    <button class="primary" onclick={handleSendInvite} disabled={sendingInvite}>
      {#if sendingInvite}<LoaderCircle size={14} class="spin" />{:else}<Gamepad2 size={14} />{/if}
      Send Game Invite
    </button>
  {/snippet}
</Modal>

<style>
  .social-page {
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
  }
  .title-wrap h1 {
    font-size: 1.5rem;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  :global(.users-icon) {
    color: var(--accent);
  }
  .lead {
    color: var(--muted);
    font-size: 0.88rem;
    margin-top: 0.2rem;
  }
  .tabs-row {
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
  .friends-layout {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .add-friend-bar {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.6rem 1rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
  }
  .add-friend-input {
    flex: 1;
    background: transparent;
    border: none;
    font-size: 0.88rem;
    color: var(--text);
    outline: none;
  }
  .filter-pills {
    display: flex;
    gap: 0.4rem;
  }
  .pill {
    padding: 0.3rem 0.75rem;
    font-size: 0.8rem;
    font-weight: 560;
    border-radius: 99rem;
    background: color-mix(in srgb, var(--text) 6%, transparent);
    border: 1px solid transparent;
    color: var(--muted);
    transition: all 0.15s;
  }
  .pill.active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
    color: var(--text);
  }
  .pending-alert {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.6rem 1rem;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .friends-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
    gap: 0.9rem;
  }
  .friend-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.9rem 1.1rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
    transition: border-color 0.15s, transform 0.12s;
  }
  .friend-card:hover {
    border-color: var(--line-strong);
    transform: translateY(-1px);
  }
  .card-left {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    cursor: pointer;
    flex: 1;
    min-width: 0;
  }
  .avatar-holder {
    position: relative;
  }
  .status-dot {
    position: absolute;
    bottom: -1px;
    right: -1px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--muted);
    border: 2px solid var(--surface);
  }
  .status-dot.online {
    background: var(--success);
  }
  .name-block {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .user-title {
    font-weight: 650;
    font-size: 0.92rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .playing-on {
    color: var(--accent);
    font-weight: 560;
  }
  .online-label {
    color: var(--success);
  }
  .friend-actions, .req-actions {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .danger-hover:hover {
    color: var(--danger);
  }
  .subheading {
    font-size: 0.92rem;
    font-weight: 700;
    margin-bottom: 0.6rem;
  }
  /* Direct Messages Split */
  .messages-split {
    display: flex;
    height: 32rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
    overflow: hidden;
  }
  .conv-sidebar {
    width: 15rem;
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--surface) 90%, black);
  }
  .conv-head {
    padding: 0.8rem 1rem 0.5rem;
    letter-spacing: 0.06em;
    font-weight: 700;
  }
  .conv-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 0.3rem 0.5rem;
    gap: 0.2rem;
  }
  .conv-item {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.55rem 0.7rem;
    border-radius: var(--radius-sm);
    background: transparent;
    border: none;
    text-align: left;
    transition: background 0.12s;
  }
  .conv-item:hover {
    background: color-mix(in srgb, var(--text) 6%, transparent);
  }
  .conv-item.active {
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .conv-meta {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .conv-name {
    font-size: 0.86rem;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chat-thread {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .thread-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.8rem 1.2rem;
    border-bottom: 1px solid var(--line);
  }
  .thread-user {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
  }
  .block {
    display: block;
  }
  .messages-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 1.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .bubble-row {
    display: flex;
    justify-content: flex-start;
  }
  .bubble-row.is-mine {
    justify-content: flex-end;
  }
  .bubble {
    max-width: 22rem;
    padding: 0.6rem 0.9rem;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--surface) 90%, var(--text));
    border: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .bubble.is-mine {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .bubble-text {
    font-size: 0.88rem;
    margin: 0;
    line-height: 1.35;
    word-break: break-word;
  }
  .bubble-time {
    align-self: flex-end;
    color: var(--muted);
    font-size: 0.68rem;
  }
  .message-composer {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.75rem 1.2rem;
    border-top: 1px solid var(--line);
  }
  .composer-input {
    flex: 1;
    background: color-mix(in srgb, var(--bg) 60%, transparent);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 0.5rem 0.85rem;
    border-radius: var(--radius-sm);
    font-size: 0.88rem;
  }
  /* Invites View */
  .invites-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
    gap: 1rem;
  }
  .invite-card {
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    padding: 1.2rem;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
  }
  .invite-top {
    display: flex;
    align-items: center;
    gap: 0.9rem;
  }
  .invite-meta {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .server-title {
    font-size: 1rem;
    font-weight: 700;
  }
  .invite-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid color-mix(in srgb, var(--line) 40%, transparent);
    padding-top: 0.7rem;
  }
  .invite-btns {
    display: flex;
    gap: 0.4rem;
  }
  .status-badge {
    text-transform: capitalize;
  }
  .status-badge.accepted {
    background: color-mix(in srgb, var(--success) 20%, transparent);
    color: var(--success);
  }
  .status-badge.declined {
    background: color-mix(in srgb, var(--danger) 20%, transparent);
    color: var(--danger);
  }
  .invite-modal-body {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }
  .modal-select {
    width: 100%;
    background: color-mix(in srgb, var(--bg) 60%, transparent);
    border: 1px solid var(--line);
    color: var(--text);
    padding: 0.45rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.88rem;
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
    padding: 3rem 1.5rem;
    text-align: center;
    border-radius: var(--radius);
  }
  .members-layout {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .members-search-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 1rem;
    border-radius: var(--radius);
  }
  .members-search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text);
    font-size: 0.95rem;
  }
  .members-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }
  .member-card.clickable { cursor: pointer; }
  .member-card.clickable:hover { border-color: var(--line-strong); }
  .member-card {
    padding: 1.1rem;
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
    transition: transform 0.15s, border-color 0.15s;
  }
  .member-card:hover {
    transform: translateY(-2px);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }
  .member-card-top {
    display: flex;
    align-items: center;
    gap: 0.85rem;
  }
  .avatar-wrap {
    position: relative;
    cursor: pointer;
  }
  .member-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .name-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .member-name {
    font-size: 1rem;
    font-weight: 700;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .member-name:hover {
    color: var(--accent);
  }
  .member-sub-info {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }
  .level-pill {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
    font-weight: 700;
  }
  .title-pill {
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #f59e0b;
  }
  .me-tag {
    background: color-mix(in srgb, var(--line) 40%, transparent);
    color: var(--muted);
  }
  .member-card-actions {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    border-top: 1px solid color-mix(in srgb, var(--line) 40%, transparent);
    padding-top: 0.75rem;
  }
  .w-full {
    width: 100%;
  }
  .member-name-btn {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    font-weight: 700;
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }
  .member-name-btn:hover {
    color: var(--accent);
  }
</style>
