<script lang="ts">
  import { ArrowLeft, ArrowRight, X, FileArchive, Hammer, Sparkles, Check, TriangleAlert, Cuboid, Copy, Wand2, PackageCheck, Eye } from '@lucide/svelte';
  import { FRAMEWORKS, KINDS, frameworkMeta, kindMeta, type Analysis, type Analyzed, type KindId } from '../lib/content';
  import { post, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import { markdownToAmp } from '../lib/mcText';
  import { spans } from '../lib/mccolor';
  import ModelViewer from './ModelViewer.svelte';
  import McTextInput from './McTextInput.svelte';
  import ContentFields from './ContentFields.svelte';
  import Slide from './Slide.svelte';

  let { open = $bindable(false), onchange, onmanual, onopen }: {
    open: boolean; onchange?: () => void; onmanual?: (kind: KindId | 'auto') => void; onopen?: (id: string, store: 'item' | 'content') => void;
  } = $props();

  type Edit = { kind: KindId; title: string; name: string; extras: Record<string, any> };
  type Done = { created: number; entries: { key: string; id: string; kind?: KindId; title?: string; status: string; note?: string; store?: 'item' | 'content' }[]; files_added: number; files_replaced: number; pack_enabled: boolean; notes: string[] };

  let step = $state(1);
  let hint = $state<KindId | 'auto'>('auto');
  let fw = $state<string>('auto');
  let busy = $state(false);
  let error = $state('');
  let drag = $state(false);
  let zipName = $state('');
  let zipData = $state<string | null>(null);
  let analysis = $state<Analysis | null>(null);
  let picked = $state<Record<string, boolean>>({});
  let edits = $state<Record<string, Edit>>({});
  let tab = $state<'all' | KindId>('all');
  let current = $state<string | null>(null);
  let result = $state<Done | null>(null);
  let input: HTMLInputElement | undefined = $state();
  let views = $state<Record<string, any>>({});

  const STEPS = ['What', 'Where from', 'Files', 'Review', 'Done'];
  const itemKinds = KINDS.filter((k) => k.store === 'item');
  const worldKinds = KINDS.filter((k) => k.store === 'content');

  function reset() {
    step = 1; hint = 'auto'; fw = 'auto'; busy = false; error = ''; zipName = ''; zipData = null; analysis = null; picked = {}; edits = {}; tab = 'all'; current = null; result = null; views = {};
  }
  function close() { open = false; reset(); }
  $effect(() => { if (!open) reset(); });

  const entries = $derived(analysis?.entries ?? []);
  const counts = $derived(Object.fromEntries(KINDS.map((k) => [k.id, entries.filter((e) => (edits[e.key]?.kind ?? e.kind) === k.id).length])) as Record<KindId, number>);
  const shown = $derived(entries.filter((e) => tab === 'all' || (edits[e.key]?.kind ?? e.kind) === tab));
  const chosen = $derived(entries.filter((e) => picked[e.key] && !e.exists));
  const cur = $derived(entries.find((e) => e.key === current) ?? null);
  const kindOf = (e: Analyzed) => edits[e.key]?.kind ?? e.kind;

  async function read(file?: File) {
    if (!file) return;
    error = '';
    zipName = file.name;
    busy = true;
    try {
      if (file.size > 36 * 1024 * 1024) throw new Error('That zip is over 36 MB. Zip only the folders the guide above names, or split it in parts.');
      zipData = await new Promise<string>((resolve, reject) => { const r = new FileReader(); r.onload = () => resolve(String(r.result).split(',')[1]); r.onerror = () => reject(new Error('Could not read that file')); r.readAsDataURL(file); });
      const a = await post<Analysis>('/api/admin/content/analyze', { data: zipData });
      analysis = a;
      edits = Object.fromEntries(a.entries.map((e) => [e.key, { kind: e.kind, title: e.title, name: e.name ?? '', extras: structuredClone(e.extras) }]));
      const matching = a.entries.filter((e) => hint === 'auto' || e.kind === hint || (kindMeta(hint).store === 'item' && false));
      const useAll = hint === 'auto' || matching.length === 0;
      picked = Object.fromEntries(a.entries.map((e) => [e.key, !e.exists && (useAll || e.kind === hint)]));
      current = a.entries[0]?.key ?? null;
      step = 4;
    } catch (e) {
      zipData = null;
      error = e instanceof Error ? e.message : String(e);
    } finally { busy = false; }
  }

  async function loadView(e: Analyzed) {
    if (!zipData || views[e.key] || e.view) return;
    try {
      const a = await post<Analysis>('/api/admin/content/analyze', { data: zipData, view_for: [e.key] });
      const found = a.entries.find((x) => x.key === e.key);
      if (found?.view) views[e.key] = found.view;
      else toast('This one has no 3D model — it is a flat sprite', 'info');
    } catch (err) { toastError(err); }
  }
  const viewOf = (e: Analyzed) => e.view ?? views[e.key] ?? null;

  function selectAll(on: boolean) { for (const e of shown) if (!e.exists) picked[e.key] = on; }

  async function run() {
    if (!zipData || !analysis) return;
    busy = true;
    try {
      const payload = chosen.map((e) => {
        const ed = edits[e.key];
        return { key: e.key, kind: ed.kind, title: ed.title, name: ed.name, extras: ed.extras };
      });
      result = await post<Done>('/api/admin/content/import', { data: zipData, entries: payload });
      step = 5;
      onchange?.();
    } catch (e) { toastError(e); } finally { busy = false; }
  }

  async function enablePack() {
    try { await put('/api/admin/resource-pack', { enabled: true, required: true, pack_format: 84 }); if (result) result.pack_enabled = true; toast('Resource pack is on and required'); } catch (e) { toastError(e); }
  }
  async function copy(text: string) { try { await navigator.clipboard.writeText(text); toast('Copied'); } catch { toast(text, 'info'); } }

  const recommended = (f: { brings: KindId[] }) => hint === 'auto' || f.brings.includes(hint as KindId);
  const fwInfo = $derived(frameworkMeta(fw));
  const colorText = (amp: string, base = '#fff') => spans(amp, base);
  const warnCount = $derived(chosen.reduce((n, e) => n + e.warnings.length + e.missing.length, 0));
  const createdKinds = $derived(result ? result.entries.filter((r) => r.status === 'created') : []);

  function next() {
    if (step === 1) step = 2;
    else if (step === 2) { if (fw === 'manual') { const k = hint; close(); onmanual?.(k); } else step = 3; }
  }
</script>

<svelte:window onkeydown={(e) => open && e.key === 'Escape' && !busy && close()} />

{#if open}
  <div class="wz" role="dialog" aria-modal="true" aria-label="Add content">
    <div class="shell">
      <header>
        <div class="title"><Wand2 size={18} /> <b>Add content</b></div>
        <ol class="steps" aria-label="Progress">
          {#each STEPS as s, i}
            <li class:on={step === i + 1} class:done={step > i + 1}><span class="dot">{#if step > i + 1}<Check size={11} />{:else}{i + 1}{/if}</span><span class="nm">{s}</span></li>
          {/each}
        </ol>
        <button class="x" aria-label="Close" onclick={close} disabled={busy}><X size={18} /></button>
      </header>

      <main>
        {#if step === 1}
          <h2>What are you adding?</h2>
          <p class="sub">Pick the closest match — you can change it for each thing later. Not sure? Let us work it out from your files.</p>
          <h4>Items players carry</h4>
          <div class="tiles">
            {#each itemKinds as k (k.id)}
              {@const Icon = k.icon}
              <button class="tile" class:on={hint === k.id} style:--c={k.color} onclick={() => (hint = k.id)} aria-pressed={hint === k.id}>
                <span class="ic"><Icon size={22} /></span><b>{k.label}</b><small>{k.blurb}</small>
              </button>
            {/each}
          </div>
          <h4>Things in the world</h4>
          <div class="tiles">
            {#each worldKinds as k (k.id)}
              {@const Icon = k.icon}
              <button class="tile" class:on={hint === k.id} style:--c={k.color} onclick={() => (hint = k.id)} aria-pressed={hint === k.id}>
                <span class="ic"><Icon size={22} /></span><b>{k.label}</b><small>{k.blurb}</small>
              </button>
            {/each}
            <button class="tile auto" class:on={hint === 'auto'} style:--c="#c792ea" onclick={() => (hint = 'auto')} aria-pressed={hint === 'auto'}>
              <span class="ic"><Sparkles size={22} /></span><b>Not sure — detect it</b><small>We'll sort everything in your files for you</small>
            </button>
          </div>

        {:else if step === 2}
          <h2>Where does it come from?</h2>
          <p class="sub">We read the files these plugins already use — nothing to convert by hand.</p>
          <div class="tiles fw">
            {#each FRAMEWORKS as f (f.id)}
              <button class="tile" class:on={fw === f.id} class:dim={!recommended(f)} style:--c={f.color} onclick={() => (fw = f.id)} aria-pressed={fw === f.id}>
                <span class="badge">{f.label.slice(0, 2).toUpperCase()}</span>
                <b>{f.label}</b><small>{f.blurb}</small>
                {#if hint !== 'auto' && f.brings.includes(hint as KindId)}<span class="rec">Brings {kindMeta(hint).plural.toLowerCase()}</span>{/if}
              </button>
            {/each}
            <button class="tile" class:on={fw === 'auto'} style:--c="#c792ea" onclick={() => (fw = 'auto')} aria-pressed={fw === 'auto'}>
              <span class="badge"><Sparkles size={16} /></span><b>A mix, or not sure</b><small>Zip everything — we detect each framework</small>
            </button>
            <button class="tile" class:on={fw === 'manual'} style:--c="#9aa0b4" onclick={() => (fw = 'manual')} aria-pressed={fw === 'manual'}>
              <span class="badge"><Hammer size={16} /></span><b>I'll build it by hand</b><small>Open the advanced editor instead</small>
            </button>
          </div>

        {:else if step === 3}
          <h2>Drop your files</h2>
          <p class="sub">{fwInfo ? fwInfo.zip : 'Zip the plugin folders that hold your items, models and mobs. Several plugins can go in one zip.'}</p>
          <button class="drop" class:drag disabled={busy} ondragover={(e) => { e.preventDefault(); drag = true; }} ondragleave={() => (drag = false)}
            ondrop={(e) => { e.preventDefault(); drag = false; read(e.dataTransfer?.files?.[0]); }} onclick={() => input?.click()}>
            {#if busy}<span class="spin"></span><b>Reading {zipName}…</b><span>Working out what's inside</span>
            {:else}<FileArchive size={38} /><b>Drop a .zip here, or click to choose</b><span>Up to 36 MB · nothing is saved until you press Import</span>{/if}
          </button>
          <input bind:this={input} type="file" accept=".zip" hidden onchange={(e) => read(e.currentTarget.files?.[0])} />
          {#if error}<div class="err"><TriangleAlert size={16} /> {error}</div>{/if}
          <div class="reads">
            <b>What we read</b>
            <ul>
              <li>Item, block, furniture and mob definitions (YAML)</li>
              <li>Textures, models, atlases and sounds</li>
              <li>ModelEngine <code>.bbmodel</code> files, converted to a resting-pose model</li>
            </ul>
          </div>

        {:else if step === 4 && analysis}
          <div class="rv">
            <div class="rv-main">
              <div class="sum">
                {#each analysis.detected as d}<span class="fwchip" style:--c={frameworkMeta(d.framework)?.color ?? '#9aa0b4'}>{d.label} · {d.entries}</span>{/each}
                <span class="grow"></span>
                <span class="muted">{chosen.length} of {entries.length} selected{warnCount ? ` · ${warnCount} heads-up` : ''}</span>
              </div>
              {#each analysis.notes as n}<div class="note"><Check size={13} /> {n}</div>{/each}
              <div class="tabs seg" role="tablist">
                <button role="tab" aria-selected={tab === 'all'} class:on={tab === 'all'} onclick={() => (tab = 'all')}>All {entries.length}</button>
                {#each KINDS.filter((k) => counts[k.id] > 0) as k}<button role="tab" aria-selected={tab === k.id} class:on={tab === k.id} onclick={() => (tab = k.id)}>{k.plural} {counts[k.id]}</button>{/each}
              </div>
              <div class="bar"><button class="lnk" onclick={() => selectAll(true)}>Select all</button><button class="lnk" onclick={() => selectAll(false)}>None</button></div>
              <div class="cards">
                {#each shown as e (e.key)}
                  {@const m = kindMeta(kindOf(e))}
                  {@const Icon = m.icon}
                  <div class="card" class:sel={current === e.key} class:off={!picked[e.key] || e.exists} style:--c={m.color} role="button" tabindex="0"
                    onclick={() => (current = e.key)} onkeydown={(ev) => (ev.key === 'Enter' || ev.key === ' ') && (current = e.key)}>
                    <span class="chk"><input type="checkbox" bind:checked={picked[e.key]} disabled={e.exists} aria-label="Import {e.title}" onclick={(ev) => ev.stopPropagation()} /></span>
                    <span class="slot">{#if e.preview}<img src={e.preview} alt="" />{:else}<Cuboid size={20} />{/if}</span>
                    <div class="meta">
                      <b class="nm">{edits[e.key]?.title || e.title}</b>
                      <span class="kind"><Icon size={12} /> {m.label}</span>
                      {#if e.exists}<em>Already in your library</em>{:else if e.warnings.length || e.missing.length}<em class="w"><TriangleAlert size={11} /> {e.missing.length ? 'missing textures' : `${e.warnings.length} heads-up`}</em>{/if}
                    </div>
                    {#if e.view || e.model}<button class="eye" title="View in 3D" aria-label="View {e.title} in 3D" onclick={(ev) => { ev.stopPropagation(); current = e.key; loadView(e); }}><Eye size={14} /></button>{/if}
                  </div>
                {/each}
              </div>
            </div>

            <aside class="insp">
              {#if cur}
                {@const ed = edits[cur.key]}
                {@const v = viewOf(cur)}
                {#if v}
                  <ModelViewer view={v} height={250} scale={1} />
                {:else}
                  <div class="flat">{#if cur.preview}<img src={cur.preview} alt="" />{:else}<Cuboid size={40} />{/if}
                    {#if cur.model}<button class="lnk" onclick={() => loadView(cur)}><Eye size={13} /> Load 3D view</button>{/if}</div>
                {/if}
                <label class="f">Title<input bind:value={ed.title} maxlength="60" /></label>
                <div class="f"><span class="l">What is it?</span>
                  <div class="kinds" role="radiogroup" aria-label="Kind">
                    {#each KINDS as k}{@const I = k.icon}<button role="radio" aria-checked={ed.kind === k.id} class="kb" class:on={ed.kind === k.id} style:--c={k.color} title={k.label} onclick={() => (ed.kind = k.id)}><I size={15} /><span>{k.label}</span></button>{/each}
                  </div>
                </div>
                <McTextInput label="In-game name" placeholder={cur.title} bind:value={() => ed.name, (x) => (ed.name = x)} />
                {#if ed.kind === 'food'}
                  <Slide label="Hunger restored" min={0} max={20} color="#f0a35e" bind:value={() => ed.extras.nutrition ?? 4, (n) => (ed.extras = { ...ed.extras, nutrition: n })} />
                  <Slide label="Saturation" min={0} max={20} step={0.5} color="#e6c35a" bind:value={() => ed.extras.saturation ?? 2, (n) => (ed.extras = { ...ed.extras, saturation: n })} />
                {:else}
                  <ContentFields kind={ed.kind} title={ed.title} bind:v={ed.extras} />
                {/if}
                {#if cur.warnings.length || cur.missing.length}
                  <div class="warn">
                    {#each cur.missing as m}<div><TriangleAlert size={12} /> Needs texture/model <code>{m}</code> which isn't in the zip</div>{/each}
                    {#each cur.warnings as w}<div><TriangleAlert size={12} /> {w}</div>{/each}
                  </div>
                {/if}
              {:else}
                <p class="muted">Pick something on the left to see it in 3D and tweak it.</p>
              {/if}
            </aside>
          </div>

        {:else if step === 5 && result}
          <div class="done">
            <div class="big"><PackageCheck size={34} /></div>
            <h2>{result.created ? `Imported ${result.created} thing${result.created > 1 ? 's' : ''}` : 'Nothing new to import'}</h2>
            <p class="sub">{result.files_added + result.files_replaced} files went into your server assets.</p>
            {#if !result.pack_enabled}
              <div class="next"><b>Turn on the resource pack</b><span>Players need it to see these models.</span><button class="primary" onclick={enablePack}>Turn on &amp; require</button></div>
            {:else}
              <div class="next ok"><Check size={16} /> The resource pack is on — players get the new models next time they join.</div>
            {/if}
            <div class="cards slim">
              {#each createdKinds as r}
                {@const m = kindMeta(r.kind ?? 'item')}
                <div class="card ro" style:--c={m.color}>
                  <div class="meta"><b class="nm">{r.title}</b><span class="kind">{m.label}</span><code>/customitem give &lt;player&gt; {r.id}</code></div>
                  <button class="lnk" onclick={() => copy(`/customitem give @p ${r.id}`)}><Copy size={13} /> Copy</button>
                  <button class="lnk" onclick={() => { close(); onopen?.(r.id, r.store ?? 'item'); }}>Open</button>
                </div>
              {/each}
              {#each result.entries.filter((r) => r.status !== 'created') as r}<div class="card ro skip"><div class="meta"><b class="nm">{r.id}</b><em>{r.note ?? r.status}</em></div></div>{/each}
            </div>
          </div>
        {/if}
      </main>

      <footer>
        {#if step > 1 && step < 5}<button class="ghost" onclick={() => (step = step === 4 ? 3 : step - 1)} disabled={busy}><ArrowLeft size={15} /> Back</button>{:else}<span></span>{/if}
        <span class="grow"></span>
        {#if step <= 2}
          <button class="primary" onclick={next}>{step === 2 && fw === 'manual' ? 'Open the editor' : 'Continue'} <ArrowRight size={15} /></button>
        {:else if step === 4}
          <button class="primary" onclick={run} disabled={busy || !chosen.length}>{busy ? 'Importing…' : `Import ${chosen.length} thing${chosen.length === 1 ? '' : 's'}`}</button>
        {:else if step === 5}
          <button class="primary" onclick={close}>Finish</button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .wz { position: fixed; inset: 0; z-index: 80; background: rgba(6, 7, 12, 0.72); backdrop-filter: blur(8px); display: grid; place-items: center; padding: 18px; animation: fade 0.18s ease; }
  @keyframes fade { from { opacity: 0; } }
  .shell { width: min(1240px, 100%); height: min(860px, 100%); display: grid; grid-template-rows: auto 1fr auto; border-radius: 20px; background: var(--bg-2); border: 1px solid var(--line-strong); box-shadow: 0 30px 80px -20px #000c; overflow: hidden; animation: rise 0.22s cubic-bezier(0.2, 0.9, 0.3, 1.1); }
  @keyframes rise { from { transform: translateY(14px) scale(0.985); opacity: 0; } }
  header { display: flex; align-items: center; gap: 18px; padding: 14px 18px; border-bottom: 1px solid var(--line); background: var(--surface); }
  .title { display: flex; align-items: center; gap: 8px; }
  .steps { display: flex; gap: 6px; list-style: none; margin: 0 auto; padding: 0; }
  .steps li { display: flex; align-items: center; gap: 7px; padding: 5px 12px 5px 6px; border-radius: 999px; color: var(--muted); font-size: 0.8rem; }
  .steps .dot { display: grid; place-items: center; width: 20px; height: 20px; border-radius: 50%; background: var(--surface-3); font-size: 0.7rem; font-weight: 700; }
  .steps li.on { background: var(--accent-soft); color: var(--text); }
  .steps li.on .dot { background: var(--accent); color: #fff; }
  .steps li.done .dot { background: var(--good); color: #fff; }
  .x { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 9px; border: 0; background: transparent; color: var(--muted); cursor: pointer; }
  .x:hover { background: var(--surface-3); color: var(--text); }
  main { overflow: auto; padding: 26px 30px; min-height: 0; }
  footer { display: flex; align-items: center; gap: 10px; padding: 14px 20px; border-top: 1px solid var(--line); background: var(--surface); }
  .grow { flex: 1; }
  h2 { font-size: 1.5rem; margin-bottom: 4px; }
  h4 { margin: 20px 0 10px; color: var(--muted); font-size: 0.74rem; text-transform: uppercase; letter-spacing: 0.08em; font-weight: 600; }
  .sub { color: var(--text-2); max-width: 46rem; }
  .muted { color: var(--muted); font-size: 0.82rem; }
  .primary { display: inline-flex; align-items: center; gap: 8px; padding: 10px 20px; border-radius: 11px; border: 0; background: var(--accent); color: #fff; font-weight: 600; cursor: pointer; box-shadow: 0 6px 20px -6px var(--accent); transition: transform 0.1s, filter 0.1s; }
  .primary:hover:not(:disabled) { filter: brightness(1.1); transform: translateY(-1px); }
  .primary:disabled { opacity: 0.5; cursor: not-allowed; }
  .ghost { display: inline-flex; align-items: center; gap: 7px; padding: 9px 16px; border-radius: 11px; border: 1px solid var(--line-strong); background: transparent; color: var(--text-2); cursor: pointer; }
  .ghost:hover { color: var(--text); background: var(--surface-2); }

  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(215px, 1fr)); gap: 12px; margin-top: 8px; }
  .tile { --c: var(--accent); position: relative; display: grid; gap: 4px; justify-items: start; text-align: left; padding: 16px 16px 15px; border-radius: 16px; border: 1px solid var(--line); background: var(--surface); color: inherit; cursor: pointer; transition: transform 0.14s, border-color 0.14s, background 0.14s, box-shadow 0.14s; }
  .tile:hover { transform: translateY(-3px); border-color: color-mix(in srgb, var(--c) 70%, transparent); box-shadow: 0 14px 30px -16px var(--c); }
  .tile.on { border-color: var(--c); background: linear-gradient(150deg, color-mix(in srgb, var(--c) 20%, var(--surface)), var(--surface)); box-shadow: 0 0 0 1px var(--c), 0 14px 30px -16px var(--c); }
  .tile.dim { opacity: 0.55; }
  .tile small { color: var(--muted); font-size: 0.78rem; line-height: 1.35; }
  .ic, .badge { display: grid; place-items: center; width: 42px; height: 42px; border-radius: 12px; background: color-mix(in srgb, var(--c) 22%, transparent); color: var(--c); margin-bottom: 6px; }
  .badge { font-weight: 800; font-size: 0.82rem; letter-spacing: 0.04em; }
  .rec { margin-top: 4px; font-size: 0.68rem; padding: 2px 8px; border-radius: 999px; background: color-mix(in srgb, var(--c) 22%, transparent); color: var(--c); font-weight: 600; }

  .drop { display: grid; justify-items: center; gap: 8px; width: 100%; margin: 22px 0 14px; padding: 56px 20px; border-radius: 18px; border: 2px dashed color-mix(in srgb, var(--accent) 55%, transparent); background: color-mix(in srgb, var(--accent) 6%, transparent); color: var(--text-2); cursor: pointer; transition: background 0.15s, transform 0.15s; }
  .drop:hover:not(:disabled), .drop.drag { background: color-mix(in srgb, var(--accent) 16%, transparent); transform: scale(1.005); }
  .drop b { color: var(--text); font-size: 1.05rem; }
  .drop span { font-size: 0.84rem; color: var(--muted); }
  .spin { width: 34px; height: 34px; border-radius: 50%; border: 3px solid var(--surface-3); border-top-color: var(--accent); animation: sp 0.8s linear infinite; }
  @keyframes sp { to { transform: rotate(360deg); } }
  .err { display: flex; gap: 8px; align-items: center; padding: 10px 14px; border-radius: 10px; background: color-mix(in srgb, var(--bad) 16%, transparent); color: #ffb4b6; margin-bottom: 12px; }
  .reads { color: var(--text-2); font-size: 0.85rem; }
  .reads ul { margin: 6px 0 0; padding-left: 18px; color: var(--muted); }

  .rv { display: grid; grid-template-columns: minmax(0, 1fr) 380px; gap: 22px; align-items: start; }
  .sum { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 8px; }
  .fwchip { padding: 4px 12px; border-radius: 999px; font-size: 0.78rem; font-weight: 600; background: color-mix(in srgb, var(--c) 20%, transparent); color: var(--c); }
  .note { display: flex; align-items: center; gap: 6px; color: var(--text-2); font-size: 0.8rem; margin-bottom: 4px; }
  .tabs { margin: 8px 0 6px; }
  .bar { display: flex; gap: 12px; margin: 4px 0 10px; }
  .lnk { display: inline-flex; align-items: center; gap: 5px; padding: 3px 8px; border: 0; border-radius: 7px; background: transparent; color: var(--accent-2); font-size: 0.8rem; cursor: pointer; }
  .lnk:hover { background: var(--accent-soft); }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 9px; }
  .cards.slim { grid-template-columns: 1fr; max-width: 640px; margin: 14px auto 0; text-align: left; }
  .card { --c: var(--accent); position: relative; display: flex; align-items: center; gap: 10px; padding: 10px; border-radius: 14px; border: 1px solid var(--line); background: var(--surface); cursor: pointer; transition: transform 0.12s, border-color 0.12s, opacity 0.15s; }
  .card:hover { transform: translateY(-2px); border-color: color-mix(in srgb, var(--c) 60%, transparent); }
  .card.sel { border-color: var(--c); box-shadow: 0 0 0 1px var(--c); background: color-mix(in srgb, var(--c) 8%, var(--surface)); }
  .card.off { opacity: 0.5; }
  .card.ro { cursor: default; }
  .card.ro:hover { transform: none; }
  .card.skip { opacity: 0.6; }
  .chk { display: grid; place-items: center; padding: 2px; }
  .slot { display: grid; place-items: center; width: 50px; height: 50px; flex-shrink: 0; border-radius: 7px; background: #8b8b8b; box-shadow: inset 2px 2px 0 #373737, inset -2px -2px 0 #fff8; color: #3a3a3a; }
  .slot img { width: 76%; height: 76%; object-fit: contain; image-rendering: pixelated; }
  .meta { display: grid; gap: 2px; min-width: 0; flex: 1; }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kind { display: inline-flex; align-items: center; gap: 4px; font-size: 0.72rem; color: var(--c); }
  .meta em { font-size: 0.7rem; color: var(--muted); font-style: normal; }
  .meta em.w { color: var(--warn); display: inline-flex; align-items: center; gap: 4px; }
  .meta code { color: var(--muted); font-size: 0.72rem; }
  .eye { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 8px; border: 0; background: var(--surface-3); color: var(--text-2); cursor: pointer; }
  .eye:hover { background: var(--accent); color: #fff; }

  .insp { position: sticky; top: 0; display: grid; gap: 14px; padding: 14px; border-radius: 16px; background: var(--surface); border: 1px solid var(--line); max-height: calc(100vh - 260px); overflow: auto; }
  .flat { display: grid; place-items: center; gap: 8px; height: 150px; border-radius: 14px; background: radial-gradient(120% 90% at 50% 20%, #2a2d44 0%, #15161f 70%); color: var(--muted); }
  .flat img { width: 84px; height: 84px; image-rendering: pixelated; object-fit: contain; }
  .f { display: grid; gap: 5px; font-size: 0.8rem; color: var(--muted); }
  .kinds { display: grid; grid-template-columns: repeat(auto-fill, minmax(104px, 1fr)); gap: 5px; }
  .kb { --c: var(--accent); display: inline-flex; align-items: center; gap: 6px; padding: 6px 9px; border-radius: 9px; border: 1px solid var(--line); background: var(--surface-2); color: var(--text-2); font-size: 0.75rem; cursor: pointer; }
  .kb:hover { border-color: var(--c); }
  .kb.on { background: color-mix(in srgb, var(--c) 22%, transparent); border-color: var(--c); color: var(--text); font-weight: 600; }
  .warn { display: grid; gap: 5px; padding: 10px; border-radius: 10px; background: color-mix(in srgb, var(--warn) 12%, transparent); font-size: 0.78rem; color: #e8cf98; }
  .warn div { display: flex; gap: 6px; align-items: flex-start; }

  .done { display: grid; justify-items: center; text-align: center; gap: 8px; padding-top: 10px; }
  .big { display: grid; place-items: center; width: 74px; height: 74px; border-radius: 22px; background: color-mix(in srgb, var(--good) 22%, transparent); color: var(--good); }
  .next { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: center; padding: 12px 16px; border-radius: 12px; background: var(--accent-soft); margin-top: 8px; }
  .next.ok { background: color-mix(in srgb, var(--good) 14%, transparent); }
  @media (max-width: 1050px) { .rv { grid-template-columns: 1fr; } .insp { position: static; max-height: none; } .steps .nm { display: none; } }
</style>
