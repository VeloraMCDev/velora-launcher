<script lang="ts">
  import { ArrowDown, ArrowUp, Copy, Download, ExternalLink, Eye, EyeOff, GripVertical, LoaderCircle, Plus, RotateCcw, Save, Trash2, Upload } from '@lucide/svelte';
  import { api, get, put, formatBytes, uploadMedia } from '../lib/api';
  import DownloadsManager from '../components/DownloadsManager.svelte';
  import ImageInput from '../components/ImageInput.svelte';
  import { toast, toastError } from '../lib/toast.svelte';
  import { BLOCK_GROUPS, BLOCK_SCHEMAS, createLandingBlock, upgradeLandingBlock } from '../lib/landingSchema';
  import type { BlockType, LandingBlock, LandingConfig } from '../lib/types';

  let config = $state<LandingConfig | null>(null);
  let saved = $state('');
  let tab = $state<'sections' | 'design' | 'downloads' | 'faq' | 'advanced'>('sections');
  let selectedId = $state<string | null>(null);
  let galleryOpen = $state(false);
  let saving = $state(false);
    let uploadingMedia = $state(false);
  let dragFrom = $state<number | null>(null);
  let frame = $state<HTMLIFrameElement | null>(null);
  const previewUrl = `${location.pathname}?landingPreview=1#/landing`;

  $effect(() => {
    get<LandingConfig>('/api/admin/landing').then((cfg) => {
      cfg.blocks.forEach(upgradeLandingBlock);
      config = cfg;
      saved = JSON.stringify(cfg);
      selectedId = cfg.blocks[0]?.id ?? null;
    }).catch(toastError);
  });

  const selected = $derived(config?.blocks.find((b) => b.id === selectedId) ?? null);
  const schema = $derived(selected ? BLOCK_SCHEMAS[selected.type] : null);
  const dirty = $derived(!!config && JSON.stringify(config) !== saved);

  $effect(() => {
    if (!config || !frame) return;
    JSON.stringify(config);
    frame.contentWindow?.postMessage({ type: 'scopenet-landing-preview', config: $state.snapshot(config) }, location.origin);
  });

  async function save() {
    if (!config) return;
    saving = true;
    try {
      const updated = await put<LandingConfig>('/api/admin/landing', $state.snapshot(config));
      updated.blocks.forEach(upgradeLandingBlock);
      config = updated;
      saved = JSON.stringify(updated);
      toast('Landing page published');
    } catch (e) { toastError(e); }
    finally { saving = false; }
  }

  function add(type: BlockType) {
    if (!config) return;
    const block = createLandingBlock(type);
    config.blocks.push(block);
    selectedId = block.id;
    galleryOpen = false;
  }

  function duplicate(index: number) {
    if (!config) return;
    const copy = structuredClone($state.snapshot(config.blocks[index]));
    copy.id = createLandingBlock(copy.type).id;
    copy.title = `${copy.title || BLOCK_SCHEMAS[copy.type].label} copy`;
    config.blocks.splice(index + 1, 0, copy);
    selectedId = copy.id;
  }

  function remove(index: number) {
    if (!config) return;
    config.blocks.splice(index, 1);
    if (!config.blocks.some((b) => b.id === selectedId)) selectedId = config.blocks[Math.min(index, config.blocks.length - 1)]?.id ?? null;
  }

  function move(from: number, to: number) {
    if (!config || to < 0 || to >= config.blocks.length || from === to) return;
    const [item] = config.blocks.splice(from, 1);
    config.blocks.splice(to, 0, item);
  }

  function drop(index: number) {
    if (dragFrom === null) return;
    move(dragFrom, index);
    dragFrom = null;
  }

  function setOption(block: LandingBlock, key: string, value: unknown) {
    block.options ??= {};
    block.options[key] = value;
  }

  async function uploadSectionMedia(file: File | undefined, apply: (url: string) => void) {
    if (!file) return;
    uploadingMedia = true;
    try { apply(await uploadMedia(file)); }
    catch (e) { toastError(e); }
    finally { uploadingMedia = false; }
  }
</script>

