<script lang="ts">
  import { UserRound, WifiOff, LoaderCircle, Check, ArrowRight, UserPlus } from '@lucide/svelte';
  import { accountAdded, app, toast } from '../lib/store.svelte';
  import { errorText, invoke, openUrl } from '../lib/tauri';
  import type { Account } from '../lib/types';

  const auth = $derived(app.manifest?.auth);
  const methods = $derived(
    [
      auth?.panel_accounts !== false && { id: 'panel', label: 'Account' },
      auth?.offline_local !== false && { id: 'offline', label: 'Offline' },
    ].filter(Boolean) as { id: string; label: string }[],
  );
  let method = $state('');
  $effect(() => {
    if (!methods.some((m) => m.id === method)) method = methods[0]?.id ?? 'offline';
  });

  let username = $state('');
  let password = $state('');
  let email = $state('');
  let registering = $state(false);
  let busy = $state(false);
  let error = $state('');
  let pendingNotice = $state(false);
  let recoveryEmail = $state('');
  let recovering = $state(false);
  let recoveryNotice = $state('');

  const canRegister = $derived(auth?.registration && auth.registration !== 'closed');

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    busy = true;
    try {
      if (method === 'offline') {
        await accountAdded(await invoke<Account>('add_offline', { username }));
      } else if (registering) {
        const r = await invoke<{ account: Account | null; pending: boolean }>('register_panel', { username, password, email: email || null });
        if (r.pending) pendingNotice = true;
        else if (r.account) await accountAdded(r.account);
      } else {
        await accountAdded(await invoke<Account>('login_panel', { username, password }));
      }
      if (!pendingNotice) toast(`Signed in as ${username}`);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  async function signInDiscord() {
    error = ''; busy = true;
    try {
      const flow = await invoke<{url: string; state: string}>('discord_sign_in_start');
      await openUrl(flow.url);
      for (let i = 0; i < 120; i++) {
        await new Promise(resolve => setTimeout(resolve, 1500));
        const result = await invoke<{account?: Account; pending?: boolean}>('discord_sign_in_poll', {oauthState: flow.state});
        if (result.account) {
          await accountAdded(result.account);
          toast(`Signed in as ${result.account.username}`);
          return;
        }
        if (!result.pending) { pendingNotice = true; return; }
      }
      throw new Error('Discord sign-in timed out. Try again.');
    } catch (err) { error = errorText(err); }
    finally { busy = false; }
  }

  async function forgotPassword() {
    error = ''; busy = true;
    try { recoveryNotice = await invoke<string>('request_password_reset', {email: recoveryEmail}); }
    catch (err) { error = errorText(err); }
    finally { busy = false; }
  }

</script>

<div class="signin">
  {#if methods.length > 1}
    <div class="segmented tabs">
      {#each methods as m}
        <button class:active={method === m.id} onclick={() => { method = m.id; error = ''; registering = false; }}>{m.label}</button>
      {/each}
    </div>
  {/if}

  {#if pendingNotice}
    <div class="notice">
      <Check size={22} />
      <div><strong>Account created!</strong><p class="small muted">An admin needs to approve it before you can sign in. Check back soon.</p></div>
    </div>
    <button onclick={() => { pendingNotice = false; registering = false; }}>Back to sign in</button>
  {:else}
    <form class="col" onsubmit={submit}>
      <label class="field">
        {method === 'offline' ? 'Username' : 'Username'}
        <div class="inp"><span class="ic">{#if method === 'offline'}<WifiOff size={16} />{:else}<UserRound size={16} />{/if}</span>
          <input bind:value={username} autocomplete="username" placeholder={method === 'offline' ? 'Steve' : ''} maxlength="16" required />
        </div>
        {#if method === 'offline'}<span class="help">Play without an account. You'll only see public instances.</span>{/if}
      </label>
      {#if method === 'panel'}
        <label class="field">Password<input type="password" bind:value={password} autocomplete={registering ? 'new-password' : 'current-password'} required minlength={registering ? 8 : undefined} /></label>
        {#if registering}
          <label class="field">Email <span class="help">Optional</span><input type="email" bind:value={email} /></label>
        {/if}
      {/if}
      <button class="primary big" disabled={busy || !username}>
        {#if busy}<LoaderCircle class="spin" size={18} />{:else if registering}<UserPlus size={18} />{:else}<ArrowRight size={18} />{/if}
        {method === 'offline' ? 'Play offline' : registering ? 'Create account' : 'Sign in'}
      </button>
      {#if method === 'panel' && canRegister}
        <button type="button" class="ghost sm switch" onclick={() => { registering = !registering; error = ''; }}>
          {registering ? 'Already have an account? Sign in' : "Don't have an account? Sign up"}
        </button>
      {/if}
      {#if method === 'panel' && !registering}
        <button type="button" class="ghost sm switch" onclick={signInDiscord} disabled={busy}>Sign in with Discord</button>
        <button type="button" class="ghost sm switch" onclick={() => recovering = !recovering}>Forgot password?</button>
        {#if recovering}
          <label class="field">Account email<input type="email" bind:value={recoveryEmail} placeholder="you@example.com" /></label>
          <button type="button" disabled={busy || !recoveryEmail} onclick={forgotPassword}>Email reset link</button>
          {#if recoveryNotice}<p class="help">{recoveryNotice}</p>{/if}
        {/if}
      {/if}
    </form>
  {/if}

  {#if error}<div class="error selectable">{error}</div>{/if}
</div>

<style>
  .signin { display: flex; flex-direction: column; gap: 1rem; }
  .tabs { width: 100%; }
  .tabs button { flex: 1; }
  .inp { position: relative; }
  .inp .ic { position: absolute; left: 0.8rem; top: 50%; transform: translateY(-50%); color: var(--muted); display: flex; }
  .inp input { padding-left: 2.4rem; }
  .big { padding: 0.75rem; font-size: 0.95rem; }
  .switch { align-self: center; color: var(--muted); }
  .error { font-size: 0.87rem; padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); color: color-mix(in srgb, var(--danger) 60%, white); background: color-mix(in srgb, var(--danger) 12%, transparent); border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent); }
  .notice { display: flex; gap: 0.8rem; align-items: flex-start; padding: 1rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--success) 12%, transparent); }
  .notice :global(svg) { color: var(--success); flex-shrink: 0; }
</style>
