<script lang="ts">
  import { onMount } from 'svelte';
  import { Mail, Plus, Trash2, Send, Save, Users, Copy } from '@lucide/svelte';
  import Toggle from '../components/Toggle.svelte';
  import Modal from '../components/Modal.svelte';
  import { get, post, put, timeAgo } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type Template = { id: string; name: string; subject: string; body: string };
  type Data = { configured: boolean; templates: Template[]; placeholders: string[]; groups: { id: number; name: string }[]; log: any[] };

  let data = $state<Data | null>(null);
  let templates = $state<Template[]>([]);
  let current = $state(0);
  let important = $state(false);
  let preview = $state<{ subject: string; html: string } | null>(null);
  let testTo = $state('');
  let kind = $state<'all' | 'group' | 'players'>('all');
  let groupId = $state(0);
  let names = $state('');
  let count = $state<{ count: number; sample: string[] } | null>(null);
  let busy = $state('');
  let confirmOpen = $state(false);
  let timer: ReturnType<typeof setTimeout>;
  let body = $state<HTMLTextAreaElement>();

  async function load() {
    try {
      const d = await get<Data>('/api/admin/email');
      data = d;
      templates = structuredClone(d.templates);
      if (!groupId && d.groups[0]) groupId = d.groups[0].id;
    } catch (e) { toastError(e); }
  }
  onMount(() => { load(); const i = setInterval(() => get<Data>('/api/admin/email').then((d) => data && (data.log = d.log)).catch(() => {}), 5000); return () => clearInterval(i); });

  const t = $derived(templates[current]);
  const audience = $derived({ kind, group_id: groupId, names: names.split(/[\n,]+/).map((n) => n.trim()).filter(Boolean) });

  $effect(() => {
    const subject = t?.subject, text = t?.body, imp = important;
    clearTimeout(timer);
    if (!t) return;
    timer = setTimeout(async () => {
      try { preview = await post('/api/admin/email/preview', { subject, body: text, important: imp }); } catch { preview = null; }
    }, 400);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    const a = audience, imp = important;
    let alive = true;
    post<{ count: number; sample: string[] }>('/api/admin/email/audience', { audience: a, important: imp }).then((r) => alive && (count = r)).catch(() => alive && (count = null));
    return () => { alive = false; };
  });

  function add() {
    const id = `mail-${Date.now().toString(36)}`;
    templates = [...templates, { id, name: 'New email', subject: 'A message from {brand}', body: 'Hi {player},\n\n' }];
    current = templates.length - 1;
  }
  function duplicate() { templates = [...templates, { ...t, id: `${t.id}-copy-${Date.now().toString(36).slice(-3)}`, name: `${t.name} copy` }]; current = templates.length - 1; }
  function remove() { templates = templates.filter((_, i) => i !== current); current = Math.max(0, current - 1); }
  function insert(p: string) {
    const token = `{${p}}`;
    const s = body?.selectionStart ?? t.body.length, e = body?.selectionEnd ?? s;
    t.body = t.body.slice(0, s) + token + t.body.slice(e);
    queueMicrotask(() => { body?.focus(); body?.setSelectionRange(s + token.length, s + token.length); });
  }

  async function save() {
    busy = 'save';
    try { await put('/api/admin/email/templates', { templates }); toast('Templates saved'); } catch (e) { toastError(e); } finally { busy = ''; }
  }
  async function sendTest() {
    busy = 'test';
    try { await post('/api/admin/email/send', { subject: t.subject, body: t.body, important, test_to: testTo }); toast(`Test sent to ${testTo}`); } catch (e) { toastError(e); } finally { busy = ''; }
  }
  async function sendAll() {
    busy = 'send'; confirmOpen = false;
    try { const r = await post<{ queued: number }>('/api/admin/email/send', { subject: t.subject, body: t.body, important, audience }); toast(`Sending to ${r.queued} players…`); await load(); } catch (e) { toastError(e); } finally { busy = ''; }
  }
