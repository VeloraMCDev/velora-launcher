<script lang="ts">
  import { onMount } from 'svelte';
  import { Check, Copy, ImagePlus, LoaderCircle, LogOut, Palette, Pencil, ShieldCheck, Smartphone, Trash2, UserRound, Link2, Unlink } from '@lucide/svelte';
  import SkinView from '../../components/SkinView.svelte';
  import Avatar from '../../components/Avatar.svelte';
  import Sheet from '../ui/Sheet.svelte';
  import { del, get, post, platform } from '../../lib/api';
  import type { PlayerProfile, SkinModel } from '../../../../../shared/http/platform.mjs';
  import { go } from '../../lib/router.svelte';
  import { logout, session, setToken } from '../../lib/session.svelte';
  import { toast, toastError } from '../../lib/toast.svelte';

  type Cape = { id: number; name: string; url: string };
  type Profile = PlayerProfile;
  type Discord = { invite_url: string | null; linked: boolean; display_name: string | null };

  let profile = $state<Profile | null>(null);
  let discord = $state<Discord | null>(null);
  let busy = $state('');
  let side = $state<'front' | 'back'>('front');
  let skinV = $state(0);
  let nameOpen = $state(false);
  let newName = $state('');
  let password = $state('');
  let copied = $state(false);
  let installEvent = $state<any>(null);
  let fileInput = $state<HTMLInputElement>();
  const standalone = matchMedia('(display-mode: standalone)').matches;
  const ios = /iPhone|iPad|iPod/i.test(navigator.userAgent);

  async function load() {
    try { profile = await platform.account.profile(); } catch (e) { toastError(e); }
    get<Discord>('/api/v1/account/discord').then((d) => (discord = d)).catch(() => {});
  }
  onMount(() => {
    void load();
    const onPrompt = (e: Event) => { e.preventDefault(); installEvent = e; };
    window.addEventListener('beforeinstallprompt', onPrompt);
    return () => window.removeEventListener('beforeinstallprompt', onPrompt);
  });

  async function run<T>(label: string, fn: () => Promise<T>, ok?: string) {
    busy = label;
    try { const r = await fn(); if (ok) toast(ok); return r; } catch (e) { toastError(e); } finally { busy = ''; }
  }

  async function uploadSkin(file: File | undefined) {
    if (!file) return;
    if (file.type !== 'image/png') { toast('Skins are PNG images (64×64 or 64×32).', 'error'); return; }
    const r = await run('skin', () => platform.account.uploadSkin(file, profile?.skin_model ?? 'classic'), 'Skin updated');
    if (r) { profile = r; skinV++; }
    if (fileInput) fileInput.value = '';
  }
  async function setModel(model: SkinModel) {
    if (!profile || profile.skin_model === model) return;
    const r = await run('model', () => platform.account.setSkinModel(model));
    if (r) { profile = r; skinV++; }
  }
  async function removeSkin() {
    const r = await run('skin', () => platform.account.deleteSkin(), 'Skin removed');
    if (r) { profile = r; skinV++; }
  }
  async function pickCape(id: number | null) {
    const r = await run('cape', () => platform.account.setCape(id), id == null ? 'Cape removed' : 'Cape equipped');
    if (r) { profile = r; skinV++; }
  }
  async function rename() {
    const r = await run('name', () => platform.account.setUsername(newName.trim(), password), 'Username changed');
    if (r) { setToken(r.token); session.user = r.user; nameOpen = false; password = ''; await load(); }
  }
  async function linkDiscord() {
    busy = 'discord';
    try {
      const flow = await get<{ url: string; state: string }>('/api/v1/auth/discord/start?kind=link');
      const popup = window.open(flow.url, 'scopenet-discord', 'width=520,height=740');
      if (!popup) { toast('Allow popups to link Discord.', 'error'); return; }
      for (let i = 0; i < 80; i++) {
        await new Promise((r) => setTimeout(r, 1500));
        const res = await get<any>(`/api/v1/auth/discord/poll?state=${encodeURIComponent(flow.state)}`);
        if (res.pending === true && !res.token) continue;
        popup.close();
        toast('Discord linked');
        break;
      }
    } catch (e) { toastError(e); } finally { busy = ''; await load(); }
  }
  async function unlinkDiscord() { await run('discord', () => del('/api/v1/account/connections/discord'), 'Discord unlinked'); await load(); }
  async function install() {
    if (!installEvent) return;
    installEvent.prompt();
    await installEvent.userChoice.catch(() => {});
    installEvent = null;
  }
  async function copyUuid() {
    try { await navigator.clipboard.writeText(profile?.uuid ?? ''); } catch { /* clipboard may be unavailable */ }
    copied = true; setTimeout(() => (copied = false), 1300);
  }
  const skinSrc = $derived(profile?.skin_url ? `${profile.skin_url}${profile.skin_url.includes('?') ? '&' : '?'}v=${skinV}` : null);
