<script lang="ts">
  import { Save, LoaderCircle, Plus, Trash2, GripVertical, Pin, Sparkles, RotateCcw } from '@lucide/svelte';
  import ColorInput from '../components/ColorInput.svelte';
  import ImageInput from '../components/ImageInput.svelte';
  import LauncherPreview from '../components/LauncherPreview.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { get, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { Branding } from '../lib/types';

  let { onsaved }: { onsaved?: (b: Branding) => void } = $props();

  let b = $state<Branding | null>(null);
  let saved = $state('');
  let saving = $state(false);
  let section = $state('identity');

  $effect(() => {
    get<Branding>('/api/admin/branding').then((x) => {
      b = x;
      saved = JSON.stringify(x);
    });
  });

  const dirty = $derived(!!b && JSON.stringify(b) !== saved);

  async function save() {
    if (!b) return;
    saving = true;
    try {
      const r = await put<Branding>('/api/admin/branding', $state.snapshot(b));
      saved = JSON.stringify(r);
      onsaved?.(r);
      toast('Published — launchers pick it up on their next refresh');
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  const presets = [
    { name: 'SCOPENET', c: { accent: '#6d6af5', accent_2: '#8b88f8', background: '#0d0e12', surface: '#16181e', text: '#ecedf1', muted: '#8b8f9a' } },
    { name: 'Nebula', c: { accent: '#7c5cff', accent_2: '#22d3ee', background: '#0b0d17', surface: '#141828', text: '#eef0ff', muted: '#8b90b3' } },
    { name: 'Creeper', c: { accent: '#22c55e', accent_2: '#a3e635', background: '#07110b', surface: '#0f1d15', text: '#ecfdf3', muted: '#86a893' } },
    { name: 'Nether', c: { accent: '#f97316', accent_2: '#ef4444', background: '#130806', surface: '#22100c', text: '#fff1eb', muted: '#b48a7e' } },
    { name: 'Ocean', c: { accent: '#0ea5e9', accent_2: '#2dd4bf', background: '#06111a', surface: '#0c1d2b', text: '#e8f6ff', muted: '#7ea1b8' } },
    { name: 'Sakura', c: { accent: '#ec4899', accent_2: '#f9a8d4', background: '#140a12', surface: '#22121e', text: '#fff0f8', muted: '#b88aa6' } },
    { name: 'End', c: { accent: '#a855f7', accent_2: '#facc15', background: '#0d0a14', surface: '#18122a', text: '#f5f0ff', muted: '#9d8fbd' } },
    { name: 'Mono', c: { accent: '#e5e7eb', accent_2: '#9ca3af', background: '#0a0a0a', surface: '#161616', text: '#fafafa', muted: '#8a8a8a' } },
  ];

  function applyPreset(p: (typeof presets)[number]) {
    if (b) b.colors = { ...b.colors, ...p.c };
  }

  const fonts = ['Outfit', 'Inter', 'Space Grotesk', 'Plus Jakarta Sans'];
  const linkIcons = ['discord', 'website', 'store', 'youtube', 'twitter', 'twitch', 'github'];

  function addNews() {
    b?.news.unshift({ id: crypto.randomUUID().slice(0, 8), title: '', body: '', image_url: null, link: null, date: new Date().toISOString().slice(0, 10), pinned: false, tag: 'News' });
  }

  let dragIndex = $state<number | null>(null);
  function dropNews(i: number) {
    if (!b || dragIndex === null || dragIndex === i) return;
    const [item] = b.news.splice(dragIndex, 1);
    b.news.splice(i, 0, item);
    dragIndex = null;
  }

  const sections = [
    { id: 'identity', label: 'Identity' },
    { id: 'colors', label: 'Colours' },
    { id: 'background', label: 'Background' },
    { id: 'style', label: 'Style' },
    { id: 'news', label: 'News' },
    { id: 'links', label: 'Links' },
    { id: 'about', label: 'About tab' },
    { id: 'features', label: 'Features' },
    { id: 'css', label: 'Custom CSS' },
  ];
</script>

<div class="page wide">
  <div class="page-head">
    <div>
      <h1>Launcher design</h1>
      <p>Everything here updates every player's launcher live — no rebuild or reinstall.</p>
    </div>
    <div class="row">
      {#if dirty}<button class="ghost" onclick={() => (b = JSON.parse(saved))}><RotateCcw size={16} /> Discard</button>{/if}
      <button class="primary" disabled={!dirty || saving} onclick={save}>
        {#if saving}<LoaderCircle class="spin" size={16} />{:else}<Save size={16} />{/if} Publish
      </button>
    </div>
  </div>

  {#if b}
    <div class="layout">
      <div class="editor">
        <div class="tabs">
          {#each sections as s}<button class:active={section === s.id} onclick={() => (section = s.id)}>{s.label}</button>{/each}
        </div>

        {#if section === 'identity'}
          <div class="card col">
            <label class="field">Launcher name<input bind:value={b.name} maxlength="40" /></label>
            <label class="field">Tagline<input bind:value={b.tagline} maxlength="120" /></label>
            <ImageInput bind:value={b.logo_url} label="Logo" help="Shown in the title bar and sign-in screen. Transparent PNG works best." />
            <ImageInput bind:value={b.icon_url} label="Avatar / small icon" />
            <label class="field">In-game version label <span class="help">Appears on the Minecraft title screen and F3 menu.</span>
              <input bind:value={b.version_label} maxlength="32" />
            </label>
          </div>
        {:else if section === 'colors'}
          <div class="card col">
            <div class="presets">
              {#each presets as p}
                <button class="preset" onclick={() => applyPreset(p)} title={p.name}>
                  <span class="sw" style:background={p.c.background}><i style:background={p.c.accent}></i><i style:background={p.c.surface}></i></span>
                  <span>{p.name}</span>
                </button>
              {/each}
            </div>
            <div class="colors">
              <ColorInput bind:value={b.colors.accent} label="Accent" />
              <ColorInput bind:value={b.colors.accent_2} label="Accent 2" />
              <ColorInput bind:value={b.colors.background} label="Background" />
              <ColorInput bind:value={b.colors.surface} label="Surface" />
              <ColorInput bind:value={b.colors.text} label="Text" />
              <ColorInput bind:value={b.colors.muted} label="Muted text" />
              <ColorInput bind:value={b.colors.success} label="Success" />
              <ColorInput bind:value={b.colors.danger} label="Danger" />
            </div>
          </div>
        {:else if section === 'background'}
          <div class="card col">
            <div class="segmented">
              {#each ['gradient', 'image', 'video'] as k}
                <button class:active={b.background.kind === k} onclick={() => b && (b.background.kind = k as Branding['background']['kind'])}>{k === 'gradient' ? 'Solid colour' : k[0].toUpperCase() + k.slice(1)}</button>
              {/each}
            </div>
            {#if b.background.kind !== 'gradient'}
              <ImageInput bind:value={b.background.url} label={b.background.kind === 'video' ? 'Video (mp4/webm)' : 'Image'} video={b.background.kind === 'video'}
                help={b.background.kind === 'video' ? 'Keep it short and under ~10 MB. Players can turn video off to save resources.' : 'At least 1600×900 looks sharp.'} />
            {:else}
              <p class="muted small">A plain background in your background colour. Clean, and the lightest option on player PCs.</p>
            {/if}
            <label class="field"><span class="row">Blur <span class="spacer"></span>{b.background.blur}px</span><input type="range" min="0" max="30" bind:value={b.background.blur} /></label>
            <label class="field"><span class="row">Darken <span class="spacer"></span>{b.background.dim}%</span><input type="range" min="0" max="90" bind:value={b.background.dim} /></label>
          </div>
        {:else if section === 'style'}
          <div class="card col">
            <div class="field">
              <span class="lbl">Font</span>
              <div class="fonts">
                {#each fonts as f}
                  <button class="font" class:active={b.font === f} style:font-family="'{f} Variable', '{f}', sans-serif" onclick={() => b && (b.font = f)}>
                    <span class="aa">Aa</span><span>{f}</span>
                  </button>
                {/each}
              </div>
            </div>
            <label class="field"><span class="row">Corner radius <span class="spacer"></span>{b.radius}px</span><input type="range" min="0" max="28" bind:value={b.radius} /></label>
            <Toggle bind:checked={b.glass} label="Frosted glass" help="Translucent, blurred panels over image/video backgrounds. Off keeps panels solid." />
          </div>
        {:else if section === 'news'}
          <div class="col">
            <button class="add" onclick={addNews}><Plus size={16} /> Add a post</button>
            {#each b.news as n, i (n.id)}
              <div class="card col news" draggable="true" role="listitem"
                ondragstart={() => (dragIndex = i)} ondragover={(e) => e.preventDefault()} ondrop={() => dropNews(i)}>
                <div class="row">
                  <span class="grip" title="Drag to reorder"><GripVertical size={16} /></span>
                  <input class="title" bind:value={n.title} placeholder="Headline" />
                  <button class="ghost icon" class:pinned={n.pinned} title="Pin to top" aria-label="Pin" onclick={() => (n.pinned = !n.pinned)}><Pin size={15} /></button>
                  <button class="ghost icon" aria-label="Delete post" onclick={() => b?.news.splice(i, 1)}><Trash2 size={15} /></button>
                </div>
                <textarea bind:value={n.body} rows="3" placeholder="What's new?"></textarea>
                <div class="grid three">
                  <label class="field">Tag<input bind:value={n.tag} placeholder="Update" /></label>
                  <label class="field">Date<input type="date" bind:value={n.date} /></label>
                  <label class="field">Link<input bind:value={n.link} placeholder="https://…" /></label>
                </div>
                <ImageInput bind:value={n.image_url} label="Image" />
              </div>
            {:else}
              <div class="card empty"><Sparkles size={28} /><h3>No news yet</h3><p>Posts show up on the launcher home screen.</p></div>
            {/each}
          </div>
        {:else if section === 'links'}
          <div class="card col">
            {#each b.links as l, i}
              <div class="row link">
                <select bind:value={l.icon} style="width: 130px">{#each linkIcons as ic}<option value={ic}>{ic}</option>{/each}</select>
                <input bind:value={l.label} placeholder="Label" style="width: 150px" />
                <input bind:value={l.url} placeholder="https://discord.gg/…" />
                <button class="ghost icon" aria-label="Remove link" onclick={() => b?.links.splice(i, 1)}><Trash2 size={15} /></button>
              </div>
            {/each}
            <button class="add" onclick={() => b?.links.push({ icon: 'discord', label: 'Discord', url: '' })}><Plus size={16} /> Add link</button>
          </div>
        {:else if section === 'about'}
          <div class="card col">
            <p class="muted small">Shown in the launcher’s Settings → About tab. Leave fields blank to use the launcher defaults.</p>
            <ImageInput bind:value={b.about.icon_url} label="About icon" />
            <label class="field">Title<input bind:value={b.about.title} placeholder="About our network" maxlength="80" /></label>
            <label class="field">Description<textarea bind:value={b.about.body} rows="6" placeholder="Tell players about your community and launcher."></textarea></label>
            <h3>Links</h3>
            {#each b.about.links as link, i}
              <div class="row">
                <input bind:value={link.label} placeholder="Label" aria-label="Link label" />
                <input bind:value={link.url} placeholder="https://..." aria-label="Link URL" />
                <button class="ghost icon" aria-label="Remove link" onclick={() => b?.about.links.splice(i, 1)}><Trash2 size={15} /></button>
              </div>
            {/each}
            <button class="add" onclick={() => b?.about.links.push({ label: 'Website', url: '', icon: 'website' })}><Plus size={16} /> Add link</button>
          </div>
        {:else if section === 'features'}
          <div class="card col">
            <Toggle bind:checked={b.features.news} label="News feed" help="Show your posts on the home screen." />
            <Toggle bind:checked={b.features.server_status} label="Live server status" help="Player count, MOTD and ping for each instance's server." />
            <Toggle bind:checked={b.features.allow_user_theme} label="Let players change the theme" help="Players can pick their own accent colour and turn off effects." />
            <Toggle bind:checked={b.features.allow_advanced_java} label="Advanced Java settings" help="Let players set custom JVM arguments and Java paths." />
          </div>
        {:else if section === 'css'}
          <div class="card col">
            <p class="muted small">Injected into the launcher after the built-in styles. Useful variables: <code>--accent</code>, <code>--surface</code>, <code>--radius</code>.</p>
            <textarea class="mono css" bind:value={b.custom_css} rows="16" spellcheck="false" placeholder={'.play-button {\n  letter-spacing: 0.2em;\n}'}></textarea>
          </div>
        {/if}
      </div>

      <div class="preview">
        <div class="sticky">
          <span class="tiny muted label">Live preview</span>
          <LauncherPreview {b} />
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .wide { max-width: 1500px; }
  .layout { display: grid; grid-template-columns: minmax(380px, 520px) 1fr; gap: 28px; align-items: start; }
  @media (max-width: 1150px) { .layout { grid-template-columns: 1fr; } .preview { order: -1; } }
  .editor .tabs { margin-bottom: 16px; }
  .sticky { position: sticky; top: 24px; display: flex; flex-direction: column; gap: 10px; }
  .label { text-transform: uppercase; letter-spacing: 0.1em; font-weight: 600; }
  .field { display: flex; flex-direction: column; gap: 7px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .presets { display: grid; grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); gap: 8px; }
  .preset { flex-direction: column; gap: 6px; padding: 10px; background: var(--bg-2); font-size: 0.8rem; position: relative; }
  .sw { width: 100%; height: 28px; border-radius: 6px; display: flex; gap: 4px; align-items: center; padding: 0 6px; border: 1px solid var(--line); }
  .sw i { width: 14px; height: 14px; border-radius: 4px; }
  .colors { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .fonts { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .font { flex-direction: column; align-items: flex-start; gap: 2px; padding: 12px 14px; background: var(--bg-2); font-weight: 400; }
  .font .aa { font-size: 1.6rem; font-weight: 700; }
  .font.active { border-color: var(--accent); background: var(--accent-soft); }
  .add { border-style: dashed; background: transparent; color: var(--text-2); }
  .news .title { font-weight: 600; }
  .grip { cursor: grab; color: var(--muted); display: flex; }
  .pinned { color: var(--accent) !important; }
  .three { grid-template-columns: 1fr 1fr 1.4fr; }
  .css { min-height: 300px; font-size: 0.85rem; }
</style>
