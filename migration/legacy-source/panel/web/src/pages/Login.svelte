<script lang="ts">
  import { LogIn, LoaderCircle } from '@lucide/svelte';
  import { get, post } from '../lib/api';
  import { setToken } from '../lib/session.svelte';

  let { brandName, logo, registration = 'closed', app = false }: { brandName: string; logo: string | null; registration?: 'closed' | 'approval' | 'open'; app?: boolean } = $props();
  let username = $state('');
  let password = $state('');
  let error = $state('');
  let busy = $state(false);
  let mode = $state(location.hash.startsWith('#/reset-password') ? 'reset' : 'login');
  let email = $state('');
  let newPassword = $state('');
  let notice = $state('');
  let discordEnabled = $state(false);
  get<{discord_enabled: boolean}>('/api/v1/auth/connections/config').then(x => discordEnabled = x.discord_enabled).catch(() => {});

  async function discordSignIn() {
    error = '';
    try {
      const flow = await get<{url: string; state: string}>('/api/v1/auth/discord/start');
      const popup = window.open(flow.url, 'scopenet-discord', 'width=520,height=740');
      if (!popup) { error = 'Allow popups to sign in with Discord.'; return; }
      for (let i = 0; i < 120; i++) {
        await new Promise(resolve => setTimeout(resolve, 1500));
        const result = await get<any>(`/api/v1/auth/discord/poll?state=${encodeURIComponent(flow.state)}`);
        if (result.pending === true && !result.token) continue;
        popup.close();
        if (result.token) {
          setToken(result.token);
        } else notice = 'Your account is waiting for approval.';
        return;
      }
      error = 'Discord sign-in timed out. Try again.';
    } catch (err) { error = err instanceof Error ? err.message : String(err); }
  }

  async function recover(e: SubmitEvent) {
    e.preventDefault(); busy = true; error = ''; notice = '';
    try {
      if (mode === 'forgot') {
        const response = await post<{message: string}>('/api/v1/auth/forgot-password', {email});
        notice = response.message;
      } else {
        const token = new URLSearchParams(location.hash.split('?')[1] || '').get('token') || '';
        await post('/api/v1/auth/reset-password', {token, password: newPassword});
        notice = 'Password updated. You can sign in now.';
        mode = 'login'; location.hash = '#/login';
      }
    } catch (err) { error = err instanceof Error ? err.message : String(err); }
    finally { busy = false; }
  }

  async function register(e: SubmitEvent) {
    e.preventDefault(); busy = true; error = ''; notice = '';
    try {
      const r = await post<{ token: string; pending?: boolean }>('/api/v1/auth/register', { username, password, email: email.trim() || null });
      if (r.pending || !r.token) { notice = 'Account created. An admin needs to approve it before you can sign in.'; mode = 'login'; password = ''; }
      else setToken(r.token);
    } catch (err) { error = err instanceof Error ? err.message : String(err); }
    finally { busy = false; }
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const r = await post<{ token: string; pending?: boolean }>('/api/v1/auth/login', { username, password });
      setToken(r.token);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="wrap">
  <form class="card login" onsubmit={mode === 'login' ? submit : mode === 'register' ? register : recover}>
    <img src={logo ?? '/favicon.svg'} alt="" class="logo" />
    <h1>{brandName}</h1>
    <p class="muted">{mode === 'forgot' ? 'Request a password reset link' : mode === 'reset' ? 'Set a new password' : mode === 'register' ? 'Create your player account' : 'Sign in to your account'}</p>
    {#if mode === 'login' || mode === 'register'}
      <label class="field">Username<input bind:value={username} autocomplete="username" required minlength={mode === 'register' ? 3 : undefined} maxlength={mode === 'register' ? 16 : undefined} /></label>
      {#if mode === 'register'}<label class="field">Email <span class="muted">(optional, for password resets)</span><input type="email" bind:value={email} autocomplete="email" /></label>{/if}
      <label class="field">Password<input type="password" bind:value={password} autocomplete={mode === 'register' ? 'new-password' : 'current-password'} required minlength={mode === 'register' ? 8 : undefined} /></label>
    {:else if mode === 'forgot'}
      <label class="field">Account email<input type="email" bind:value={email} autocomplete="email" required /></label>
    {:else}
      <label class="field">New password<input type="password" bind:value={newPassword} minlength="8" autocomplete="new-password" required /></label>
    {/if}
    {#if notice}<div class="notice">{notice}</div>{/if}
    {#if error}<div class="error">{error}</div>{/if}
    <button class="primary big" disabled={busy}>
      {#if busy}<LoaderCircle class="spin" size={18} />{:else}<LogIn size={18} />{/if}
      {mode === 'login' ? 'Sign in' : mode === 'register' ? 'Create account' : mode === 'forgot' ? 'Email reset link' : 'Reset password'}
    </button>
    {#if mode === 'login' && discordEnabled}<button type="button" onclick={discordSignIn}>Sign in with Discord</button>{/if}
    {#if mode === 'login'}
      <button type="button" class="ghost" onclick={() => mode = 'forgot'}>Forgot password?</button>
      {#if registration !== 'closed'}<button type="button" class="ghost" onclick={() => { mode = 'register'; error = ''; notice = ''; }}>New here? Create an account</button>{/if}
    {:else}<button type="button" class="ghost" onclick={() => { mode = 'login'; error = ''; location.hash = '#/login'; }}>Back to sign in</button>{/if}
    {#if !app}<p class="tiny muted hint">Admins and players sign in here. First start? The admin password is in the container logs (or set <code>ADMIN_PASSWORD</code>).</p>{/if}
  </form>
</div>

<style>
  .wrap { min-height: 100vh; display: grid; place-items: center; padding: 20px; position: relative; overflow: hidden; }
  .login { width: 100%; max-width: 400px; display: flex; flex-direction: column; gap: 16px; padding: 32px; position: relative; animation: fade-in 0.3s ease; }
  .logo { width: 44px; height: 44px; border-radius: 10px; margin-bottom: 4px; object-fit: cover; }
  h1 { font-size: 1.4rem; letter-spacing: 0.04em; }
  .muted { margin-top: -10px; }
  .big { padding: 12px; margin-top: 6px; }
  .error { background: rgba(244, 63, 94, 0.1); border: 1px solid rgba(244, 63, 94, 0.3); color: #fda4af; padding: 10px 12px; border-radius: 10px; font-size: 0.88rem; }
  .notice { color: var(--good); font-size: 0.88rem; }
  .hint { text-align: center; margin-top: 0; line-height: 1.5; }
</style>
