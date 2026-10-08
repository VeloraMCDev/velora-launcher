<script lang="ts">
  import { onMount } from 'svelte';
  import { X, Send, UserPlus, Sparkles, MapPin, LoaderCircle, Shield, Clock, Swords, Star } from '@lucide/svelte';
  import Avatar from './Avatar.svelte';
  import PlayerLink from './PlayerLink.svelte';
  import { activeAccount, toast } from '../lib/store.svelte';
  import { invoke } from '../lib/tauri';
  import type { Guild, GuildMember, UserProfileView } from '../lib/types';

  let { serverId, instanceId, uuid, nameHint, onclose }: { serverId: number; instanceId: string; uuid: string; nameHint: string; onclose: () => void } = $props();

  let profile = $state<UserProfileView | null>(null);
  let loading = $state(true);
  let failed = $state('');
  let message = $state('');
  let busy = $state('');
  let myGuild = $state<Guild | null>(null);
  let iCanInvite = $state(false);

  const me = $derived(activeAccount());
  // Acting on someone needs a panel account; offline accounts can still look.
  const signedIn = $derived(me?.kind === 'panel');
  const isMe = $derived(me?.uuid?.toLowerCase() === uuid.toLowerCase());

  async function load() {
    loading = true;
    failed = '';
    try {
      profile = await invoke<UserProfileView>('get_user_profile', { uuid });
    } catch (e: any) {
      failed = typeof e === 'string' ? e : e?.message ?? 'Could not load this player';
    } finally {
      loading = false;
    }
    if (signedIn) {
      try {
        myGuild = await invoke<Guild | null>('get_my_guild', { instanceId });
        if (myGuild) {
          const members = await invoke<GuildMember[]>('get_guild_members', { guildId: myGuild.id });
          const mine = members.find((m) => m.uuid.toLowerCase() === me?.uuid?.toLowerCase());
          iCanInvite = mine?.role === 'leader' || mine?.role === 'officer';
        }
      } catch { /* invites just stay off */ }
    }
  }

  // A new click on the map loads the new player.
  $effect(() => { void uuid; message = ''; load(); });
  onMount(() => { const t = setInterval(() => { if (!busy) load(); }, 20000); return () => clearInterval(t); });

  async function run(kind: string, job: () => Promise<unknown>, done: string) {
    busy = kind;
    try { await job(); toast(done, 'ok'); }
    catch (e: any) { toast(typeof e === 'string' ? e : e?.message ?? 'That did not work', 'error'); }
    finally { busy = ''; }
  }

  const tpa = () => run('tpa', () => invoke('map_action', { serverId, action: 'tpa', targetUuid: uuid, text: null }), 'Teleport request sent');
  const invite = () => run('invite', () => invoke('guild_send_invite', { guildId: myGuild!.id, targetUuid: uuid }), `Invitation sent to ${profile?.username ?? nameHint}`);
  async function send() {
    const text = message.trim();
    if (!text) return;
    await run('message', () => invoke('map_action', { serverId, action: 'message', targetUuid: uuid, text }), 'Message sent');
    message = '';
  }

  const hours = (s: number) => (s >= 3600 ? `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m` : `${Math.floor(s / 60)}m`);
  const name = $derived(profile?.username ?? nameHint ?? 'Player');
  const online = $derived(!!profile?.online);
  const kd = $derived.by(() => {
    const k = Number(profile?.stats?.player_kills ?? 0), d = Number(profile?.stats?.deaths ?? 0);
    return d === 0 ? (k ? `${k}.0` : '–') : (k / d).toFixed(2);
  });
  const canAct = $derived(signedIn && !isMe && online);
</script>

