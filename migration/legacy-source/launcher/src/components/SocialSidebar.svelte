<script lang="ts">
  import { Users, X, MessageSquare, RefreshCw, Gamepad2, Search, ChevronRight, UserPlus } from '@lucide/svelte';
  import Avatar from './Avatar.svelte';
  import { activeAccount, app, instances, saveSettings } from '../lib/store.svelte';
  import { errorText, invoke } from '../lib/tauri';
  import type { FriendInfo, DirectMessage } from '../lib/types';

  let friends = $state<FriendInfo[]>([]);
  let selected = $state<FriendInfo | null>(null);
  let messages = $state<DirectMessage[]>([]);
  let search = $state('');
  let error = $state('');
  let previewError = $state('');
  let loading = $state(false);
  let previewLoading = $state(false);
  let previewRequest = 0;
  let refresh = $state<() => void>(() => {});

  const accepted = $derived(friends.filter((f) => f.status === 'accepted'));
  const pending = $derived(friends.filter((f) => f.status === 'pending_incoming').length);
  const matches = $derived(accepted.filter((f) => f.username.toLowerCase().includes(search.trim().toLowerCase())));
  const online = $derived(matches.filter((f) => f.online).sort((a, b) => a.username.localeCompare(b.username)));
  const offline = $derived(matches.filter((f) => !f.online).sort((a, b) => a.username.localeCompare(b.username)));
  const playingNow = $derived(
    app.running.length ? app.running.map((run) => instances().find((i) => i.id === run.instance_id)?.name ?? run.instance_id).join(', ') : ''
  );

  $effect(() => {
    const account = app.active;
    const panel = app.panelUrl;
    friends = []; selected = null; messages = []; previewRequest++;
    let alive = true;
    let busy = false;
    async function load() {
      if (busy || !account || !panel) return;
      busy = true; loading = true;
      try {
        const list = await invoke<FriendInfo[]>('get_friends');
        if (alive) {
          friends = list; error = '';
          if (selected && !list.some((f) => f.uuid === selected?.uuid && f.status === 'accepted')) { selected = null; messages = []; previewRequest++; }
        }
      } catch (e) { if (alive) error = errorText(e); }
      finally { busy = false; if (alive) loading = false; }
    }
    refresh = () => { void load(); };
    void load();
    const timer = setInterval(load, 20000);
    return () => { alive = false; clearInterval(timer); previewRequest++; };
  });

  async function peek(friend: FriendInfo) {
    if (selected?.uuid === friend.uuid) { selected = null; messages = []; previewRequest++; return; }
    selected = friend; messages = []; previewError = ''; previewLoading = true;
    const request = ++previewRequest;
    try {
      const list = await invoke<DirectMessage[]>('get_direct_messages', { friendUuid: friend.uuid, markRead: false });
      if (request === previewRequest) messages = list.slice().sort((a, b) => a.id - b.id).slice(-4);
    } catch (e) { if (request === previewRequest) previewError = errorText(e); }
    finally { if (request === previewRequest) previewLoading = false; }
  }
  function openChat() { if (selected) { app.chatWith = selected.uuid; app.view = 'social'; } }
  function hide() { if (app.settings) { app.settings.show_social_sidebar = false; saveSettings(); } }
</script>

