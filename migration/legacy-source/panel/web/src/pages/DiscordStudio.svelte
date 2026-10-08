<script lang="ts">
  import { onMount } from 'svelte';
  import { Send, RefreshCw, MessageSquare, Radio, Save, RotateCw } from '@lucide/svelte';
  import EmbedEditor from '../components/EmbedEditor.svelte';
  import EmbedPreview from '../components/EmbedPreview.svelte';
  import ChannelPicker from '../components/ChannelPicker.svelte';
  import { get, post, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { EmbedStyle, GameServer, LiveEmbed } from '../lib/types';

  type Studio = { templates: Record<string, EmbedStyle>; live: Record<string, LiveEmbed>; placeholders: Record<string, string[]>; live_placeholders: Record<string, string[]>; webhook_set: boolean };

  const events = [
    { id: 'achievement', title: 'Achievement unlocked', blurb: 'When a player earns an achievement.' },
    { id: 'member', title: 'New player', blurb: 'When a new player account is approved.' },
    { id: 'guild', title: 'New guild', blurb: 'When a guild is founded.' }
  ];
  const lives = [
    { id: 'status', title: 'Server status', blurb: 'Every server: online, players, TPS and version.' },
    { id: 'guilds', title: 'Active guilds', blurb: 'The top guilds with members, land and level.' },
    { id: 'leaderboard', title: 'Player leaderboard', blurb: 'Top players by playtime, level, kills or blocks.' },
    { id: 'baltop', title: 'Richest players', blurb: 'The top balances on a server economy.' }
  ];

  let data = $state<Studio | null>(null);
  let servers = $state<GameServer[]>([]);
  let tab = $state<'events' | 'live'>('events');
  let current = $state('achievement');
  let preview = $state<any>(null);
  let busy = $state('');
  let timer: ReturnType<typeof setTimeout>;

  async function load() {
    try {
      data = await get<Studio>('/api/admin/discord/studio');
      servers = await get<GameServer[]>('/api/admin/servers').catch(() => []);
    } catch (e) { toastError(e); }
  }
  onMount(load);

  const isLive = $derived(lives.some((l) => l.id === current));
  const style = $derived(data ? (isLive ? data.live[current]?.style : data.templates[current]) : null);
  const placeholders = $derived(data ? (isLive ? [...data.live_placeholders[current], ...(data.live_placeholders[current + '_row'] ?? [])] : data.placeholders[current]) : []);

  // The preview is built by the panel from real data, so what you see is what Discord gets.
  async function refreshPreview() {
    if (!data || !style) return;
    try {
      preview = await post('/api/admin/discord/studio/preview', isLive ? { kind: current, live: data.live[current] } : { kind: current, style: data.templates[current] });
    } catch (e: any) { preview = null; }
  }
  $effect(() => {
    void JSON.stringify(data?.templates[current] ?? data?.live[current]);
    clearTimeout(timer);
    timer = setTimeout(refreshPreview, 450);
    return () => clearTimeout(timer);
  });

  function pick(id: string) { current = id; preview = null; }
  function switchTab(t: 'events' | 'live') { tab = t; pick(t === 'events' ? 'achievement' : 'status'); }

  async function save() {
    if (!data) return;
    busy = 'save';
    try {
      if (isLive) {
        const live: Record<string, any> = {};
        for (const [k, v] of Object.entries(data.live)) live[k] = { ...v, webhook_url: '' };
        await put('/api/admin/discord/studio/live', { live });
      } else {
        await put('/api/admin/discord/studio/templates', { templates: data.templates });
      }
      toast('Saved');
      await load();
    } catch (e) { toastError(e); } finally { busy = ''; }
  }

  async function sendNow() {
    busy = 'send';
    try { await save(); await post(`/api/admin/discord/studio/send/${current}`, {}); toast(isLive ? 'Embed updated in Discord' : 'Sample message sent to Discord'); await load(); }
    catch (e) { toastError(e); } finally { busy = ''; }
  }
  async function repost() {
    busy = 'repost';
    try { await save(); await post(`/api/admin/discord/studio/repost/${current}`, {}); toast('Posted a new message'); await load(); }
    catch (e) { toastError(e); } finally { busy = ''; }
  }
</script>

<div class="page wide">
  <header>
    <div>
      <h1>Discord</h1>
      <p>Customise the messages the panel posts, and keep live status boards in your channels.{#if data && !data.webhook_set} <strong class="warn" style="margin-left:6px">Choose the announcement channel under Settings → Discord first.</strong>{/if}</p>
    </div>
  </header>

  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === 'events'} class:active={tab === 'events'} onclick={() => switchTab('events')}><MessageSquare size={15} /> Announcements</button>
    <button role="tab" aria-selected={tab === 'live'} class:active={tab === 'live'} onclick={() => switchTab('live')}><Radio size={15} /> Live embeds</button>
  </div>

  {#if data && style}
    <div class="layout">
      <aside class="list card">
        {#each tab === 'events' ? events : lives as item}
          <button class="item" class:on={current === item.id} onclick={() => pick(item.id)}>
            <span class="dot" class:live={(isLive ? data.live[item.id]?.style.enabled : data.templates[item.id]?.enabled)}></span>
            <span><strong>{item.title}</strong><small>{item.blurb}</small></span>
          </button>
        {/each}
      </aside>

      <section class="card form">
        {#if isLive}
          {#key current}<EmbedEditor bind:style={data.live[current].style} {placeholders} toggleLabel="Keep this embed updated" />{/key}
        {:else}
          {#key current}<EmbedEditor bind:style={data.templates[current]} {placeholders} toggleLabel="Post this message" />{/key}
        {/if}

        {#if isLive}
          {@const c = data.live[current]}
          <div class="live-opts">
            <h3>Live settings</h3>
            <div class="grid">
              <label class="field wide">Line for each row <small>one line per server, guild or player. Fills {'{rows}'} in the description.</small><input bind:value={c.row} placeholder="Leave empty for the default" /></label>
              <label class="field">Update every
                <select bind:value={c.interval_secs}>
                  {#each [[60, '1 minute'], [120, '2 minutes'], [300, '5 minutes'], [900, '15 minutes'], [3600, '1 hour']] as [v, l]}<option value={v}>{l}</option>{/each}
                </select>
              </label>
              {#if current !== 'status'}<label class="field">Rows shown<input type="number" min="1" max="25" bind:value={c.limit} /></label>{/if}
              {#if current === 'leaderboard'}
                <label class="field">Ranked by<select bind:value={c.sort}><option value="playtime">Playtime</option><option value="level">Level</option><option value="kills">Player kills</option><option value="blocks">Blocks broken</option></select></label>
              {/if}
              {#if current === 'baltop'}
                <label class="field">Economy of<select bind:value={c.server_id}><option value={0}>The first server</option>{#each servers as s}<option value={s.id}>{s.name}</option>{/each}</select></label>
              {/if}
              <div class="field wide"><ChannelPicker bind:value={c.channel_id} label="Post in a different channel" hint="optional. Empty = the announcement channel." /></div>
            </div>
            <p class="muted small">{c.posted ? 'The embed is posted and is edited in place on every update.' : 'Not posted yet. Save and press Update now.'}</p>
          </div>
        {/if}

        <div class="actions">
          <button class="primary" onclick={save} disabled={!!busy}><Save size={15} /> Save</button>
          <button onclick={sendNow} disabled={!!busy || !data.webhook_set && !(isLive && (data.live[current].channel_id || data.live[current].webhook_set))}>{#if isLive}<RefreshCw size={15} /> Save &amp; update now{:else}<Send size={15} /> Save &amp; send a sample{/if}</button>
          {#if isLive && data.live[current].posted}<button class="ghost" onclick={repost} disabled={!!busy} title="Post a new message instead of editing the old one"><RotateCw size={15} /> Post as new message</button>{/if}
        </div>
      </section>

      <section class="preview">
        <h3>Preview</h3>
        <p class="muted small">Built from your real data{isLive ? '' : ' (sample names for announcements)'}.</p>
        {#if preview}<EmbedPreview body={preview} />{:else}<p class="muted">Loading preview…</p>{/if}
      </section>
    </div>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 18px; }
  header h1 { margin: 0; }
  header p { margin: 4px 0 0; color: var(--muted); }
  .warn { color: #fbbf24; }
  .tabs { display: inline-flex; gap: 4px; padding: 4px; border-radius: 12px; background: var(--bg-2); border: 1px solid var(--line); align-self: flex-start; }
  .tabs button { display: inline-flex; align-items: center; gap: 6px; border: 0; background: transparent; padding: 8px 14px; border-radius: 9px; color: var(--muted); }
  .tabs button.active { background: var(--surface-2); color: var(--text); }
  .layout { display: grid; grid-template-columns: 240px minmax(0, 1.4fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  .list { display: flex; flex-direction: column; gap: 6px; padding: 10px; position: sticky; top: 16px; }
  .item { display: flex; align-items: flex-start; gap: 10px; text-align: left; padding: 10px; border-radius: 10px; border: 1px solid transparent; background: transparent; }
  .item.on { background: color-mix(in srgb, var(--accent) 14%, transparent); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
  .item small { display: block; color: var(--muted); font-size: 0.74rem; font-weight: 400; margin-top: 2px; }
  .dot { width: 9px; height: 9px; margin-top: 5px; border-radius: 50%; background: var(--muted); opacity: 0.5; flex-shrink: 0; }
  .dot.live { background: #34d399; opacity: 1; box-shadow: 0 0 0 3px color-mix(in srgb, #34d399 25%, transparent); }
  .form { display: flex; flex-direction: column; gap: 18px; }
  .live-opts h3, .preview h3 { margin: 0 0 6px; font-size: 0.95rem; }
  .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; }
  .field.wide { grid-column: 1 / -1; }
  .field small { color: var(--muted); font-weight: 400; }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .actions button { display: inline-flex; align-items: center; gap: 6px; }
  .preview { position: sticky; top: 16px; display: flex; flex-direction: column; gap: 8px; }
  @media (max-width: 1280px) { .layout { grid-template-columns: 220px minmax(0, 1fr); } .preview { grid-column: 1 / -1; position: static; } }
  @media (max-width: 800px) { .layout { grid-template-columns: 1fr; } .list { position: static; flex-direction: row; flex-wrap: wrap; } }
</style>
