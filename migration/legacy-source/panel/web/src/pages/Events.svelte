<script lang="ts">
  import { CalendarDays, Pencil, Plus, RefreshCw, Trash2 } from '@lucide/svelte';
  import { del, get, post, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type EventRow = { id: string; title: string; description: string; status: string; starts_at: string; ends_at: string; instance_id?: string | null; server_id?: number | null; objectives: unknown[]; rewards?: unknown[] };
  let events = $state<EventRow[]>([]);
  let loading = $state(false);
  let editing = $state<string | null>(null);
  let draft = $state({ title: '', description: '', starts_at: '', ends_at: '', status: 'draft', instance_id: '', server_id: '', objectives: '[]', rewards: '[]' });

  async function load() { loading = true; try { events = await get<EventRow[]>('/api/admin/events'); } catch (e) { toastError(e); } finally { loading = false; } }
  function edit(event: EventRow) {
    editing = event.id;
    draft = { title: event.title, description: event.description, starts_at: toInput(event.starts_at), ends_at: toInput(event.ends_at), status: event.status, instance_id: event.instance_id || '', server_id: event.server_id ? String(event.server_id) : '', objectives: JSON.stringify(event.objectives || [], null, 2), rewards: JSON.stringify(event.rewards || [], null, 2) };
  }
  function toInput(value: string) { const date = new Date(value); return new Date(date.getTime() - date.getTimezoneOffset() * 60000).toISOString().slice(0, 16); }
  function payload() { return { ...draft, starts_at: new Date(draft.starts_at).toISOString(), ends_at: new Date(draft.ends_at).toISOString(), instance_id: draft.instance_id || null, server_id: draft.server_id ? Number(draft.server_id) : null, objectives: JSON.parse(draft.objectives || '[]'), rewards: JSON.parse(draft.rewards || '[]') }; }
  async function save() {
    try {
      if (editing) await put(`/api/admin/events/${editing}`, payload());
      else await post('/api/admin/events', payload());
      cancel(); toast(editing ? 'Event updated' : 'Event created'); await load();
    } catch (e) { toastError(e); }
  }
  function cancel() { editing = null; draft = { title: '', description: '', starts_at: '', ends_at: '', status: 'draft', instance_id: '', server_id: '', objectives: '[]', rewards: '[]' }; }
  async function remove(event: EventRow) { if (!confirm(`Delete ${event.title}? This also removes its participant records.`)) return; try { await del(`/api/admin/events/${event.id}`); toast('Event deleted'); await load(); } catch (e) { toastError(e); } }
  $effect(() => { void load(); });
</script>

<div class="page">
  <header><div><h1><CalendarDays size={22} /> Community events</h1><p>Create scheduled server activities with shared objectives and rewards.</p></div><button class="ghost icon" onclick={load} title="Refresh"><RefreshCw size={16} /></button></header>
  <section class="card form">
    <h2>{#if editing}<Pencil size={16} /> Edit event{:else}<Plus size={16} /> New event{/if}</h2>
    <div class="grid fields">
      <label>Title<input bind:value={draft.title} placeholder="Nether expedition" /></label>
      <label>Status<select bind:value={draft.status}><option value="draft">Draft</option><option value="published">Published</option><option value="active">Active</option></select></label>
      <label>Starts<input type="datetime-local" bind:value={draft.starts_at} /></label>
      <label>Ends<input type="datetime-local" bind:value={draft.ends_at} /></label>
      <label>Launcher instance ID<input bind:value={draft.instance_id} placeholder="All instances" /></label>
      <label>Game server ID<input type="number" min="1" bind:value={draft.server_id} placeholder="All servers" /></label>
    </div>
    <label>Description<textarea bind:value={draft.description} rows="2" placeholder="What should players accomplish?"></textarea></label>
    <label>Objectives JSON<textarea bind:value={draft.objectives} rows="2" placeholder="Example: stat objective for mob kills"></textarea></label>
    <label>Reward actions JSON<textarea bind:value={draft.rewards} rows="2" placeholder="Reward JSON array"></textarea></label>
    <div class="actions">
      <button class="primary" disabled={!draft.title || !draft.starts_at || !draft.ends_at} onclick={save}>
        {#if editing}<Pencil size={16} /> Save changes{:else}<Plus size={16} /> Create event{/if}
      </button>
      {#if editing}<button class="ghost" onclick={cancel}>Cancel</button>{/if}
    </div>
  </section>
  <section class="card"><div class="section-title"><h2>Scheduled events</h2><span class="hint">{events.length}</span></div>
    {#if !events.length}<p class="muted">No events yet.</p>{:else}<div class="list">{#each events as event}<article><div><strong>{event.title}</strong><p>{event.description || 'No description'}</p><small>{event.objectives.length} objectives · {event.instance_id || 'All instances'} · {event.server_id ? `Server ${event.server_id}` : 'All servers'}</small></div><span class="badge">{event.status}</span><span class="muted small">{new Date(event.starts_at).toLocaleString()}</span><div class="row-actions"><button class="ghost icon" onclick={() => edit(event)} title="Edit event"><Pencil size={15} /></button><button class="ghost icon danger" onclick={() => remove(event)} title="Delete event"><Trash2 size={15} /></button></div></article>{/each}</div>{/if}
  </section>
</div>

<style>
  .page { padding: 32px; max-width: 1100px; margin: 0 auto; display: flex; flex-direction: column; gap: 20px; }
  header,.section-title { display:flex; align-items:center; justify-content:space-between; gap:16px; } h1 { display:flex; align-items:center; gap:9px; margin:0 0 6px; } header p { margin:0; color:var(--muted); }
  .form { display:flex; flex-direction:column; gap:14px; } h2 { display:flex; align-items:center; gap:7px; margin:0; font-size:1rem; } .fields { grid-template-columns:repeat(2,minmax(0,1fr)); gap:12px; } label { display:flex; flex-direction:column; gap:6px; font-size:.8rem; color:var(--muted); } textarea,input,select { width:100%; box-sizing:border-box; } .actions,.row-actions { display:flex; align-items:center; gap:8px; } .list { display:flex; flex-direction:column; } article { display:grid; grid-template-columns:minmax(0,1fr) auto auto auto; align-items:center; gap:14px; padding:14px 0; border-top:1px solid var(--line); } article p { margin:4px 0; color:var(--muted); font-size:.85rem; } article small { color:var(--muted); } .hint { color:var(--muted); } .danger { color:var(--red); } @media(max-width:700px){.page{padding:20px}.fields{grid-template-columns:1fr}article{grid-template-columns:1fr auto}.small{grid-column:1}}
</style>
