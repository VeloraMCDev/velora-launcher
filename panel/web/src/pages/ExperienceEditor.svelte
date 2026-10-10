<script lang="ts">
  import { FEATURES, CORE_MODULES, corePolicy, coreFeatures, defaultExperience, preset, type CorePolicy, type Experience, type Feature } from '@velora/experience';
  import { experienceContext } from '../lib/experience.svelte';
  import { get, post, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import { route } from '../lib/router.svelte';
  import InstanceGroupLinks from '../components/InstanceGroupLinks.svelte';
  import DarknetCatalog from '../components/DarknetCatalog.svelte';
  import type { Branding } from '../lib/types';
  const initial = structuredClone($state.snapshot(experienceContext.instance?.experience ?? defaultExperience()));
  let form = $state<Experience>(initial);
  let core = $state<CorePolicy>(structuredClone({ ...corePolicy(), ...(initial.modules.velora_core as CorePolicy | undefined) }));
  let advanced = $state(JSON.stringify({ navigation: initial.navigation, widgets: initial.widgets, modules: initial.modules }, null, 2));
  let saving = $state(false);
  function applyPreset(kind: 'velora-smp') {
    const next = preset(kind);
    form = { ...next, branding: form.branding };
    core = corePolicy();
    advanced = JSON.stringify({ navigation: form.navigation, widgets: form.widgets, modules: form.modules }, null, 2);
  }
  async function activate() {
    saving = true;
    try {
      const result = await post<{ experience: Experience }>(`/api/admin/instances/${encodeURIComponent(route.instanceId!)}/velora-core/activate`, {});
      form = result.experience;
      core = structuredClone(form.modules.velora_core as CorePolicy);
      advanced = JSON.stringify({ navigation: form.navigation, widgets: form.widgets, modules: form.modules }, null, 2);
      if (experienceContext.instance) { experienceContext.instance.experience = form; experienceContext.instance.enabled = true; }
      toast('Velora SMP activated. Other experiences are hidden; their data is retained.');
    } catch (e) { toastError(e); } finally { saving = false; }
  }
  function toggle(feature: Feature) {
    form.features = form.features.includes(feature) ? form.features.filter(f => f !== feature) : [...form.features, feature];
  }
  async function ownBranding() {
    try {
      const platform = await get<Branding>('/api/admin/branding');
      form.branding = { ...platform, name: experienceContext.instance?.name ?? platform.name, news: [], links: [] };
    } catch (e) { toastError(e); }
  }
  async function save() {
    saving = true;
    try {
      const presentation = JSON.parse(advanced);
      const body = { ...$state.snapshot(form), navigation: presentation.navigation, widgets: presentation.widgets, modules: presentation.modules };
      if (form.kind === 'velora-smp') {
        body.modules = { ...body.modules, velora_core: $state.snapshot(core) };
        body.features = coreFeatures(core);
      }
      const saved = await put<Experience>(`/api/admin/instances/${encodeURIComponent(route.instanceId!)}/experience`, body);
      if (experienceContext.instance) experienceContext.instance.experience = saved;
      form = saved;
      toast('Experience saved');
    } catch (e) { toastError(e); } finally { saving = false; }
  }
</script>

<div class="editor">
  <p class="muted">{experienceContext.instance?.name} / Experience design</p>
  <h1>Make this instance its own world</h1>
  <p class="muted">Choose its systems, identity, navigation and overview. Accounts, updates and installation remain on the platform.</p>
  <section><h2>Experience template</h2><p>Applying a template changes the enabled systems and presentation. Existing game data is retained.</p>
    <div class="row"><button onclick={() => applyPreset('velora-smp')}>Velora SMP · Fabric 1.20.1</button></div>
    <label>Experience kind<input bind:value={form.kind} maxlength="48" /></label>
  </section>
  {#if form.kind === 'velora-smp'}
  <section><h2>Velora Core modules</h2><div class="features">
    {#each CORE_MODULES as module}<label class="feature"><input type="checkbox" bind:checked={core.modules[module]} /> {module.replaceAll('_', ' ')}</label>{/each}
  </div><p>Casino and factions require economy. Server policy controls access; client preferences can hide enabled modules. Casino wagers use in-game dollars only.</p>
    <h3>Global map layers</h3><div class="features">{#each ['players', 'claims', 'spawn', 'warps', 'homes', 'shops'] as layer}<label class="feature"><input type="checkbox" bind:checked={core.map_layers[layer as keyof CorePolicy['map_layers']]} /> {layer}</label>{/each}</div>
    <div class="features">
      <label>Starting balance (cents)<input type="number" min="0" step="1" bind:value={core.starting_balance_cents} /></label>
      <label>Virtual market fee (basis points)<input type="number" min="0" max="10000" step="1" bind:value={core.virtual_market_fee_bps} /></label>
      <label>Faction creation (cents)<input type="number" min="0" step="1" bind:value={core.faction_creation_cents} /></label>
      <label>Starting faction chunks<input type="number" min="1" step="1" bind:value={core.faction_base_claims} /></label>
      <label>Faction member limit<input type="number" min="1" step="1" bind:value={core.faction_max_members} /></label>
      <label>Daily cents per claimed chunk<input type="number" min="0" step="1" bind:value={core.upkeep_chunk_cents}/></label>
      <label>Daily cents per faction member<input type="number" min="0" step="1" bind:value={core.upkeep_member_cents}/></label>
      <label>Upkeep grace period (days)<input type="number" min="1" max="30" bind:value={core.upkeep_grace_days}/></label>
      <label>Repeat rivalry reward cooldown (minutes)<input type="number" min="1" max="43200" bind:value={core.rivalry_cooldown_minutes}/></label>
      <label>Personal vault unlock (cents)<input type="number" min="0" step="1" bind:value={core.vault_price_cents}/></label>
      <label>Physical shop map listing per day (cents)<input type="number" min="0" step="1" bind:value={core.shop_promotion_cents_per_day}/></label>
      <label><input type="checkbox" bind:checked={core.shop_requires_claim}/> Physical shops require the seller's faction claim</label>
      <label>Chunks per faction upgrade<input type="number" min="1" max="10000" bind:value={core.upgrade_claims_chunks}/></label>
      <label>Chunk upgrade (cents)<input type="number" min="0" step="1" bind:value={core.upgrade_claims_cents}/></label>
      <label>Maximum chunk upgrades<input type="number" min="0" max="100" bind:value={core.upgrade_claims_max}/></label>
      <label>Members per faction upgrade<input type="number" min="1" max="1000" bind:value={core.upgrade_members_slots}/></label>
      <label>Member upgrade (cents)<input type="number" min="0" step="1" bind:value={core.upgrade_members_cents}/></label>
      <label>Maximum member upgrades<input type="number" min="0" max="100" bind:value={core.upgrade_members_max}/></label>
      <label>Initial disconnected territories<input type="number" min="0" max="100" bind:value={core.faction_outpost_limit}/></label>
      <label>Territory upgrade (cents)<input type="number" min="0" step="1" bind:value={core.upgrade_outposts_cents}/></label>
      <label>Maximum territory upgrades<input type="number" min="0" max="100" bind:value={core.upgrade_outposts_max}/></label>
      <label>Faction vault page (cents)<input type="number" min="0" step="1" bind:value={core.faction_vault_cents}/></label>
      <label>Maximum faction vault pages<input type="number" min="0" max="9" bind:value={core.faction_vault_max}/></label>
    </div>
    <p>Activate this Fabric 1.20.1 installation as the sole visible experience. Activation restores default module settings and preserves existing files, accounts and game data.</p>
    <button disabled={saving} onclick={activate}>Activate Velora SMP and hide other experiences</button>
  </section>
  <DarknetCatalog />
  {:else}
  <section><h2>Enabled systems</h2><div class="features">
    {#each FEATURES as feature}<label class="feature"><input type="checkbox" checked={form.features.includes(feature)} onchange={() => toggle(feature)} /> {feature === 'guilds' ? 'Factions' : feature}</label>{/each}
  </div><p>Disabled systems are hidden from navigation and rejected by the instance API. Their stored data is preserved.</p></section>
  {/if}
  <section><h2>Instance identity</h2>
    {#if form.branding}
      <label>Name<input bind:value={form.branding.name} /></label>
      <label>Tagline<input bind:value={form.branding.tagline} /></label>
      <label>Logo URL<input bind:value={form.branding.logo_url} /></label>
      <label>Accent colour<input type="color" bind:value={form.branding.colors.accent} /></label>
      <label>Background image URL<input bind:value={form.branding.background.url} onchange={() => { if (form.branding) form.branding.background.kind = 'image'; }} /></label>
      <button onclick={() => (form.branding = null)}>Use platform theme</button>
    {:else}<p>This instance currently inherits the platform theme.</p><button onclick={ownBranding}>Create instance branding</button>{/if}
  </section>
  <section><h2>Navigation, widgets & module configuration</h2>
    <p>Configure built-in page labels and order, overview widgets, and module-owned data. Supported widget components: text, links, frontiers. Custom components require an installed experience module.</p>
    <label>Experience presentation JSON<textarea rows="18" bind:value={advanced} spellcheck="false"></textarea></label>
  </section>
  <InstanceGroupLinks />
  <button class="primary" disabled={saving} onclick={save}>{saving ? 'Saving…' : 'Save experience'}</button>
</div>

<style>
  .editor { max-width: 1000px; margin: auto; padding: 2rem; } section { margin: 1.5rem 0; padding: 1.5rem; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  h2 { margin: 0 0 1rem; } p { color: var(--muted); line-height: 1.6; } label { display: grid; gap: .5rem; margin: 1rem 0; } .row { display: flex; gap: .75rem; }
  .features { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: .5rem; } .feature { display: flex; align-items: center; gap: .6rem; margin: .3rem 0; }
  textarea { width: 100%; font-family: monospace; } input:not([type=checkbox]):not([type=color]) { width: 100%; }
</style>