</script>

<div class="pl-page">
  <div class="pl-head"><div><h1><UserRound size={26} /> Account</h1><p>Your look, your name and how you sign in.</p></div></div>

  <div class="pl-grid" style="--min: 330px">
    <section class="pl-card ident">
      <div class="stage">
        {#if profile}
          {#if skinSrc}
            <div class="skin"><SkinView skin={skinSrc} cape={profile.cape?.url ?? null} slim={profile.skin_model === 'slim'} {side} scale={9} /></div>
          {:else}
            <div class="noskin"><Avatar name={profile.name} uuid={profile.uuid} size={120} v={skinV} /><span>No skin yet</span></div>
          {/if}
        {:else}<div class="pl-skel" style="height: 280px; width: 160px"></div>{/if}
      </div>
      <div class="who">
        <h2>{session.user?.username}</h2>
        <span class="pl-chip" class:accent={session.user?.role === 'admin'}>{session.user?.role === 'admin' ? 'Administrator' : 'Player'}</span>
        <button class="uuid" onclick={copyUuid} title="Copy your UUID">{#if copied}<Check size={13} /> Copied{:else}<Copy size={13} /> {profile?.uuid ?? '…'}{/if}</button>
        {#if skinSrc}<div class="pl-tabs" style="margin-top: 6px"><button class:on={side === 'front'} onclick={() => (side = 'front')}>Front</button><button class:on={side === 'back'} onclick={() => (side = 'back')}>Back</button></div>{/if}
      </div>
    </section>

    <div class="pl-stack">
      <section class="pl-card">
        <div class="pl-card-head"><h2><ImagePlus size={17} /> Skin</h2></div>
        <p class="note">Upload a 64×64 PNG. It shows up in game, the launcher and on the map.</p>
        <div class="pl-row-flex pl-wrap">
          <input bind:this={fileInput} type="file" accept="image/png" hidden onchange={(e) => uploadSkin(e.currentTarget.files?.[0])} />
          <button class="pl-btn primary" onclick={() => fileInput?.click()} disabled={busy === 'skin'} aria-busy={busy === 'skin'}>{#if busy === 'skin'}<LoaderCircle size={16} class="spin" />{:else}<ImagePlus size={16} />{/if} Upload skin</button>
          {#if profile?.skin_url}<button class="pl-btn" onclick={removeSkin} disabled={busy === 'skin'}><Trash2 size={15} /> Remove</button>{/if}
        </div>
        <div class="field"><span>Arm style</span><div class="pl-tabs"><button class:on={profile?.skin_model !== 'slim'} onclick={() => setModel('classic')}>Classic (Steve)</button><button class:on={profile?.skin_model === 'slim'} onclick={() => setModel('slim')}>Slim (Alex)</button></div></div>
      </section>

      <section class="pl-card">
        <div class="pl-card-head"><h2><Palette size={17} /> Cape</h2></div>
        {#if profile?.available_capes.length}
          <div class="capes">
            <button class="cape" class:on={!profile.cape} onclick={() => pickCape(null)}><span class="none">None</span></button>
            {#each profile.available_capes as c (c.id)}
              <button class="cape" class:on={profile.cape?.id === c.id} onclick={() => pickCape(c.id)} title={c.name} aria-label="Equip {c.name}"><img src={c.url} alt="" /><small>{c.name}</small></button>
            {/each}
          </div>
        {:else}<p class="note">No capes are available to your account yet.</p>{/if}
      </section>
    </div>

    <div class="pl-stack">
      <section class="pl-card">
        <div class="pl-card-head"><h2><Pencil size={17} /> Username</h2></div>
        <p class="note">Change it while you're offline. You'll need your password.</p>
        <button class="pl-btn" onclick={() => { newName = ''; password = ''; nameOpen = true; }}>Change username</button>
      </section>

      <section class="pl-card">
        <div class="pl-card-head"><h2><Link2 size={17} /> Discord</h2>{#if discord?.linked}<span class="pl-chip good">Linked</span>{/if}</div>
        {#if discord?.linked}
          <p class="note">Connected as <b>{discord.display_name}</b>. Your roles sync automatically.</p>
          <button class="pl-btn" onclick={unlinkDiscord} disabled={busy === 'discord'}><Unlink size={15} /> Unlink</button>
        {:else}
          <p class="note">Link Discord to sign in with it and get your roles.</p>
          <div class="pl-row-flex pl-wrap">
            <button class="pl-btn primary" onclick={linkDiscord} disabled={busy === 'discord'} aria-busy={busy === 'discord'}>Link Discord</button>
            {#if discord?.invite_url}<a class="btn pl-btn" href={discord.invite_url} target="_blank" rel="noopener noreferrer">Join the server</a>{/if}
          </div>
        {/if}
      </section>

      <section class="pl-card">
        <div class="pl-card-head"><h2><Smartphone size={17} /> Use it like an app</h2></div>
        {#if standalone}
          <p class="note"><Check size={14} /> You're running the installed app.</p>
        {:else if installEvent}
          <p class="note">Add this panel to your home screen for one-tap access.</p>
          <button class="pl-btn primary" onclick={install}>Install app</button>
        {:else if ios}
          <p class="note">In Safari, tap <b>Share</b>, then <b>Add to Home Screen</b>.</p>
        {:else}
          <p class="note">Open your browser menu and choose <b>Install app</b> or <b>Add to Home screen</b>.</p>
        {/if}
      </section>

      <section class="pl-card">
        <div class="pl-card-head"><h2><ShieldCheck size={17} /> Session</h2></div>
        <div class="pl-row-flex pl-wrap">
          <button class="pl-btn" onclick={() => { logout(); go('login'); }}><LogOut size={15} /> Sign out</button>
          {#if session.user?.role === 'admin'}<a class="btn pl-btn" href="#/dashboard">Admin panel</a>{/if}
        </div>
      </section>
    </div>
  </div>
</div>

<Sheet bind:open={nameOpen} title="Change username" width={420}>
  <label class="f">New username<input class="pl-input" bind:value={newName} minlength="3" maxlength="16" placeholder="3–16 letters, numbers or _" autocomplete="off" /></label>
  <label class="f">Your password<input class="pl-input" type="password" bind:value={password} autocomplete="current-password" /></label>
  {#snippet footer()}
    <button class="pl-btn" onclick={() => (nameOpen = false)}>Cancel</button>
    <button class="pl-btn primary" onclick={rename} disabled={busy === 'name' || newName.trim().length < 3 || !password}>Change username</button>
  {/snippet}
</Sheet>

<style>
  .ident { display: flex; flex-direction: column; align-items: center; gap: 14px; text-align: center; background: radial-gradient(90% 70% at 50% 0%, color-mix(in srgb, var(--accent) 24%, transparent), transparent 70%), var(--pl-glass); }
  .stage { min-height: 290px; display: grid; place-items: center; }
  .skin { filter: drop-shadow(0 18px 24px #000a); animation: float 5s ease-in-out infinite; }
  @keyframes float { 50% { transform: translateY(-6px); } }
  .noskin { display: flex; flex-direction: column; gap: 10px; align-items: center; color: var(--muted); }
  .who { display: flex; flex-direction: column; align-items: center; gap: 8px; }
  .who h2 { font-size: 1.5rem; }
  .uuid { font-family: var(--mono); font-size: 0.7rem; color: var(--muted); border: none; background: rgba(255, 255, 255, 0.05); padding: 4px 10px; border-radius: 99px; max-width: 100%; overflow: hidden; text-overflow: ellipsis; }
  .note { color: var(--muted); font-size: 0.86rem; margin-bottom: 12px; display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .field { display: flex; flex-direction: column; gap: 8px; margin-top: 14px; font-size: 0.82rem; color: var(--text-2); }
  .capes { display: grid; grid-template-columns: repeat(auto-fill, minmax(84px, 1fr)); gap: 10px; }
  .cape { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 10px 6px; border-radius: 14px; background: var(--surface-2); border: 2px solid transparent; }
  .cape.on { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 16%, var(--surface-2)); }
  .cape img { width: 44px; height: 70px; object-fit: contain; image-rendering: pixelated; }
  .cape small { font-size: 0.68rem; color: var(--muted); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cape .none { height: 70px; display: grid; place-items: center; color: var(--muted); font-size: 0.82rem; }
  .f { display: flex; flex-direction: column; gap: 6px; font-size: 0.84rem; color: var(--text-2); }
  @media (prefers-reduced-motion: reduce) { .skin { animation: none; } }
</style>