{#snippet row(friend: FriendInfo)}
  <li>
    <button class="friend" class:selected={selected?.uuid === friend.uuid} class:away={!friend.online} onclick={() => peek(friend)} aria-expanded={selected?.uuid === friend.uuid}>
      <span class="av">
        <Avatar uuid={friend.uuid} name={friend.username} size={2} />
        <i class="dot" class:on={friend.online} aria-hidden="true"></i>
      </span>
      <span class="who">
        <strong>{friend.username}</strong>
        <small class:on={friend.online}>{friend.playing_on ? `Playing ${friend.playing_on}` : friend.online ? 'Online' : 'Offline'}</small>
      </span>
      {#if friend.unread}<b class="unread" aria-label="{friend.unread} unread">{friend.unread > 9 ? '9+' : friend.unread}</b>{:else}<ChevronRight size={14} class="chev {selected?.uuid === friend.uuid ? 'open' : ''}" />{/if}
    </button>
    {#if selected?.uuid === friend.uuid}
      <div class="preview" aria-label="Message preview">
        {#if previewLoading}<p class="muted">Loading messages…</p>
        {:else if previewError}<p class="error" role="alert">{previewError}</p>
        {:else}
          {#each messages as message (message.id)}
            {@const mine = message.sender_uuid === activeAccount()?.uuid}
            <div class="bubble" class:mine><p>{message.content}</p></div>
          {:else}<p class="muted">No messages yet — say hi!</p>{/each}
        {/if}
        <button class="primary sm" onclick={openChat}><MessageSquare size={13} /> Open conversation</button>
      </div>
    {/if}
  </li>
{/snippet}

<aside class="social-sidebar" aria-label="Friends and activity">
  <header>
    <strong><Users size={16} /> Friends</strong>
    <div class="actions">
      <button class="ghost icon sm" aria-label="Refresh friends" disabled={loading} onclick={refresh}><RefreshCw size={14} class={loading ? 'spin' : ''} /></button>
      <button class="ghost icon sm" aria-label="Hide social sidebar" onclick={hide}><X size={16} /></button>
    </div>
  </header>

  <div class="activity glass" class:live={!!playingNow}>
    <Gamepad2 size={16} />
    <span><small>Your activity</small><strong>{playingNow || 'Browsing the launcher'}</strong></span>
  </div>

  {#if pending}
    <button class="requests" onclick={() => (app.view = 'social')}><UserPlus size={14} /> {pending} friend request{pending === 1 ? '' : 's'} waiting</button>
  {/if}

  <label class="search"><Search size={14} /><input aria-label="Search friends" placeholder="Find a friend…" bind:value={search} /></label>
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="lists">
    {#if online.length}
      <h2><i class="dot on"></i> Online — {online.length}</h2>
      <ul>{#each online as f (f.uuid)}{@render row(f)}{/each}</ul>
    {/if}
    {#if offline.length}
      <h2><i class="dot"></i> Offline — {offline.length}</h2>
      <ul>{#each offline as f (f.uuid)}{@render row(f)}{/each}</ul>
    {/if}
    {#if !online.length && !offline.length}
      <div class="empty">
        <Users size={22} />
        <p>{loading ? 'Loading friends…' : search ? 'No matching friends.' : 'No friends yet.'}</p>
        {#if !loading && !search}<button class="primary sm" onclick={() => (app.view = 'social')}>Find friends</button>{/if}
      </div>
    {/if}
  </div>

  <button class="ghost sm open" onclick={() => (app.view = 'social')}>Open Friends &amp; Social <ChevronRight size={13} /></button>
</aside>

<style>
  .social-sidebar { width: 16.5rem; flex-shrink: 0; min-height: 0; overflow: hidden; padding: 0.9rem; border-left: 1px solid var(--line); background: color-mix(in srgb, var(--surface) 92%, transparent); backdrop-filter: blur(10px); display: flex; flex-direction: column; gap: 0.75rem; animation: slide 0.2s ease; }
  @keyframes slide { from { opacity: 0; transform: translateX(10px); } }
  header { display: flex; align-items: center; justify-content: space-between; }
  header strong { display: flex; align-items: center; gap: 0.5rem; font-size: 0.9rem; }
  .actions { display: flex; gap: 0.15rem; }
  .activity { display: flex; align-items: center; gap: 0.65rem; padding: 0.6rem 0.7rem; border-radius: var(--radius-sm); font-size: 0.78rem; color: var(--muted); }
  .activity span { display: flex; flex-direction: column; min-width: 0; }
  .activity small { font-size: 0.66rem; text-transform: uppercase; letter-spacing: 0.06em; opacity: 0.8; }
  .activity strong { color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .activity.live { border-color: color-mix(in srgb, var(--accent) 45%, transparent); background: color-mix(in srgb, var(--accent) 10%, transparent); color: var(--accent); }
  .requests { display: flex; align-items: center; gap: 0.5rem; width: 100%; font-size: 0.76rem; padding: 0.5rem 0.7rem; border-color: color-mix(in srgb, var(--warn) 45%, transparent); background: color-mix(in srgb, var(--warn) 12%, transparent); color: var(--warn); }
  .search { display: flex; align-items: center; gap: 0.5rem; padding: 0 0.65rem; border: 1px solid var(--line); border-radius: var(--radius-sm); background: var(--bg-2, transparent); color: var(--muted); }
  .search:focus-within { border-color: var(--accent); }
  .search input { border: 0; background: transparent; box-shadow: none; padding: 0.5rem 0; width: 100%; outline: none; }
  .lists { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.3rem; margin: 0 -0.3rem; padding: 0 0.3rem; }
  h2 { display: flex; align-items: center; gap: 0.4rem; font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.07em; color: var(--muted); margin: 0.5rem 0 0.2rem; font-weight: 650; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.15rem; }
  .friend { width: 100%; display: flex; align-items: center; gap: 0.6rem; padding: 0.4rem 0.5rem; text-align: left; background: transparent; border-color: transparent; border-radius: var(--radius-sm); }
  .friend:hover { background: color-mix(in srgb, var(--text) 7%, transparent); }
  .friend.selected { background: color-mix(in srgb, var(--accent) 14%, transparent); border-color: color-mix(in srgb, var(--accent) 35%, transparent); }
  .friend.away .av { opacity: 0.6; }
  .av { position: relative; display: inline-flex; }
  .dot { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: color-mix(in srgb, var(--muted) 70%, transparent); display: inline-block; }
  .dot.on { background: var(--success); box-shadow: 0 0 6px color-mix(in srgb, var(--success) 70%, transparent); }
  .av .dot { position: absolute; right: -0.1rem; bottom: -0.1rem; border: 2px solid var(--surface); box-sizing: content-box; }
  .who { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .who strong { font-size: 0.82rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who small { font-size: 0.7rem; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who small.on { color: var(--success); }
  .unread { min-width: 1.15rem; height: 1.15rem; padding: 0 0.3rem; border-radius: 99rem; background: var(--accent); color: white; font-size: 0.66rem; display: grid; place-items: center; }
  :global(.chev) { color: var(--muted); transition: transform 0.15s; }
  :global(.chev.open) { transform: rotate(90deg); }
  .preview { margin: 0.2rem 0 0.4rem 0.5rem; padding: 0.5rem; border-left: 2px solid color-mix(in srgb, var(--accent) 45%, transparent); display: flex; flex-direction: column; gap: 0.35rem; }
  .bubble { max-width: 92%; padding: 0.35rem 0.55rem; border-radius: 0.7rem 0.7rem 0.7rem 0.2rem; background: color-mix(in srgb, var(--text) 8%, transparent); font-size: 0.76rem; overflow-wrap: anywhere; }
  .bubble.mine { align-self: flex-end; border-radius: 0.7rem 0.7rem 0.2rem 0.7rem; background: color-mix(in srgb, var(--accent) 22%, transparent); }
  .bubble p { margin: 0; }
  .preview .primary { display: inline-flex; align-items: center; justify-content: center; gap: 0.4rem; margin-top: 0.2rem; }
  .muted { font-size: 0.76rem; color: var(--muted); margin: 0; }
  .error { color: var(--danger); font-size: 0.76rem; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; padding: 1.6rem 0.5rem; color: var(--muted); text-align: center; }
  .empty p { margin: 0; font-size: 0.8rem; }
  .open { display: inline-flex; align-items: center; justify-content: center; gap: 0.3rem; }
  :global(.spin) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 1150px) { .social-sidebar { position: absolute; right: 0; top: 2.75rem; bottom: 0; z-index: 25; width: min(19rem, 85vw); box-shadow: -8px 0 28px #0005; background: var(--surface); } }
</style>
