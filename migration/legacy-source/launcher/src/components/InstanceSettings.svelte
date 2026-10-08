<script lang="ts">
  import { FolderOpen, Wrench, Trash2, FileCode } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { app, repair, saveSettings, toast } from '../lib/store.svelte';
  import { bytes, gb } from '../lib/format';
  import { errorText, invoke, pickFile } from '../lib/tauri';
  import type { InstanceOverride } from '../lib/types';

  let open = $state(false);
  $effect(() => {
    open = !!app.instanceSettings;
  });
  const inst = $derived(app.manifest?.instances.find((i) => i.id === app.instanceSettings) ?? null);
  const advanced = $derived(app.manifest?.branding.features.allow_advanced_java !== false);
  const ram = $derived(app.boot?.app.total_ram_mb ?? 8192);
  let local = $state<{ installed: boolean; size: number } | null>(null);

  // Create the override entry up front (never mutate state inside $derived).
  $effect(() => {
    if (inst && app.settings && !app.settings.instance_overrides[inst.id]) {
      app.settings.instance_overrides[inst.id] = { memory_max_mb: 0, memory_min_mb: 0, jvm_args: '', java_path: null };
    }
  });
  const ov = $derived<InstanceOverride | undefined>(inst ? app.settings?.instance_overrides[inst.id] : undefined);

  $effect(() => {
    if (inst) invoke<{ installed: boolean; size: number }>('instance_local', { instanceId: inst.id }).then((l) => (local = l)).catch(() => {});
  });

  const folders = ['instance', 'mods', 'resourcepacks', 'shaderpacks', 'saves', 'screenshots', 'logs'];

  async function del() {
    if (!inst) return;
    try {
      await invoke('delete_instance_data', { instanceId: inst.id });
      toast('Local files deleted — they download again next time you play');
      local = { installed: false, size: 0 };
    } catch (e) {
      toast(errorText(e), 'error');
    }
  }
</script>

<Modal bind:open title={inst ? `${inst.name} settings` : ''} width={34} onclose={() => (app.instanceSettings = null)}>
  {#if inst && app.settings && ov}
    <div class="grid2">
      {#each folders as f}
        <button class="sm" onclick={() => invoke('open_folder', { kind: f, instanceId: inst.id })}><FolderOpen size={14} /> {f === 'instance' ? 'Game folder' : f}</button>
      {/each}
    </div>

    <label class="field">
      <span class="row">Memory for this instance <span class="spacer"></span><strong>{ov.memory_max_mb ? gb(ov.memory_max_mb) : `Default (${gb(app.settings.memory_max_mb || inst.memory.max_mb)})`}</strong></span>
      <input type="range" min="0" max={Math.max(2048, ram - 1024)} step="512" bind:value={ov.memory_max_mb} oninput={saveSettings} />
      <span class="help">Slide to the far left to use the global/admin default.</span>
    </label>

    {#if advanced}
      <label class="field">Extra JVM arguments
        <input class="mono" bind:value={ov.jvm_args} oninput={saveSettings} placeholder="-XX:+UseStringDeduplication" />
      </label>
      <label class="field">Java for this instance
        <div class="row">
          <input value={ov.java_path ?? ''} placeholder="Automatic" readonly />
          <button class="sm" onclick={async () => { const p = await pickFile('Choose java executable', [{ name: 'Java', extensions: ['exe', ''] }]); if (p) { ov.java_path = p; saveSettings(); } }}><FileCode size={14} /> Browse</button>
          {#if ov.java_path}<button class="sm ghost" onclick={() => { ov.java_path = null; saveSettings(); }}>Reset</button>{/if}
        </div>
      </label>
    {/if}

    <div class="card col">
      <strong>Vanilla game settings</strong>
      <span class="help">Brightness, graphics, FOV, view distance, sound and other vanilla options are saved automatically when the game closes, then restored after instance updates.</span>
      <label class="field">
        <span class="row">Field of view <span class="spacer"></span><strong>{Math.round(70 + 40 * Number(app.settings.instance_game_options?.[inst.id]?.fov ?? '0'))}°</strong></span>
        <input type="range" min="-1" max="1" step="0.025" value={app.settings.instance_game_options?.[inst.id]?.fov ?? '0'} oninput={(e) => {
          app.settings!.instance_game_options[inst.id] ??= {};
          app.settings!.instance_game_options[inst.id].fov = e.currentTarget.value;
          invoke('set_instance_fov', { instanceId: inst.id, fov: Number(e.currentTarget.value) }).catch((err) => toast(errorText(err), 'error'));
        }} />
        <span class="help">Applied the next time you launch this instance.</span>
      </label>
      <span class="tiny muted">{Object.keys(app.settings.instance_game_options?.[inst.id] ?? {}).length} saved options for this instance</span>
      <button class="sm" disabled={app.running.some((g) => g.instance_id === inst.id)} onclick={async () => {
        try {
          const options = await invoke<Record<string, string>>('save_instance_options', { instanceId: inst.id });
          app.settings!.instance_game_options[inst.id] = options;
          toast('Vanilla game settings saved');
        } catch (e) { toast(errorText(e), 'error'); }
      }}>Save options.txt now</button>
    </div>

    <div class="danger-zone">
      <span class="tiny muted">{local?.installed ? `Uses ${bytes(local.size)} on disk` : 'Not downloaded yet'}</span>
      <span class="spacer"></span>
      <button class="sm" onclick={() => { app.instanceSettings = null; repair(inst.id); }}><Wrench size={14} /> Repair files</button>
      <button class="sm danger" onclick={del}><Trash2 size={14} /> Delete local files</button>
    </div>
  {/if}
</Modal>

<style>
  .grid2 { display: grid; grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr)); gap: 0.45rem; }
  .grid2 button { justify-content: flex-start; text-transform: capitalize; }
  .danger-zone { display: flex; align-items: center; gap: 0.5rem; padding-top: 0.9rem; border-top: 1px solid var(--line); flex-wrap: wrap; }
</style>
