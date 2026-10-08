<script lang="ts">
  import { onMount } from 'svelte';
  import { Plus, Trash2, Save, MapPin, Layers, Eraser, Flag, RotateCcw, Users } from '@lucide/svelte';
  import { get, post, patch, put, del } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Server = { id: number; name: string };
  type Claim = {
    id: string; name: string; description: string; color: string; chunks: number; dimensions: string[];
    bounds: { min_x: number; max_x: number; min_z: number; max_z: number } | null;
    flags: Record<string, boolean>;
  };
  type FlagInfo = { id: string; label: string; help: string; group: string; default: boolean };
  let servers = $state<Server[]>([]);
  let sid = $state(0);
  let claims = $state<Claim[]>([]);
  let catalog = $state<FlagInfo[]>([]);
  const groups = $derived([...new Set(catalog.map((f) => f.group))]);
  let draft = $state({ name: '', description: '', color: '#f59e0b' });
  let area = $state<Record<string, { dimension: string; x1: number; z1: number; x2: number; z2: number }>>({});
  let busy = $state('');

  // Which land rules guilds may change on their own claims (the rest stay at their defaults).
  type GuildRule = { id: string; label: string; help: string; group: string; default: boolean; editable: boolean };
  let guildRules = $state<GuildRule[]>([]);
  async function loadGuildRules() {
    try { guildRules = (await get<{ catalog: GuildRule[] }>('/api/admin/guild-flag-policy')).catalog; } catch (e) { toastError(e); }
  }
  async function toggleGuildRule(r: GuildRule) {
    const next = guildRules.map((x) => (x.id === r.id ? { ...x, editable: !x.editable } : x));
    try {
      guildRules = (await put<{ catalog: GuildRule[] }>('/api/admin/guild-flag-policy', { editable: next.filter((x) => x.editable).map((x) => x.id) })).catalog;
      toast(`${r.label}: ${r.editable ? 'now managed by you' : 'guilds can change it'}`);
    } catch (e) { toastError(e); }
  }

  onMount(async () => {
    try {
      void loadGuildRules();
      servers = await get<Server[]>('/api/admin/servers');
      if (servers[0]) { sid = servers[0].id; await load(); }
    } catch (e) { toastError(e); }
  });
  async function load() {
    if (!sid) return;
    try { const r = await get<{ claims: Claim[]; flag_catalog: FlagInfo[] }>(`/api/admin/servers/${sid}/admin-claims`); claims = r.claims; catalog = r.flag_catalog; for (const c of claims) box(c.id); } catch (e) { toastError(e); }
  }
  async function create() {
    busy = 'create';
    try {
      await post(`/api/admin/servers/${sid}/admin-claims`, draft);
      draft = { name: '', description: '', color: '#f59e0b' };
      toast('Admin claim created — now give it some land'); await load();
    } catch (e) { toastError(e); } finally { busy = ''; }
  }
  async function save(c: Claim) {
    busy = c.id;
    try { await patch(`/api/admin/admin-claims/${c.id}`, { name: c.name, description: c.description, color: c.color, flags: c.flags }); toast('Saved'); await load(); }
    catch (e) { toastError(e); } finally { busy = ''; }
  }
  async function remove(c: Claim) {
    if (!confirm(`Delete "${c.name}" and release its ${c.chunks} chunks?`)) return;
    try { await del(`/api/admin/admin-claims/${c.id}`); await load(); } catch (e) { toastError(e); }
  }
  const standard = (c: Claim) => { for (const f of catalog) c.flags[f.id] = f.default; };
  const changed = (c: Claim) => catalog.filter((f) => c.flags[f.id] !== f.default).length;
  function box(id: string) { if (!area[id]) area[id] = { dimension: 'minecraft:overworld', x1: 0, z1: 0, x2: 0, z2: 0 }; return area[id]; }
  async function apply(c: Claim, remove: boolean) {
    busy = c.id;
    try {
      const r = await post<{ added?: number; skipped?: number; removed?: number }>(`/api/admin/admin-claims/${c.id}/area`, { ...box(c.id), remove });
      toast(remove ? `Released ${r.removed} chunks` : `Claimed ${r.added} chunks${r.skipped ? ` (${r.skipped} were already taken)` : ''}`);
      await load();
    } catch (e) { toastError(e); } finally { busy = ''; }
  }
</script>

