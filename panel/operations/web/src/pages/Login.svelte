<script lang="ts">
  import { LoaderCircle, LockKeyhole } from '@lucide/svelte';
  import { signIn } from '../lib/state.svelte';

  let username = $state('');
  let password = $state('');
  let busy = $state(false);
  let error = $state('');

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      await signIn(username, password);
    } catch (err) {
      error = (err as Error).message;
      password = '';
    } finally {
      busy = false;
    }
  }
</script>

<div class="wrap">
  <form class="card" onsubmit={submit}>
    <div class="brand"><span class="logo"></span><div><h1>Velora Operations</h1><p class="muted small">Sign in with your Velora administrator account.</p></div></div>
    <label class="field">Username<input bind:value={username} autocomplete="username" required /></label>
    <label class="field">Password<input type="password" bind:value={password} autocomplete="current-password" required /></label>
    {#if error}<div class="error" role="alert">{error}</div>{/if}
    <button class="primary" type="submit" disabled={busy || !username || !password}>
      {#if busy}<LoaderCircle size={16} class="spin" />{:else}<LockKeyhole size={16} />{/if} Sign in
    </button>
    <p class="tiny faint">Every action is recorded in the audit log. Sessions last 12 hours.</p>
  </form>
</div>

<style>
  .wrap { min-height: 100vh; display: grid; place-items: center; padding: 20px; }
  form { width: min(400px, 100%); display: flex; flex-direction: column; gap: 16px; padding: 30px; box-shadow: var(--shadow); }
  .brand { display: flex; gap: 14px; align-items: center; margin-bottom: 6px; }
  .brand h1 { font-size: 21px; }
  .brand p { margin: 2px 0 0; }
  .logo { width: 42px; height: 42px; border-radius: 12px; background: linear-gradient(135deg, #8f6bff, #3dd6c6); flex-shrink: 0; }
  button { height: 42px; }
  .error { background: #f2555a17; border: 1px solid #f2555a55; color: #ffb3b5; padding: 9px 12px; border-radius: 10px; font-size: 14px; }
  p.tiny { margin: 0; text-align: center; }
</style>
