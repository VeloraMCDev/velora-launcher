<script lang="ts">
  import { Check, Copy, Search, Terminal } from '@lucide/svelte';
  import { COMMAND_GROUPS, COMMAND_TIPS, type Cmd } from '@scopenet/commands';

  let query = $state((() => { try { const q = sessionStorage.getItem('scopenet.play.cmdq') ?? ''; sessionStorage.removeItem('scopenet.play.cmdq'); return q; } catch { return ''; } })());
  let group = $state('all');
  let copied = $state('');
  const q = $derived(query.trim().toLowerCase());
  const shown = $derived(
    COMMAND_GROUPS
      .filter((g) => group === 'all' || g.title === group)
      .map((g) => ({ ...g, cmds: g.cmds.filter((c) => !q || `${c.usage} ${c.does} ${c.perm} ${(c.aliases ?? []).join(' ')}`.toLowerCase().includes(q)) }))
      .filter((g) => g.cmds.length)
  );
  const total = $derived(shown.reduce((n, g) => n + g.cmds.length, 0));

  async function copy(c: Cmd) {
    const text = c.usage.split(' · ')[0].replace(/\s[<[].*$/, '');
    try { await navigator.clipboard.writeText(text); } catch { /* clipboard may be unavailable */ }
    copied = c.usage;
    setTimeout(() => (copied = ''), 1200);
  }
</script>

<div class="pl-page">
  <div class="pl-head">
    <div><h1><Terminal size={26} /> Command guide</h1><p>Everything you can type in game. Tap a command to copy it, then paste it in chat.</p></div>
  </div>

  <div class="bar">
    <label class="pl-search"><Search size={16} /><input class="pl-input" bind:value={query} placeholder="Search {COMMAND_GROUPS.reduce((n, g) => n + g.cmds.length, 0)} commands" aria-label="Search commands" /></label>
    <div class="pl-tabs" role="tablist">
      <button class:on={group === 'all'} onclick={() => (group = 'all')}>All</button>
      {#each COMMAND_GROUPS as g}<button class:on={group === g.title} onclick={() => (group = g.title)}>{g.title}</button>{/each}
    </div>
  </div>

  {#each shown as g (g.title)}
    <section class="group">
      <div class="gh"><h2>{g.title}</h2><span>{g.blurb}</span></div>
      <div class="pl-grid" style="--min: 340px">
        {#each g.cmds as c, i (c.usage)}
          <button class="pl-card tight cmd" style="animation-delay:{Math.min(i, 12) * 22}ms" onclick={() => copy(c)} aria-label="Copy {c.usage}">
            <code class="usage">{c.usage}</code>
            <span class="does">{c.does}</span>
            <span class="meta">
              {#each c.aliases ?? [] as a}<span class="pl-chip">/{a.replace(/^\//, '')}</span>{/each}
              <span class="perm">{c.perm}</span>
            </span>
            <span class="copy">{#if copied === c.usage}<Check size={15} />{:else}<Copy size={15} />{/if}</span>
          </button>
        {/each}
      </div>
    </section>
  {:else}
    <div class="pl-card"><div class="pl-empty"><Search size={28} /><b>No commands match “{query}”</b><span>Try a shorter word, like “guild” or “home”.</span></div></div>
  {/each}
  {#if q}<p class="count">{total} match{total === 1 ? '' : 'es'}</p>{/if}

  <section class="pl-card tips">
    <h2>Good to know</h2>
    <ul>{#each COMMAND_TIPS as t}<li>{t}</li>{/each}</ul>
  </section>
</div>

<style>
  .bar { display: flex; flex-direction: column; gap: 12px; margin-bottom: 20px; position: sticky; top: calc(var(--pl-top) + env(safe-area-inset-top)); z-index: 10; padding: 6px 0 4px; background: linear-gradient(var(--bg) 60%, transparent); }
  .group { margin-bottom: 26px; }
  .gh { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; margin: 0 2px 12px; }
  .gh h2 { font-size: 1.12rem; }
  .gh span { color: var(--muted); font-size: 0.84rem; }
  .cmd { position: relative; display: flex !important; flex-direction: column; gap: 6px; padding: 14px 40px 14px 15px !important; animation: up 0.35s var(--ease, ease) backwards; }
  @keyframes up { from { opacity: 0; transform: translateY(8px); } }
  .usage { font-family: var(--mono); font-size: 0.88rem; font-weight: 650; color: var(--accent-2); overflow-wrap: anywhere; }
  .does { font-size: 0.86rem; line-height: 1.4; color: var(--text); }
  .meta { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .perm { font-family: var(--mono); font-size: 0.66rem; color: var(--muted); overflow-wrap: anywhere; }
  .copy { position: absolute; top: 14px; right: 14px; color: var(--muted); }
  .cmd:active .copy { color: var(--good); }
  .count { color: var(--muted); font-size: 0.82rem; text-align: center; margin: 4px 0 12px; }
  .tips h2 { margin-bottom: 8px; }
  .tips ul { margin: 0; padding-left: 1.1rem; display: flex; flex-direction: column; gap: 8px; color: var(--text-2); font-size: 0.88rem; line-height: 1.45; }
  @media (prefers-reduced-motion: reduce) { .cmd { animation: none; } }
</style>
