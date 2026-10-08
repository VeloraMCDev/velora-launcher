<script lang="ts">
  import {
    UserRound, Gamepad2, Cpu, Keyboard, Palette, Rocket, HardDrive, Info, Plus, LogOut, Check, FolderOpen, Trash2,
    FileCode, RotateCcw, Download, LoaderCircle, RefreshCw, MousePointerClick, Zap, Gauge, Feather, Link2, Shirt, Upload, Ban,
    BookmarkPlus, Sparkles,
  } from '@lucide/svelte';
  import Avatar from '../components/Avatar.svelte';
  import Modal from '../components/Modal.svelte';
  import SkinView from '../components/SkinView.svelte';
  import CompanionSettings from '../components/CompanionSettings.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { abs, activeAccount, app, connect, instances, refresh, removeAccount, saveSettings, switchAccount, toast } from '../lib/store.svelte';
  import { BIND_GROUPS, defaults, fromKeyboard, fromMouse, keyLabel } from '../lib/keys';
  import { bytes, gb } from '../lib/format';
  import { errorText, invoke, openUrl, pickFile } from '../lib/tauri';
  import type { Account, Gc, PlayerProfile, SkinProfile, UpdateInfo } from '../lib/types';

  const serverAccount = $derived(activeAccount()?.kind === 'panel');
  let discordConnection = $state<{id: string; name: string} | null>(null);
  let discordBusy = $state(false);
  let discordInvite = $state('');
  $effect(() => {
    if (serverAccount && app.settingsTab === 'account') {
      invoke<{id: string; name: string} | null>('discord_connection').then(x => discordConnection = x).catch(() => discordConnection = null);
      invoke<{invite_url: string}>('discord_info').then(x => discordInvite = x.invite_url ?? '').catch(() => discordInvite = '');
    }
  });
  async function linkDiscord() {
    discordBusy = true;
    try {
      const flow = await invoke<{url: string; state: string}>('discord_link_start');
      await openUrl(flow.url);
      for (let i = 0; i < 120; i++) {
        await new Promise(resolve => setTimeout(resolve, 1500));
        const result = await invoke<{linked?: boolean; pending?: boolean}>('discord_sign_in_poll', {oauthState: flow.state});
        if (result.linked) {
          discordConnection = await invoke('discord_connection');
          toast('Discord connected'); return;
        }
      }
      toast('Discord link timed out', 'error');
    } catch (e) { toast(errorText(e), 'error'); }
    finally { discordBusy = false; }
  }
  async function unlinkDiscord() {
    discordBusy = true;
    try { await invoke('discord_unlink'); discordConnection = null; toast('Discord disconnected'); }
    catch (e) { toast(errorText(e), 'error'); }
    finally { discordBusy = false; }
  }
  const tabs = $derived([
    { id: 'account', label: 'Accounts', icon: UserRound },
    ...(serverAccount ? [{ id: 'skin', label: 'Skin & cape', icon: Shirt }] : []),
    { id: 'game', label: 'Game', icon: Gamepad2 },
    { id: 'java', label: 'Java & performance', icon: Cpu },
    { id: 'controls', label: 'Controls', icon: Keyboard },
    { id: 'companion', label: 'Companion', icon: Sparkles },
    { id: 'appearance', label: 'Appearance', icon: Palette },
    { id: 'launcher', label: 'Launcher', icon: Rocket },
    { id: 'storage', label: 'Storage', icon: HardDrive },
    { id: 'about', label: 'About', icon: Info },
  ]);

  const s = $derived(app.settings!);
  const b = $derived(app.manifest?.branding);
  const ram = $derived(app.boot?.app.total_ram_mb ?? 8192);
  const maxRam = $derived(Math.max(2048, Math.floor((ram - 1024) / 512) * 512));
  const recommended = $derived(Math.min(8192, Math.max(2048, Math.floor(ram / 2 / 512) * 512)));
  const advanced = $derived(b?.features.allow_advanced_java !== false);
  const themeAllowed = $derived(b?.features.allow_user_theme !== false);
  const kindLabel = { panel: 'Server account', offline: 'Offline' };
  let newUsername = $state('');
  let renamePassword = $state('');
  let renaming = $state(false);
  async function renameAccount() {
    renaming = true;
    try {
      const account = await invoke<Account>('set_username', { username: newUsername, password: renamePassword });
      app.accounts = app.accounts.map((a) => a.id === account.id ? account : a);
      newUsername = '';
      renamePassword = '';
      await refresh();
      toast('Username updated. Your UUID and inventory are unchanged.');
    } catch (e) { toast(errorText(e), 'error'); }
    finally { renamePassword = ''; renaming = false; }
  }

  // ---- controls ----
  let capturing = $state<string | null>(null);
  const binds = $derived({ ...defaults(), ...s.keybinds });

  function capture(e: KeyboardEvent | MouseEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopPropagation();
    let key: string | null = null;
    if (e instanceof KeyboardEvent) key = e.code === 'Escape' ? 'key.keyboard.unknown' : fromKeyboard(e.code);
    else key = fromMouse(e.button);
    if (key) {
      s.keybinds = { ...s.keybinds, [capturing]: key };
      saveSettings();
    }
    capturing = null;
  }
  const conflicts = $derived.by(() => {
    const seen = new Map<string, number>();
    Object.values(binds).forEach((k) => k !== 'key.keyboard.unknown' && seen.set(k, (seen.get(k) ?? 0) + 1));
    return new Set([...seen].filter(([, n]) => n > 1).map(([k]) => k));
  });

  // ---- skin & cape ----
  let profile = $state<PlayerProfile | null>(null);
  let profileError = $state<string | null>(null);
  let skinBusy = $state(false);
  let dragging = $state(false);
  $effect(() => {
    if (app.settingsTab !== 'skin') return;
    void app.active;
    profile = null;
    profileError = null;
    invoke<PlayerProfile>('account_profile').then((p) => (profile = p)).catch((e) => (profileError = errorText(e)));
  });
  $effect(() => {
    if (app.settingsTab === 'skin' && !serverAccount) app.settingsTab = 'account';
  });
  async function skinAction(cmd: string, args: Record<string, unknown> = {}, done?: string) {
    skinBusy = true;
    try {
      profile = await invoke<PlayerProfile>(cmd, args);
      app.skinVersion++;
      if (done) toast(done);
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      skinBusy = false;
    }
  }
  async function uploadSkin(file: File | undefined) {
    if (!file) return;
    if (file.type && file.type !== 'image/png') return toast('Skins must be PNG files', 'error');
    if (file.size > 2_000_000) return toast('That file is too large for a skin', 'error');
    const buf = new Uint8Array(await file.arrayBuffer());
    let bin = '';
    for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
    await skinAction('upload_skin', { data: btoa(bin), model: profile?.skin_model ?? 'classic' }, 'Skin updated');
  }
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    uploadSkin(e.dataTransfer?.files[0]);
  }

  // ---- saved skin profiles (wardrobe) ----
  let skinProfiles = $state<SkinProfile[]>([]);
  let profileNameInput = $state('');
  let savingCurrentLook = $state(false);
  let applyingProfileId = $state<string | null>(null);
  let newProfileModal = $state(false);
  let newProfileName = $state('');
  let newProfileFile = $state<File | null>(null);
  let newProfileModel = $state<'classic' | 'slim'>('classic');
  let newProfileCapeId = $state<number | null>(null);

  async function loadSkinProfiles() {
    try {
      skinProfiles = await invoke<SkinProfile[]>('get_skin_profiles');
    } catch {
      skinProfiles = [];
    }
  }

  $effect(() => {
    if (app.settingsTab === 'skin') {
      loadSkinProfiles();
    }
  });

  async function saveCurrentLook() {
    if (!profile) return;
    const name = profileNameInput.trim() || `Look ${skinProfiles.length + 1}`;
    try {
      skinProfiles = await invoke<SkinProfile[]>('save_skin_profile', {
        profile: {
          id: 'sp_' + Math.random().toString(36).slice(2, 10),
          name,
          skin_data: profile.skin_url,
          skin_model: profile.skin_model,
          cape_id: profile.cape?.id ?? null,
          cape_name: profile.cape?.name ?? null,
          cape_url: profile.cape?.url ?? null,
          account_id: activeAccount()?.id ?? null,
          created_at: new Date().toISOString(),
        }
      });
      profileNameInput = '';
      savingCurrentLook = false;
      toast(`Saved "${name}" to wardrobe`);
    } catch (e) {
      toast(errorText(e), 'error');
    }
  }

  async function equipProfile(p: SkinProfile) {
    applyingProfileId = p.id;
    try {
      profile = await invoke<PlayerProfile>('apply_skin_profile', { id: p.id });
      app.skinVersion++;
      toast(`Equipped "${p.name}"`);
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      applyingProfileId = null;
    }
  }

  async function deleteProfile(id: string, name: string) {
    try {
      skinProfiles = await invoke<SkinProfile[]>('delete_skin_profile', { id });
      toast(`Removed "${name}"`);
    } catch (e) {
      toast(errorText(e), 'error');
    }
  }

  async function createProfileFromFile() {
    if (!newProfileFile) return toast('Please select a skin PNG file', 'error');
    const name = newProfileName.trim() || 'Custom Look';
    const buf = new Uint8Array(await newProfileFile.arrayBuffer());
    let bin = '';
    for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
    const base64Data = 'data:image/png;base64,' + btoa(bin);
    const chosenCape = profile?.available_capes.find(c => c.id === newProfileCapeId);
    try {
      skinProfiles = await invoke<SkinProfile[]>('save_skin_profile', {
        profile: {
          id: 'sp_' + Math.random().toString(36).slice(2, 10),
          name,
          skin_data: base64Data,
          skin_model: newProfileModel,
          cape_id: newProfileCapeId,
          cape_name: chosenCape?.name ?? null,
          cape_url: chosenCape?.url ?? null,
          account_id: activeAccount()?.id ?? null,
          created_at: new Date().toISOString(),
        }
      });
      newProfileModal = false;
      newProfileName = '';
      newProfileFile = null;
      toast(`Saved "${name}" to wardrobe`);
    } catch (e) {
      toast(errorText(e), 'error');
    }
  }

  // ---- storage ----
  let storage = $state<{ shared: number; runtimes: number; instances: [string, number][] } | null>(null);
  function loadStorage() {
    invoke<typeof storage>('storage_info').then((x) => (storage = x)).catch(() => {});
  }
  $effect(() => {
    if (app.settingsTab === 'storage') loadStorage();
  });
  const storageTotal = $derived(storage ? storage.shared + storage.runtimes + storage.instances.reduce((a, [, n]) => a + n, 0) : 0);
  const instName = (id: string) => instances().find((i) => i.id === id)?.name ?? id;

  // ---- java ----
  let javaMajor = $state<number | null>(null);
  async function browseJava() {
    const p = await pickFile('Choose your java executable', [{ name: 'Java', extensions: ['exe', ''] }]);
    if (!p) return;
    s.java_path = p;
    saveSettings();
    javaMajor = await invoke<number | null>('detect_java', { path: p });
  }

  // ---- launcher ----
  let panelInput = $state(app.panelUrl ?? '');
  let panelBusy = $state(false);
  async function changePanel() {
    panelBusy = true;
    try {
      await connect(panelInput);
      toast('Connected');
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      panelBusy = false;
    }
  }

  let checking = $state(false);
  let installing = $state(false);
  async function checkUpdate() {
    checking = true;
    try {
      app.update = await invoke<UpdateInfo | null>('check_update');
      if (!app.update) toast("You're on the latest version");
    } catch (e) {
      toast(errorText(e), 'error');
    } finally {
      checking = false;
    }
  }
  async function installUpdate() {
    if (!app.update) return;
    installing = true;
    try {
      await invoke('install_update', { url: app.update.url });
    } catch (e) {
      toast(errorText(e), 'error');
      installing = false;
    }
  }

  const gcPresets: { id: Gc; title: string; text: string; icon: any }[] = [
    { id: 'g1', title: 'Balanced', text: 'Tuned G1 — smooth on most PCs', icon: Gauge },
    { id: 'zgc', title: 'Low latency', text: 'ZGC — fewest stutters, needs more RAM', icon: Zap },
    { id: 'default', title: 'Java default', text: 'No tuning flags', icon: Feather },
  ];
  const accents = ['#7c5cff', '#22d3ee', '#22c55e', '#f59e0b', '#f97316', '#ef4444', '#ec4899', '#a855f7', '#e5e7eb'];
