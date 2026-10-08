<script lang="ts">
  import { Upload, RotateCcw, Clock, Skull, Pickaxe, Sword, LogIn, Circle } from '@lucide/svelte';
  import SkinView from './SkinView.svelte';
  import CapePreview from './CapePreview.svelte';
  import { api, del, duration, get, put, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Cape, User, UserActivity } from '../lib/types';

  let { user, capes, onchange }: { user: User; capes: Cape[]; onchange: (u: User) => void } = $props();

  let activity = $state<UserActivity | null>(null);
  let busy = $state(false);
  let version = $state(0);

  $effect(() => {
    activity = null;
    get<UserActivity>(`/api/admin/users/${user.id}/activity`).then((a) => (activity = a)).catch(toastError);
  });

  const cape = $derived(capes.find((c) => c.id === user.cape_id) ?? null);
  const skin = $derived(user.skin_url ? `${user.skin_url}?v=${version}` : null);
  const totals = $derived.by(() => {
    const t = { playtime: 0, deaths: 0, mined: 0, kills: 0, joins: 0 };
    for (const s of activity?.servers ?? []) {
      t.playtime += s.playtime_secs;
      t.deaths += s.deaths;
      t.mined += s.blocks_broken;
      t.kills += s.player_kills + s.mob_kills;
      t.joins += s.joins;
    }
    return t;
  });

  async function run(p: Promise<User>, done: string) {
    busy = true;
    try {
      const u = await p;
      version++;
      onchange(u);
      toast(done);
    } catch (e) {
      toastError(e);
    } finally {
      busy = false;
    }
  }
  function upload(e: Event & { currentTarget: HTMLInputElement }) {
    const f = e.currentTarget.files?.[0];
    e.currentTarget.value = '';
    if (!f) return;
    const form = new FormData();
    form.append('model', user.skin_model);
    form.append('file', f);
    run(api<User>(`/api/admin/users/${user.id}/skin`, { method: 'POST', form }), 'Skin updated');
  }
  const setModel = (model: string) => model !== user.skin_model && run(put<User>(`/api/admin/users/${user.id}/skin/model`, { model }), 'Arm style updated');
  const resetSkin = () => run(del<User>(`/api/admin/users/${user.id}/skin`), 'Skin reset');
  const setCape = (id: number | null) => run(put<User>(`/api/admin/users/${user.id}/cape`, { cape_id: id }), id ? 'Cape given' : 'Cape removed');
</script>

