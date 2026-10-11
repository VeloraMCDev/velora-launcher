<script lang="ts">
  import { onMount } from 'svelte';
  import { get, put } from '../lib/api';
  type Route = { id: string; experience: string; hostname: string; address: string; port: number; enabled: boolean };
  type Snapshot = { revision: number; routes: Route[] };
  type Overview = { snapshot: Snapshot; public_endpoint: string | null; status: null | { received_at: string; report: { revision: number; connections: number; error: string | null; routes: { id: string; reachable: boolean }[]; events?: {at: string; hostname: string; kind: string}[] } } };
  let data = $state<Overview | null>(null);
  let error = $state(''); let busy = $state(false); let draft = $state<Route | null>(null); let editing = $state(false);
  let now = $state(Date.now());
  const online = $derived(!!data?.status && now - Date.parse(data.status.received_at) < 90000);
  const synced = $derived(online && data?.status?.report.revision === data?.snapshot.revision && !data?.status?.report.error);
  async function refresh() { try { data = await get<Overview>('/api/admin/gateway'); error = ''; } catch (e) { error = String(e); } }
  onMount(() => { void refresh(); const timer = setInterval(() => { now = Date.now(); if (!busy && !draft) void refresh(); }, 5000); return () => clearInterval(timer); });
  function edit(route?: Route) {
    editing = !!route;
    draft = route ? { ...route } : { id: crypto.randomUUID(), experience: '', hostname: '', address: '', port: 25565, enabled: false };
    error = '';
  }
  async function save(routes: Route[]) {
    if (!data) return; busy = true; error = '';
    try { data.snapshot = await put<Snapshot>('/api/admin/gateway', { revision: data.snapshot.revision, routes }); draft = null; await refresh(); }
    catch (e) { error = String(e); } finally { busy = false; }
  }
  function submit(event: SubmitEvent) { event.preventDefault(); if (draft && data) void save([...data.snapshot.routes.filter(r => r.id !== draft?.id), { ...draft }]); }
</script>

<div class="gateway">
  <header><div><h1>Gateway</h1><p>Independent Minecraft experiences, routed through your private tunnel.</p></div><button onclick={() => edit()} disabled={busy || !data}>Add route</button></header>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#if data}
    <div class="overview">
      <div><small>Gateway</small><strong>{online ? 'Online' : 'Offline / no recent report'}</strong></div>
      <div><small>Public endpoint</small><strong>{data.public_endpoint ?? 'Not configured'}</strong></div>
      <div><small>Connections</small><strong>{online ? data.status?.report.connections : 'Unknown'}</strong></div>
      <div><small>Routes</small><strong>{data.snapshot.routes.filter(r => r.enabled).length} enabled / {data.snapshot.routes.length} total</strong></div>
    </div>
    <p aria-live="polite">{synced ? `Confirmed: revision ${data.snapshot.revision} applied` : `Awaiting gateway confirmation for revision ${data.snapshot.revision}`}</p>
    {#if data.status?.report.error}<p role="alert" class="error">{data.status.report.error}</p>{/if}
    <div class="table-wrap"><table><thead><tr><th>Experience</th><th>Hostname</th><th>Private backend</th><th>Tunnel TCP status</th><th>Actions</th></tr></thead><tbody>
      {#each data.snapshot.routes as r (r.id)}
        <tr><td>{r.experience}</td><td>{r.hostname}</td><td>{r.address}:{r.port}</td><td>{!r.enabled ? 'Disabled' : !synced ? 'Unconfirmed' : data.status?.report.routes.find(h => h.id === r.id)?.reachable ? 'Reachable' : 'Unreachable'}</td><td><button disabled={busy} onclick={() => edit(r)}>Edit</button> <button disabled={busy} onclick={() => save(data!.snapshot.routes.map(x => x.id === r.id ? { ...x, enabled: !x.enabled } : x))}>{r.enabled ? 'Disable' : 'Enable'}</button></td></tr>
      {:else}<tr><td colspan="5">Add your first hostname route. Enable it after authentication and modpack compatibility testing.</td></tr>{/each}
    </tbody></table></div>
    <p class="hint">Use smp.velora.scopedd.lol and unleashed.velora.scopedd.lol for your experiences. Backend reachability checks confirm TCP access only; Minecraft login and gameplay require separate testing. Calagopus retains server lifecycle management.</p>
    <h2>Recent connections</h2>
    <p>Latest 50 assignment and closure events. Player identities and addresses are omitted.</p>
    {#each [...(data.status?.report.events ?? [])].reverse() as event}<p><time>{event.at}</time> · {event.hostname} · {event.kind}</p>{:else}<p>No connection events reported.</p>{/each}
  {:else if error}<p>Unable to load gateway. Retrying automatically…</p><button onclick={() => refresh()}>Retry now</button>
  {:else}<p>Loading gateway…</p>{/if}
</div>
{#if draft}
  <div class="shade"></div><aside aria-label={editing ? 'Edit route' : 'Add route'}>
    <form onsubmit={submit}><h2>{editing ? 'Edit route' : 'Add route'}</h2>
      <label>Experience<input required maxlength="120" bind:value={draft.experience} /></label>
      <label>Hostname<input required placeholder="smp.velora.scopedd.lol" bind:value={draft.hostname} /></label>
      <label>Private tunnel IPv4<input required placeholder="Private server address" bind:value={draft.address} /></label>
      <label>Backend port<input type="number" required min="1" max="65535" bind:value={draft.port} /></label>
      <label class="toggle"><input type="checkbox" bind:checked={draft.enabled} /> Enabled</label>
      <p>Manual endpoint. Configure the server in Calagopus separately. Route changes disconnect affected sessions; players reconnect to the same experience.</p>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <footer><button type="button" disabled={busy} onclick={() => draft = null}>Cancel</button><button type="submit" disabled={busy}>Save</button></footer>
      {#if editing}<button class="delete" type="button" disabled={busy} onclick={() => { if (confirm('Delete this route? Active sessions will disconnect.')) void save(data!.snapshot.routes.filter(r => r.id !== draft?.id)); }}>Delete route</button>{/if}
    </form>
  </aside>
{/if}
<style>
  .gateway { padding: 2rem; max-width: 1400px; margin: auto; } header { display:flex; justify-content:space-between; gap:1rem; align-items:center; } h1 { margin:0; } p, small { color:var(--muted); } .overview { display:grid; grid-template-columns:repeat(auto-fit,minmax(180px,1fr)); gap:1rem; margin:2rem 0; } .overview > div { background:var(--surface-2); padding:1.25rem; border-radius:12px; } small,strong { display:block; } strong { margin-top:.6rem; } .table-wrap { overflow:auto; } table { width:100%; border-collapse:collapse; } th,td { text-align:left; padding:1rem; border-bottom:1px solid var(--surface-3); } button { cursor:pointer; } .error { color:#fb7185; } .hint { line-height:1.7; } .shade { position:fixed; inset:0; background:#0009; z-index:40; } aside { position:fixed; right:0; top:0; bottom:0; width:min(480px,100%); overflow:auto; background:var(--surface-1,#18121f); padding:2rem; z-index:41; box-shadow:-12px 0 40px #0006; } label { display:block; margin:1rem 0; } input:not([type=checkbox]) { width:100%; margin-top:.5rem; } footer { display:flex; gap:1rem; justify-content:flex-end; } .delete { margin-top:2rem; color:#fb7185; } @media(max-width:600px) { .gateway { padding:1rem; } header { align-items:flex-start; } }
</style>
