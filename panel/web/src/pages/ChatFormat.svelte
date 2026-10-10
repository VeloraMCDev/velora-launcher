<script lang="ts">
  import { onMount } from 'svelte';
  import { Save, RotateCcw, Sparkles } from '@lucide/svelte';
  import Toggle from '../components/Toggle.svelte';
  import { get, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type S = {
    enabled: boolean; format: string; guild_format: string; title_format: string; group_format: string; level_format: string;
    markdown: boolean; allow_colors: boolean;
  };
  let s = $state<S | null>(null);
  let defaults = $state<S | null>(null);
  let saved = $state('');
  let busy = $state(false);

  // What the preview pretends the player is.
  let sample = $state({ name: 'Steve', message: 'Hello **everyone**, my base is at spawn!', guild: 'IRON', guildName: 'Iron Fortress', title: 'Veteran', group: 'Admin', prefix: '&c[Admin]', suffix: '&7*', level: 12 });

  async function load() {
    try {
      const d = await get<{ settings: S; defaults: S }>('/api/admin/chat');
      s = d.settings; defaults = d.defaults; saved = JSON.stringify(d.settings);
    } catch (e) { toastError(e); }
  }
  onMount(load);
  const dirty = $derived(s ? JSON.stringify(s) !== saved : false);

  const PRESETS: { name: string; desc: string; v: Partial<S> }[] = [
    { name: 'Classic', desc: 'Faction, title, then LuckPerms rank', v: { format: '{guild}{title}{prefix}{name}{suffix}&7: &f{message}', guild_format: '&3[{tag}] ', title_format: '&6[{title}] ', group_format: '', level_format: '' } },
    { name: 'Rank first', desc: 'LuckPerms prefix leads', v: { format: '{prefix}{name}{suffix} {guild}&8» &f{message}', guild_format: '&3[{tag}] ', title_format: '', group_format: '', level_format: '' } },
    { name: 'Level badge', desc: 'Shows the global level', v: { format: '{level}{guild}{prefix}{name}{suffix}&7: &f{message}', level_format: '&8[&e{level}&8] ', guild_format: '&3{tag} ', title_format: '', group_format: '' } },
    { name: 'Everything', desc: 'Every part at once', v: { format: '{level}{guild}{title}{group}{prefix}{name}{suffix}&7: &f{message}', level_format: '&8[&e{level}&8] ', guild_format: '&3[{tag}] ', title_format: '&6{title} ', group_format: '&b({group}) ' } },
    { name: 'Minimal', desc: 'Just names and messages', v: { format: '{name} &8» &f{message}', guild_format: '', title_format: '', group_format: '', level_format: '' } },
  ];
  function apply(v: Partial<S>) { if (s) Object.assign(s, v); }

  const COLORS: Record<string, string> = {
    '0': '#000000', '1': '#0000aa', '2': '#00aa00', '3': '#00aaaa', '4': '#aa0000', '5': '#aa00aa', '6': '#ffaa00', '7': '#aaaaaa',
    '8': '#555555', '9': '#5555ff', a: '#55ff55', b: '#55ffff', c: '#ff5555', d: '#ff55ff', e: '#ffff55', f: '#ffffff',
  };
  type Span = { text: string; color: string; bold: boolean; italic: boolean; underline: boolean; strike: boolean };
  function spans(src: string): Span[] {
    const out: Span[] = [];
    let st = { color: '#ffffff', bold: false, italic: false, underline: false, strike: false };
    const re = /&#([0-9a-fA-F]{6})|&([0-9a-fk-orA-FK-OR])/g;
    let last = 0, m: RegExpExecArray | null;
    const push = (t: string) => t && out.push({ text: t, ...st });
    while ((m = re.exec(src))) {
      push(src.slice(last, m.index)); last = re.lastIndex;
      if (m[1]) st = { color: '#' + m[1], bold: false, italic: false, underline: false, strike: false };
      else {
        const c = m[2].toLowerCase();
        if (COLORS[c]) st = { color: COLORS[c], bold: false, italic: false, underline: false, strike: false };
        else if (c === 'l') st.bold = true; else if (c === 'o') st.italic = true; else if (c === 'n') st.underline = true; else if (c === 'm') st.strike = true;
        else if (c === 'r') st = { color: '#ffffff', bold: false, italic: false, underline: false, strike: false };
      }
    }
    push(src.slice(last));
    return out;
  }
  // The same rules the game plugin follows, so the preview matches what players see.
  const plain = (t: string) => t.replace(/&#[0-9a-fA-F]{6}/g, '').replace(/&[0-9a-fk-orA-FK-OR]/g, '').replace(/[%[\]{}]/g, ' ').trim();
  function md(t: string) {
    return t.replace(/\*\*(.+?)\*\*/g, '&l$1&r&f').replace(/__(.+?)__/g, '&n$1&r&f').replace(/~~(.+?)~~/g, '&m$1&r&f').replace(/`(.+?)`/g, '&7$1&r&f').replace(/\*(.+?)\*/g, '&o$1&r&f');
  }
  function fill(t: string, kv: Record<string, string>) { return t.replace(/\{(\w+)}/g, (_, k) => kv[k] ?? ''); }
  const rendered = $derived.by(() => {
    if (!s) return [];
    const v = sample;
    const guild = v.guild.trim() ? fill(s.guild_format, { tag: plain(v.guild), guild_name: plain(v.guildName) }) : '';
    const title = v.title.trim() ? fill(s.title_format, { title: plain(v.title) }) : '';
    const group = v.group.trim() ? fill(s.group_format, { group: plain(v.group) }) : '';
    const level = v.level > 0 ? fill(s.level_format, { level: String(v.level) }) : '';
    const prefix = v.prefix.trim() ? '&r' + v.prefix.trimEnd() + ' ' : '';
    const suffix = v.suffix.trim() ? ' &r' + v.suffix.trimStart() : '';
    const msg = s.markdown ? md(v.message) : v.message;
    const line = s.format.replace(/\{(\w+)}/g, (_, k) => ({ guild, title, rank_title: title, group, level, prefix, suffix, name: v.name, message: msg } as Record<string, string>)[k] ?? '');
    return spans(line);
  });

  const insert = (key: 'format', ph: string) => { if (s) s[key] += `{${ph}}`; };

  async function save() {
    if (!s) return;
    busy = true;
    try { await put('/api/admin/chat', s); saved = JSON.stringify(s); toast('Chat layout saved — servers pick it up within seconds'); } catch (e) { toastError(e); } finally { busy = false; }
  }
</script>

<div class="page wide">
  <header>
    <div>
      <h1>Chat</h1>
      <p>Design how messages look in-game. LuckPerms prefixes and suffixes (including <code>&amp;</code> and <code>&amp;#RRGGBB</code> colours) are inserted automatically.</p>
    </div>
    <button onclick={save} disabled={!dirty || busy}><Save size={15} /> Save</button>
  </header>

  {#if s}
    <section class="card preview">
      <div class="mc" aria-label="Chat preview">
        {#each rendered as sp}<span style:color={sp.color} class:b={sp.bold} class:i={sp.italic} class:u={sp.underline} class:st={sp.strike}>{sp.text}</span>{/each}
      </div>
      <div class="sample">
        <label>Name<input bind:value={sample.name} /></label>
        <label>Message<input bind:value={sample.message} /></label>
        <label>Faction tag<input bind:value={sample.guild} placeholder="none" /></label>
        <label>Rank title<input bind:value={sample.title} placeholder="none" /></label>
        <label>LuckPerms prefix<input bind:value={sample.prefix} placeholder="none" /></label>
        <label>LuckPerms suffix<input bind:value={sample.suffix} placeholder="none" /></label>
        <label>LuckPerms group<input bind:value={sample.group} placeholder="none" /></label>
        <label>Level<input type="number" min="0" bind:value={sample.level} /></label>
      </div>
    </section>

    <div class="cols">
      <section class="card">
        <Toggle bind:checked={s.enabled} label="Use this layout" help="Turn off to leave chat to the server or another chat plugin." />
        <h3>Layout</h3>
        <input class="mono" bind:value={s.format} spellcheck="false" />
        <div class="chips">
          {#each ['guild', 'title', 'rank_title', 'group', 'level', 'prefix', 'name', 'suffix', 'message'] as ph}
            <button class="chip" onclick={() => insert('format', ph)}>{`{${ph}}`}</button>
          {/each}
        </div>
        <p class="hint"><code>{'{rank_title}'}</code> draws the player's rank as its PNG badge in game (needs the server resource pack turned on, and <em>required</em> so nobody sees empty boxes); players whose rank has no uploaded PNG get the text <code>{'{title}'}</code> instead. <code>%rank_title%</code> works too. The preview below shows the text version.</p>
        <p class="hint">Must contain <code>{'{name}'}</code> and <code>{'{message}'}</code>. Use <code>&amp;a</code>…<code>&amp;f</code>, <code>&amp;l</code> bold, <code>&amp;o</code> italic, <code>&amp;#ff8800</code> for custom colours.</p>

        <h3>Parts</h3>
        <p class="hint">Each part appears only when the player has it, so there are no empty brackets. Leave blank to hide a part everywhere.</p>
        <label class="f">Faction <small>{'{tag} {guild_name}'}</small><input class="mono" bind:value={s.guild_format} spellcheck="false" /></label>
        <label class="f">Rank title <small>{'{title}'}</small><input class="mono" bind:value={s.title_format} spellcheck="false" /></label>
        <label class="f">LuckPerms group <small>{'{group}'}</small><input class="mono" bind:value={s.group_format} spellcheck="false" /></label>
        <label class="f">Level <small>{'{level}'}</small><input class="mono" bind:value={s.level_format} spellcheck="false" /></label>

        <h3>Messages</h3>
        <Toggle bind:checked={s.allow_colors} label="Allow colours in messages" help="Players with the velora.chat.color permission can type & colour codes." />
        <Toggle bind:checked={s.markdown} label="Light markdown" help="**bold**, *italic*, __underline__, ~~strike~~ and `code`." />
      </section>

      <aside class="card presets">
        <h3><Sparkles size={15} /> Presets</h3>
        {#each PRESETS as p}
          <button class="preset" onclick={() => apply(p.v)}><b>{p.name}</b><small>{p.desc}</small></button>
        {/each}
        <button class="ghost" onclick={() => defaults && apply(defaults)}><RotateCcw size={14} /> Reset to default</button>
        <p class="hint">LuckPerms: set prefixes with <code>/lp group admin meta setprefix 100 "&amp;c[Admin]"</code>. The highest priority wins, and changes show on the player's next message.</p>
      </aside>
    </div>
  {/if}
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 16px; }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  header button, .ghost { display: inline-flex; align-items: center; gap: 6px; }
  .card { display: flex; flex-direction: column; gap: 10px; }
  .preview { gap: 14px; }
  .mc { background: #1c1c1c url('data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="%23222"/><rect width="4" height="4" fill="%23262626"/><rect x="4" y="4" width="4" height="4" fill="%23262626"/></svg>'); border: 1px solid var(--line); border-radius: 10px; padding: 14px 16px; font: 15px/1.5 'Minecraft', ui-monospace, 'Cascadia Mono', Consolas, monospace; text-shadow: 1px 1px 0 #0006; min-height: 2.6rem; white-space: pre-wrap; word-break: break-word; }
  .b { font-weight: 700; } .i { font-style: italic; } .u { text-decoration: underline; } .st { text-decoration: line-through; } .u.st { text-decoration: underline line-through; }
  .sample { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
  .sample label, .f { display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--muted); }
  .f small { font-weight: 400; margin-left: 6px; font-family: ui-monospace, monospace; }
  .cols { display: grid; grid-template-columns: minmax(0, 1.8fr) minmax(0, 1fr); gap: 16px; align-items: start; }
  h3 { margin: 8px 0 0; font-size: 0.95rem; display: flex; align-items: center; gap: 6px; }
  .mono { font-family: ui-monospace, 'Cascadia Mono', Consolas, monospace; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { padding: 3px 9px; border-radius: 999px; font: 500 0.76rem ui-monospace, monospace; background: color-mix(in srgb, var(--accent) 14%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); }
  .hint { margin: 0; font-size: 0.78rem; color: var(--muted); line-height: 1.5; }
  code { font: 0.78rem ui-monospace, monospace; background: var(--bg-2); padding: 1px 5px; border-radius: 5px; }
  .presets { position: sticky; top: 16px; }
  .preset { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; text-align: left; padding: 10px 12px; }
  .preset small { color: var(--muted); font-weight: 400; }
  @media (max-width: 900px) { .cols { grid-template-columns: 1fr; } .sample { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