<div class="page wide">
  <header>
    <div>
      <h1>Admin claims</h1>
      <p>Protected regions that belong to the server — spawn, shops, arenas. Players see the name and description when they walk in, and they appear on the map in the colour you pick. Staff with <code>scopenet.claims.bypass</code> can still build. You can also manage them in game with <code>/adminclaim</code>.</p>
    </div>
    <select bind:value={sid} onchange={load}>{#each servers as s}<option value={s.id}>{s.name}</option>{/each}</select>
  </header>

  <section class="card new">
    <h3><Plus size={16} /> New admin claim</h3>
    <div class="row">
      <label>Name<input bind:value={draft.name} placeholder="Spawn" maxlength="32" /></label>
      <label class="grow">Description<input bind:value={draft.description} placeholder="Welcome! PvP is disabled here." maxlength="300" /></label>
      <label>Colour<input type="color" bind:value={draft.color} /></label>
      <button onclick={create} disabled={!draft.name.trim() || busy === 'create'}>Create</button>
    </div>
  </section>

  <details class="card guild-rules">
    <summary><Users size={16} /> Guild land rules <small>which rules guild leaders may change on their own claims</small></summary>
    <p class="hint">Guilds can open their land to visitors or tighten it from the Land tab in the player panel and launcher. Switch a rule off here to keep it at its default for every guild.</p>
    <div class="rule-grid">
      {#each guildRules as r (r.id)}
        <button type="button" class="rule" class:on={r.editable} role="switch" aria-checked={r.editable} onclick={() => toggleGuildRule(r)} title={r.help}>
          <span><b>{r.label}</b><small>{r.group} · default {r.default ? 'on' : 'off'}</small></span>
          <span class="state">{r.editable ? 'Guilds choose' : 'Locked'}</span>
        </button>
      {/each}
    </div>
  </details>

  {#each claims as c (c.id)}
    <section class="card claim" style:--c={c.color}>
      <div class="head">
        <span class="swatch"><MapPin size={16} /></span>
        <div class="grow">
          <div class="row">
            <label class="grow">Name<input bind:value={c.name} maxlength="32" /></label>
            <label>Colour<input type="color" bind:value={c.color} /></label>
          </div>
          <label>Description <small>shown when players enter</small><textarea rows="2" bind:value={c.description} maxlength="300"></textarea></label>
        </div>
      </div>
      <div class="stats">
        <span><Layers size={13} /> {c.chunks} chunk{c.chunks === 1 ? '' : 's'}</span>
        {#if c.bounds}<span>blocks {c.bounds.min_x * 16} … {c.bounds.max_x * 16 + 15} × {c.bounds.min_z * 16} … {c.bounds.max_z * 16 + 15}</span>{/if}
        {#each c.dimensions as d}<span class="chip">{d.replace('minecraft:', '')}</span>{/each}
      </div>
      <details class="flags" open>
        <summary><Flag size={14} /> Flags <small>what is allowed inside{#if changed(c)} · {changed(c)} changed{/if}</small></summary>
        <p class="hint">Green means <b>allowed</b>. Staff with <code>scopenet.claims.bypass</code> are never held back by a flag. Flags are enforced by the Paper plugin; save to apply them (they reach the server within seconds).</p>
        {#each groups as g}
          <h4>{g}</h4>
          <div class="flagrow">
            {#each catalog.filter((f) => f.group === g) as f (f.id)}
              <button type="button" class="flag" class:on={c.flags[f.id]} role="switch" aria-checked={c.flags[f.id]} title={f.help} onclick={() => (c.flags[f.id] = !c.flags[f.id])}>
                <span class="knob"></span><span class="txt"><b>{f.label}</b><small>{f.help}</small></span>
              </button>
            {/each}
          </div>
        {/each}
        <button type="button" class="ghost sm" onclick={() => standard(c)}><RotateCcw size={13} /> Standard flags</button>
      </details>
      <details>
        <summary>Add or remove land</summary>
        <p class="hint">Enter two opposite corners in <b>block</b> coordinates (press F3 in game). Land that already belongs to a guild is skipped.</p>
        <div class="row coords">
          <label>Dimension<select bind:value={area[c.id].dimension}><option>minecraft:overworld</option><option>minecraft:the_nether</option><option>minecraft:the_end</option></select></label>
          <label>X 1<input type="number" bind:value={area[c.id].x1} /></label><label>Z 1<input type="number" bind:value={area[c.id].z1} /></label>
          <label>X 2<input type="number" bind:value={area[c.id].x2} /></label><label>Z 2<input type="number" bind:value={area[c.id].z2} /></label>
          <button onclick={() => apply(c, false)} disabled={busy === c.id}><Plus size={14} /> Claim area</button>
          <button class="ghost" onclick={() => apply(c, true)} disabled={busy === c.id}><Eraser size={14} /> Release area</button>
        </div>
      </details>
      <div class="foot">
        <button onclick={() => save(c)} disabled={busy === c.id}><Save size={14} /> Save</button>
        <button class="danger" onclick={() => remove(c)}><Trash2 size={14} /> Delete</button>
      </div>
    </section>
  {:else}
    <p class="muted">No admin claims on this server yet.</p>
  {/each}
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  header { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; }
  header select { min-width: 12rem; }
  code { font: 0.8rem ui-monospace, monospace; background: var(--bg-2); padding: 1px 5px; border-radius: 5px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8rem; color: var(--muted); }
  label small { font-weight: 400; }
  .row { display: flex; gap: 10px; align-items: flex-end; flex-wrap: wrap; }
  .grow { flex: 1; min-width: 12rem; }
  h3 { display: flex; align-items: center; gap: 6px; margin: 0 0 10px; font-size: 0.95rem; }
  .claim { border-left: 4px solid var(--c); display: flex; flex-direction: column; gap: 12px; }
  .head { display: flex; gap: 12px; }
  .swatch { width: 2.2rem; height: 2.2rem; border-radius: 0.7rem; display: grid; place-items: center; flex-shrink: 0; color: var(--c); background: color-mix(in srgb, var(--c) 18%, transparent); }
  .stats { display: flex; gap: 10px 16px; flex-wrap: wrap; font-size: 0.78rem; color: var(--muted); align-items: center; }
  .stats span { display: inline-flex; align-items: center; gap: 5px; }
  .chip { padding: 1px 8px; border-radius: 99px; background: var(--bg-2); border: 1px solid var(--line); }
  details summary { cursor: pointer; font-size: 0.85rem; font-weight: 600; }
  .hint { color: var(--muted); font-size: 0.78rem; margin: 8px 0; }
  .coords input[type='number'] { width: 6.5rem; }
  .foot { display: flex; gap: 8px; }
  .foot button, .coords button, .new button { display: inline-flex; align-items: center; gap: 6px; }
  .muted { color: var(--muted); }
  details.flags summary { display: flex; gap: 6px; align-items: center; } details.flags summary small { font-weight: 400; color: var(--muted); }
  h4 { margin: 12px 0 6px; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); }
  .flagrow { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 8px; }
  .flag { display: flex; gap: 10px; align-items: flex-start; text-align: left; padding: 9px 11px; border-radius: 10px; border: 1px solid var(--line); background: var(--surface-2, var(--bg-2)); color: var(--text); cursor: pointer; transition: border-color 0.12s, background 0.12s; }
  .flag:hover { border-color: var(--line-strong); }
  .flag.on { border-color: color-mix(in srgb, var(--good, #22c55e) 55%, transparent); background: color-mix(in srgb, var(--good, #22c55e) 9%, var(--surface-2, var(--bg-2))); }
  .knob { flex: none; width: 30px; height: 17px; border-radius: 99px; background: color-mix(in srgb, var(--text) 18%, transparent); position: relative; margin-top: 2px; transition: background 0.15s; }
  .knob::after { content: ''; position: absolute; top: 2px; left: 2px; width: 13px; height: 13px; border-radius: 50%; background: #fff; transition: transform 0.15s; }
  .flag.on .knob { background: var(--good, #22c55e); } .flag.on .knob::after { transform: translateX(13px); }
  .txt { display: flex; flex-direction: column; gap: 1px; min-width: 0; } .txt b { font-size: 0.84rem; } .txt small { color: var(--muted); font-size: 0.72rem; font-weight: 400; line-height: 1.3; }
  .sm { padding: 5px 10px; font-size: 0.8rem; margin-top: 10px; display: inline-flex; gap: 5px; align-items: center; }
  input[type='color'] { height: 2.2rem; padding: 2px; width: 3.4rem; }
  .guild-rules summary { display: flex; gap: 8px; align-items: center; cursor: pointer; font-weight: 600; }
  .guild-rules summary small { font-weight: 400; color: var(--muted); }
  .guild-rules .hint { color: var(--muted); font-size: 0.85rem; margin: 10px 0 14px; }
  .rule-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 8px; }
  .rule { display: flex; justify-content: space-between; align-items: center; gap: 10px; text-align: left; padding: 10px 12px; background: var(--bg-2); }
  .rule span:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .rule small { color: var(--muted); font-weight: 400; }
  .rule .state { font-size: 0.74rem; padding: 3px 9px; border-radius: 99px; background: rgba(255, 255, 255, 0.06); color: var(--muted); white-space: nowrap; }
  .rule.on { border-color: rgba(52, 211, 153, 0.35); }
  .rule.on .state { background: rgba(52, 211, 153, 0.14); color: #6ee7b7; }
</style>
