<script lang="ts">
  import { onMount } from 'svelte';
  import { Play, Clock, CircleCheck, CircleAlert, ChevronDown, Save, Lock, Loader } from '@lucide/svelte';
  import Toggle from '../components/Toggle.svelte';
  import { get, post, put, timeAgo, duration } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Run = { at: string; duration_ms: number; ok: boolean; message: string };
  type Task = {
    id: string; name: string; description: string; category: string; driven: boolean;
    enabled: boolean; interval_secs: number; default_interval_secs: number; settings: Record<string, any>;
    last_run_at: string | null; last_ok: boolean | null; last_message: string | null; last_duration_ms: number | null;
    next_run_at: string | null; runs: number; failures: number; history: Run[];
  };

  let tasks = $state<Task[]>([]);
  let open = $state<string | null>(null);
  let running = $state<string | null>(null);
  let loaded = $state(false);

  async function load() {
    try {
      const d = await get<{ tasks: Task[] }>('/api/admin/tasks');
      // Keep unsaved edits to the open task while background refreshes land.
      tasks = d.tasks.map((t) => (t.id === open && tasks.find((x) => x.id === t.id) ? { ...t, interval_secs: tasks.find((x) => x.id === t.id)!.interval_secs, settings: tasks.find((x) => x.id === t.id)!.settings } : t));
      loaded = true;
    } catch (e) { toastError(e); }
  }
  onMount(() => { load(); const i = setInterval(load, 10_000); return () => clearInterval(i); });

  const groups = $derived([...new Set(tasks.map((t) => t.category))].map((c) => ({ c, items: tasks.filter((t) => t.category === c) })));

  const UNITS: [string, number][] = [['seconds', 1], ['minutes', 60], ['hours', 3600], ['days', 86400]];
  function split(secs: number): [number, number] {
    for (const [, m] of [...UNITS].reverse()) if (secs % m === 0) return [secs / m, m];
    return [secs, 1];
  }
  function every(secs: number) {
    const [n, m] = split(secs);
    const name = UNITS.find((u) => u[1] === m)![0];
    return n === 1 ? `every ${name.slice(0, -1)}` : `every ${n} ${name}`;
  }
  function until(iso: string | null) {
    if (!iso) return '—';
    const s = Math.round((new Date(iso).getTime() - Date.now()) / 1000);
    return s <= 5 ? 'any moment' : `in ${duration(s)}`;
  }

  async function toggle(t: Task) {
    try { await put(`/api/admin/tasks/${t.id}`, { enabled: t.enabled }); toast(t.enabled ? `${t.name} is on` : `${t.name} paused`); await load(); } catch (e) { toastError(e); t.enabled = !t.enabled; }
  }
  async function save(t: Task) {
    try {
      await put(`/api/admin/tasks/${t.id}`, { interval_secs: t.driven ? t.interval_secs : undefined, settings: t.settings });
      toast('Saved'); await load();
    } catch (e) { toastError(e); }
  }
  async function runNow(t: Task) {
    running = t.id;
    try {
      const r = await post<{ ok: boolean; message: string }>(`/api/admin/tasks/${t.id}/run`);
      toast(r.message || (r.ok ? 'Done' : 'Failed'), r.ok ? 'ok' : 'error');
      await load();
    } catch (e) { toastError(e); } finally { running = null; }
  }
  function setUnit(t: Task, mult: number) { const [n] = split(t.interval_secs); t.interval_secs = Math.max(1, n) * mult; }
  function setNum(t: Task, n: number) { const [, m] = split(t.interval_secs); t.interval_secs = Math.max(1, Math.round(n || 1)) * m; }
</script>

