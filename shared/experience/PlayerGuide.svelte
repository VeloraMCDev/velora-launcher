<script lang="ts">
  import InterfaceIcon from '@velora/experience/InterfaceIcon.svelte';
  import { BookOpen, Search, Copy, Check } from '@lucide/svelte';
  import { COMMAND_GROUPS, COMMAND_TIPS } from '@velora/commands';
  import type { PlayerGuide } from './branding';
  import { guideFor, safeGuideLink } from './player-guide';
  let { guide, details = [], initialQuery = '' }: { guide?: PlayerGuide | null; details?: { label: string; value: string }[]; initialQuery?: string } = $props();
  const content = $derived(guideFor(guide));
  let query = $state((() => initialQuery)());
  let category = $state('All');
  let copied = $state('');
  const q = $derived(query.trim().toLowerCase());
  const categories = $derived(['All', ...new Set(content.articles.map(a => a.category)), ...(content.show_commands ? ['Commands'] : [])]);
  const articles = $derived(content.articles.filter(a => (category === 'All' || category === a.category) && (!q || `${a.category} ${a.title} ${a.body} ${a.links.map(l => l.label).join(' ')}`.toLowerCase().includes(q))));
  const commands = $derived(content.show_commands && (category === 'All' || category === 'Commands') ? COMMAND_GROUPS.map(g => ({ ...g, cmds: g.cmds.filter(c => !q || `${g.title} ${c.usage} ${c.does} ${c.perm} ${(c.aliases ?? []).join(' ')}`.toLowerCase().includes(q)) })).filter(g => g.cmds.length) : []);
  async function copy(usage: string) {
    try { await navigator.clipboard.writeText(usage.split(' · ')[0]); copied = usage; setTimeout(() => copied = '', 1200); } catch { copied = ''; }
  }
  $effect(() => { if (!categories.includes(category)) category = 'All'; });
</script>
<div class="guide">
  <header><h1><InterfaceIcon name="BookOpen" fallback={BookOpen} size={26} /> {content.title}</h1><p>{content.introduction}</p></header>
  <label class="search"><InterfaceIcon name="Search" fallback={Search} size={18} /><input bind:value={query} placeholder="Search the Player Guide" aria-label="Search the Player Guide" /></label>
  <nav aria-label="Guide categories">{#each categories as c}<button class:active={category === c} aria-pressed={category === c} onclick={() => category = c}>{c}</button>{/each}</nav>
  {#if details.length && !q && category === 'All'}<section class="details"><h2>Your instance</h2><dl>{#each details as d}<div><dt>{d.label}</dt><dd>{d.value}</dd></div>{/each}</dl></section>{/if}
  <div class="articles">{#each articles as article (article.id)}
    <article><span class="category">{article.category}</span><h2><InterfaceIcon name="BookOpen" value={article.icon} fallback={BookOpen} size={28} />{article.title}</h2><p class="body">{article.body}</p>
      {#if article.links.length}<div class="links">{#each article.links as link}{#if safeGuideLink(link.url)}<a href={link.url} target="_blank" rel="noopener noreferrer">{link.label}</a>{/if}{/each}</div>{/if}
    </article>
  {/each}</div>
  {#if commands.length}<h2>Command reference</h2>{/if}
  {#each commands as g}<section><h3>{g.title}</h3><p>{g.blurb}</p><div class="commands">{#each g.cmds as c}<button class="command" onclick={() => copy(c.usage)} aria-label="Copy {c.usage}"><code>{c.usage}</code><span>{c.does}</span><small>{c.perm}{c.aliases?.length ? ' · Aliases: ' + c.aliases.join(', ') : ''}</small><span class="copy">{#if copied === c.usage}<InterfaceIcon name="Check" fallback={Check} size={16} />{:else}<InterfaceIcon name="Copy" fallback={Copy} size={16} />{/if}</span></button>{/each}</div></section>{/each}
  {#if !articles.length && !commands.length}<p role="status">No guide entries match your search.</p>{/if}
  {#if content.show_commands && !q && (category === 'All' || category === 'Commands')}<section><h2>Good to know</h2><ul>{#each COMMAND_TIPS as tip}<li>{tip}</li>{/each}</ul></section>{/if}
</div>
<style>
  .guide { padding: 1.5rem; max-width: 1200px; margin: auto; height: 100%; overflow: auto; box-sizing: border-box; color: var(--text); }
  h1,h2 { display: flex; align-items: center; gap: .6rem; } h1 { font-size: 1.7rem; } h2 { font-size: 1.2rem; } h3 { margin-bottom: .3rem; }
  p,li { line-height: 1.6; color: var(--muted); } header p { max-width: 800px; }
  .search { display: flex; align-items: center; gap: .6rem; padding: .6rem .9rem; border: 1px solid var(--line); border-radius: 8px; } input { width: 100%; background: transparent; border: 0; color: inherit; outline: none; font: inherit; }
  nav { display: flex; flex-wrap: wrap; gap: .5rem; margin: 1rem 0; } nav button { padding: .5rem .8rem; } .active { border-color: var(--accent); color: var(--state-info-text, color-mix(in srgb, var(--accent) 65%, var(--text))); }
  section,article { border: 1px solid var(--line); background: var(--surface, var(--bg-2)); border-radius: 10px; padding: 1rem; margin-bottom: 1rem; }
  .articles,.commands { display: grid; grid-template-columns: repeat(auto-fit,minmax(min(100%,340px),1fr)); gap: 1rem; } .body { white-space: pre-wrap; overflow-wrap: anywhere; } .category { font-size: .75rem; color: var(--state-info-text, color-mix(in srgb, var(--accent) 65%, var(--text))); }
  .links { display: flex; flex-wrap: wrap; gap: 1rem; } a,code { color: var(--state-info-text, color-mix(in srgb, var(--accent) 65%, var(--text))); } .command { position: relative; display: flex; flex-direction: column; align-items: flex-start; gap: .4rem; text-align: left; padding: 1rem 2rem 1rem 1rem; } code,small { overflow-wrap: anywhere; } small { color: var(--muted); } .copy { position: absolute; right: .5rem; top: .8rem; }
  dl { display: flex; flex-wrap: wrap; gap: 1rem 2rem; } dt { color: var(--muted); font-size: .8rem; } dd { margin: .3rem 0 0; overflow-wrap: anywhere; } li { margin-bottom: .4rem; }
</style>
