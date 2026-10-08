<script lang="ts">
  import { onMount } from 'svelte';
  import { Target, Trophy, Gift, Search, Save, RotateCcw, Wand2, Percent, Equal, Plus, Eraser, Power, Table2, Coins, Sparkles } from '@lucide/svelte';
  import { get, post } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Kind = 'quest' | 'achievement' | 'level_reward';
  type Row = {
    id: string; title: string; group: string; xp: number | null; count: number | null; enabled: boolean | null; money: number | null; auto: boolean; other_rewards: number;
    // what the table currently shows (edits live here until saved)
    d: { xp: number | null; money: number | null; enabled: boolean | null; level: number | null; reset: boolean };
  };

  const KINDS: { id: Kind; label: string; icon: typeof Target; blurb: string }[] = [
    { id: 'quest', label: 'Quests', icon: Target, blurb: 'XP and money for every daily, weekly and standing quest' },
    { id: 'achievement', label: 'Achievements', icon: Trophy, blurb: 'XP and money for every achievement' },
    { id: 'level_reward', label: 'Level rewards', icon: Gift, blurb: 'The money on each rank milestone, and the level it unlocks at' },
  ];

  let kind = $state<Kind>('quest');
  let rows = $state<Row[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let search = $state('');
  let group = $state('all');
  let selected = $state<Set<string>>(new Set());
  let lastClicked: string | null = null;

  // the bulk action
  let field = $state<'money' | 'xp' | 'level'>('money');
  let mode = $state<'set' | 'multiply' | 'add'>('multiply');
  let value = $state(1.5);
  let roundTo5 = $state(true);

  const hasXp = $derived(kind !== 'level_reward');
  const fields = $derived(kind === 'level_reward' ? ['money', 'level'] as const : ['money', 'xp'] as const);

  async function load() {
    loading = true;
    try {
      const r = await get<{ rows: Omit<Row, 'd'>[] }>(`/api/admin/bulk/${kind}`);
      rows = r.rows.map((x) => ({ ...x, d: { xp: x.xp, money: x.money, enabled: x.enabled, level: x.count, reset: false } }));
      selected = new Set();
      group = 'all';
      field = 'money';
    } catch (e) {
      toastError(e);
    } finally {
      loading = false;
    }
  }
  onMount(load);
  function pick(k: Kind) {
    if (k === kind) return;
    if (dirtyRows.length && !confirm('Discard your unsaved edits?')) return;
    kind = k;
    load();
  }

  const groups = $derived([...new Set(rows.map((r) => r.group))].sort());
  const shown = $derived(
    rows.filter((r) => (group === 'all' || r.group === group) && (!search.trim() || (r.title + ' ' + r.id).toLowerCase().includes(search.trim().toLowerCase()))),
  );
  const isDirty = (r: Row) => r.d.xp !== r.xp || r.d.enabled !== r.enabled || r.d.reset || (r.d.money !== r.money) || (kind === 'level_reward' && r.d.level !== r.count);
  const dirtyRows = $derived(rows.filter(isDirty));
  const allShownSelected = $derived(shown.length > 0 && shown.every((r) => selected.has(r.id)));

  function toggle(id: string, e?: MouseEvent) {
    const next = new Set(selected);
    if (e?.shiftKey && lastClicked) {
      const a = shown.findIndex((r) => r.id === lastClicked);
      const b = shown.findIndex((r) => r.id === id);
      if (a >= 0 && b >= 0) for (const r of shown.slice(Math.min(a, b), Math.max(a, b) + 1)) next.add(r.id);
    } else if (next.has(id)) next.delete(id);
    else next.add(id);
    lastClicked = id;
    selected = next;
  }
  const toggleAll = () => (selected = allShownSelected ? new Set() : new Set(shown.map((r) => r.id)));

  const round5 = (x: number) => Math.max(5, Math.round(x / 5) * 5);
  function calc(old: number, v: number): number {
    const out = mode === 'set' ? v : mode === 'multiply' ? old * v : old + v;
    return Math.max(0, field === 'money' ? (roundTo5 ? round5(out) : Math.round(out * 100) / 100) : Math.round(out));
  }
  const targets = $derived(rows.filter((r) => selected.has(r.id)));
  /** What the first few selected rows would become, so the action is never a surprise. */
  const sample = $derived(
    targets.slice(0, 3).map((r) => {
      const cur = field === 'money' ? r.d.money ?? 0 : field === 'xp' ? r.d.xp ?? 0 : r.d.level ?? 1;
      return { id: r.id, title: r.title, from: cur, to: calc(cur, Number(value) || 0) };
    }),
  );

  function runAction() {
    const v = Number(value);
    if (!Number.isFinite(v)) return;
    for (const r of targets) {
      r.d.reset = false;
      if (field === 'money') r.d.money = calc(r.d.money ?? 0, v);
      else if (field === 'xp') r.d.xp = calc(r.d.xp ?? 0, v);
      else r.d.level = Math.max(1, calc(r.d.level ?? 1, v));
    }
    toast(`Changed ${targets.length} rows — save to keep it`);
  }
  function roundSelected() {
    for (const r of targets) if (r.d.money != null) { r.d.reset = false; r.d.money = round5(r.d.money); }
  }
  function autoSelected() {
    for (const r of targets) { r.d.reset = true; r.d.money = null; }
  }
  function enableSelected(on: boolean) {
    for (const r of targets) if (r.d.enabled != null) r.d.enabled = on;
  }

  function edit(r: Row, f: 'money' | 'xp' | 'level', raw: string) {
    const v = raw === '' ? null : Number(raw);
    if (v != null && !Number.isFinite(v)) return;
    if (f === 'money') { r.d.money = v; r.d.reset = v == null; }
    else if (f === 'xp') r.d.xp = v;
    else r.d.level = v;
  }

  const revert = () => { for (const r of rows) r.d = { xp: r.xp, money: r.money, enabled: r.enabled, level: r.count, reset: false }; };

  async function save() {
    saving = true;
    try {
      const patches = dirtyRows.map((r) => ({
        id: r.id,
        xp: hasXp && r.d.xp !== r.xp ? r.d.xp : undefined,
        enabled: r.d.enabled !== r.enabled ? r.d.enabled : undefined,
        level_req: kind === 'level_reward' && r.d.level !== r.count ? r.d.level : undefined,
        money: !r.d.reset && r.d.money !== r.money && r.d.money != null ? r.d.money : undefined,
        reset_money: r.d.reset || undefined,
      }));
      const res = await post<{ changed: number }>(`/api/admin/bulk/${kind}`, { patches });
      toast(`Saved ${res.changed} rows`);
      await load();
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  function onkey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (dirtyRows.length && !saving) save();
    }
  }
  const usd = (v: number | null) => (v == null ? '—' : `$${v.toLocaleString()}`);
  const total = $derived(rows.reduce((a, r) => a + (r.d.money ?? 0), 0));
</script>

<svelte:window onkeydown={onkey} />

<div class="page">
  <div class="page-head">
    <div>
      <h1><Table2 size={22} /> Bulk editor</h1>
      <p class="muted">Pick rows, change their XP or money in one go, and save once. Tick several (shift-click for a range) or type straight into a cell.</p>
    </div>
  </div>

  <div class="tabs" role="tablist">
    {#each KINDS as k}
      <button role="tab" aria-selected={kind === k.id} class:on={kind === k.id} onclick={() => pick(k.id)}><k.icon size={15} /> {k.label}</button>
    {/each}
  </div>
  <p class="muted small blurb">{KINDS.find((k) => k.id === kind)?.blurb}</p>

  <section class="card tools">
    <div class="search"><Search size={16} /><input type="search" placeholder="Search by name…" bind:value={search} /></div>
    <select bind:value={group} aria-label="Filter">
      <option value="all">All groups</option>
      {#each groups as g}<option value={g}>{g}</option>{/each}
    </select>
    <span class="muted small count">{shown.length} of {rows.length} · {usd(total)} in total</span>
  </section>

  <section class="card action" class:idle={!targets.length}>
    <div class="who"><strong>{targets.length}</strong> selected</div>
    <div class="seg" role="group" aria-label="Field">
      {#each fields as f}<button class:on={field === f} onclick={() => (field = f)}>{f === 'money' ? 'Money' : f === 'xp' ? 'XP' : 'Level'}</button>{/each}
    </div>
    <div class="seg" role="group" aria-label="Operation">
      <button class:on={mode === 'set'} onclick={() => (mode = 'set')} title="Set to"><Equal size={14} /> Set</button>
      <button class:on={mode === 'multiply'} onclick={() => (mode = 'multiply')} title="Multiply by"><Percent size={14} /> ×</button>
      <button class:on={mode === 'add'} onclick={() => (mode = 'add')} title="Add"><Plus size={14} /> Add</button>
    </div>
    <input class="val" type="number" step="any" bind:value aria-label="Value" />
    {#if field === 'money'}<label class="chk"><input type="checkbox" bind:checked={roundTo5} /> round to 5s</label>{/if}
    <button class="primary" disabled={!targets.length} onclick={runAction}><Wand2 size={15} /> Apply</button>
    <span class="sep"></span>
    <button class="ghost" disabled={!targets.length} onclick={roundSelected} title="Round every selected money amount to the nearest 5"><Coins size={15} /> Round to 5s</button>
    <button class="ghost" disabled={!targets.length} onclick={autoSelected} title="Go back to the automatic, difficulty-scaled amount"><Sparkles size={15} /> Auto money</button>
    {#if kind === 'quest'}
      <button class="ghost" disabled={!targets.length} onclick={() => enableSelected(true)}><Power size={15} /> Enable</button>
      <button class="ghost" disabled={!targets.length} onclick={() => enableSelected(false)}><Eraser size={15} /> Disable</button>
    {/if}
    {#if sample.length}
      <div class="sample muted small">
        {#each sample as s}<span>{s.title}: <b>{field === 'money' ? usd(s.from) : s.from}</b> → <b>{field === 'money' ? usd(s.to) : s.to}</b></span>{/each}
        {#if targets.length > sample.length}<span>and {targets.length - sample.length} more</span>{/if}
      </div>
    {/if}
  </section>

  <section class="card tablecard">
    {#if loading}
      <div class="skeleton"></div>
    {:else if !shown.length}
      <p class="muted pad">Nothing matches.</p>
    {:else}
      <div class="scroll">
        <table>
          <thead>
            <tr>
              <th class="c"><input type="checkbox" checked={allShownSelected} onchange={toggleAll} aria-label="Select all" /></th>
              <th>Name</th>
              <th>Group</th>
              {#if kind === 'level_reward'}<th class="num">Level</th>{:else}<th class="num">XP</th>{/if}
              <th class="num">Money</th>
              {#if kind === 'quest'}<th class="c">On</th>{/if}
            </tr>
          </thead>
          <tbody>
            {#each shown as r (r.id)}
              <tr class:sel={selected.has(r.id)} class:dirty={isDirty(r)}>
                <td class="c"><input type="checkbox" checked={selected.has(r.id)} onclick={(e) => toggle(r.id, e)} aria-label="Select {r.title}" /></td>
                <td class="name"><strong>{r.title}</strong>{#if r.other_rewards}<small class="muted">+ {r.other_rewards} other reward{r.other_rewards === 1 ? '' : 's'}</small>{/if}</td>
                <td><span class="tag">{r.group}</span></td>
                {#if kind === 'level_reward'}
                  <td class="num"><input type="number" min="1" value={r.d.level ?? ''} oninput={(e) => edit(r, 'level', e.currentTarget.value)} /></td>
                {:else}
                  <td class="num"><input type="number" min="0" value={r.d.xp ?? ''} oninput={(e) => edit(r, 'xp', e.currentTarget.value)} /></td>
                {/if}
                <td class="num money">
                  <input type="number" min="0" step="5" value={r.d.reset ? '' : r.d.money ?? ''} placeholder="auto" oninput={(e) => edit(r, 'money', e.currentTarget.value)} />
                  {#if r.d.reset || (r.auto && r.d.money === r.money)}<span class="auto" title="Scaled automatically from the difficulty">auto</span>{/if}
                </td>
                {#if kind === 'quest'}<td class="c"><input type="checkbox" checked={!!r.d.enabled} onchange={(e) => (r.d.enabled = e.currentTarget.checked)} aria-label="Enabled" /></td>{/if}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>

{#if dirtyRows.length}
  <div class="savebar" role="status">
    <span>{dirtyRows.length} unsaved {dirtyRows.length === 1 ? 'change' : 'changes'}</span>
    <button class="ghost" onclick={revert}><RotateCcw size={15} /> Discard</button>
    <button class="primary" disabled={saving} onclick={save}><Save size={15} /> {saving ? 'Saving…' : 'Save changes'}</button>
  </div>
{/if}

<style>
  .page { padding-bottom: 100px; }
  .page-head h1 { display: flex; align-items: center; gap: 10px; }
  .tabs { display: inline-flex; gap: 4px; padding: 4px; border-radius: 12px; background: var(--bg-2); flex-wrap: wrap; }
  .tabs button { border: 0; background: transparent; padding: 8px 16px; border-radius: 9px; color: var(--muted); gap: 8px; }
  .tabs button.on { background: var(--accent-soft); color: var(--text); }
  .blurb { margin: 8px 2px 14px; }
  .tools { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 12px; padding: 10px 12px; }
  .search { display: flex; align-items: center; gap: 8px; padding: 0 12px; border-radius: 10px; background: var(--bg-2); flex: 1 1 220px; }
  .search input { border: 0; background: transparent; }
  .tools select { width: auto; min-width: 150px; }
  .count { margin-left: auto; }
  .action { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-bottom: 12px; padding: 12px 14px; border-color: color-mix(in srgb, var(--accent) 35%, var(--line)); background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 9%, var(--surface)), var(--surface)); transition: opacity 0.2s; }
  .action.idle { opacity: 0.6; }
  .who strong { font-size: 1.1rem; color: var(--accent-2); }
  .seg { display: inline-flex; padding: 3px; border-radius: 10px; background: var(--bg-2); gap: 2px; }
  .seg button { border: 0; background: transparent; padding: 6px 12px; border-radius: 8px; color: var(--muted); gap: 5px; }
  .seg button.on { background: var(--accent-soft); color: var(--text); }
  .val { width: 110px; font-variant-numeric: tabular-nums; }
  .chk { display: inline-flex; align-items: center; gap: 6px; font-size: 0.85rem; color: var(--text-2); }
  .sep { width: 1px; align-self: stretch; background: var(--line); margin: 2px 4px; }
  .sample { display: flex; flex-wrap: wrap; gap: 6px 16px; flex-basis: 100%; }
  .sample b { color: var(--text); font-variant-numeric: tabular-nums; }
  .tablecard { padding: 0; overflow: hidden; }
  .scroll { overflow: auto; max-height: calc(100vh - 380px); min-height: 260px; }
  table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
  thead th { position: sticky; top: 0; z-index: 1; background: var(--surface); text-align: left; padding: 10px 12px; font-size: 0.74rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); border-bottom: 1px solid var(--line); }
  td { padding: 6px 12px; border-bottom: 1px solid color-mix(in srgb, var(--line) 60%, transparent); }
  tbody tr:hover { background: color-mix(in srgb, var(--accent) 5%, transparent); }
  tr.sel { background: var(--accent-soft); }
  tr.dirty td:first-child { box-shadow: inset 3px 0 0 var(--accent-2); }
  .c { width: 44px; text-align: center; }
  .num { text-align: right; width: 150px; }
  td.num input { width: 112px; text-align: right; font-variant-numeric: tabular-nums; padding: 5px 8px; }
  .money { white-space: nowrap; }
  .auto { margin-left: 6px; font-size: 0.65rem; padding: 2px 6px; border-radius: 99px; background: var(--accent-soft); color: var(--accent-2); text-transform: uppercase; letter-spacing: 0.05em; }
  .name { display: flex; flex-direction: column; min-width: 160px; }
  .name small { font-size: 0.72rem; }
  .tag { padding: 2px 9px; border-radius: 99px; background: var(--bg-2); font-size: 0.75rem; color: var(--text-2); }
  .skeleton { height: 320px; background: var(--bg-2); }
  .pad { padding: 24px; }
  .savebar { position: fixed; left: 50%; bottom: 22px; transform: translateX(-50%); z-index: 40; display: flex; align-items: center; gap: 12px; padding: 10px 12px 10px 18px; border-radius: 14px; background: var(--surface); border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent); box-shadow: 0 14px 40px -12px rgba(0, 0, 0, 0.6); }
</style>
