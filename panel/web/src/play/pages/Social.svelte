<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import {
    Users, UserPlus, MessageSquare, Gamepad2, Send, Check, X, ChevronLeft, Search, RefreshCw, UserRound, Clock, Inbox, Sparkles, LoaderCircle, Server
  } from '@lucide/svelte';
  import Avatar from '../../components/Avatar.svelte';
  import Sheet from '../ui/Sheet.svelte';
  import Empty from '../ui/Empty.svelte';
  import ProfileSheet from '../ui/ProfileSheet.svelte';
  import { del, get, post, timeAgo } from '../../lib/api';
  import { session } from '../../lib/session.svelte';
  import { toast, toastError } from '../../lib/toast.svelte';
  import { play } from '../store.svelte';

  type Friend = { uuid: string; username: string; status: 'pending_outgoing' | 'pending_incoming' | 'accepted'; online: boolean; playing_on: string | null; unread?: number };
  type Msg = { id: number; sender_uuid: string; sender_name?: string; recipient_uuid: string; content: string; created_at: string; is_read: boolean; pending?: boolean };
  type Invite = { id: string; sender_uuid: string; sender_name: string; instance_id: string; instance_name: string; server_name: string | null; status: string; created_at: string };
  type Member = { uuid: string; username: string; global_level: number; title: string | null; online: boolean; last_seen: string | null; friendship_status: string; is_friend: boolean };

  const me = $derived(session.user?.uuid ?? '');
  let tab = $state<'friends' | 'messages' | 'find' | 'invites'>('friends');
  let friends = $state<Friend[]>([]);
  let invites = $state<Invite[]>([]);
  let loading = $state(true);
  let error = $state('');
  let refreshing = $state(false);
  let filter = $state<'all' | 'online'>('all');
  let isPhone = $state(false);

  // profile sheet
  let profileOpen = $state(false);
  let profileUuid = $state<string | null>(null);
  const openProfile = (uuid: string) => { profileUuid = uuid; profileOpen = true; };

  const accepted = $derived(friends.filter((f) => f.status === 'accepted'));
  const incoming = $derived(friends.filter((f) => f.status === 'pending_incoming'));
  const outgoing = $derived(friends.filter((f) => f.status === 'pending_outgoing'));
  const shownFriends = $derived(filter === 'online' ? accepted.filter((f) => f.online) : accepted);
  const unreadTotal = $derived(accepted.reduce((n, f) => n + (f.unread ?? 0), 0));
  const convos = $derived([...accepted].sort((a, b) => (b.unread ?? 0) - (a.unread ?? 0) || Number(b.online) - Number(a.online) || a.username.localeCompare(b.username)));

  async function loadAll(quiet = false) {
    if (!quiet) refreshing = true;
    try {
      const [f, i] = await Promise.all([get<Friend[]>('/api/v1/friends'), get<Invite[]>('/api/v1/invites').catch(() => [] as Invite[])]);
      friends = f;
      invites = i;
      error = '';
    } catch (e) {
      if (!quiet || !friends.length) error = e instanceof Error ? e.message : 'Could not load your friends';
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  // ---------- friend actions ----------
  let addName = $state('');
  let adding = $state(false);
  let busyUuid = $state<string | null>(null);

  async function sendRequest(username: string, uuid?: string) {
    if (!username.trim()) return;
    if (uuid) busyUuid = uuid; else adding = true;
    try {
      await post('/api/v1/friends/request', { username: username.trim() });
      toast(`Friend request sent to ${username.trim()}`);
      if (!uuid) addName = '';
      if (uuid) sentTo = new Set([...sentTo, uuid]);
      await loadAll(true);
    } catch (e) { toastError(e); } finally { busyUuid = null; adding = false; }
  }

  async function answer(f: Friend, accept: boolean) {
    busyUuid = f.uuid;
    try {
      await post('/api/v1/friends/respond', { target_uuid: f.uuid, accept });
      toast(accept ? `You and ${f.username} are friends` : 'Request declined');
      await loadAll(true);
    } catch (e) { toastError(e); } finally { busyUuid = null; }
  }

  async function cancelRequest(f: Friend) {
    busyUuid = f.uuid;
    try {
      await del(`/api/v1/friends/${f.uuid}`);
      friends = friends.filter((x) => x.uuid !== f.uuid);
      toast('Request cancelled');
    } catch (e) { toastError(e); } finally { busyUuid = null; }
  }

  // ---------- find players ----------
  let query = $state('');
  let members = $state<Member[]>([]);
  let searching = $state(false);
  let searched = $state(false);
  let sentTo = $state(new Set<string>());
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let searchSeq = 0;

  async function search() {
    const mine = ++searchSeq;
    searching = true;
    try {
      const r = await get<Member[]>(`/api/v1/members/search?q=${encodeURIComponent(query.trim())}`);
      if (mine === searchSeq) { members = r; searched = true; }
    } catch (e) { if (mine === searchSeq) toastError(e); } finally { if (mine === searchSeq) searching = false; }
  }
  function onQuery() { clearTimeout(searchTimer); searchTimer = setTimeout(search, 280); }
  const relOf = (m: Member) => (sentTo.has(m.uuid) && m.friendship_status === 'none' ? 'pending_outgoing' : m.friendship_status);

  // ---------- conversation ----------
  let chat = $state<Friend | null>(null);
  let messages = $state<Msg[]>([]);
  let msgLoading = $state(false);
  let msgError = $state('');
  let draft = $state('');
  let sending = $state(false);
  let scroller = $state<HTMLDivElement | null>(null);
  let composer = $state<HTMLTextAreaElement | null>(null);
  let tempId = 0;
  let stick = true;
  let pollTimer: ReturnType<typeof setInterval> | undefined;
  let chatSeq = 0;

  const near = () => !scroller || scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 140;
  async function toBottom(smooth = false) {
    await tick();
    scroller?.scrollTo({ top: scroller.scrollHeight, behavior: smooth ? 'smooth' : 'auto' });
  }

  async function openChat(f: Friend) {
    chat = f; tab = 'messages'; messages = []; msgError = ''; msgLoading = true; stick = true;
    const mine = ++chatSeq;
    try {
      const m = await get<Msg[]>(`/api/v1/messages/${f.uuid}`);
      if (mine !== chatSeq) return;
      messages = m;
      friends = friends.map((x) => (x.uuid === f.uuid ? { ...x, unread: 0 } : x));
      await toBottom();
    } catch (e) { if (mine === chatSeq) msgError = e instanceof Error ? e.message : 'Could not load messages'; } finally { if (mine === chatSeq) msgLoading = false; }
  }
  function closeChat() { chat = null; chatSeq++; }

  async function pollChat() {
    if (!chat || msgLoading || document.hidden) return;
    const id = chat.uuid, mine = chatSeq;
    try {
      const fresh = await get<Msg[]>(`/api/v1/messages/${id}`);
      if (mine !== chatSeq || chat?.uuid !== id) return;
      const real = messages.filter((m) => !m.pending);
      const changed = fresh.length !== real.length || fresh.at(-1)?.id !== real.at(-1)?.id;
      if (!changed) return;
      const wasNear = near();
      messages = [...fresh, ...messages.filter((m) => m.pending)];
      if (wasNear) void toBottom(true);
    } catch { /* keep what we have */ }
  }

  async function send() {
    const text = draft.trim();
    if (!chat || !text || sending) return;
    const to = chat.uuid;
    const tmp: Msg = { id: --tempId, sender_uuid: me, recipient_uuid: to, content: text, created_at: new Date().toISOString(), is_read: false, pending: true };
    messages = [...messages, tmp];
    draft = '';
    sending = true;
    resize();
    void toBottom(true);
    try {
      const real = await post<Msg>(`/api/v1/messages/${to}`, { content: text });
      messages = messages.map((m) => (m.id === tmp.id ? real : m));
    } catch (e) {
      messages = messages.filter((m) => m.id !== tmp.id);
      if (!draft) draft = text;
      toastError(e);
    } finally { sending = false; composer?.focus(); }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !isPhone) { e.preventDefault(); void send(); }
  }
  function resize() {
    if (!composer) return;
    composer.style.height = 'auto';
    composer.style.height = Math.min(composer.scrollHeight, 120) + 'px';
  }

  // day separators and bubble grouping
  const dayKey = (iso: string) => new Date(iso).toDateString();
  const dayLabel = (iso: string) => {
    const d = new Date(iso), t = new Date(), y = new Date(Date.now() - 864e5);
    if (d.toDateString() === t.toDateString()) return 'Today';
    if (d.toDateString() === y.toDateString()) return 'Yesterday';
    return d.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' });
  };
  const clock = (iso: string) => new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  const rows = $derived(messages.map((m, i) => {
    const p = messages[i - 1], n = messages[i + 1];
    return {
      m,
      day: !p || dayKey(p.created_at) !== dayKey(m.created_at) ? dayLabel(m.created_at) : null,
      first: !p || p.sender_uuid !== m.sender_uuid || dayKey(p.created_at) !== dayKey(m.created_at),
      last: !n || n.sender_uuid !== m.sender_uuid || dayKey(n.created_at) !== dayKey(m.created_at),
    };
  }));

  // ---------- game invites ----------
  let inviteTarget = $state<Friend | null>(null);
  let inviteOpen = $state(false);
  let inviting = $state<number | null>(null);
  let answering = $state<string | null>(null);

  function askInvite(f: Friend) { inviteTarget = f; inviteOpen = true; }
  async function sendInvite(s: (typeof play.servers)[number]) {
    if (!inviteTarget) return;
    const instance = s.instance_id ?? play.manifest?.instances[0]?.id;
    if (!instance) { toast('That server is not linked to a modpack yet', 'error'); return; }
    inviting = s.id;
    try {
      await post('/api/v1/invites', { recipient_uuid: inviteTarget.uuid, instance_id: instance, server_id: s.id });
      toast(`Invite to ${s.name} sent to ${inviteTarget.username}`);
      inviteOpen = false;
    } catch (e) { toastError(e); } finally { inviting = null; }
  }
  async function respondInvite(inv: Invite, accept: boolean) {
    answering = inv.id;
    try {
      await post(`/api/v1/invites/${inv.id}/respond`, { action: accept ? 'accept' : 'decline' });
      invites = invites.filter((i) => i.id !== inv.id);
      toast(accept ? `Accepted. Open the launcher and join ${inv.server_name ?? inv.instance_name}!` : 'Invite declined', accept ? 'ok' : 'info');
    } catch (e) { toastError(e); loadAll(true); } finally { answering = null; }
  }

  // ---------- lifecycle ----------
  function consumeChatRequest(id?: string | null) {
    let target = id ?? null;
    if (!target) { try { target = sessionStorage.getItem('scopenet.play.chat'); } catch { /* ignore */ } }
    try { sessionStorage.removeItem('scopenet.play.chat'); } catch { /* ignore */ }
    if (!target) return;
    const f = friends.find((x) => x.uuid === target && x.status === 'accepted');
    if (f) void openChat(f);
  }

  onMount(() => {
    const mq = window.matchMedia('(max-width: 880px)');
    const setPhone = () => (isPhone = mq.matches);
    setPhone();
    mq.addEventListener('change', setPhone);
    const onChat = (e: Event) => consumeChatRequest((e as CustomEvent<string>).detail);
    window.addEventListener('play:chat', onChat);
    void loadAll().then(() => consumeChatRequest());
    const slow = setInterval(() => { if (!document.hidden) void loadAll(true); }, 20_000);
    pollTimer = setInterval(pollChat, 8_000);
    return () => {
      mq.removeEventListener('change', setPhone);
      window.removeEventListener('play:chat', onChat);
      clearInterval(slow); clearInterval(pollTimer); clearTimeout(searchTimer);
    };
  });

  $effect(() => { if (tab === 'find' && !searched) untrack(() => void search()); });
  // keep the open chat's friend row (online dot) fresh
  $effect(() => { const c = chat; if (c) { const f = friends.find((x) => x.uuid === c.uuid); if (f && (f.online !== c.online || f.playing_on !== c.playing_on)) untrack(() => (chat = f)); } });

  const threadMode = $derived(tab === 'messages' && !!chat && isPhone);
  const pendingCount = $derived(incoming.length);
</script>

<div class="pl-page soc" class:thread-mode={threadMode}>
  {#if !threadMode}
    <div class="pl-head">
      <div>
        <h1><Users size={26} /> Friends</h1>
        <p>Friends, messages and invites in one place.</p>
      </div>
      <div class="pl-actions">
        <button class="pl-btn sm" onclick={() => me && openProfile(me)}><UserRound size={15} /> My profile</button>
        <button class="pl-iconbtn" aria-label="Refresh" aria-busy={refreshing} onclick={() => loadAll()}><RefreshCw size={16} class={refreshing ? 'spin' : ''} /></button>
      </div>
    </div>

    <div class="pl-tabs tabs" role="tablist">
      <button role="tab" aria-selected={tab === 'friends'} class:on={tab === 'friends'} onclick={() => (tab = 'friends')}><Users size={15} /> Friends {#if pendingCount}<span class="count">{pendingCount}</span>{/if}</button>
      <button role="tab" aria-selected={tab === 'messages'} class:on={tab === 'messages'} onclick={() => (tab = 'messages')}><MessageSquare size={15} /> Messages {#if unreadTotal}<span class="count">{unreadTotal}</span>{/if}</button>
      <button role="tab" aria-selected={tab === 'find'} class:on={tab === 'find'} onclick={() => (tab = 'find')}><Search size={15} /> Find</button>
      <button role="tab" aria-selected={tab === 'invites'} class:on={tab === 'invites'} onclick={() => (tab = 'invites')}><Gamepad2 size={15} /> Invites {#if invites.length}<span class="count">{invites.length}</span>{/if}</button>
    </div>
  {/if}

  {#if error && !friends.length}
    <div class="pl-alert err stackgap">{error} <button class="pl-btn sm" onclick={() => loadAll()}>Retry</button></div>
  {:else if loading}
    <div class="pl-stack stackgap" aria-busy="true">
      {#each [0, 1, 2, 3, 4] as i}<div class="pl-skel" style="height:64px;border-radius:16px;animation-delay:{i * 80}ms"></div>{/each}
    </div>

  {:else if tab === 'friends'}
    <div class="pl-stack stackgap">
      <form class="pl-card tight addbar" onsubmit={(e) => { e.preventDefault(); sendRequest(addName); }}>
        <UserPlus size={18} class="muted-ic" />
        <input class="pl-input" placeholder="Add a friend by username" bind:value={addName} autocomplete="off" autocapitalize="off" spellcheck="false" aria-label="Username" />
        <button class="pl-btn primary" disabled={adding || !addName.trim()} aria-busy={adding}>{#if adding}<LoaderCircle size={15} class="spin" />{:else}Add{/if}</button>
      </form>

      {#if incoming.length}
        <section class="pl-card requests">
          <div class="pl-card-head"><h2><Inbox size={17} /> Friend requests <span class="pl-chip accent">{incoming.length}</span></h2></div>
          <div class="pl-list">
            {#each incoming as f, i (f.uuid)}
              <div class="pl-item rise" style:--i={i}>
                <button class="who" onclick={() => openProfile(f.uuid)}><Avatar name={f.username} uuid={f.uuid} size={44} />
                  <div class="grow"><b>{f.username}</b><span class="sub">wants to be friends</span></div></button>
                <button class="pl-btn primary sm" disabled={busyUuid === f.uuid} aria-busy={busyUuid === f.uuid} onclick={() => answer(f, true)}><Check size={15} /> Accept</button>
                <button class="pl-iconbtn" aria-label="Decline" disabled={busyUuid === f.uuid} onclick={() => answer(f, false)}><X size={16} /></button>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <div class="listhead">
        <div class="pl-tabs mini"><button class:on={filter === 'all'} onclick={() => (filter = 'all')}>All <span class="count">{accepted.length}</span></button>
          <button class:on={filter === 'online'} onclick={() => (filter = 'online')}>Online <span class="count">{accepted.filter((f) => f.online).length}</span></button></div>
      </div>

      {#if shownFriends.length === 0}
        <div class="pl-card">
          <Empty icon={Users} title={filter === 'online' ? 'Nobody is online right now' : 'No friends yet'} text={filter === 'online' ? 'Your friends will show up here when they log in.' : 'Add someone by username above, or find players to befriend.'}>
            {#if filter === 'all'}<button class="pl-btn primary" onclick={() => (tab = 'find')}><Search size={15} /> Find players</button>{/if}
          </Empty>
        </div>
      {:else}
        <div class="pl-grid friendgrid" style="--min: 300px">
          {#each shownFriends as f, i (f.uuid)}
            <div class="pl-card tight friend rise" style:--i={i}>
              <button class="who" onclick={() => openProfile(f.uuid)} aria-label="Open {f.username}'s profile">
                <span class="av"><Avatar name={f.username} uuid={f.uuid} size={46} /><i class="pl-dot" class:on={f.online}></i></span>
                <div class="grow">
                  <b>{f.username}</b>
                  {#if f.online && f.playing_on}<span class="sub on">Playing on {f.playing_on}</span>
                  {:else if f.online}<span class="sub on">Online</span>
                  {:else}<span class="sub">Offline</span>{/if}
                </div>
              </button>
              <button class="pl-iconbtn msg" aria-label="Message {f.username}" onclick={() => openChat(f)}><MessageSquare size={17} />{#if f.unread}<span class="badge">{f.unread}</span>{/if}</button>
              <button class="pl-iconbtn" aria-label="Invite {f.username} to play" onclick={() => askInvite(f)}><Gamepad2 size={17} /></button>
            </div>
          {/each}
        </div>
      {/if}

      {#if outgoing.length}
        <section class="pl-card">
          <div class="pl-card-head"><h2><Clock size={17} /> Waiting for an answer</h2></div>
          <div class="pl-list">
            {#each outgoing as f (f.uuid)}
              <div class="pl-item">
                <button class="who" onclick={() => openProfile(f.uuid)}><Avatar name={f.username} uuid={f.uuid} size={38} /><div class="grow"><b>{f.username}</b><span class="sub">Request sent</span></div></button>
                <button class="pl-btn sm" disabled={busyUuid === f.uuid} onclick={() => cancelRequest(f)}>Cancel</button>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </div>

  {:else if tab === 'find'}
    <div class="pl-stack stackgap">
      <div class="pl-search"><Search size={17} /><input class="pl-input" type="search" placeholder="Search players by name" bind:value={query} oninput={onQuery} autocomplete="off" aria-label="Search players" /></div>
      {#if searching && !members.length}
        <div class="pl-stack" aria-busy="true">{#each [0, 1, 2, 3] as i}<div class="pl-skel" style="height:66px;border-radius:16px;animation-delay:{i * 80}ms"></div>{/each}</div>
      {:else if members.length === 0}
        <div class="pl-card"><Empty icon={Search} title="No players found" text={query ? `Nobody matches “${query}”.` : 'No players to show yet.'} /></div>
      {:else}
        <div class="pl-grid" style="--min: 300px">
          {#each members as m, i (m.uuid)}
            {@const rel = relOf(m)}
            <div class="pl-card tight friend rise" style:--i={Math.min(i, 12)}>
              <button class="who" onclick={() => openProfile(m.uuid)}>
                <span class="av"><Avatar name={m.username} uuid={m.uuid} size={46} /><i class="pl-dot" class:on={m.online}></i></span>
                <div class="grow">
                  <b>{m.username}</b>
                  <span class="sub"><span class="lvl">Lv {m.global_level}</span>{#if m.title} · {m.title}{/if} · {m.online ? 'Online' : m.last_seen ? timeAgo(m.last_seen) : 'Offline'}</span>
                </div>
              </button>
              {#if rel === 'accepted'}
                <button class="pl-btn sm" onclick={() => { const f = friends.find((x) => x.uuid === m.uuid); if (f) openChat(f); }}><MessageSquare size={14} /> Message</button>
              {:else if rel === 'pending_outgoing'}
                <span class="pl-chip">Requested</span>
              {:else if rel === 'pending_incoming'}
                <button class="pl-btn primary sm" disabled={busyUuid === m.uuid} onclick={() => { const f = friends.find((x) => x.uuid === m.uuid); if (f) answer(f, true); }}><Check size={14} /> Accept</button>
              {:else}
                <button class="pl-btn primary sm" disabled={busyUuid === m.uuid} aria-busy={busyUuid === m.uuid} onclick={() => sendRequest(m.username, m.uuid)}>{#if busyUuid === m.uuid}<LoaderCircle size={14} class="spin" />{:else}<UserPlus size={14} />{/if} Add</button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

  {:else if tab === 'invites'}
    <div class="pl-stack stackgap">
      {#if invites.length === 0}
        <div class="pl-card"><Empty icon={Gamepad2} title="No game invites" text="When a friend invites you to play, it lands here. You can invite friends from the Friends tab.">
          <button class="pl-btn" onclick={() => (tab = 'friends')}>Invite a friend</button></Empty></div>
      {:else}
        {#each invites as inv, i (inv.id)}
          <div class="pl-card invite rise" style:--i={i}>
            <div class="pl-row-flex">
              <Avatar name={inv.sender_name} uuid={inv.sender_uuid} size={48} />
              <div class="grow">
                <b>{inv.sender_name}</b> invited you to play
                <div class="sub"><Server size={13} /> {inv.server_name ?? inv.instance_name} · {timeAgo(inv.created_at)}</div>
              </div>
            </div>
            <div class="pl-row-flex act">
              <button class="pl-btn primary grow" disabled={answering === inv.id} aria-busy={answering === inv.id} onclick={() => respondInvite(inv, true)}><Check size={15} /> Accept</button>
              <button class="pl-btn" disabled={answering === inv.id} onclick={() => respondInvite(inv, false)}>Decline</button>
            </div>
          </div>
        {/each}
        <p class="hint">Invites expire after 30 minutes. After accepting, open the launcher to jump in.</p>
      {/if}
    </div>

  {:else}
    <!-- Messages -->
    <div class="messenger stackgap" class:has-chat={!!chat}>
      <aside class="convos pl-card flush" aria-label="Conversations">
        {#if convos.length === 0}
          <Empty icon={MessageSquare} title="No conversations" text="Messages are for friends. Add someone first.">
            <button class="pl-btn primary" onclick={() => (tab = 'find')}>Find players</button></Empty>
        {:else}
          <div class="clist">
            {#each convos as f (f.uuid)}
              <button class="conv" class:on={chat?.uuid === f.uuid} onclick={() => openChat(f)}>
                <span class="av"><Avatar name={f.username} uuid={f.uuid} size={44} /><i class="pl-dot" class:on={f.online}></i></span>
                <span class="grow"><b>{f.username}</b><span class="sub">{f.online ? (f.playing_on ? `Playing on ${f.playing_on}` : 'Online') : 'Offline'}</span></span>
                {#if f.unread}<span class="badge static">{f.unread}</span>{/if}
              </button>
            {/each}
          </div>
        {/if}
      </aside>

      <section class="thread pl-card flush" aria-label="Conversation">
        {#if !chat}
          <div class="blank"><Empty icon={MessageSquare} title="Pick a conversation" text="Choose a friend on the left to start chatting." /></div>
        {:else}
          <header class="thead">
            <button class="pl-iconbtn back" aria-label="Back to conversations" onclick={closeChat}><ChevronLeft size={20} /></button>
            <button class="who" onclick={() => openProfile(chat!.uuid)}>
              <span class="av"><Avatar name={chat.username} uuid={chat.uuid} size={40} /><i class="pl-dot" class:on={chat.online}></i></span>
              <span class="grow"><b>{chat.username}</b><span class="sub" class:on={chat.online}>{chat.online ? (chat.playing_on ? `Playing on ${chat.playing_on}` : 'Online now') : 'Offline'}</span></span>
            </button>
            <button class="pl-iconbtn" aria-label="Invite to play" onclick={() => askInvite(chat!)}><Gamepad2 size={18} /></button>
          </header>

          <div class="msgs" bind:this={scroller} onscroll={() => (stick = near())}>
            {#if msgLoading}
              <div class="pl-stack" aria-busy="true">{#each [0, 1, 2] as i}<div class="pl-skel" style="height:40px;width:{60 - i * 12}%;align-self:{i % 2 ? 'flex-end' : 'flex-start'};border-radius:18px"></div>{/each}</div>
            {:else if msgError}
              <div class="pl-alert err">{msgError} <button class="pl-btn sm" onclick={() => chat && openChat(chat)}>Retry</button></div>
            {:else if messages.length === 0}
              <Empty icon={Sparkles} title="Say hello" text={`This is the start of your chat with ${chat.username}.`} />
            {:else}
              {#each rows as r (r.m.id)}
                {#if r.day}<div class="day"><span>{r.day}</span></div>{/if}
                <div class="row" class:mine={r.m.sender_uuid === me} class:first={r.first} class:last={r.last}>
                  <div class="bubble" class:pending={r.m.pending}>
                    <span class="txt">{r.m.content}</span>
                    {#if r.last}<time>{clock(r.m.created_at)}{#if r.m.pending} · sending{/if}</time>{/if}
                  </div>
                </div>
              {/each}
            {/if}
          </div>

          <form class="composer" onsubmit={(e) => { e.preventDefault(); void send(); }}>
            <textarea bind:this={composer} bind:value={draft} rows="1" maxlength="1000" placeholder="Message {chat.username}" aria-label="Message" oninput={resize} onkeydown={onKey}></textarea>
            <button class="send" aria-label="Send" disabled={!draft.trim()} aria-busy={sending}><Send size={18} /></button>
          </form>
        {/if}
      </section>
    </div>
  {/if}
</div>

<Sheet bind:open={inviteOpen} title={inviteTarget ? `Invite ${inviteTarget.username}` : 'Invite'}>
  <p class="hint" style="margin:0">Pick the server you are playing on. They get a notification and can accept from the panel or the launcher.</p>
  {#if play.servers.length === 0}
    <Empty icon={Server} title="No servers available" />
  {:else}
    <div class="pl-list">
      {#each play.servers as s (s.id)}
        <button class="pl-item press srv" disabled={inviting !== null} onclick={() => sendInvite(s)}>
          <span class="pl-dot" class:on={s.online}></span>
          <div class="grow"><b>{s.name}</b><span class="sub">{s.online ? `${s.players_online}/${s.players_max} online` : 'Offline'}{#if s.mc_version} · {s.mc_version}{/if}</span></div>
          {#if inviting === s.id}<LoaderCircle size={16} class="spin" />{:else}<Send size={16} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</Sheet>

<ProfileSheet bind:open={profileOpen} uuid={profileUuid} onchange={() => loadAll(true)} />

<style>
  .soc { --gap: 14px; }
  .stackgap { margin-top: 14px; }
  .tabs { width: 100%; }
  .tabs button { flex: 1; justify-content: center; }
  .mini { width: fit-content; }
  :global(.muted-ic) { color: var(--muted); flex-shrink: 0; }
  .addbar { display: flex; align-items: center; gap: 10px; }
  .addbar input { flex: 1; min-width: 0; }
  .who { display: flex; align-items: center; gap: 12px; min-width: 0; flex: 1; text-align: left; background: none; border: none; padding: 0; color: inherit; min-height: 0 !important; }
  .grow { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .grow b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { font-size: 0.8rem; color: var(--muted); display: flex; align-items: center; gap: 5px; }
  .sub.on { color: var(--good); }
  .lvl { color: var(--accent-2); font-weight: 600; }
  .av { position: relative; display: inline-flex; flex-shrink: 0; }
  .av :global(img), .av :global(.fallback) { border-radius: 14px; }
  .av .pl-dot { position: absolute; right: -2px; bottom: -2px; width: 12px; height: 12px; border: 2px solid var(--surface); }
  .friend { display: flex; align-items: center; gap: 10px; }
  .friend.rise, .rise { animation: rise 0.4s var(--ease, ease) both; animation-delay: calc(var(--i, 0) * 40ms); }
  @keyframes rise { from { opacity: 0; transform: translateY(10px); } }
  .msg { position: relative; }
  .badge { position: absolute; top: -5px; right: -5px; min-width: 18px; height: 18px; padding: 0 5px; border-radius: 99px; background: var(--accent); color: #fff; font-size: 0.68rem; font-weight: 700; display: grid; place-items: center; }
  .badge.static { position: static; flex-shrink: 0; }
  .requests { border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
  .invite { display: flex; flex-direction: column; gap: 14px; }
  .invite .sub { margin-top: 3px; }
  .act { gap: 8px; }
  .hint { color: var(--muted); font-size: 0.84rem; text-align: center; }
  .srv { width: 100%; background: none; border-left: none; border-right: none; border-top: none; text-align: left; color: inherit; }

  /* ---- messenger ---- */
  .messenger { display: grid; grid-template-columns: 300px minmax(0, 1fr); gap: 14px; height: min(720px, calc(100dvh - 300px)); min-height: 460px; }
  .convos { overflow-y: auto; }
  .clist { display: flex; flex-direction: column; padding: 6px; }
  .conv { display: flex; align-items: center; gap: 12px; padding: 10px; border-radius: 14px; background: none; border: none; text-align: left; color: inherit; width: 100%; transition: background 0.15s; }
  .conv:hover { background: rgba(255, 255, 255, 0.05); }
  .conv.on { background: color-mix(in srgb, var(--accent) 20%, transparent); }
  .thread { display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
  .blank { flex: 1; display: grid; place-items: center; }
  .thead { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-bottom: 1px solid var(--pl-line); background: color-mix(in srgb, var(--surface-2) 60%, transparent); }
  .back { display: none; }
  .msgs { flex: 1; min-height: 0; overflow-y: auto; padding: 14px 14px 8px; display: flex; flex-direction: column; overscroll-behavior: contain; scroll-behavior: smooth; }
  .day { display: flex; justify-content: center; margin: 12px 0 8px; }
  .day span { font-size: 0.7rem; color: var(--muted); background: var(--surface-2); padding: 3px 12px; border-radius: 99px; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 600; }
  .row { display: flex; margin-top: 2px; animation: pop 0.22s var(--ease, ease); }
  .row.first { margin-top: 10px; }
  .row.mine { justify-content: flex-end; }
  @keyframes pop { from { opacity: 0; transform: translateY(6px) scale(0.97); } }
  .bubble { max-width: min(78%, 460px); padding: 8px 12px; border-radius: 18px; background: var(--surface-3); color: var(--text); line-height: 1.38; overflow-wrap: anywhere; white-space: pre-wrap; display: flex; flex-direction: column; gap: 2px; }
  .row:not(.mine) .bubble { border-bottom-left-radius: 6px; }
  .row:not(.mine):not(.first) .bubble { border-top-left-radius: 6px; }
  .row.mine .bubble { background: linear-gradient(160deg, var(--accent), color-mix(in srgb, var(--accent) 70%, var(--accent-2))); color: #fff; border-bottom-right-radius: 6px; }
  .row.mine:not(.first) .bubble { border-top-right-radius: 6px; }
  .bubble.pending { opacity: 0.65; }
  .bubble time { font-size: 0.66rem; opacity: 0.65; align-self: flex-end; }
  .composer { display: flex; align-items: flex-end; gap: 8px; padding: 10px 12px; border-top: 1px solid var(--pl-line); background: color-mix(in srgb, var(--bg-2) 70%, transparent); }
  .composer textarea { flex: 1; resize: none; border-radius: 20px; padding: 10px 16px; max-height: 120px; line-height: 1.35; min-height: 42px; }
  .send { width: 44px; height: 44px; padding: 0; border-radius: 50%; display: grid; place-items: center; background: var(--accent); border-color: transparent; color: #fff; flex-shrink: 0; transition: transform 0.15s, opacity 0.15s; }
  .send:disabled { opacity: 0.4; }
  .send:not(:disabled):active { transform: scale(0.92); }

  @media (max-width: 880px) {
    .messenger { grid-template-columns: minmax(0, 1fr); height: auto; min-height: 0; }
    .messenger .convos { max-height: none; }
    .messenger.has-chat .convos { display: none; }
    .messenger:not(.has-chat) .thread { display: none; }
    .back { display: inline-grid; }
    .thread-mode { padding-top: 8px; }
    .thread-mode .messenger { margin-top: 0; height: calc(100dvh - 148px - env(safe-area-inset-top) - env(safe-area-inset-bottom) + 8px); margin-bottom: -44px; }
    .thread-mode .thread { height: 100%; }
    .tabs button { padding: 8px 6px; font-size: 0.82rem; gap: 5px; }
  }
  @media (prefers-reduced-motion: reduce) { .rise, .row { animation: none; } .msgs { scroll-behavior: auto; } }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
