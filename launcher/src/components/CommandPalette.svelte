<script lang="ts">
  import { pageEnabled, enabled } from '@scopenet/experience';
  import { selectedInstance } from '../lib/store.svelte';
  import { tick } from 'svelte';
  import { Search, Home, Target, Shield, Store, Dice5, Users, Terminal, Trophy, Settings, CornerDownLeft, Smartphone, Play, RefreshCw } from '@lucide/svelte';
  import { COMMAND_GROUPS } from '@scopenet/commands';
  import { app, instances, refresh, selectInstance, toast } from '../lib/store.svelte';
  import { openUrl } from '../lib/tauri';

  type Item = { id: string; label: string; group: string; hint?: string; icon?: typeof Home; keywords?: string; run: () => void };

  let open = $state(false);
  let query = $state('');
  let cursor = $state(0);
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLDivElement>();

  const go = (view: typeof app.view) => () => { app.view = view; };
  const pages: Item[] = [
    { id: 'p-home', label: 'Home', group: 'Go to', hint: 'Play and server status', icon: Home, run: go('home') },
    { id: 'p-quests', label: 'Quests & objectives', group: 'Go to', icon: Target, run: go('quests'), keywords: 'daily weekly achievements level' },
    { id: 'p-guilds', label: 'Guilds & claims', group: 'Go to', icon: Shield, run: go('guilds'), keywords: 'land bank clan' },
    { id: 'p-market', label: 'Market & auctions', group: 'Go to', icon: Store, run: go('market'), keywords: 'buy sell bid shop' },
    { id: 'p-casino', label: 'Casino', group: 'Go to', icon: Dice5, run: go('casino'), keywords: 'slots wheel plinko mines bounty bet gamble' },
    { id: 'p-social', label: 'Friends & social', group: 'Go to', icon: Users, run: go('social'), keywords: 'messages dm chat profile' },
    { id: 'p-commands', label: 'Command guide', group: 'Go to', icon: Terminal, run: go('commands'), keywords: 'help in-game' },
    { id: 'p-stats', label: 'Stats & leaderboards', group: 'Go to', icon: Trophy, run: go('stats'), keywords: 'ranking top' },
    { id: 'p-settings', label: 'Settings', group: 'Go to', icon: Settings, run: go('settings') },
  ];

  const items = $derived<Item[]>([
    ...pages.filter(p => pageEnabled(selectedInstance()?.experience, p.id.replace('p-', ''))),
    ...instances().map((i) => ({ id: `i-${i.id}`, label: `Open ${i.name}`, group: 'Instances', hint: `${i.mc_version} · ${i.loader}`, icon: Play, run: () => { selectInstance(i.id); app.view = 'home'; } })),
    ...(app.panelUrl ? [{ id: 'a-web', label: 'Open the player panel in your browser', group: 'Actions', hint: 'Everything here, on any device', icon: Smartphone, run: () => void openUrl(`${app.panelUrl}/#/play`) }] : []),
    { id: 'a-refresh', label: 'Refresh from the server', group: 'Actions', icon: RefreshCw, run: () => void refresh().then(() => toast('Up to date')) },
    ...(enabled(selectedInstance()?.experience, 'commands') ? COMMAND_GROUPS : []).flatMap((g) => g.cmds.map((c) => ({
      id: `c-${c.usage}`, label: c.usage, group: 'In-game commands', hint: c.does, keywords: `${g.title} ${(c.aliases ?? []).join(' ')} ${c.perm}`,
      run: () => { void navigator.clipboard?.writeText(c.usage.split(' · ')[0].replace(/\s[<[].*$/, '')).catch(() => {}); toast(`Copied ${c.usage.split(' ')[0]}`); },
    }))),
  ]);

  const score = (it: Item, q: string) => {
    const l = it.label.toLowerCase();
    return l.startsWith(q) ? 3 : l.includes(q) ? 2 : `${it.hint ?? ''} ${it.keywords ?? ''} ${it.group}`.toLowerCase().includes(q) ? 1 : 0;
  };
  const results = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items.slice(0, 40);
    return items.map((it) => ({ it, s: score(it, q) })).filter((x) => x.s > 0).sort((a, b) => b.s - a.s).map((x) => x.it).slice(0, 40);
  });
  const grouped = $derived.by(() => {
    const out: { group: string; rows: { it: Item; index: number }[] }[] = [];
    results.forEach((it, index) => {
      let g = out.find((x) => x.group === it.group);
      if (!g) out.push((g = { group: it.group, rows: [] }));
      g.rows.push({ it, index });
    });
    return out;
  });

  $effect(() => { if (open) { query = ''; cursor = 0; void tick().then(() => input?.focus()); } });
  $effect(() => { void query; cursor = 0; });
  function choose(it?: Item) { if (!it) return; open = false; it.run(); }
  async function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); cursor = Math.min(results.length - 1, cursor + 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); cursor = Math.max(0, cursor - 1); }
    else if (e.key === 'Enter') { e.preventDefault(); choose(results[cursor]); }
    else if (e.key === 'Escape') open = false;
    else return;
    await tick();
    list?.querySelector('.on')?.scrollIntoView({ block: 'nearest' });
  }
  function global(e: KeyboardEvent) {
    if (!app.accounts.length || !app.manifest) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); open = !open; }
  }