<div class="panel">
  <div class="look" class:busy>
    <div class="figures">
      <SkinView {skin} cape={cape?.url ?? null} slim={user.skin_model === 'slim'} side="front" scale={6} />
      <SkinView {skin} cape={cape?.url ?? null} slim={user.skin_model === 'slim'} side="back" scale={6} />
    </div>
    <div class="segmented">
      <button class:active={user.skin_model === 'classic'} disabled={busy} onclick={() => setModel('classic')}>Classic</button>
      <button class:active={user.skin_model === 'slim'} disabled={busy} onclick={() => setModel('slim')}>Slim</button>
    </div>
    <div class="row">
      <label class="btn sm up"><Upload size={14} /> Upload skin<input type="file" accept="image/png" onchange={upload} disabled={busy} /></label>
      {#if user.skin_url}<button class="sm ghost" disabled={busy} onclick={resetSkin}><RotateCcw size={14} /> Reset</button>{/if}
    </div>
  </div>

  <div class="info">
    <div class="field">
      <span class="lbl">Cape</span>
      <div class="capes">
        <button class="cape" class:on={!user.cape_id} disabled={busy} onclick={() => user.cape_id && setCape(null)} title="No cape"><span class="none"></span></button>
        {#each capes as c (c.id)}
          <button class="cape" class:on={user.cape_id === c.id} disabled={busy} onclick={() => user.cape_id !== c.id && setCape(c.id)} title={c.name}><CapePreview src={c.url} scale={3} /></button>
        {/each}
      </div>
      {#if !capes.length}<span class="muted tiny">Upload capes on the <a href="#/capes">Capes</a> page.</span>{:else}<span class="muted tiny">{cape ? cape.name : 'No cape'} · admins can give any cape, including "given only" ones.</span>{/if}
    </div>

    <div class="field">
      <span class="lbl row between">In game {#if activity?.online_on.length}<span class="badge good"><Circle size={8} fill="currentColor" /> Online on {activity.online_on.map((s) => s.name).join(', ')}</span>{/if}</span>
      <div class="stats">
        <span><Clock size={14} /><strong>{duration(totals.playtime)}</strong><small>played</small></span>
        <span><LogIn size={14} /><strong>{totals.joins.toLocaleString()}</strong><small>joins</small></span>
        <span><Pickaxe size={14} /><strong>{totals.mined.toLocaleString()}</strong><small>mined</small></span>
        <span><Sword size={14} /><strong>{totals.kills.toLocaleString()}</strong><small>kills</small></span>
        <span><Skull size={14} /><strong>{totals.deaths.toLocaleString()}</strong><small>deaths</small></span>
      </div>
    </div>

    {#if activity?.servers.length}
      <table class="table mini">
        <thead><tr><th>Server</th><th class="num">Playtime</th><th class="num">Last seen</th></tr></thead>
        <tbody>
          {#each activity.servers as s (s.server_id)}
            <tr><td><a href="#/servers/{s.server_id}">{s.server_name}</a></td><td class="num">{duration(s.playtime_secs)}</td><td class="num muted">{timeAgo(s.last_seen)}</td></tr>
          {/each}
        </tbody>
      </table>
    {:else if activity}
      <p class="muted small">Hasn't played on a connected server yet.</p>
    {/if}

    {#if activity?.events.length}
      <div class="field">
        <span class="lbl">Recent</span>
        <div class="events">
          {#each activity.events.slice(0, 8) as e (e.id)}
            <div class="ev"><span class="kind">{e.kind}</span><span class="grow">{e.detail ?? ''}</span><span class="muted tiny">{timeAgo(e.created_at)}</span></div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .panel { display: grid; grid-template-columns: auto 1fr; gap: 24px; }
  .look { display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 18px; background: var(--bg-2); border: 1px solid var(--line); border-radius: var(--radius); transition: opacity 0.15s; align-self: start; }
  .look.busy { opacity: 0.6; }
  .figures { display: flex; gap: 14px; }
  .up { position: relative; overflow: hidden; padding: 6px 11px; font-size: 0.85rem; }
  .up input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .info { display: flex; flex-direction: column; gap: 18px; min-width: 0; }
  .field { display: flex; flex-direction: column; gap: 8px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .between { justify-content: space-between; }
  .capes { display: flex; flex-wrap: wrap; gap: 6px; }
  .cape { padding: 5px; background: var(--bg-2); border-radius: 6px; }
  .cape.on { border-color: var(--accent); background: var(--accent-soft); }
  .none { display: block; width: 30px; height: 48px; border-radius: 3px; background: repeating-linear-gradient(135deg, transparent 0 5px, rgba(255, 255, 255, 0.06) 5px 10px); }
  .stats { display: grid; grid-template-columns: repeat(5, 1fr); gap: 6px; }
  .stats span { display: flex; flex-direction: column; gap: 2px; padding: 10px; border-radius: 8px; background: var(--bg-2); border: 1px solid var(--line); }
  .stats :global(svg) { color: var(--muted); margin-bottom: 4px; }
  .stats strong { font-variant-numeric: tabular-nums; font-size: 0.95rem; }
  .stats small { color: var(--muted); font-size: 0.72rem; }
  .mini td, .mini th { padding: 8px 10px; }
  .num { text-align: right; font-variant-numeric: tabular-nums; }
  .events { display: flex; flex-direction: column; gap: 6px; }
  .ev { display: flex; gap: 10px; align-items: baseline; font-size: 0.83rem; }
  .kind { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.04em; color: var(--muted); width: 84px; flex-shrink: 0; }
  .grow { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-2); }
  @media (max-width: 700px) { .panel { grid-template-columns: 1fr; } .stats { grid-template-columns: repeat(3, 1fr); } }
</style>
