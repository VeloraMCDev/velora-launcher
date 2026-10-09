<script lang="ts">
  import { onMount } from 'svelte';
  import { Mail, Send, Save, LoaderCircle, KeyRound, BellRing, SlidersHorizontal } from '@lucide/svelte';
  import { api } from '../lib/api';
  import { toast } from '../lib/state.svelte';
  import type { Settings } from '../lib/types';

  let settings = $state<Settings | null>(null);
  let apiKey = $state('');
  let githubToken = $state('');
  let recipients = $state('');
  let saving = $state(false);
  let testing = $state(false);

  const EVENTS: Record<string, string> = {
    service_down: 'A Velora service stops or becomes unhealthy',
    endpoint_down: 'velora.scopedd.lol or the docs stop responding',
    backup_failed: 'A backup fails, is overdue or misses the Hermes copy',
    disk_low: 'The VPS disk is nearly full',
    certificate_expiring: 'An HTTPS certificate is close to expiry',
    launcher_release: 'A new launcher release is waiting for approval',
    image_update: 'A new Panel or dashboard build is available',
    action_failed: 'An operation started here fails',
  };

  onMount(async () => {
    settings = await api<Settings>('/api/settings');
    recipients = settings.notifications.recipients.join(', ');
  });

  async function save() {
    if (!settings) return;
    saving = true;
    try {
      settings = await api<Settings>('/api/settings', { method: 'PUT', body: {
        notifications: { ...settings.notifications, resend_api_key: apiKey, recipients: recipients.split(/[,\s]+/).filter(Boolean) },
        github_token: githubToken, alerts: settings.alerts,
      } });
      apiKey = ''; githubToken = '';
      toast('Settings saved');
    } catch (e) { toast((e as Error).message, 'error'); }
    saving = false;
  }
  async function test() {
    testing = true;
    try { await save(); await api('/api/settings/test-email', { method: 'POST' }); toast('Test email sent'); }
    catch (e) { toast((e as Error).message, 'error'); }
    testing = false;
  }
</script>

<div class="head"><h1>Settings</h1><p class="muted">Email notifications through Resend, alert thresholds and integrations.</p></div>

{#if !settings}
  <div class="card skeleton" style="height:300px"></div>
{:else}
  <div class="grid g2">
    <div class="card col">
      <div class="card-head"><Mail size={18} /><h2>Resend email</h2><span class="spacer"></span>
        <label class="switch"><input type="checkbox" bind:checked={settings.notifications.enabled} /><span></span> {settings.notifications.enabled ? 'On' : 'Off'}</label></div>
      <label class="field">API key
        <input type="password" bind:value={apiKey} autocomplete="off" placeholder={settings.notifications.resend_api_key_set ? '•••••••• saved — enter a new key to replace it' : 're_…'} />
        <span class="help">Create a key with "Sending access" at resend.com/api-keys. It is stored on the VPS and never shown again.</span>
      </label>
      <label class="field">Sender
        <input bind:value={settings.notifications.from} placeholder="Velora <alerts@scopedd.lol>" />
        <span class="help">Must use a domain you verified in Resend.</span>
      </label>
      <label class="field">Recipients
        <input bind:value={recipients} placeholder="you@example.com, team@example.com" />
        <span class="help">Comma-separated, up to 20 addresses.</span>
      </label>
      <div class="row">
        <button class="primary" onclick={save} disabled={saving}>{#if saving}<LoaderCircle size={16} class="spin" />{:else}<Save size={16} />{/if} Save</button>
        <button onclick={test} disabled={testing || saving}>{#if testing}<LoaderCircle size={16} class="spin" />{:else}<Send size={16} />{/if} Send test email</button>
      </div>
    </div>

    <div class="card col">
      <div class="card-head"><BellRing size={18} /><h2>Notify me when…</h2></div>
      {#each Object.entries(EVENTS) as [key, label]}
        <label class="check"><input type="checkbox" bind:checked={settings.notifications.events[key]} /> <span>{label}</span></label>
      {/each}
      <p class="tiny faint">Problems email after two consecutive failed checks (about a minute) and again when they clear.</p>
    </div>

    <div class="card col">
      <div class="card-head"><SlidersHorizontal size={18} /><h2>Thresholds</h2></div>
      <label class="field">Disk usage alert (%)<input type="number" min="50" max="99" bind:value={settings.alerts.disk_percent} /></label>
      <label class="field">Backup considered overdue after (hours)<input type="number" min="2" max="168" bind:value={settings.alerts.backup_max_age_hours} /></label>
      <label class="field">Certificate warning (days before expiry)<input type="number" min="1" max="60" bind:value={settings.alerts.certificate_days} /></label>
      <div><button onclick={save} disabled={saving}><Save size={16} /> Save thresholds</button></div>
    </div>

    <div class="card col">
      <div class="card-head"><KeyRound size={18} /><h2>GitHub</h2></div>
      <label class="field">Personal access token (optional)
        <input type="password" bind:value={githubToken} autocomplete="off" placeholder={settings.github_token_set ? '•••••••• saved' : 'github_pat_…'} />
        <span class="help">Release checks work without a token. A read-only token raises GitHub's rate limit from 60 to 5,000 requests an hour.</span>
      </label>
      <div><button onclick={save} disabled={saving}><Save size={16} /> Save</button></div>
    </div>
  </div>
{/if}

<style>
  .head { margin-bottom: 22px; }
  .head p { margin: 4px 0 0; }
  .col { gap: 14px; }
  .check { display: flex; gap: 10px; align-items: center; font-size: 14px; cursor: pointer; }
  .check input { width: 16px; height: 16px; accent-color: var(--accent); }
  .switch { display: inline-flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); cursor: pointer; }
  .switch input { display: none; }
  .switch span { width: 38px; height: 22px; border-radius: 99px; background: var(--line-2); position: relative; transition: background 0.2s; }
  .switch span::after { content: ''; position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: #fff; transition: transform 0.2s; }
  .switch input:checked + span { background: var(--accent); }
  .switch input:checked + span::after { transform: translateX(16px); }
  p.tiny { margin: 0; }
</style>
