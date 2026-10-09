<script lang="ts">
  import { Save, LoaderCircle, KeyRound, UserRound, WifiOff, Download, CircleCheck, Copy, Plug } from '@lucide/svelte';
  import Toggle from '../components/Toggle.svelte';
  import DownloadsManager from '../components/DownloadsManager.svelte';
  import McTextures from '../components/McTextures.svelte';
    import ChannelPicker from '../components/ChannelPicker.svelte';
  import { copy, get, post, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { AuthServerInfo, Settings } from '../lib/types';

  let tab = $state<'signin' | 'integrations' | 'launcher'>('signin');
  let s = $state<Settings | null>(null);
  let cfKey = $state('');
  let saving = $state(false);
  let usernameRules = $state('');
  let connections = $state<any>(null);
  let discordSecret = $state('');
  let discordBot = $state('');
  let resendKey = $state('');
  let testAddress = $state('');
  let testingEmail = $state(false);
  let savingConnections = $state(false);
  let disc = $state<any>(null);
  let discWebhook = $state('');
  let discRoles = $state<{ id: string; name: string }[]>([]);
  let savingDisc = $state(false);
  let syncingDisc = $state(false);
  const loadDisc = () => get('/api/admin/discord').then((x) => (disc = x)).catch(toastError);
  async function saveDisc() {
    savingDisc = true;
    try {
      disc = await put('/api/admin/discord', { role_sync: disc.role_sync, base_role_id: disc.base_role_id, invite_url: disc.invite_url, channel_id: disc.channel_id ?? '', webhook_url: discWebhook, notify_achievements: disc.notify_achievements, notify_guilds: disc.notify_guilds, notify_members: disc.notify_members, public_key: disc.public_key ?? '' });
      discWebhook = '';
      toast('Discord community settings saved');
      loadDisc();
    } catch (e) { toastError(e); } finally { savingDisc = false; }
  }
  async function syncDisc() {
    syncingDisc = true;
    try {
      const r = await post<any>('/api/admin/discord/sync', {});
      toast(`Checked ${r.checked} · ${r.groups_added} groups added · ${r.roles_added} roles added${r.not_in_server ? ` · ${r.not_in_server} not in server` : ''}`);
    } catch (e) { toastError(e); } finally { syncingDisc = false; }
  }
  let registering = $state(false);
  async function registerCommands() {
    registering = true;
    try {
      await saveDisc();
      const r = await post<any>('/api/admin/discord/commands/register', {});
      toast(`Registered ${r.commands} slash commands — type / in your Discord server`);
    } catch (e) { toastError(e); } finally { registering = false; }
  }
  async function testDisc() {
    try { await post('/api/admin/discord/test', {}); toast('Test message sent'); } catch (e) { toastError(e); }
  }


  let info = $state<AuthServerInfo | null>(null);
  const loadInfo = () => get<AuthServerInfo>('/api/admin/auth-server').then((x) => (info = x)).catch(toastError);

  $effect(() => {
    get<Settings>('/api/admin/settings').then((x) => { s = x; usernameRules = x.username_blocklist.join('\n'); }).catch(toastError);
    get('/api/admin/connections').then((x) => (connections = x)).catch(toastError);
    loadDisc();
    loadInfo();
  });

  async function copyText(t: string) {
    await copy(t);
    toast('Copied');
  }

  async function save() {
    if (!s) return;
    saving = true;
    try {
      s = await put<Settings>('/api/admin/settings', { ...$state.snapshot(s), username_blocklist: usernameRules.split(/[\n,]+/).map(x => x.trim()).filter(Boolean), curseforge_api_key: cfKey });
      cfKey = '';
      loadInfo();
      toast('Settings saved');
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  async function saveConnections() {
    if (!connections) return;
    savingConnections = true;
    try {
      connections = await put('/api/admin/connections', {
        discord_client_id: connections.discord_client_id,
        discord_client_secret: discordSecret,
        discord_bot_token: discordBot,
        discord_guild_id: connections.discord_guild_id,
        resend_api_key: resendKey,
        sender_email: connections.sender_email,
        sender_name: connections.sender_name,
      });
      discordSecret = discordBot = resendKey = '';
      toast('Connections saved');
    } catch (e) { toastError(e); }
    finally { savingConnections = false; }
  }

  async function testEmail() {
    testingEmail = true;
    try {
      const result = await post<{message: string}>('/api/admin/connections/test-email', {email: testAddress});
      toast(result.message);
    } catch (e) { toastError(e); }
    finally { testingEmail = false; }
  }
</script>

<div class="page narrow">
  <div class="page-head">
    <div>
      <h1>Settings</h1>
      <p>How players sign in, and connections to other services.</p>
    </div>
    <button class="primary" disabled={!s || saving} onclick={save}>
      {#if saving}<LoaderCircle class="spin" size={16} />{:else}<Save size={16} />{/if} Save
    </button>
  </div>

  {#if s}
    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === 'signin'} class:active={tab === 'signin'} onclick={() => (tab = 'signin')}><UserRound size={15} /> Sign-in &amp; accounts</button>
      <button role="tab" aria-selected={tab === 'integrations'} class:active={tab === 'integrations'} onclick={() => (tab = 'integrations')}><Plug size={15} /> Integrations</button>
      <button role="tab" aria-selected={tab === 'launcher'} class:active={tab === 'launcher'} onclick={() => (tab = 'launcher')}><Download size={15} /> Launcher &amp; apps</button>
    </div>

    {#if tab === 'launcher'}<DownloadsManager /><McTextures />{/if}

    {#if tab === 'signin'}
    <section class="card col">
      <div class="section-title"><UserRound size={18} /><h2>Panel accounts</h2></div>
      <Toggle bind:checked={s.auth.panel_accounts} label="Allow panel accounts" help="Players sign in with a username and password you manage here. Their UUID, skin and cape come from the panel's auth server." />
      {#if s.auth.panel_accounts}
        <div class="field">
          <span class="lbl">Sign-ups from the launcher</span>
          <div class="segmented">
            <button class:active={s.auth.registration === 'closed'} onclick={() => s && (s.auth.registration = 'closed')}>Closed</button>
            <button class:active={s.auth.registration === 'approval'} onclick={() => s && (s.auth.registration = 'approval')}>Needs approval</button>
            <button class:active={s.auth.registration === 'open'} onclick={() => s && (s.auth.registration = 'open')}>Open</button>
          </div>
          <span class="help">
            {s.auth.registration === 'closed' ? 'Only admins create accounts.' : s.auth.registration === 'approval' ? 'Anyone can sign up; you approve them on the Players page.' : 'Anyone can create an account and play immediately.'}
          </span>
        </div>
      {/if}
    </section>

    <section class="card col">
      <div class="section-title"><WifiOff size={18} /><h2>Offline play</h2></div>
      <Toggle bind:checked={s.auth.offline_local} label="Allow local offline accounts" help="Players can type any username and play without an account. They only see public instances." />
    </section>

    <section class="card col">
      <div class="section-title"><UserRound size={18} /><h2>Username blacklist</h2></div>
      <p class="help">Blocked for account sign-ups, admin-created accounts, Discord-created accounts and username changes. One name or word per line. An exact entry blocks that name; <code>*word*</code> also blocks it inside longer names. Keep this list short to avoid blocking innocent names.</p>
      <textarea rows="9" bind:value={usernameRules} spellcheck="false" aria-label="Blocked usernames and words"></textarea>
      <span class="help">Changes apply when you save Settings above. Existing accounts keep their names.</span>
    </section>

    <section class="card col">
      <div class="section-title"><KeyRound size={18} /><h2>Auth server</h2><span class="badge good">Yggdrasil</span></div>
      <p class="help">
        The panel is a complete authlib-injector auth server: accounts, UUIDs, skins and capes all live here.
        Point your game servers at it and they verify players against the panel, just like official servers do with Mojang.
      </p>
      <label class="field">Public address
        <input bind:value={s.public_url} placeholder={info?.public_url ?? 'https://panel.example.com'} />
        <span class="help">The address players and game servers reach the panel at. Skin URLs and the auth server use it. Leave empty to detect it from each request.</span>
      </label>
      {#if info}
        <div class="field">
          <span class="lbl">Auth server URL</span>
          <div class="copyline"><code>{info.yggdrasil_url}</code><button class="ghost icon sm" aria-label="Copy auth server URL" onclick={() => copyText(info!.yggdrasil_url)}><Copy size={14} /></button></div>
        </div>
        <div class="field">
          <span class="lbl">Game server start flag</span>
          <div class="copyline"><code>-javaagent:authlib-injector.jar={info.yggdrasil_url}</code><button class="ghost icon sm" aria-label="Copy start flag" onclick={() => copyText(`-javaagent:authlib-injector.jar=${info!.yggdrasil_url}`)}><Copy size={14} /></button></div>
          <span class="help">
            Add this before <code>-jar</code> on each server, keep <code>online-mode=true</code>, and download
            <a href="/api/v1/launcher/authlib-injector.jar">authlib-injector.jar</a> next to the server jar.
            Then add the server under <a href="#/servers">Servers</a> for stats and access control.
          </span>
        </div>
        <details>
          <summary>Signing public key</summary>
          <pre class="key">{info.public_key}</pre>
        </details>
      {/if}
    </section>

    {/if}

    {#if tab === 'integrations'}
    <section class="card col">
      <div class="section-title"><Download size={18} /><h2>Integrations</h2></div>
      <label class="field">CurseForge API key
        {#if s.curseforge_key_from_env}
          <span class="set"><CircleCheck size={15} /> Set via the <code>CURSEFORGE_API_KEY</code> environment variable</span>
        {:else}
          <input type="password" bind:value={cfKey} placeholder={s.curseforge_key_set ? '•••••••• (saved — type to replace, "-" to remove)' : 'Paste your key'} autocomplete="off" />
          <span class="help">Needed to search and import CurseForge modpacks. Get a free key at <a href="https://console.curseforge.com" target="_blank" rel="noreferrer">console.curseforge.com</a>.</span>
        {/if}
      </label>
      <label class="field">Launcher download link
        <input bind:value={s.launcher_download_url} placeholder="https://github.com/VeloraMCDev/velora-launcher/releases/latest" />
        <span class="help">Shown on the dashboard so you can share it easily.</span>
      </label>
    </section>
    {#if connections}
      <section class="card col">
        <div class="section-title"><KeyRound size={18} /><h2>Discord</h2></div>
        <p class="help">Create an OAuth2 application in the Discord Developer Portal. Add <code>{s.public_url || info?.public_url || 'https://panel.example.com'}/api/v1/auth/discord/callback</code> as its redirect URL. A bot token and server ID are optional for role sync.</p>
        <label class="field">Application client ID<input bind:value={connections.discord_client_id} autocomplete="off" /></label>
        <label class="field">Client secret<input type="password" bind:value={discordSecret} placeholder={connections.discord_client_secret_set ? 'Saved — type to replace' : 'Client secret'} autocomplete="off" /></label>
        <label class="field">Bot token<input type="password" bind:value={discordBot} placeholder={connections.discord_bot_token_set ? 'Saved — type to replace' : 'Optional bot token'} autocomplete="off" /></label>
        <label class="field">Discord server ID<input bind:value={connections.discord_guild_id} placeholder="Optional" autocomplete="off" /></label>
        <span class="help">Secrets are never returned to your browser after saving. Type <code>-</code> in a secret field to remove it.</span>
      </section>
      {#if disc}
        <section class="card col">
          <div class="section-title"><KeyRound size={18} /><h2>Discord community</h2></div>
          <p class="help">{disc.linked_accounts} linked account{disc.linked_accounts === 1 ? '' : 's'}. Role sync needs the bot token and server ID above{disc.bot_ready ? '' : ' (not saved yet)'}. Map group roles under Players → Groups and title roles under Leveling. The highest earned title role takes priority; obsolete title roles are removed.</p>
          <label class="field">Role sync
            <select bind:value={disc.role_sync}>
              <option value="off">Off</option>
              <option value="discord_to_panel">Discord roles → panel groups</option>
              <option value="panel_to_discord">Panel groups → Discord roles</option>
              <option value="both">Both directions</option>
            </select>
          </label>
          <label class="field">Base level Discord role ID<input bind:value={disc.base_role_id} inputmode="numeric" placeholder="Optional role for new players" /></label>
          <label class="field">Invite link shown in the launcher<input bind:value={disc.invite_url} placeholder="https://discord.gg/yourcode" autocomplete="off" /></label>
          <ChannelPicker bind:value={disc.channel_id} label="Announcement channel" hint="where the bot posts achievements, new guilds and status boards" canList={disc.bot_ready} />
          {#if !disc.bot_token_set}
            <p class="help">The bot needs its token first: paste it in <strong>Bot token</strong> above and save.</p>
          {:else}
            <p class="help">The bot must be in your server and able to see this channel{#if disc.bot_invite_url}. <a href={disc.bot_invite_url} target="_blank" rel="noreferrer">Add the bot to your server</a> (opens Discord){/if}.</p>
          {/if}
          {#if disc.webhook_set}
            <p class="help">An older webhook address is also saved. It is only used when no channel is set. <button type="button" class="ghost sm" onclick={() => { discWebhook = '-'; saveDisc(); }}>Remove the webhook</button></p>
          {/if}
          <Toggle bind:checked={disc.notify_achievements} label="Announce achievements" />
          <Toggle bind:checked={disc.notify_guilds} label="Announce new guilds" />
          <Toggle bind:checked={disc.notify_members} label="Announce new players" />
          <div class="row wrap">
            <button class="primary" disabled={savingDisc} onclick={saveDisc}>{savingDisc ? 'Saving…' : 'Save community settings'}</button>
            <button disabled={syncingDisc || disc.role_sync === 'off' || !disc.bot_ready} onclick={syncDisc}>{syncingDisc ? 'Syncing…' : 'Sync roles now'}</button>
            <button disabled={!disc.channel_id && !disc.webhook_set} onclick={testDisc}>Send test announcement</button>
          </div>
          <span class="help">Roles also sync automatically every 15 minutes.</span>
        </section>
        <section class="card col">
          <div class="section-title"><KeyRound size={18} /><h2>Discord slash commands</h2></div>
          <p class="help">Let members type <code>/status</code>, <code>/players</code>, <code>/quests</code>, <code>/stats</code>, <code>/leaderboard</code>, <code>/guild</code> and <code>/guilds</code> in your server. Discord calls the panel when someone uses one, so nothing else needs to run.</p>
          <ol class="help steps">
            <li>In the <strong>Developer Portal → your application → General Information</strong>, copy the <strong>Public Key</strong> and paste it below.</li>
            <li>Set <strong>Interactions Endpoint URL</strong> on that same page to <code>{s.public_url || info?.public_url || 'https://panel.example.com'}/api/v1/discord/interactions</code> and save it there.</li>
            <li>Make sure the bot was invited with the <code>applications.commands</code> scope (use the invite link above), then press <strong>Register commands</strong>.</li>
          </ol>
          <label class="field">Application public key<input bind:value={disc.public_key} placeholder="64 hex characters" autocomplete="off" spellcheck="false" /></label>
          <div class="row wrap">
            <button class="primary" disabled={registering || !disc.public_key || !disc.bot_ready} onclick={registerCommands}>{registering ? 'Registering…' : 'Save & register commands'}</button>
          </div>
          {#if !disc.bot_ready}<span class="help">Save the application ID, bot token and server ID above first.</span>{/if}
        </section>
      {/if}
      <section class="card col">
        <div class="section-title"><Download size={18} /><h2>Resend SMTP</h2></div>
        <p class="help">Verify the sender domain in Resend, then enter an API key. Velora connects to <code>smtp.resend.com</code> over TLS.</p>
        <label class="field">Resend API key<input type="password" bind:value={resendKey} placeholder={connections.resend_api_key_set ? 'Saved — type to replace' : 're_...'} autocomplete="off" /></label>
        <label class="field">Sender name<input bind:value={connections.sender_name} placeholder="Velora" /></label>
        <label class="field">Sender email<input type="email" bind:value={connections.sender_email} placeholder="hello@example.com" /></label>
        <button class="primary" disabled={savingConnections} onclick={saveConnections}>{savingConnections ? 'Saving…' : 'Save Discord & email settings'}</button>
        <div class="test-row">
          <label class="field">Send a test email<input type="email" bind:value={testAddress} placeholder="you@example.com" /></label>
          <button disabled={testingEmail || !testAddress || !connections.resend_api_key_set} onclick={testEmail}>{testingEmail ? 'Sending…' : 'Send test'}</button>
        </div>
        <p class="help">A successful test means Resend accepted the message. Confirm delivery in the destination inbox or Resend activity log.</p>
      </section>
    {/if}
    {/if}
  {/if}
</div>

<style>
  .narrow { max-width: 820px; }
  .tabs button { display: inline-flex; align-items: center; gap: 7px; white-space: nowrap; }
  section { margin-bottom: 16px; }
  .section-title { margin-bottom: 4px; }
  .section-title :global(svg) { color: var(--accent); }
  .field { display: flex; flex-direction: column; gap: 7px; font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .help { color: var(--muted); font-weight: 400; font-size: 0.8rem; line-height: 1.5; }
  .segmented { align-self: flex-start; }
  .copyline { display: flex; align-items: center; gap: 6px; background: var(--bg-2); border: 1px solid var(--line); border-radius: var(--radius-sm); padding: 4px 4px 4px 12px; }
  .copyline code { flex: 1; overflow-x: auto; white-space: nowrap; color: var(--text); font-weight: 400; padding: 6px 0; }
  details summary { cursor: pointer; font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .key { margin: 10px 0 0; padding: 12px; background: var(--bg-2); border: 1px solid var(--line); border-radius: var(--radius-sm); font-family: var(--mono); font-size: 0.72rem; color: var(--muted); overflow-x: auto; }
  .set { display: flex; align-items: center; gap: 8px; color: var(--good); font-weight: 400; }
  .test-row { display: flex; align-items: end; flex-wrap: wrap; gap: 12px; }
  .test-row .field { flex: 1; min-width: 220px; }
  code { overflow-wrap: anywhere; }
  .steps { margin: 0; padding-left: 1.2rem; display: flex; flex-direction: column; gap: 4px; }
</style>