</script>

<svelte:window onkeydown={capturing ? capture : undefined} onmousedown={capturing ? capture : undefined} />

<div class="settings">
  <nav class="tabs glass">
    <h2>Settings</h2>
    {#each tabs as t}
      <button class="tab" class:active={app.settingsTab === t.id} onclick={() => (app.settingsTab = t.id)}>
        <t.icon size={17} /> {t.label}
        {#if t.id === 'about' && app.update}<span class="dot"></span>{/if}
      </button>
    {/each}
  </nav>

  <div class="content">
    {#key app.settingsTab}
      <div class="pane">
        {#if app.settingsTab === 'account'}
          <h1>Accounts</h1>
          <p class="lead">Switch between accounts or add another one.</p>
          <div class="card glass list">
            {#each app.accounts as a (a.id)}
              <div class="acc" class:current={a.id === app.active}>
                <Avatar account={a} size={2.6} />
                <div class="who"><strong>{a.username}</strong><span class="tiny muted">{kindLabel[a.kind]}{a.role === 'admin' ? ' · Admin' : ''}</span></div>
                {#if a.id === app.active}
                  <span class="active-badge"><Check size={13} /> Active</span>
                {:else}
                  <button class="sm" onclick={() => switchAccount(a.id)}>Use</button>
                {/if}
                <button class="ghost icon" title="Sign out" aria-label="Sign out {a.username}" onclick={() => removeAccount(a.id)}><LogOut size={16} /></button>
              </div>
            {:else}
              <p class="muted small">No accounts yet.</p>
            {/each}
            <button class="add" onclick={() => (app.addAccount = true)}><Plus size={16} /> Add account</button>
          </div>
          {#if serverAccount}
            <div class="card glass col">
              <h3>Connections → Discord</h3>
              <p class="muted small">Connect your Discord account to sign in and show your Discord identity in Velora.</p>
              {#if discordConnection}
                <p>Connected as <strong>{discordConnection.name}</strong></p>
                <button disabled={discordBusy} onclick={unlinkDiscord}>Disconnect Discord</button>
              {:else}
                <button disabled={discordBusy} onclick={linkDiscord}>{discordBusy ? 'Waiting for Discord…' : 'Connect Discord'}</button>
              {/if}
              {#if discordInvite}
                <button class="primary" onclick={() => openUrl(discordInvite)}>Join the community Discord</button>
              {/if}
            </div>
          {/if}
        {:else if app.settingsTab === 'skin'}
          <h1>Skin & cape</h1>
          <form class="card glass col" onsubmit={(e) => { e.preventDefault(); renameAccount(); }}>
            <h3>Username</h3>
            <p class="muted small">Your UUID, inventory and server progress stay with your account. Close Minecraft first. Previous names remain reserved to you.</p>
            <label>New username<input bind:value={newUsername} required minlength="3" maxlength="16" pattern="[A-Za-z0-9_]+" autocomplete="username" /></label>
            <label>Current password<input bind:value={renamePassword} required type="password" autocomplete="current-password" /></label>
            <button disabled={renaming || app.running.length > 0} type="submit">{renaming ? 'Updating…' : 'Change username'}</button>
          </form>
          <p class="lead">Stored on {b?.name ?? 'the server'} — visible to everyone on servers that use its sign-in.</p>
          {#if profileError}
            <div class="card glass"><p class="warn small">{profileError}</p></div>
          {:else if !profile}
            <div class="card glass center"><LoaderCircle size={20} class="spin" /></div>
          {:else}
            <div class="card glass skin-card" class:busy={skinBusy}>
              <div class="preview">
                <div class="figure"><SkinView skin={profile.skin_url} cape={profile.cape?.url ?? null} slim={profile.skin_model === 'slim'} side="front" scale={7} /><span class="tiny muted">Front</span></div>
                <div class="figure"><SkinView skin={profile.skin_url} cape={profile.cape?.url ?? null} slim={profile.skin_model === 'slim'} side="back" scale={7} /><span class="tiny muted">Back</span></div>
              </div>
              <div class="col grow skin-controls">
                <label class="drop" class:over={dragging} ondragover={(e) => { e.preventDefault(); dragging = true; }} ondragleave={() => (dragging = false)} ondrop={onDrop}>
                  <Upload size={18} />
                  <span><strong>Upload a skin</strong><span class="tiny muted">PNG, 64×64 or 64×32 — drop it here or click to browse</span></span>
                  <input type="file" accept="image/png" onchange={(e) => { uploadSkin(e.currentTarget.files?.[0]); e.currentTarget.value = ''; }} />
                </label>
                <label class="field">Arm style
                  <div class="segmented full">
                    {#each [['classic', 'Classic (4px)'], ['slim', 'Slim (3px)']] as [id, label]}
                      <button class:active={profile.skin_model === id} disabled={skinBusy} onclick={() => profile!.skin_model !== id && skinAction('set_skin_model', { model: id })}>{label}</button>
                    {/each}
                  </div>
                </label>
                {#if profile.skin_url}
                  <button class="sm link-btn" disabled={skinBusy} onclick={() => skinAction('delete_skin', {}, 'Skin reset')}><RotateCcw size={14} /> Reset to default</button>
                {/if}
              </div>
            </div>
            <div class="card glass col">
              <span class="label">Cape</span>
              {#if profile.available_capes.length}
                <div class="capes">
                  <button class="cape" class:active={!profile.cape} disabled={skinBusy} onclick={() => profile!.cape && skinAction('set_cape', { capeId: null })}>
                    <span class="cape-img none"><Ban size={18} /></span>No cape
                  </button>
                  {#each profile.available_capes as c (c.id)}
                    <button class="cape" class:active={profile.cape?.id === c.id} disabled={skinBusy} onclick={() => profile!.cape?.id !== c.id && skinAction('set_cape', { capeId: c.id })}>
                      <span class="cape-img" style:background-image={c.url ? `url(${c.url})` : undefined}></span>{c.name}
                    </button>
                  {/each}
                </div>
              {:else}
                <p class="muted small">No capes are available to you yet. Server staff can hand them out from the panel.</p>
              {/if}
            </div>

            <div class="card glass col">
              <div class="row align-center justify-between">
                <div>
                  <h3 class="row gap-sm"><Sparkles size={18} class="accent-icon" /> Saved Profiles & Wardrobe</h3>
                  <p class="muted small">Save and swap between multiple skin + cape combinations in one click.</p>
                </div>
                <div class="row gap-sm">
                  {#if !savingCurrentLook}
                    <button class="sm" onclick={() => (savingCurrentLook = true)}>
                      <BookmarkPlus size={15} /> Save current look
                    </button>
                    <button class="sm ghost" onclick={() => (newProfileModal = true)}>
                      <Plus size={15} /> New preset
                    </button>
                  {/if}
                </div>
              </div>

              {#if savingCurrentLook}
                <form class="save-look-box row gap-sm" onsubmit={(e) => { e.preventDefault(); saveCurrentLook(); }}>
                  <input bind:value={profileNameInput} placeholder="Profile name (e.g. PvP, Casual, Outfit 1)" required />
                  <button type="submit" class="sm primary">Save</button>
                  <button type="button" class="sm ghost" onclick={() => { savingCurrentLook = false; profileNameInput = ''; }}>Cancel</button>
                </form>
              {/if}

              {#if skinProfiles.length}
                <div class="profiles-grid">
                  {#each skinProfiles as sp (sp.id)}
                    {@const isCurrent = !!sp.skin_data && sp.skin_data === profile.skin_url && profile.skin_model === sp.skin_model && sp.cape_id === (profile.cape?.id ?? null)}
                    <div class="profile-card" class:active-look={isCurrent}>
                      <div class="profile-preview">
                        <SkinView skin={sp.skin_data} cape={sp.cape_url} slim={sp.skin_model === 'slim'} side="front" scale={3.5} />
                      </div>
                      <div class="profile-info">
                        <div class="row align-center justify-between">
                          <strong class="profile-name" title={sp.name}>{sp.name}</strong>
                          {#if isCurrent}
                            <span class="active-badge tiny-badge"><Check size={11} /> Active</span>
                          {/if}
                        </div>
                        <div class="profile-meta tiny muted">
                          <span>{sp.skin_model === 'slim' ? 'Slim' : 'Classic'}</span>
                          <span>·</span>
                          <span>{sp.cape_name ?? 'No cape'}</span>
                        </div>
                        <div class="profile-actions row gap-xs">
                          <button
                            class="sm grow"
                            disabled={isCurrent || applyingProfileId === sp.id}
                            onclick={() => equipProfile(sp)}
                          >
                            {#if applyingProfileId === sp.id}
                              <LoaderCircle size={13} class="spin" /> Equipping…
                            {:else if isCurrent}
                              Equipped
                            {:else}
                              Equip
                            {/if}
                          </button>
                          <button class="sm ghost icon" title="Delete preset" onclick={() => deleteProfile(sp.id, sp.name)}>
                            <Trash2 size={13} />
                          </button>
                        </div>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <p class="muted small">No saved profiles yet. Click "Save current look" to bookmark your current skin and cape, or "New preset" to add one.</p>
              {/if}
            </div>

            {#if newProfileModal}
              <Modal bind:open={newProfileModal} title="Create Skin & Cape Preset" width={28}>
                <form class="col gap-sm" onsubmit={(e) => { e.preventDefault(); createProfileFromFile(); }}>
                  <label class="field">
                    <span>Preset name</span>
                    <input bind:value={newProfileName} placeholder="e.g. Red Hoodie, PvP" required />
                  </label>
                  <label class="field">
                    <span>Skin file (PNG)</span>
                    <input type="file" accept="image/png" required onchange={(e) => (newProfileFile = e.currentTarget.files?.[0] ?? null)} />
                  </label>
                  <label class="field">
                    <span>Arm model</span>
                    <div class="segmented full">
                      <button type="button" class:active={newProfileModel === 'classic'} onclick={() => (newProfileModel = 'classic')}>Classic (4px)</button>
                      <button type="button" class:active={newProfileModel === 'slim'} onclick={() => (newProfileModel = 'slim')}>Slim (3px)</button>
                    </div>
                  </label>
                  <label class="field">
                    <span>Cape</span>
                    <select bind:value={newProfileCapeId}>
                      <option value={null}>No cape</option>
                      {#each profile.available_capes as c (c.id)}
                        <option value={c.id}>{c.name}</option>
                      {/each}
                    </select>
                  </label>
                  <div class="row justify-end gap-sm" style="margin-top: 0.8rem;">
                    <button type="button" class="ghost" onclick={() => (newProfileModal = false)}>Cancel</button>
                    <button type="submit" class="primary">Save preset</button>
                  </div>
                </form>
              </Modal>
            {/if}
          {/if}
        {:else if app.settingsTab === 'game'}
          <h1>Game</h1>
          <p class="lead">Applies to every instance unless you override it in the instance's settings.</p>
          <div class="card glass col">
            <label class="field">
              <span class="row">Memory (RAM) <span class="spacer"></span><strong class="val">{s.memory_max_mb ? gb(s.memory_max_mb) : 'Server default'}</strong></span>
              <input type="range" min="0" max={maxRam} step="512" bind:value={s.memory_max_mb} oninput={saveSettings} />
              <div class="scale tiny muted"><span>Default</span><span>Recommended for your PC: {gb(recommended)}</span><span>{gb(maxRam)}</span></div>
              {#if s.memory_max_mb > ram * 0.75}<span class="warn tiny">That's most of your {gb(ram)} — your PC may slow down.</span>{/if}
            </label>
            <button class="sm link-btn" onclick={() => { s.memory_max_mb = recommended; saveSettings(); }}>Use recommended</button>
          </div>
          <div class="card glass col">
            <Toggle bind:checked={s.custom_resolution} onchange={saveSettings} label="Custom window size" help="Start Minecraft at a specific resolution." />
            {#if s.custom_resolution}
              <div class="row">
                <label class="field grow">Width<input type="number" min="640" bind:value={s.width} oninput={saveSettings} /></label>
                <span class="x">×</span>
                <label class="field grow">Height<input type="number" min="480" bind:value={s.height} oninput={saveSettings} /></label>
              </div>
            {/if}
            <Toggle bind:checked={s.fullscreen} onchange={saveSettings} label="Start in fullscreen" />
          </div>
          <div class="card glass col">
            <label class="field">When the game starts
              <div class="segmented full">
                {#each [['minimize', 'Minimise'], ['hide', 'Hide'], ['keep', 'Stay open'], ['close', 'Close launcher']] as [id, label]}
                  <button class:active={s.after_launch === id} onclick={() => { s.after_launch = id as typeof s.after_launch; saveSettings(); }}>{label}</button>
                {/each}
              </div>
              <span class="help">{s.after_launch === 'close' ? 'Frees the most memory, but game logs and crash help are unavailable.' : s.after_launch === 'hide' ? 'The launcher comes back when you quit the game.' : 'Minimising keeps the console and crash help available.'}</span>
            </label>
            <Toggle bind:checked={s.show_console} onchange={saveSettings} label="Open the console when the game starts" />
          </div>
        {:else if app.settingsTab === 'java'}
          <h1>Java & performance</h1>
          <p class="lead">The launcher downloads the right Java for each Minecraft version automatically.</p>
          <div class="card glass col">
            <span class="label">Garbage collector</span>
            <div class="presets">
              {#each gcPresets as g}
                <button class="preset" class:active={s.gc === g.id} onclick={() => { s.gc = g.id; saveSettings(); }}>
                  <g.icon size={20} /><strong>{g.title}</strong><span class="tiny muted">{g.text}</span>
                </button>
              {/each}
            </div>
          </div>
          {#if advanced}
            <div class="card glass col">
              <label class="field">Java executable
                <div class="row">
                  <input value={s.java_path ?? ''} placeholder="Automatic (recommended)" readonly />
                  <button class="sm" onclick={browseJava}><FileCode size={14} /> Browse</button>
                  {#if s.java_path}<button class="sm ghost" onclick={() => { s.java_path = null; javaMajor = null; saveSettings(); }}><RotateCcw size={14} /> Automatic</button>{/if}
                </div>
                {#if javaMajor}<span class="help">Detected Java {javaMajor}.</span>{/if}
              </label>
              <label class="field">JVM arguments <span class="help">Added to every instance. Leave empty unless you know what you're doing.</span>
                <input class="mono" bind:value={s.jvm_args} oninput={saveSettings} placeholder="-XX:+UseStringDeduplication" />
              </label>
            </div>
          {/if}
          <div class="card glass col">
            <label class="field">
              <span class="row">Parallel downloads <span class="spacer"></span><strong class="val">{s.concurrent_downloads}</strong></span>
              <input type="range" min="2" max="32" bind:value={s.concurrent_downloads} oninput={saveSettings} />
              <span class="help">Lower this on slow or unstable connections.</span>
            </label>
          </div>
        {:else if app.settingsTab === 'controls'}
          <h1>Controls</h1>
          <p class="lead">Customize keyboard, mouse, and movement controls across your instances.</p>
          <div class="card glass col">
            <Toggle bind:checked={s.vanilla_controls_enabled} onchange={saveSettings} label="Apply movement and mouse settings" help="Sets vanilla Auto-Jump and mouse sensitivity before launch." />
            {#if s.vanilla_controls_enabled}
              <Toggle bind:checked={s.auto_jump} onchange={saveSettings} label="Auto-Jump" />
              <label class="field"><span class="row">Mouse sensitivity <span class="spacer"></span><strong>{Math.round(s.sensitivity * 200)}%</strong></span>
                <input type="range" min="0" max="1" step="0.01" bind:value={s.sensitivity} oninput={saveSettings} />
              </label>
            {/if}
          </div>
          <div class="card glass col">
            <Toggle bind:checked={s.keybinds_enabled} onchange={saveSettings} label="Apply these keybinds to the game" help="Writes to options.txt before launch (Minecraft 1.13 and newer)." />
          </div>
          <div class="binds" class:disabled={!s.keybinds_enabled}>
            {#each BIND_GROUPS as group}
              <div class="card glass">
                <h3 class="group">{group.title}</h3>
                {#each group.binds as bind}
                  {@const key = binds[bind.id]}
                  <div class="bind">
                    <span>{bind.label}</span>
                    <button class="key" class:capturing={capturing === bind.id} class:conflict={conflicts.has(key)} class:changed={key !== bind.default}
                      onclick={(e) => { e.stopPropagation(); capturing = bind.id; }}>
                      {#if capturing === bind.id}<MousePointerClick size={13} /> Press a key…{:else}{keyLabel(key)}{/if}
                    </button>
                  </div>
                {/each}
              </div>
            {/each}
          </div>
          <div class="row">
            <button class="sm" onclick={() => { s.keybinds = {}; saveSettings(); }}><RotateCcw size={14} /> Reset to defaults</button>
            <span class="tiny muted">Press Esc while choosing to unbind. Shortcut in the launcher: <kbd>Ctrl</kbd> + <kbd>Enter</kbd> to play.</span>
          </div>
        {:else if app.settingsTab === 'companion'}
          <CompanionSettings />
        {:else if app.settingsTab === 'appearance'}
          <h1>Appearance</h1>
          <p class="lead">Make the launcher feel like yours{themeAllowed ? '' : ' — your server has locked the theme'}.</p>
          {#if themeAllowed}
            <div class="card glass col">
              <div class="segmented">
                <button class:active={s.theme_mode === 'server'} onclick={() => { s.theme_mode = 'server'; saveSettings(); }}>Server theme</button>
                <button class:active={s.theme_mode === 'custom'} onclick={() => { s.theme_mode = 'custom'; s.accent ??= accents[0]; saveSettings(); }}>Custom accent</button>
              </div>
              {#if s.theme_mode === 'custom'}
                <div class="swatches">
                  {#each accents as c}
                    <button class="swatch" class:on={s.accent === c} style:background={c} aria-label="Accent {c}" onclick={() => { s.accent = c; saveSettings(); }}>{#if s.accent === c}<Check size={15} />{/if}</button>
                  {/each}
                  <label class="swatch picker" title="Custom colour"><input type="color" value={s.accent ?? '#7c5cff'} oninput={(e) => { s.accent = (e.target as HTMLInputElement).value; saveSettings(); }} /></label>
                </div>
              {/if}
            </div>
          {/if}
          <div class="card glass col">
            <Toggle bind:checked={() => s.glass ?? b?.glass ?? true, (v) => (s.glass = v)} onchange={saveSettings}
              label="Frosted glass effects" help="Turn off for better performance on older graphics cards." />
            <Toggle bind:checked={s.background_video} onchange={saveSettings} label="Animated backgrounds" help="Video backgrounds pause automatically while you play." />
            <Toggle bind:checked={s.reduce_motion} onchange={saveSettings} label="Reduce motion" help="Turns off animations and transitions." />
            <label class="field">
              <span class="row">Interface size <span class="spacer"></span><strong class="val">{s.ui_scale}%</strong></span>
              <input type="range" min="80" max="130" step="5" bind:value={s.ui_scale} oninput={saveSettings} />
            </label>
          </div>
        {:else if app.settingsTab === 'launcher'}
          <h1>Launcher</h1>
          <p class="lead">Connection and behaviour.</p>
          <div class="card glass col">
            <label class="field">Server panel
              {#if app.boot?.app.panel_locked}
                <input value={app.panelUrl} readonly />
                <span class="help">This launcher is built for this server.</span>
              {:else}
                <div class="row">
                  <input bind:value={panelInput} placeholder="panel.yourserver.com" />
                  <button class="sm" disabled={panelBusy || !panelInput || panelInput === app.panelUrl} onclick={changePanel}>{#if panelBusy}<LoaderCircle class="spin" size={14} />{:else}<Link2 size={14} />{/if} Connect</button>
                </div>
              {/if}
            </label>
            <button class="sm link-btn" onclick={() => refresh().then(() => toast('Refreshed')).catch((e) => toast(errorText(e), 'error'))}><RefreshCw size={14} /> Refresh from server</button>
          </div>
          <div class="card glass col">
            <Toggle bind:checked={s.check_updates} onchange={saveSettings} label="Check for launcher updates" />
            <Toggle bind:checked={s.show_social_sidebar} onchange={saveSettings} label="Show social sidebar" help="Friends, activity and message previews beside every page." />
            <Toggle bind:checked={s.show_news} onchange={saveSettings} label="Show news and announcements" help="Hide the news feed on the launcher home screen." />
            <p class="help">Official server accounts report launcher activity and gameplay to the panel, linked to your permanent UUID. Chat is counted; message text, passwords and command arguments are not collected.</p>
          </div>
        {:else if app.settingsTab === 'storage'}
          <h1>Storage</h1>
          <p class="lead">Game files are shared between instances, so extra instances of the same version take almost no space.</p>
          {#if storage}
            <div class="card glass col">
              <div class="row"><strong class="big">{bytes(storageTotal)}</strong><span class="muted small">used in total</span><span class="spacer"></span>
                <button class="sm" onclick={() => invoke('open_folder', { kind: 'data' })}><FolderOpen size={14} /> Open folder</button>
              </div>
              <div class="usage">
                <span style:flex={storage.shared} class="u1" title="Game files"></span>
                <span style:flex={storage.runtimes} class="u2" title="Java"></span>
                {#each storage.instances as [, n]}<span style:flex={n} class="u3"></span>{/each}
              </div>
              <div class="legend tiny">
                <span><i class="u1"></i> Game files {bytes(storage.shared)}</span>
                <span><i class="u2"></i> Java {bytes(storage.runtimes)}</span>
                <span><i class="u3"></i> Instances {bytes(storage.instances.reduce((a, [, n]) => a + n, 0))}</span>
              </div>
            </div>
            <div class="card glass list">
              {#each storage.instances as [id, n]}
                <div class="acc">
                  <div class="who"><strong>{instName(id)}</strong><span class="tiny muted">{bytes(n)} · worlds, mods & settings</span></div>
                  <button class="sm" onclick={() => invoke('open_folder', { kind: 'instance', instanceId: id })}><FolderOpen size={14} /></button>
                  <button class="sm danger" onclick={async () => { try { await invoke('delete_instance_data', { instanceId: id }); loadStorage(); toast('Deleted'); } catch (e) { toast(errorText(e), 'error'); } }}><Trash2 size={14} /> Delete</button>
                </div>
              {:else}
                <p class="muted small">No instances downloaded yet.</p>
              {/each}
            </div>
            <button class="sm link-btn" onclick={async () => { await invoke('clear_cache'); toast('Cache cleared'); loadStorage(); }}><Trash2 size={14} /> Clear download cache</button>
          {:else}
            <p class="muted row"><LoaderCircle class="spin" size={16} /> Measuring…</p>
          {/if}
        {:else if app.settingsTab === 'about'}
          <h1>About</h1>
          <div class="card glass col about">
            <div class="row">
              {#if b?.about?.icon_url}<img class="about-icon" src={abs(b.about.icon_url)} alt="" />{:else}<div class="mark"></div>{/if}
              <div><strong class="big">{b?.about?.title || `${b?.name ?? 'Velora'} Launcher`}</strong><p class="muted small">Installed launcher v{app.boot?.app.version} · {app.boot?.app.os} {app.boot?.app.arch}</p></div>
            </div>
            {#if b?.about?.body}<p class="about-body selectable">{b.about.body}</p>{/if}
            {#if b?.about?.links?.length}
              <div class="row about-links">
                {#each b.about.links as link}
                  {#if link.label && link.url}<button class="sm" onclick={() => openUrl(link.url)}>{link.label}</button>{/if}
                {/each}
              </div>
            {/if}
            {#if app.update}
              <div class="update">
                <Download size={20} />
                <div class="grow"><strong>Version {app.update.version} is available</strong><p class="tiny muted">{bytes(app.update.size)}</p></div>
                <button class="primary" disabled={installing} onclick={installUpdate}>{#if installing}<LoaderCircle class="spin" size={16} /> Downloading…{:else}Update now{/if}</button>
              </div>
              {#if app.update.notes}<pre class="notes selectable">{app.update.notes}</pre>{/if}
            {/if}
            <button class="sm link-btn" disabled={checking || installing} onclick={checkUpdate}>{#if checking}<LoaderCircle class="spin" size={14} />{:else}<RefreshCw size={14} />{/if} Check for updates</button>
            <p class="tiny muted">Built with Tauri, Rust and Svelte. Minecraft is a trademark of Mojang Studios; this launcher is not affiliated with Mojang or Microsoft.</p>
          </div>
        {/if}
      </div>
    {/key}
  </div>
</div>

<style>
  .settings { height: 100%; display: flex; min-height: 0; }
  .tabs { width: 13.5rem; flex-shrink: 0; padding: 1.2rem 0.6rem; display: flex; flex-direction: column; gap: 0.1rem; border: none; border-right: 1px solid var(--line); border-radius: 0; background: transparent; }
  .tabs h2 { font-size: 0.8rem; font-weight: 560; color: var(--muted); padding: 0 0.75rem 0.7rem; }
  .tab { justify-content: flex-start; gap: 0.7rem; background: transparent; border-color: transparent; color: var(--muted); padding: 0.55rem 0.75rem; font-weight: 500; position: relative; }
  .tab:hover { color: var(--text); }
  .tab.active { background: color-mix(in srgb, var(--text) 7%, transparent); color: var(--text); }
  .tab.active :global(svg) { color: var(--text); }
  .dot { width: 0.45rem; height: 0.45rem; border-radius: 50%; background: var(--accent); margin-left: auto; }
  .content { flex: 1; overflow-y: auto; min-width: 0; padding: 1.2rem 2rem 2.5rem; }
  .pane { max-width: 42rem; display: flex; flex-direction: column; gap: 0.9rem; animation: fade 0.2s ease; }
  h1 { font-size: 1.4rem; font-weight: 600; }
  .lead { color: var(--muted); margin-top: -0.5rem; line-height: 1.5; font-size: 0.9rem; margin-bottom: 0.3rem; }
  .card { padding: 1rem 1.1rem; background: var(--surface); }
  .card.col { gap: 1rem; }
  .about-icon { width: 3rem; height: 3rem; object-fit: contain; border-radius: 0.6rem; }
  .about-body { white-space: pre-line; line-height: 1.6; }
  .about-links { flex-wrap: wrap; }
  .list { display: flex; flex-direction: column; gap: 0.4rem; padding: 0.6rem; }
  .acc { display: flex; align-items: center; gap: 0.8rem; padding: 0.6rem 0.7rem; border-radius: var(--radius-sm); }
  .acc.current { background: color-mix(in srgb, var(--text) 5%, transparent); }
  .who { flex: 1; display: flex; flex-direction: column; gap: 0.1rem; min-width: 0; }
  .active-badge { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.78rem; font-weight: 650; color: var(--accent); padding: 0.3rem 0.6rem; }
  .add { border-style: dashed; background: transparent; margin: 0.2rem; }
  .val { color: var(--text); font-weight: 560; font-variant-numeric: tabular-nums; }
  .scale { display: flex; justify-content: space-between; }
  .warn { color: var(--warn); }
  .link-btn { align-self: flex-start; }
  .grow { flex: 1; }
  .x { color: var(--muted); padding-top: 1.4rem; }
  .full { width: 100%; }
  .full button { flex: 1; }
  .label { font-size: 0.85rem; font-weight: 520; color: color-mix(in srgb, var(--text) 80%, transparent); }
  .presets { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.6rem; }
  .preset { flex-direction: column; align-items: flex-start; gap: 0.3rem; padding: 0.9rem; white-space: normal; text-align: left; background: color-mix(in srgb, var(--bg) 50%, transparent); }
  .preset :global(svg) { color: var(--muted); margin-bottom: 0.2rem; }
  .preset.active { border-color: color-mix(in srgb, var(--accent) 60%, transparent); background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .preset.active :global(svg) { color: var(--accent); }
  .binds { display: grid; grid-template-columns: 1fr 1fr; gap: 0.8rem; transition: opacity 0.2s; }
  .binds.disabled { opacity: 0.55; }
  .group { font-size: 0.8rem; font-weight: 560; color: var(--muted); margin-bottom: 0.4rem; }
  .bind { display: flex; align-items: center; justify-content: space-between; gap: 0.6rem; padding: 0.3rem 0; font-size: 0.88rem; }
  .key { min-width: 7.5rem; padding: 0.4rem 0.7rem; font-size: 0.82rem; font-family: var(--mono); font-weight: 600; background: color-mix(in srgb, var(--bg) 60%, transparent); }
  .key.changed { border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
  .key.conflict { border-color: color-mix(in srgb, var(--warn) 60%, transparent); color: var(--warn); }
  .key.capturing { border-color: var(--accent); color: var(--accent); font-family: var(--font); }
  kbd { font-family: var(--mono); font-size: 0.72rem; padding: 0.1rem 0.35rem; border-radius: 0.3rem; border: 1px solid var(--line-strong); background: color-mix(in srgb, var(--text) 6%, transparent); }
  .swatches { display: flex; gap: 0.55rem; flex-wrap: wrap; }
  .swatch { width: 2.3rem; height: 2.3rem; border-radius: 50%; padding: 0; border: 2px solid transparent; color: white; box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.15); }
  .swatch.on { border-color: var(--text); }
  .picker { background: conic-gradient(red, yellow, lime, cyan, blue, magenta, red); position: relative; overflow: hidden; cursor: pointer; display: block; }
  .picker input { opacity: 0; width: 100%; height: 100%; cursor: pointer; }
  .big { font-size: 1.15rem; font-weight: 600; }
  .usage { display: flex; height: 0.8rem; border-radius: 99rem; overflow: hidden; gap: 2px; background: color-mix(in srgb, var(--text) 8%, transparent); }
  .usage span { min-width: 2px; }
  .u1 { background: var(--accent); }
  .u2 { background: color-mix(in srgb, var(--accent) 50%, var(--text)); }
  .u3 { background: var(--success); }
  .legend { display: flex; gap: 1.2rem; color: var(--muted); }
  .legend i { display: inline-block; width: 0.6rem; height: 0.6rem; border-radius: 0.2rem; margin-right: 0.3rem; vertical-align: -0.05rem; }
  .about .mark { width: 2.8rem; height: 2.8rem; border-radius: 50%; border: 3px solid var(--accent); flex-shrink: 0; position: relative; }
  .about .mark::after { content: ''; position: absolute; inset: 0.55rem; border-radius: 50%; background: var(--accent); }
  .update { display: flex; align-items: center; gap: 0.9rem; padding: 0.9rem 1rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--accent) 12%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); }
  .update :global(svg) { color: var(--accent); }
  .notes { margin: 0; white-space: pre-wrap; font-family: var(--font); font-size: 0.84rem; color: var(--muted); max-height: 12rem; overflow: auto; }
  .center { display: grid; place-items: center; min-height: 8rem; }
  .skin-card { display: flex; gap: 1.6rem; align-items: center; transition: opacity 0.15s; }
  .skin-card.busy { opacity: 0.6; }
  .preview { display: flex; gap: 1rem; padding: 1rem 1.2rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--bg) 60%, transparent); }
  .figure { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; }
  .skin-controls { display: flex; flex-direction: column; gap: 1rem; }
  .drop { position: relative; display: flex; align-items: center; gap: 0.8rem; padding: 0.9rem 1rem; border: 1px dashed var(--line-strong); border-radius: var(--radius-sm); cursor: pointer; transition: border-color 0.15s, background 0.15s; }
  .drop:hover, .drop.over { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 7%, transparent); }
  .drop > span { display: flex; flex-direction: column; gap: 0.15rem; }
  .drop :global(svg) { color: var(--accent); flex-shrink: 0; }
  .drop input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .capes { display: grid; grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr)); gap: 0.6rem; }
  .cape { flex-direction: column; gap: 0.5rem; padding: 0.8rem 0.5rem 0.6rem; font-size: 0.8rem; background: color-mix(in srgb, var(--bg) 50%, transparent); white-space: normal; }
  .cape.active { border-color: color-mix(in srgb, var(--accent) 60%, transparent); background: color-mix(in srgb, var(--accent) 8%, transparent); }
  /* Cape texture: the outer face is the 10×16 region at (1,1) of a 64×32 sheet. */
  .cape-img { width: 2.5rem; height: 4rem; border-radius: 0.2rem; image-rendering: pixelated; background-color: color-mix(in srgb, var(--text) 8%, transparent); background-size: 640% 200%; background-position: 1.85% 6.25%; background-repeat: no-repeat; }
  .cape-img.none { display: grid; place-items: center; color: var(--muted); }
  .align-center { align-items: center; }
  .justify-between { justify-content: space-between; }
  .justify-end { justify-content: flex-end; }
  .gap-xs { gap: 0.35rem; }
  .gap-sm { gap: 0.6rem; }
  :global(.accent-icon) { color: var(--accent); }
  .save-look-box { padding: 0.6rem 0.8rem; background: color-mix(in srgb, var(--bg) 60%, transparent); border-radius: var(--radius-sm); border: 1px solid var(--line); }
  .save-look-box input { flex: 1; }
  .profiles-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(13rem, 1fr)); gap: 0.8rem; }
  .profile-card { display: flex; gap: 0.8rem; align-items: center; padding: 0.75rem 0.85rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--bg) 50%, transparent); border: 1px solid var(--line); transition: border-color 0.15s, background 0.15s; }
  .profile-card:hover { border-color: var(--line-strong); background: color-mix(in srgb, var(--bg) 70%, transparent); }
  .profile-card.active-look { border-color: color-mix(in srgb, var(--accent) 60%, transparent); background: color-mix(in srgb, var(--accent) 8%, transparent); }
  .profile-preview { flex-shrink: 0; padding: 0.4rem 0.5rem; background: color-mix(in srgb, var(--bg) 80%, transparent); border-radius: var(--radius-sm); display: grid; place-items: center; }
  .profile-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.35rem; }
  .profile-name { font-size: 0.88rem; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .profile-meta { display: flex; gap: 0.3rem; align-items: center; }
  .tiny-badge { padding: 0.15rem 0.4rem; font-size: 0.72rem; }
  .profile-actions { margin-top: 0.2rem; }
  @media (max-width: 1050px) { .binds { grid-template-columns: 1fr; } .tabs { width: 12rem; } .skin-card { flex-direction: column; align-items: stretch; } .preview { justify-content: center; } }
</style>