{#if !config}
  <div class="center-loading"><LoaderCircle class="spin" size={28} /></div>
{:else}
  <div class="builder">
    <header class="builder-head">
      <div><h1>Landing page studio</h1><p>Compose the public page from reusable sections. Your changes appear in the preview before publishing.</p></div>
      <div class="head-actions">
        <label class="enabled"><input type="checkbox" bind:checked={config.enabled} /> Public page enabled</label>
        <a class="btn secondary" href="#/landing" target="_blank"><ExternalLink size={15} /> Live page</a>
        <button class="btn secondary" disabled={!dirty} onclick={() => { config = JSON.parse(saved); selectedId = config?.blocks[0]?.id ?? null; }}><RotateCcw size={15} /> Discard</button>
        <button class="btn primary" disabled={!dirty || saving} onclick={save}>{#if saving}<LoaderCircle class="spin" size={15} />{:else}<Save size={15} />{/if} Publish</button>
      </div>
    </header>

    <div class="studio-tabs">
      {#each [['sections', 'Sections'], ['design', 'Design'], ['downloads', 'Downloads'], ['faq', 'FAQ'], ['advanced', 'Advanced']] as [id, label]}
        <button class:active={tab === id} onclick={() => (tab = id as typeof tab)}>{label}</button>
      {/each}
    </div>

    {#if tab === 'sections'}
      <div class="studio-grid">
        <aside class="section-list card">
          <div class="list-head"><strong>Sections <span class="muted">{config.blocks.length}</span></strong><button class="btn primary sm" onclick={() => (galleryOpen = !galleryOpen)}><Plus size={15} /> Add</button></div>
          {#if galleryOpen}
            <div class="gallery">
              {#each BLOCK_GROUPS as group}
                <strong class="group-label">{group}</strong>
                {#each Object.values(BLOCK_SCHEMAS).filter((s) => s.group === group) as item}
                  <button class="gallery-item" onclick={() => add(item.type)}><strong>{item.label}</strong><small>{item.description}</small></button>
                {/each}
              {/each}
            </div>
          {/if}
          <div class="list-items" role="list">
            {#each config.blocks as block, index (block.id)}
              <div class="list-item" role="listitem" class:selected={selectedId === block.id} class:muted-block={!block.enabled}
                draggable="true" ondragstart={() => (dragFrom = index)} ondragend={() => (dragFrom = null)} ondragover={(e) => e.preventDefault()} ondrop={(e) => { e.preventDefault(); drop(index); }}>
                <button class="select-block" onclick={() => (selectedId = block.id)}><GripVertical size={14} /><span><strong>{block.title || BLOCK_SCHEMAS[block.type]?.label || block.type}</strong><small>{BLOCK_SCHEMAS[block.type]?.label || block.type}</small></span></button>
                <button class="icon-action" aria-label={block.enabled ? 'Hide section' : 'Show section'} onclick={() => (block.enabled = !block.enabled)}>{#if block.enabled}<Eye size={14} />{:else}<EyeOff size={14} />{/if}</button>
              </div>
            {/each}
          </div>
          {#if !config.blocks.length}<p class="empty">Start with a Hero, then add any sections you need.</p>{/if}
        </aside>

        <div class="inspector card">
          {#if selected && schema}
            {@const index = config.blocks.findIndex((b) => b.id === selected.id)}
            <div class="inspector-head"><div><span class="eyebrow">{schema.group} / {schema.label}</span><h2>{selected.title || schema.label}</h2><p>{schema.description}</p></div></div>
            <div class="section-actions">
              <button class="btn secondary sm" disabled={index === 0} onclick={() => move(index, index - 1)}><ArrowUp size={14} /> Up</button>
              <button class="btn secondary sm" disabled={index === config.blocks.length - 1} onclick={() => move(index, index + 1)}><ArrowDown size={14} /> Down</button>
              <button class="btn secondary sm" onclick={() => duplicate(index)}><Copy size={14} /> Duplicate</button>
              <button class="btn secondary sm danger" onclick={() => remove(index)}><Trash2 size={14} /> Delete</button>
            </div>
            <fieldset><legend>Content</legend>
              <label>Section title<input bind:value={selected.title} placeholder={schema.label} /></label>
              <label>Subtitle<textarea bind:value={selected.subtitle} rows="2"></textarea></label>
              {#each schema.fields as field (field.key)}
                {#if field.type === 'list'}
                  <div class="list-field"><div class="list-head"><strong>{field.label}</strong><button class="btn secondary sm" onclick={() => setOption(selected, field.key, [...(selected.options?.[field.key] ?? []), structuredClone(field.defaultItem ?? {})])}><Plus size={14} /> Add</button></div>
                    {#each (selected.options?.[field.key] ?? []) as item, itemIndex}
                      <div class="item-editor">
                        <div class="list-head"><strong>Item {itemIndex + 1}</strong><button class="icon-action danger" aria-label="Remove item" onclick={() => setOption(selected, field.key, selected.options?.[field.key].filter((_: unknown, i: number) => i !== itemIndex))}><Trash2 size={14} /></button></div>
                        {#each field.items ?? [] as sub}
                          <label>{sub.label}
                            {#if sub.type === 'textarea'}<textarea rows="2" value={item[sub.key] ?? ''} oninput={(e) => (item[sub.key] = e.currentTarget.value)}></textarea>
                            {:else if sub.type === 'select'}<select value={item[sub.key] ?? ''} onchange={(e) => (item[sub.key] = e.currentTarget.value)}>{#each sub.options ?? [] as opt}<option value={opt.value}>{opt.label}</option>{/each}</select>
                            {:else}<input type={sub.type === 'url' || sub.type === 'media' ? 'url' : 'text'} value={item[sub.key] ?? ''} oninput={(e) => (item[sub.key] = e.currentTarget.value)} />{/if}
                          </label>
                          {#if sub.type === 'media'}<label class="upload-button">{uploadingMedia ? 'Uploading…' : 'Upload image'}<input type="file" accept="image/*" hidden disabled={uploadingMedia} onchange={(e) => uploadSectionMedia(e.currentTarget.files?.[0], (url) => (item[sub.key] = url))} /></label>{/if}
                        {/each}
                      </div>
                    {/each}
                  </div>
                {:else if field.type === 'toggle'}
                  <label class="check"><input type="checkbox" checked={selected.options?.[field.key] ?? false} onchange={(e) => setOption(selected, field.key, e.currentTarget.checked)} /> {field.label}</label>
                {:else}<label>{field.label}
                  {#if field.type === 'textarea'}<textarea rows="4" value={selected.options?.[field.key] ?? ''} oninput={(e) => setOption(selected, field.key, e.currentTarget.value)}></textarea>
                  {:else if field.type === 'select'}<select value={selected.options?.[field.key] ?? ''} onchange={(e) => setOption(selected, field.key, e.currentTarget.value)}>{#each field.options ?? [] as opt}<option value={opt.value}>{opt.label}</option>{/each}</select>
                  {:else}<input type={field.type === 'number' ? 'number' : field.type === 'color' ? 'color' : field.type === 'url' || field.type === 'media' ? 'url' : 'text'} value={selected.options?.[field.key] ?? ''} oninput={(e) => setOption(selected, field.key, field.type === 'number' ? Number(e.currentTarget.value) : e.currentTarget.value)} />{/if}
                </label>{#if field.type === 'media'}<label class="upload-button">{uploadingMedia ? 'Uploading…' : 'Upload media'}<input type="file" accept={schema.type === 'video' && field.key === 'url' ? 'video/mp4,video/webm' : 'image/*'} hidden disabled={uploadingMedia} onchange={(e) => uploadSectionMedia(e.currentTarget.files?.[0], (url) => setOption(selected, field.key, url))} /></label>{/if}{/if}
              {/each}
            </fieldset>
            <fieldset><legend>Section layout</legend>
              <div class="two"><label>Width<select value={selected.options?.width ?? 'default'} onchange={(e) => setOption(selected, 'width', e.currentTarget.value)}><option value="narrow">Narrow</option><option value="default">Default</option><option value="wide">Wide</option><option value="full">Full</option></select></label>
                <label>Spacing<select value={selected.options?.spacing ?? 'normal'} onchange={(e) => setOption(selected, 'spacing', e.currentTarget.value)}><option value="none">None</option><option value="compact">Compact</option><option value="normal">Normal</option><option value="roomy">Roomy</option></select></label></div>
              <div class="two"><label>Background<select value={selected.options?.background ?? 'default'} onchange={(e) => setOption(selected, 'background', e.currentTarget.value)}><option value="default">Page</option><option value="surface">Surface</option><option value="accent">Accent</option><option value="gradient">Gradient</option><option value="image">Image</option></select></label>
                <label>Anchor<input value={selected.options?.anchor ?? ''} placeholder="e.g. join" oninput={(e) => setOption(selected, 'anchor', e.currentTarget.value)} /></label></div>
              {#if selected.options?.background === 'image'}<label>Background image URL<input type="url" value={selected.options?.background_image ?? ''} oninput={(e) => setOption(selected, 'background_image', e.currentTarget.value)} /></label><label class="upload-button">Upload background<input type="file" accept="image/*" hidden onchange={(e) => uploadSectionMedia(e.currentTarget.files?.[0], (url) => setOption(selected, 'background_image', url))} /></label>{/if}
            </fieldset>
          {:else}<div class="empty">Select a section or add a new one.</div>{/if}
        </div>

        <aside class="preview card"><div class="list-head"><strong>Live preview</strong><span class="muted">Unsaved changes shown</span></div><iframe title="Landing page preview" src={previewUrl} bind:this={frame} onload={() => config && frame?.contentWindow?.postMessage({ type: 'scopenet-landing-preview', config: $state.snapshot(config) }, location.origin)}></iframe></aside>
      </div>
    {:else if tab === 'design'}
      <div class="settings-grid card"><h2>Site identity & theme</h2><p>These settings apply to every section. Each section can override its own background and spacing.</p>
        <div class="two"><label>Brand name<input bind:value={config.brand_name} /></label><ImageInput bind:value={config.logo_url} label="Brand logo" /></div>
        <label>Tagline<input bind:value={config.tagline} /></label>
        <div class="two"><label>Server address<input bind:value={config.server_ip} /></label><label>Port<input type="number" min="1" max="65535" bind:value={config.server_port} /></label></div>
        <div class="two"><label>Default hero title<input bind:value={config.hero_title} /></label><label>Hero button text<input bind:value={config.hero_cta_text} /></label></div>
        <label>Default hero subtitle<textarea rows="3" bind:value={config.hero_subtitle}></textarea></label>
        <div class="two"><label>Hero background<select bind:value={config.hero_bg_type}><option value="gradient">Theme gradient</option><option value="image">Image</option><option value="video">Video</option></select></label>
          {#if config.hero_bg_type !== 'gradient'}<ImageInput bind:value={config.hero_bg_url} label="Background media" video={config.hero_bg_type === 'video'} />{/if}</div>
        <div class="theme-grid"><label>Background<input type="color" bind:value={config.theme.background} /></label><label>Surface<input type="color" bind:value={config.theme.surface} /></label><label>Accent<input type="color" bind:value={config.theme.accent} /></label><label>Text<input type="color" bind:value={config.theme.text} /></label><label>Muted<input type="color" bind:value={config.theme.muted} /></label></div>
        <div class="two"><label>Content width (px)<input type="number" min="720" max="1800" bind:value={config.theme.max_width} /></label><label>Corner radius (px)<input type="number" min="0" max="32" bind:value={config.theme.radius} /></label></div>
        <label>Font<select bind:value={config.theme.font}><option>Inter</option><option>Outfit</option><option>Space Grotesk</option><option>Plus Jakarta Sans</option><option>system-ui</option></select></label>
        <label>Footer text<input bind:value={config.theme.footer_text} /></label>
      </div>
    {:else if tab === 'downloads'}
      <div class="settings-grid card"><h2>Launcher downloads</h2><p>Upload an installer for each platform, or link to your GitHub releases and anything you have not uploaded is picked up from the latest release. Windows installers also reach installed launchers as updates.</p>
        <label>External download URL<input type="url" bind:value={config.external_download_url} placeholder="https://github.com/VeloraMCDev/velora-launcher/releases" /></label>
        <DownloadsManager compact />
      </div>
    {:else if tab === 'faq'}
      <div class="settings-grid card"><div class="list-head"><div><h2>Frequently asked questions</h2><p>Shown by each FAQ section.</p></div><button class="btn primary sm" onclick={() => config?.faqs.push({ id: crypto.randomUUID(), question: '', answer: '' })}><Plus size={15} /> Add question</button></div>
        {#each config.faqs as item, index (item.id)}
          <div class="item-editor"><div class="list-head"><strong>Question {index + 1}</strong><button class="icon-action danger" aria-label="Remove question" onclick={() => config?.faqs.splice(index, 1)}><Trash2 size={15} /></button></div><label>Question<input bind:value={item.question} /></label><label>Answer<textarea rows="4" bind:value={item.answer}></textarea></label></div>
        {/each}
      </div>
    {:else}
      <div class="settings-grid card"><h2>Advanced CSS</h2><p>Custom CSS applies to the public page after the built-in styles. Section IDs are stable for targeted styling.</p><textarea class="code" rows="20" spellcheck="false" bind:value={config.custom_css} placeholder="Add custom CSS rules"></textarea></div>
    {/if}
  </div>
{/if}

<style>
  .builder { max-width: 1800px; margin: 0 auto; padding: 24px; display: flex; flex-direction: column; gap: 20px; }
  .builder-head { align-items: flex-start; flex-wrap: wrap; }
  h1 { margin: 0 0 5px; font-size: 1.7rem; } h2 { margin: 0; font-size: 1.15rem; } p { color: var(--text-2); margin: 4px 0; line-height: 1.5; }
  .head-actions { flex-wrap: wrap; justify-content: flex-end; } .enabled { display: flex; align-items: center; gap: 6px; white-space: nowrap; font-size: .84rem; }
  .studio-tabs { justify-content: flex-start; border-bottom: 1px solid var(--line); gap: 4px; overflow-x: auto; }
  .studio-tabs button { border: 0; border-radius: 7px 7px 0 0; padding: 10px 16px; background: transparent; color: var(--text-2); }
  .studio-tabs button.active { color: var(--text); background: var(--bg-2); box-shadow: inset 0 -2px var(--accent); }
  .studio-grid { display: grid; grid-template-columns: minmax(220px, 270px) minmax(350px, 470px) minmax(350px, 1fr); gap: 16px; align-items: start; }
  .card { background: var(--bg-2); border: 1px solid var(--line); border-radius: 12px; padding: 18px; min-width: 0; }
  .section-list, .inspector, .preview { min-height: 450px; } .section-list { position: sticky; top: 12px; max-height: calc(100vh - 100px); overflow-y: auto; }
  .list-head { margin-bottom: 12px; } .list-head .muted { font-weight: 400; color: var(--text-2); font-size: .8rem; }
  .list-items { display: flex; flex-direction: column; gap: 5px; }
  .list-item { display: flex; align-items: center; border: 1px solid transparent; border-radius: 9px; background: var(--bg); }
  .list-item.selected { border-color: var(--accent); } .list-item.muted-block { opacity: .55; }
  .select-block { display: flex; align-items: center; gap: 8px; flex: 1; min-width: 0; padding: 9px; text-align: left; border: 0; background: transparent; color: var(--text); }
  .select-block span { display: flex; flex-direction: column; min-width: 0; } .select-block strong { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; font-size: .84rem; } .select-block small { color: var(--text-2); }
  .icon-action { padding: 6px; border: 0; background: transparent; color: var(--text-2); } .icon-action:hover { color: var(--text); } .danger { color: #e5484d; }
  .gallery { max-height: 350px; overflow-y: auto; border: 1px solid var(--line); border-radius: 8px; padding: 8px; margin-bottom: 12px; background: var(--bg); }
  .group-label { display: block; color: var(--accent); font-size: .68rem; letter-spacing: .09em; text-transform: uppercase; margin: 10px 5px 4px; }
  .gallery-item { display: flex; flex-direction: column; align-items: flex-start; text-align: left; gap: 2px; width: 100%; border: 0; background: transparent; padding: 7px; border-radius: 6px; color: var(--text); }
  .gallery-item:hover { background: var(--bg-2); } .gallery-item small { color: var(--text-2); line-height: 1.35; }
  .inspector-head { margin-bottom: 12px; } .eyebrow { font-size: .7rem; color: var(--accent); text-transform: uppercase; letter-spacing: .08em; }
  .section-actions { justify-content: flex-start; flex-wrap: wrap; padding-bottom: 16px; border-bottom: 1px solid var(--line); }
  fieldset { border: 0; border-bottom: 1px solid var(--line); padding: 16px 0; margin: 0; display: flex; flex-direction: column; gap: 12px; } legend { font-weight: 700; padding: 0; }
  label { display: flex; flex-direction: column; gap: 5px; font-size: .82rem; font-weight: 550; } label.check { flex-direction: row; align-items: center; }
  input, textarea, select { width: 100%; font: inherit; color: var(--text); background: var(--bg); border: 1px solid var(--line); border-radius: 7px; padding: 8px 10px; box-sizing: border-box; }
  input[type='checkbox'], input[type='color'] { width: auto; } input[type='color'] { min-width: 64px; height: 38px; padding: 2px; }
  textarea { resize: vertical; } .two, .theme-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; } .theme-grid { grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); }
  .list-field { display: flex; flex-direction: column; gap: 10px; } .item-editor { display: flex; flex-direction: column; gap: 10px; padding: 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--bg); }
  .preview { position: sticky; top: 12px; } .preview iframe { border: 1px solid var(--line); border-radius: 8px; width: 100%; height: min(72vh, 850px); background: white; }
  .settings-grid { width: min(100%, 850px); display: flex; flex-direction: column; gap: 14px; } .settings-grid h2 { margin-bottom: 0; }
  .code { font-family: monospace; min-height: 300px; } .empty { color: var(--text-2); padding: 24px 8px; }
  .upload-button { display: inline-flex; width: fit-content; padding: 6px 10px; border: 1px solid var(--line); border-radius: 7px; cursor: pointer; color: var(--accent); background: var(--bg); }
  @media (max-width: 1250px) { .studio-grid { grid-template-columns: 240px minmax(350px, 1fr); } .preview { grid-column: 1 / -1; position: static; } }
  @media (max-width: 700px) { .builder { padding: 12px; } .studio-grid { grid-template-columns: 1fr; } .section-list { position: static; max-height: none; } .two { grid-template-columns: 1fr; } }
</style>