</script>

<div class="page wide">
  <header>
    <div>
      <h1>Emails</h1>
      <p>Write reusable emails and send them to everyone, a group, or chosen players. Messages use your launcher colours.{#if data && !data.configured} <strong class="warn">Set up email under Settings first (sender and Resend key).</strong>{/if}</p>
    </div>
  </header>

  {#if data && t}
    <div class="layout">
      <aside class="card list">
        {#each templates as tpl, i}
          <button class="item" class:on={i === current} onclick={() => (current = i)}><Mail size={15} /><span><strong>{tpl.name || 'Untitled'}</strong><small>{tpl.subject}</small></span></button>
        {/each}
        <button class="ghost add" onclick={add}><Plus size={15} /> New email</button>
      </aside>

      <section class="card form">
        <div class="grid">
          <label class="field">Name <small>only you see this</small><input bind:value={t.name} maxlength="80" /></label>
          <label class="field">Subject<input bind:value={t.subject} maxlength="200" /></label>
        </div>
        <div class="ph"><span class="lbl">Placeholders <small>click to insert</small></span>
          <div>{#each data.placeholders as p}<button type="button" class="chip" onclick={() => insert(p)}>{`{${p}}`}</button>{/each}</div></div>
        <label class="field">Message
          <textarea rows="12" bind:value={t.body} bind:this={body} maxlength="20000"></textarea>
          <small># Heading · **bold** · - list item · [link text](https://…) · [button: Label](https://…)</small>
        </label>
        <Toggle bind:checked={important} label="Account notice" help="Also reaches players who unsubscribed, and leaves out the unsubscribe link. Use only for things about their account." />
        <div class="actions">
          <button class="primary" onclick={save} disabled={!!busy}><Save size={15} /> Save templates</button>
          <button class="ghost" onclick={duplicate}><Copy size={15} /> Duplicate</button>
          <button class="ghost danger" onclick={remove} disabled={templates.length < 2}><Trash2 size={15} /> Delete</button>
        </div>

        <div class="send">
          <h3>Send</h3>
          <div class="testrow">
            <input bind:value={testTo} type="email" placeholder="you@example.com" />
            <button onclick={sendTest} disabled={!testTo || !!busy || !data.configured}><Send size={15} /> Send a test</button>
          </div>
          <div class="aud">
            <label class="field">Send to
              <select bind:value={kind}><option value="all">Everyone with an email</option><option value="group">A group</option><option value="players">Chosen players</option></select>
            </label>
            {#if kind === 'group'}<label class="field">Group<select bind:value={groupId}>{#each data.groups as g}<option value={g.id}>{g.name}</option>{/each}</select></label>{/if}
            {#if kind === 'players'}<label class="field wide">Player names <small>separated by commas or new lines</small><textarea rows="2" bind:value={names}></textarea></label>{/if}
          </div>
          <p class="muted small"><Users size={13} /> {count ? `${count.count} recipient${count.count === 1 ? '' : 's'}${count.sample.length ? ` (${count.sample.join(', ')}${count.count > 5 ? '…' : ''})` : ''}` : '…'}</p>
          <button class="primary" onclick={() => (confirmOpen = true)} disabled={!!busy || !count?.count || !data.configured}><Send size={15} /> Send to {count?.count ?? 0} players</button>
        </div>
      </section>

      <section class="preview">
        <h3>Preview</h3>
        {#if preview}
          <div class="subject"><span>Subject</span>{preview.subject}</div>
          <iframe title="Email preview" sandbox="" srcdoc={preview.html}></iframe>
        {:else}<p class="muted">Loading…</p>{/if}
      </section>
    </div>

    <section class="card history">
      <h3>Sent emails</h3>
      {#if data.log.length}
        <table class="table">
          <thead><tr><th>When</th><th>Subject</th><th>To</th><th>Sent</th><th>Failed</th></tr></thead>
          <tbody>
            {#each data.log as l}
              <tr><td class="muted small">{timeAgo(l.started_at)}</td><td>{l.subject}</td><td>{l.audience}</td><td>{l.sent}/{l.total}{#if !l.finished_at} …{/if}</td><td class:bad={l.failed}>{l.failed}{#if l.last_error} <small class="muted" title={l.last_error}>{l.last_error}</small>{/if}</td></tr>
            {/each}
          </tbody>
        </table>
      {:else}<p class="muted">Nothing sent yet.</p>{/if}
    </section>
  {:else}<p class="muted">Loading…</p>{/if}
</div>

<Modal bind:open={confirmOpen} title="Send this email?">
  <p>This sends <strong>{t?.subject}</strong> to <strong>{count?.count ?? 0}</strong> players. It can't be undone.</p>
  <div class="actions end"><button class="ghost" onclick={() => (confirmOpen = false)}>Cancel</button><button class="primary" onclick={sendAll}><Send size={15} /> Send now</button></div>
</Modal>

<style>
  .page { display: flex; flex-direction: column; gap: 18px; }
  header h1 { margin: 0; }
  header p { margin: 4px 0 0; color: var(--muted); }
  .warn { color: #fbbf24; margin-left: 6px; }
  .layout { display: grid; grid-template-columns: 240px minmax(0, 1.3fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  .list { display: flex; flex-direction: column; gap: 6px; padding: 10px; position: sticky; top: 16px; }
  .item { display: flex; align-items: flex-start; gap: 9px; text-align: left; padding: 10px; border-radius: 10px; border: 1px solid transparent; background: transparent; }
  .item.on { background: color-mix(in srgb, var(--accent) 14%, transparent); border-color: color-mix(in srgb, var(--accent) 40%, transparent); }
  .item small { display: block; color: var(--muted); font-weight: 400; font-size: 0.74rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 150px; }
  .add { justify-content: flex-start; }
  .form { display: flex; flex-direction: column; gap: 14px; }
  .grid, .aud { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; }
  .field.wide { grid-column: 1 / -1; }
  .field small, .ph small { color: var(--muted); font-weight: 400; }
  .ph { display: flex; flex-direction: column; gap: 6px; }
  .ph > div { display: flex; flex-wrap: wrap; gap: 6px; }
  .lbl { font-size: 0.82rem; font-weight: 600; }
  .chip { padding: 3px 9px; border-radius: 999px; font: 500 0.76rem ui-monospace, monospace; background: color-mix(in srgb, var(--accent) 14%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .actions.end { justify-content: flex-end; margin-top: 14px; }
  .actions button, .send button, .testrow button { display: inline-flex; align-items: center; gap: 6px; }
  .send { display: flex; flex-direction: column; gap: 10px; padding-top: 14px; border-top: 1px solid var(--line); }
  .send h3, .preview h3, .history h3 { margin: 0; font-size: 0.95rem; }
  .testrow { display: flex; gap: 8px; }
  .testrow input { flex: 1; }
  .preview { position: sticky; top: 16px; display: flex; flex-direction: column; gap: 8px; }
  .subject { padding: 8px 12px; border-radius: 10px; background: var(--bg-2); border: 1px solid var(--line); font-weight: 600; }
  .subject span { display: block; font-size: 0.7rem; color: var(--muted); text-transform: uppercase; letter-spacing: .06em; font-weight: 500; }
  iframe { width: 100%; height: 520px; border: 1px solid var(--line); border-radius: 12px; background: #0f1220; }
  .history { display: flex; flex-direction: column; gap: 10px; }
  .bad { color: #fb7185; }
  @media (max-width: 1300px) { .layout { grid-template-columns: 220px minmax(0, 1fr); } .preview { grid-column: 1 / -1; position: static; } }
  @media (max-width: 800px) { .layout { grid-template-columns: 1fr; } .list { position: static; } .grid, .aud { grid-template-columns: 1fr; } }
</style>