<div class="page wide">
  <header>
    <div>
      <h1>Scheduled tasks</h1>
      <p>Everything the panel does on its own. Pause, retune or run a task right now, and see how its last runs went.</p>
    </div>
  </header>

  {#if !loaded}
    <p class="muted">Loading…</p>
  {/if}

  {#each groups as g}
    <h2 class="cat">{g.c}</h2>
    <div class="list">
      {#each g.items as t (t.id)}
        <article class="card task" class:off={t.driven && !t.enabled}>
          <div class="row">
            <span class="state" class:ok={t.last_ok === true} class:bad={t.last_ok === false}>
              {#if t.last_ok === false}<CircleAlert size={18} />{:else if t.last_ok}<CircleCheck size={18} />{:else}<Clock size={18} />{/if}
            </span>
            <div class="main">
              <h3>{t.name}
                {#if !t.driven}<span class="pill"><Lock size={11} /> built-in</span>{/if}
                {#if t.driven && !t.enabled}<span class="pill warn">paused</span>{/if}
              </h3>
              <p>{t.description}</p>
              <div class="meta">
                <span>{every(t.interval_secs)}</span>
                <span>last run {t.last_run_at ? timeAgo(t.last_run_at) : 'never'}{#if t.last_duration_ms != null} · {t.last_duration_ms} ms{/if}</span>
                {#if t.next_run_at}<span>next {until(t.next_run_at)}</span>{/if}
                <span>{t.runs} runs{t.failures ? `, ${t.failures} failed` : ''}</span>
              </div>
              {#if t.last_message}<p class="msg" class:bad={t.last_ok === false}>{t.last_message}</p>{/if}
            </div>
            <div class="side">
              {#if t.driven}<div class="tg"><Toggle bind:checked={t.enabled} label="" onchange={() => toggle(t)} /></div>{/if}
              <button onclick={() => runNow(t)} disabled={running === t.id}>
                {#if running === t.id}<Loader size={14} class="spin" />{:else}<Play size={14} />{/if} Run now
              </button>
              <button class="ghost" onclick={() => (open = open === t.id ? null : t.id)} aria-expanded={open === t.id}>
                Details <ChevronDown size={14} class={open === t.id ? 'flip' : ''} />
              </button>
            </div>
          </div>

          {#if open === t.id}
            <div class="detail">
              {#if t.driven}
                <div class="field">
                  <span class="lbl">Runs every</span>
                  <div class="iv">
                    <input type="number" min="1" value={split(t.interval_secs)[0]} oninput={(e) => setNum(t, +e.currentTarget.value)} />
                    <select value={split(t.interval_secs)[1]} onchange={(e) => setUnit(t, +e.currentTarget.value)}>
                      {#each UNITS as [name, m]}<option value={m}>{name}</option>{/each}
                    </select>
                  </div>
                </div>
              {/if}
              {#if t.id === 'purge_deleted_players'}
                <Toggle bind:checked={t.settings.dry_run} label="Dry run" help="Only report what would be deleted. Nothing is removed until you turn this off." />
                <Toggle bind:checked={t.settings.include_unregistered} label="Also clean players who never had an account" help="Offline-mode players with no panel account and no activity for the number of days below. Leave off unless your server is online-mode." />
                {#if t.settings.include_unregistered}
                  <label class="field"><span class="lbl">Inactive for (days)</span><input type="number" min="1" bind:value={t.settings.grace_days} /></label>
                {/if}
              {/if}
              <div class="actions"><button onclick={() => save(t)}><Save size={14} /> Save changes</button></div>

              <h4>Recent runs</h4>
              {#if t.history.length}
                <ul class="hist">
                  {#each t.history as r}
                    <li class:bad={!r.ok}>
                      {#if r.ok}<CircleCheck size={14} />{:else}<CircleAlert size={14} />{/if}
                      <span class="when">{timeAgo(r.at)}</span>
                      <span class="what">{r.message}</span>
                      <span class="dur">{r.duration_ms} ms</span>
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="muted">No runs recorded yet.</p>
              {/if}
            </div>
          {/if}
        </article>
      {/each}
    </div>
  {/each}
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  .cat { margin: 10px 0 0; font-size: 0.78rem; text-transform: uppercase; letter-spacing: 0.09em; color: var(--muted); }
  .list { display: flex; flex-direction: column; gap: 10px; }
  .task { padding: 0; overflow: hidden; transition: opacity 0.2s; }
  .task.off { opacity: 0.65; }
  .row { display: flex; gap: 14px; padding: 16px 18px; align-items: flex-start; }
  .state { width: 2.2rem; height: 2.2rem; border-radius: 0.7rem; display: grid; place-items: center; flex-shrink: 0; background: var(--bg-2); color: var(--muted); }
  .state.ok { color: #34d399; background: color-mix(in srgb, #34d399 14%, transparent); }
  .state.bad { color: #f87171; background: color-mix(in srgb, #f87171 14%, transparent); }
  .main { flex: 1; min-width: 0; }
  h3 { margin: 0 0 4px; font-size: 1rem; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .main p { margin: 0 0 8px; color: var(--muted); font-size: 0.85rem; line-height: 1.45; }
  .pill { display: inline-flex; align-items: center; gap: 4px; font-size: 0.68rem; font-weight: 600; padding: 2px 8px; border-radius: 99px; background: var(--bg-2); color: var(--muted); border: 1px solid var(--line); }
  .pill.warn { color: #fbbf24; border-color: color-mix(in srgb, #fbbf24 40%, transparent); }
  .meta { display: flex; flex-wrap: wrap; gap: 6px 16px; font-size: 0.76rem; color: var(--muted); }
  .msg { margin: 8px 0 0 !important; font: 0.78rem ui-monospace, monospace; color: var(--text) !important; background: var(--bg-2); padding: 6px 10px; border-radius: 8px; word-break: break-word; }
  .msg.bad { color: #f87171 !important; }
  .side { display: flex; flex-direction: column; align-items: flex-end; gap: 8px; }
  .side button { display: inline-flex; align-items: center; gap: 6px; }
  .tg :global(.text) { display: none; }
  .detail { border-top: 1px solid var(--line); padding: 16px 18px 18px 70px; display: flex; flex-direction: column; gap: 12px; background: color-mix(in srgb, var(--bg-2) 50%, transparent); }
  .field { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; }
  .lbl { font-weight: 600; font-size: 0.82rem; }
  .iv { display: flex; gap: 8px; max-width: 18rem; }
  .iv input { width: 6rem; }
  .actions { display: flex; }
  .actions button { display: inline-flex; align-items: center; gap: 6px; }
  h4 { margin: 6px 0 0; font-size: 0.85rem; }
  .hist { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .hist li { display: grid; grid-template-columns: 16px 90px 1fr auto; gap: 10px; align-items: center; font-size: 0.78rem; padding: 5px 8px; border-radius: 8px; color: #34d399; }
  .hist li.bad { color: #f87171; background: color-mix(in srgb, #f87171 8%, transparent); }
  .hist .when, .hist .dur { color: var(--muted); }
  .hist .what { color: var(--text); word-break: break-word; }
  .muted { color: var(--muted); font-size: 0.82rem; }
  :global(.flip) { transform: rotate(180deg); }
  :global(.spin) { animation: spin 0.9s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 760px) { .row { flex-wrap: wrap; } .side { flex-direction: row; align-items: center; } .detail { padding-left: 18px; } }
</style>
