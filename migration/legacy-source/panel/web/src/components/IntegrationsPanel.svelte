<script lang="ts">
  import { Plug, RefreshCw, Shield, Gauge, History, Map as MapIcon, Archive } from '@lucide/svelte';
  import Avatar from './Avatar.svelte';
  import { get, put, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Entry = { name: string; version: string; data: any; updated_at: string };
  type Rank = { uuid: string; name: string; primary: string; display: string; prefix: string; weight: number; groups: string[]; permissions: string[] };
  type Payload = { integrations: Entry[]; settings: { luckperms_sync: string }; ranks: Rank[] };

  let { id }: { id: number } = $props();
  let p = $state<Payload | null>(null);
  let regionFilter = $state('');

  async function refresh() {
    try {
      p = await get<Payload>(`/api/admin/servers/${id}/integrations`);
    } catch (e) {
      toastError(e);
    }
  }
  $effect(() => {
    void id;
    refresh();
    const t = setInterval(refresh, 20000);
    return () => clearInterval(t);
  });

  const by = (name: string) => p?.integrations.find((i) => i.name === name);
  const lp = $derived(by('luckperms'));
  const wg = $derived(by('worldguard'));
  const spark = $derived(by('spark'));
  const cp = $derived(by('coreprotect'));
  const regions = $derived(((wg?.data?.regions ?? []) as any[]).filter((r) => !regionFilter || `${r.id} ${r.world}`.toLowerCase().includes(regionFilter.toLowerCase())));
  const fmtMb = (n: number) => (n >= 1024 ? `${(n / 1024).toFixed(1)} GB` : `${n} MB`);
  const fmtBytes = (n: number) => (n >= 1 << 30 ? `${(n / (1 << 30)).toFixed(1)} GB` : `${Math.round(n / (1 << 20))} MB`);
  const tpsClass = (v?: number) => (v == null ? '' : v >= 19 ? 'good' : v >= 15 ? 'warn' : 'bad');
  const spread = (r: any) => `${r.min?.[0]}, ${r.min?.[1]}, ${r.min?.[2]} → ${r.max?.[0]}, ${r.max?.[1]}, ${r.max?.[2]}`;

  async function setSync(mode: string) {
    try {
      await put('/api/admin/integrations/settings', { luckperms_sync: mode });
      toast('LuckPerms sync updated');
      refresh();
    } catch (e) {
      toastError(e);
    }
  }
  const modes = [
    { id: 'off', label: 'Show ranks only', hint: 'Ranks appear in profiles; nothing is changed.' },
    { id: 'game_to_panel', label: 'Game → Panel', hint: 'Mapped LuckPerms groups add the matching panel group.' },
    { id: 'panel_to_game', label: 'Panel → Game', hint: 'Mapped panel groups are added in LuckPerms (needs apply-panel-groups).' },
    { id: 'both', label: 'Both ways', hint: 'Adds in both directions, never removes.' },
  ];
</script>

{#if p && p.integrations.length}
  <div class="grid integ">
    {#if lp}
      <section class="card wide">
        <div class="section-title">
          <h2><Shield size={17} /> LuckPerms ranks</h2>
          <span class="hint">v{lp.version} · {timeAgo(lp.updated_at)}</span>
          <button class="ghost icon" aria-label="Refresh" onclick={refresh}><RefreshCw size={15} /></button>
        </div>
        <div class="modes" role="radiogroup" aria-label="Group sync">
          {#each modes as m}
            <button class="mode" class:on={p.settings.luckperms_sync === m.id} role="radio" aria-checked={p.settings.luckperms_sync === m.id} onclick={() => setSync(m.id)}>
              <strong>{m.label}</strong><span class="muted tiny">{m.hint}</span>
            </button>
          {/each}
        </div>
        <p class="muted tiny">LuckPerms always decides real permissions. Map panel groups to LuckPerms groups under Players → Groups.</p>
        {#if p.ranks.length}
          <div class="ranks">
            {#each p.ranks as r (r.uuid)}
              <div class="rank">
                <Avatar name={r.name} uuid={r.uuid} size={28} />
                <div class="who"><strong>{r.name}</strong><span class="muted tiny">{r.permissions.length} permission{r.permissions.length === 1 ? '' : 's'} shown</span></div>
                <span class="chips">
                  <span class="chip main">{r.display || r.primary}</span>
                  {#each r.groups.filter((g) => g !== r.primary).slice(0, 4) as g}<span class="chip">{g}</span>{/each}
                </span>
              </div>
            {/each}
          </div>
        {:else}
          <p class="muted small">Ranks appear once players are online.</p>
        {/if}
      </section>
    {/if}

    {#if spark}
      {@const tps = spark.data?.tps ?? {}}
      <section class="card">
        <div class="section-title"><h2><Gauge size={17} /> Spark</h2><span class="hint">{timeAgo(spark.updated_at)}</span></div>
        <div class="stats">
          <div class="stat {tpsClass(tps.m1)}"><span class="muted tiny">TPS (1m)</span><b>{tps.m1 ?? '–'}</b></div>
          <div class="stat"><span class="muted tiny">MSPT (10s)</span><b>{spark.data?.mspt?.s10?.mean ?? '–'}<small>ms</small></b></div>
          <div class="stat"><span class="muted tiny">CPU (proc)</span><b>{spark.data?.cpu?.process?.s10 != null ? Math.round(spark.data.cpu.process.s10 * 100) : '–'}<small>%</small></b></div>
          <div class="stat"><span class="muted tiny">Memory</span><b>{spark.data?.memory ? fmtMb(spark.data.memory.used_mb) : '–'}</b></div>
        </div>
        {#if spark.data?.memory}
          <div class="bar"><span style:width="{Math.min(100, (spark.data.memory.used_mb / spark.data.memory.max_mb) * 100)}%"></span></div>
          <span class="muted tiny">of {fmtMb(spark.data.memory.max_mb)} allocated</span>
        {/if}
      </section>
    {/if}

    {#if cp}
      {@const d = cp.data ?? {}}
      <section class="card">
        <div class="section-title"><h2><History size={17} /> CoreProtect</h2><span class="hint">{timeAgo(cp.updated_at)}</span></div>
        <div class="stats">
          <div class="stat"><span class="muted tiny">Logged last hour</span><b>{d.logged_last_hour ?? 0}</b></div>
          <div class="stat"><span class="muted tiny">Since restart</span><b>{d.logged_total ?? 0}</b></div>
        </div>
        {#if d.per_minute?.length}
          {@const max = Math.max(1, ...d.per_minute)}
          <div class="spark" aria-hidden="true">{#each d.per_minute as v}<i style:height="{Math.max(3, (v / max) * 100)}%"></i>{/each}</div>
        {/if}
        <div class="row backup">
          <Archive size={15} />
          {#if d.backups?.found}
            <span class="small">{d.backups.count} backup{d.backups.count === 1 ? '' : 's'} · {fmtBytes(d.backups.bytes)}{#if d.backups.latest_at} · latest {timeAgo(new Date(d.backups.latest_at).toISOString())}{/if}</span>
          {:else}
            <span class="muted small">No backups found in the configured folders.</span>
          {/if}
        </div>
        {#if d.top_actors?.length}
          <div class="chips">{#each d.top_actors as a}<span class="chip">{a.name} · {a.count}</span>{/each}</div>
        {/if}
      </section>
    {/if}

    {#if wg}
      <section class="card wide">
        <div class="section-title">
          <h2><MapIcon size={17} /> WorldGuard regions</h2>
          <span class="hint">{wg.data?.regions?.length ?? 0}{wg.data?.truncated ? '+' : ''} · {timeAgo(wg.updated_at)}</span>
          <input class="filter" bind:value={regionFilter} placeholder="Filter" aria-label="Filter regions" />
        </div>
        {#if regions.length}
          <div class="table-wrap">
            <table class="table">
              <thead><tr><th>Region</th><th>World</th><th>Area</th><th class="num">Priority</th><th>Owners</th></tr></thead>
              <tbody>
                {#each regions.slice(0, 100) as r (r.world + r.id)}
                  <tr>
                    <td><strong>{r.id}</strong>{#if r.id === '__global__'} <span class="chip">global</span>{/if}</td>
                    <td class="muted">{r.world}</td>
                    <td class="muted small nowrap">{r.id === '__global__' ? 'Everywhere' : spread(r)}</td>
                    <td class="num">{r.priority}</td>
                    <td class="muted small">{r.owners?.length ? r.owners.join(', ') : r.owner_count ? `${r.owner_count} owners` : 'Server-owned'}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="muted small">No regions match.</p>
        {/if}
      </section>
    {/if}
  </div>
{:else if p}
  <section class="card empty">
    <div class="section-title"><h2><Plug size={17} /> Plugin integrations</h2></div>
    <p class="muted small">LuckPerms, WorldGuard, Spark and CoreProtect appear here automatically once they are installed on this server and it has synced. Turn them off in <code>config.yml</code> under <code>integrations:</code>.</p>
  </section>
{/if}

<style>
  .integ { grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); margin-bottom: 16px; align-items: start; }
  .wide { grid-column: 1 / -1; }
  .section-title h2 { display: inline-flex; align-items: center; gap: 8px; }
  .modes { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 8px; margin: 4px 0 10px; }
  .mode { display: flex; flex-direction: column; gap: 3px; align-items: flex-start; text-align: left; padding: 10px 12px; border-radius: 10px; background: var(--bg-2); border: 1px solid var(--border); color: var(--text); }
  .mode.on { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 10%, var(--bg-2)); }
  .ranks { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 8px; margin-top: 12px; max-height: 360px; overflow-y: auto; }
  .rank { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: 10px; background: var(--bg-2); }
  .who { display: flex; flex-direction: column; min-width: 0; flex: 1; }
  .chips { display: flex; flex-wrap: wrap; gap: 5px; }
  .chip { padding: 2px 9px; border-radius: 99px; font-size: 0.72rem; background: var(--surface); border: 1px solid var(--border); color: var(--text-2); }
  .chip.main { background: color-mix(in srgb, var(--accent) 16%, transparent); border-color: transparent; color: var(--text); font-weight: 600; }
  .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(110px, 1fr)); gap: 10px; }
  .stat { display: flex; flex-direction: column; gap: 4px; padding: 10px 12px; border-radius: 10px; background: var(--bg-2); }
  .stat b { font-size: 1.3rem; font-variant-numeric: tabular-nums; letter-spacing: -0.02em; }
  .stat small { font-size: 0.75rem; color: var(--muted); margin-left: 2px; font-weight: 500; }
  .stat.good b { color: #6ee7b7; } .stat.warn b { color: #fcd34d; } .stat.bad b { color: #fda4af; }
  .bar { height: 6px; border-radius: 99px; background: var(--bg-2); overflow: hidden; margin-top: 12px; }
  .bar span { display: block; height: 100%; background: linear-gradient(90deg, var(--accent), #22d3ee); border-radius: 99px; }
  .spark { display: flex; align-items: flex-end; gap: 2px; height: 46px; margin: 12px 0; }
  .spark i { flex: 1; background: color-mix(in srgb, var(--accent) 70%, transparent); border-radius: 2px; min-height: 3px; }
  .backup { gap: 8px; margin-bottom: 8px; color: var(--muted); }
  .filter { width: 160px; margin-left: auto; padding: 6px 10px; font-size: 0.85rem; }
  .table-wrap { overflow-x: auto; max-height: 380px; overflow-y: auto; }
  .num { text-align: right; } .nowrap { white-space: nowrap; }
  .empty code { background: var(--bg-2); padding: 1px 6px; border-radius: 6px; }
</style>
