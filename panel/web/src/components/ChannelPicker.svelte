<script lang="ts">
  import { Hash, ListFilter } from '@lucide/svelte';
  import { get } from '../lib/api';
  import { toastError } from '../lib/toast.svelte';

  // A Discord channel, chosen by pasting its ID (right click the channel with Developer Mode on, "Copy Channel ID") or picked from the
  // server's channels once the bot is set up. The ID is what gets saved either way.
  let { value = $bindable(''), label = 'Channel', hint = '', canList = true, placeholder = 'Channel ID, e.g. 123456789012345678' }: {
    value: string; label?: string; hint?: string; canList?: boolean; placeholder?: string;
  } = $props();

  type Channel = { id: string; name: string; category: string };
  let channels = $state<Channel[] | null>(null);
  let loading = $state(false);

  async function load() {
    loading = true;
    try { channels = await get<Channel[]>('/api/admin/discord/channels'); }
    catch (e) { toastError(e); }
    finally { loading = false; }
  }

  const picked = $derived(channels?.find((c) => c.id === value));
  const looksValid = $derived(!value || /^\d{15,25}$/.test(value.trim()));
</script>

<div class="picker">
  <span class="label">{label}{#if hint} <small>{hint}</small>{/if}</span>
  <div class="row">
    <div class="id" class:bad={!looksValid}>
      <Hash size={15} />
      <input bind:value inputmode="numeric" {placeholder} autocomplete="off" aria-label="{label} ID" aria-invalid={!looksValid} />
    </div>
    {#if canList}
      {#if channels}
        <select aria-label="Pick a channel from your server" value={picked ? picked.id : ''} onchange={(e) => { if (e.currentTarget.value) value = e.currentTarget.value; }}>
          <option value="">Pick from your server…</option>
          {#each channels as c (c.id)}<option value={c.id}>#{c.name}{c.category ? ` · ${c.category}` : ''}</option>{/each}
        </select>
      {:else}
        <button type="button" class="ghost" onclick={load} disabled={loading} title="List the channels of your Discord server (needs the bot token and server ID)">
          <ListFilter size={15} /> {loading ? 'Loading…' : 'Pick from server'}
        </button>
      {/if}
    {/if}
  </div>
  {#if !looksValid}<span class="err">A channel ID is a long number (15–25 digits). Copy it from Discord with Developer Mode on.</span>{/if}
  {#if picked}<span class="ok">#{picked.name}{picked.category ? ` in ${picked.category}` : ''}</span>{/if}
</div>

<style>
  .picker { display: flex; flex-direction: column; gap: 6px; }
  .label { font-size: 0.85rem; font-weight: 600; }
  .label small { font-weight: 400; color: var(--muted); margin-left: 4px; }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: stretch; }
  .id { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 230px; padding: 0 12px; border: 1px solid var(--line); border-radius: var(--radius-sm, 8px); background: var(--bg-2, transparent); color: var(--muted); }
  .id:focus-within { border-color: var(--accent); color: var(--text); }
  .id.bad { border-color: #ef4444; }
  .id input { border: 0; background: transparent; box-shadow: none; padding: 9px 0; flex: 1; min-width: 0; font-variant-numeric: tabular-nums; }
  select { min-width: 200px; }
  .err { color: #f87171; font-size: 0.8rem; }
  .ok { color: #34d399; font-size: 0.8rem; }
</style>