<aside class="panel glass" style:--pa={profile?.accent_color || 'var(--accent)'} aria-label="Player">
  <button class="ghost icon close" onclick={onclose} aria-label="Close"><X size={16} /></button>

  {#if loading && !profile}
    <div class="center"><LoaderCircle class="spin" size={24} /></div>
  {:else if failed && !profile}
    <p class="muted pad">{failed}</p>
  {:else if profile}
    <div class="head">
      <div class="ring"><Avatar {uuid} name={name} size={4} /></div>
      <div class="who">
        <h3><PlayerLink {uuid} {name} /></h3>
        {#if profile.level_info?.title}<span class="title"><Sparkles size={12} /> {profile.level_info.title}</span>{/if}
        <span class="presence"><i class:on={online}></i>{online ? 'Online now' : 'Offline'}</span>
      </div>
    </div>

    <div class="chips">
      <span class="chip lv"><Star size={12} /> Level {profile.level_info?.level ?? 1}</span>
      {#if profile.guild}<span class="chip guild"><Shield size={12} /> [{profile.guild.tag}] {profile.guild.name}</span>{/if}
      {#if profile.rank}<span class="chip rank">{profile.rank.display}</span>{/if}
    </div>

    <div class="facts">
      <div><Clock size={13} /><span>Playtime</span><strong>{hours(profile.stats?.playtime_secs ?? 0)}</strong></div>
      <div><Swords size={13} /><span>K/D</span><strong>{kd}</strong></div>
      <div><MapPin size={13} /><span>Favourite</span><strong>{profile.favorite_server ?? '–'}</strong></div>
      <div><Star size={13} /><span>Awards</span><strong>{profile.achievements_count ?? 0}</strong></div>
    </div>
    {#if profile.bio}<p class="bio">{profile.bio}</p>{/if}

    <div class="actions">
      {#if isMe}
        <p class="muted small">This is you.</p>
      {:else if !signedIn}
        <p class="muted small">Sign in with a SCOPENET account to send requests and messages.</p>
      {:else if !online}
        <p class="muted small">{name} isn't online on this server, so they can't be reached from the map.</p>
      {/if}

      <button class="primary" disabled={!canAct || !!busy} onclick={tpa}>
        {#if busy === 'tpa'}<LoaderCircle class="spin" size={14} />{:else}<MapPin size={14} />{/if} Send teleport request
      </button>

      <div class="msg">
        <input bind:value={message} placeholder="Message in game…" maxlength="200" disabled={!canAct} onkeydown={(e) => e.key === 'Enter' && send()} />
        <button disabled={!canAct || !message.trim() || !!busy} onclick={send} aria-label="Send message">
          {#if busy === 'message'}<LoaderCircle class="spin" size={14} />{:else}<Send size={14} />{/if}
        </button>
      </div>

      <button disabled={!signedIn || isMe || !iCanInvite || !!busy} onclick={invite}
        title={!myGuild ? 'You are not in a guild here' : !iCanInvite ? 'Only guild leaders and officers can invite' : ''}>
        {#if busy === 'invite'}<LoaderCircle class="spin" size={14} />{:else}<UserPlus size={14} />{/if}
        Invite to {myGuild ? `[${myGuild.tag}]` : 'my guild'}
      </button>
    </div>
  {/if}
</aside>

<style>
  .panel { position: absolute; top: 0.8rem; right: 0.8rem; bottom: 0.8rem; width: min(20rem, calc(100% - 1.6rem)); display: flex; flex-direction: column; gap: 0.9rem; padding: 1rem; overflow-y: auto; background: var(--surface); border: 1px solid var(--line-strong); border-radius: var(--radius); box-shadow: 0 1rem 3rem -1rem rgba(0, 0, 0, 0.7); animation: slide 0.18s ease; }
  @keyframes slide { from { transform: translateX(1.5rem); opacity: 0; } }
  .close { position: absolute; top: 0.5rem; right: 0.5rem; }
  .center { flex: 1; display: grid; place-items: center; }
  .pad { padding: 1rem; }
  .head { display: flex; gap: 0.8rem; align-items: center; padding-right: 1.6rem; }
  .ring { border: 3px solid var(--pa); border-radius: 28%; overflow: hidden; flex-shrink: 0; }
  .who { display: flex; flex-direction: column; gap: 0.15rem; min-width: 0; }
  .who h3 { margin: 0; font-size: 1.1rem; overflow: hidden; text-overflow: ellipsis; }
  .title { display: inline-flex; align-items: center; gap: 0.3rem; color: #fbbf24; font-size: 0.78rem; }
  .presence { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.75rem; color: var(--muted); }
  .presence i { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: color-mix(in srgb, var(--muted) 60%, transparent); }
  .presence i.on { background: #34d399; box-shadow: 0 0 0 3px color-mix(in srgb, #34d399 25%, transparent); }
  .chips { display: flex; flex-wrap: wrap; gap: 0.35rem; }
  .chip { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; font-weight: 650; padding: 0.18rem 0.55rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 8%, transparent); }
  .chip.lv { color: var(--pa); }
  .chip.guild { color: #22d3ee; }
  .chip.rank { color: #fbbf24; }
  .facts { display: grid; grid-template-columns: 1fr 1fr; gap: 0.5rem; }
  .facts div { display: grid; grid-template-columns: auto 1fr; gap: 0.1rem 0.4rem; align-items: center; padding: 0.5rem 0.6rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 5%, transparent); font-size: 0.72rem; color: var(--muted); }
  .facts strong { grid-column: 1 / -1; color: var(--text); font-size: 0.9rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bio { margin: 0; font-size: 0.82rem; color: var(--text-2, var(--text)); line-height: 1.4; }
  .actions { display: flex; flex-direction: column; gap: 0.5rem; margin-top: auto; }
  .actions button { display: inline-flex; align-items: center; justify-content: center; gap: 0.4rem; }
  .msg { display: flex; gap: 0.4rem; }
  .msg input { flex: 1; min-width: 0; }
  .small { font-size: 0.75rem; margin: 0; }
</style>