</script>

<svelte:window onkeydown={global} />

{#if open}
  <div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && (open = false)}>
    <div class="palette glass" role="dialog" aria-modal="true" aria-label="Command palette">
      <label class="search">
        <Search size={17} />
        <input bind:this={input} bind:value={query} onkeydown={key} placeholder="Jump to a page, instance or in-game command…" autocomplete="off" spellcheck="false" aria-label="Search" />
        <kbd>esc</kbd>
      </label>
      <div class="list" bind:this={list}>
        {#each grouped as g (g.group)}
          <div class="group">{g.group}</div>
          {#each g.rows as { it, index } (it.id)}
            <button class="item" class:on={index === cursor} onmousemove={() => (cursor = index)} onclick={() => choose(it)}>
              <span class="ico">{#if it.icon}<it.icon size={15} />{/if}</span>
              <span class="lbl">{it.label}</span>
              {#if it.hint}<span class="hint">{it.hint}</span>{/if}
              {#if index === cursor}<CornerDownLeft size={12} />{/if}
            </button>
          {/each}
        {:else}
          <p class="none">Nothing matches “{query}”.</p>
        {/each}
      </div>
      <footer><span><kbd>↑</kbd><kbd>↓</kbd> move</span><span><kbd>↵</kbd> open or copy</span><span><kbd>Ctrl</kbd><kbd>K</kbd> toggle</span></footer>
    </div>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 120; background: rgba(4, 5, 9, 0.62); backdrop-filter: blur(6px); display: grid; place-items: start center; padding: 11vh 1rem 1rem; animation: fade 0.14s ease; }
  .palette { width: min(40rem, 100%); max-height: min(34rem, 76vh); display: flex; flex-direction: column; overflow: hidden; border-radius: calc(var(--radius) + 6px); background: var(--surface); border: 1px solid var(--line-strong); box-shadow: 0 30px 80px -20px #000; animation: pop 0.18s var(--ease); }
  .search { display: flex; align-items: center; gap: 0.7rem; padding: 0.85rem 1rem; border-bottom: 1px solid var(--line); color: var(--muted); }
  .search input { flex: 1; background: transparent; border: none; outline: none; padding: 0; font-size: 1rem; color: var(--text); box-shadow: none; }
  kbd { font-family: var(--mono); font-size: 0.68rem; padding: 0.1rem 0.4rem; border-radius: 0.3rem; border: 1px solid var(--line-strong); background: color-mix(in srgb, var(--text) 6%, transparent); color: var(--muted); margin-right: 0.15rem; }
  .list { overflow-y: auto; padding: 0.35rem 0.5rem 0.6rem; }
  .group { padding: 0.65rem 0.6rem 0.25rem; font-size: 0.66rem; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); font-weight: 650; }
  .item { width: 100%; display: flex; align-items: center; justify-content: flex-start; gap: 0.7rem; padding: 0.5rem 0.6rem; border: none; background: transparent; border-radius: var(--radius-sm); text-align: left; color: var(--text); font-weight: 500; }
  .item.on { background: color-mix(in srgb, var(--accent) 20%, transparent); }
  .ico { width: 1.7rem; height: 1.7rem; display: grid; place-items: center; border-radius: 0.5rem; background: color-mix(in srgb, var(--text) 7%, transparent); color: var(--muted); flex-shrink: 0; }
  .item.on .ico { background: var(--accent); color: #fff; }
  .lbl { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-family: inherit; }
  .hint { margin-left: auto; color: var(--muted); font-size: 0.76rem; font-weight: 400; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 55%; }
  .none { padding: 1.6rem; text-align: center; color: var(--muted); }
  footer { display: flex; gap: 1rem; padding: 0.5rem 1rem; border-top: 1px solid var(--line); font-size: 0.7rem; color: var(--muted); }
</style>
