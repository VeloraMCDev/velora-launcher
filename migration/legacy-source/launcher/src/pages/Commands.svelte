<script lang="ts">
  import { Terminal, Search, Copy, Check } from '@lucide/svelte';

  import { COMMAND_GROUPS as groups, COMMAND_TIPS as tips, type Cmd } from '@scopenet/commands';

  let query = $state('');
  let copied = $state('');
  const q = $derived(query.trim().toLowerCase());
  const shown = $derived(
    groups
      .map((g) => ({ ...g, cmds: g.cmds.filter((c) => !q || `${c.usage} ${c.does} ${c.perm} ${(c.aliases ?? []).join(' ')}`.toLowerCase().includes(q)) }))
      .filter((g) => g.cmds.length)
  );

  async function copy(c: Cmd) {
    const text = c.usage.split(' · ')[0];
    try { await navigator.clipboard.writeText(text); } catch { /* clipboard may be unavailable */ }
    copied = c.usage;
    setTimeout(() => (copied = ''), 1200);
  }
</script>

<div class="page">
  <div class="head">
    <div>
      <h1><Terminal size={26} class="ic" /> Command guide</h1>
      <p class="lead">Everything you can type in-game on SCOPENET servers. Click a command to copy it.</p>
    </div>
    <label class="search"><Search size={15} /><input bind:value={query} placeholder="Search commands" aria-label="Search commands" /></label>
  </div>

  {#each shown as g (g.title)}
    <section class="group">
      <div class="gh"><h2>{g.title}</h2><span class="muted tiny">{g.blurb}</span></div>
      <div class="list">
        {#each g.cmds as c (c.usage)}
          <button class="cmd glass" onclick={() => copy(c)} title="Copy command">
            <code class="usage">{c.usage}</code>
            <span class="does">{c.does}</span>
            <span class="meta">
              {#each c.aliases ?? [] as a}<span class="alias">/{a.replace(/^\//, '')}</span>{/each}
              <span class="perm" title="LuckPerms permission node">{c.perm}</span>
            </span>
            <span class="copy">{#if copied === c.usage}<Check size={14} />{:else}<Copy size={14} />{/if}</span>
          </button>
        {/each}
      </div>
    </section>
  {:else}
    <p class="muted">No commands match “{query}”.</p>
  {/each}

  <section class="tips glass">
    <h2>Good to know</h2>
    <ul>{#each tips as t}<li>{t}</li>{/each}</ul>
  </section>
</div>

<style>
  .page { height: 100%; display: flex; flex-direction: column; overflow-y: auto; min-height: 0; padding: 1.6rem 2.2rem 2.5rem; gap: 1.2rem; }
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 1rem; flex-wrap: wrap; border-bottom: 1px solid var(--line); padding-bottom: 1rem; }
  h1 { font-size: 1.5rem; font-weight: 700; display: flex; align-items: center; gap: 0.6rem; margin: 0; }
  :global(.ic) { color: var(--accent); }
  .lead { color: var(--muted); font-size: 0.88rem; margin: 0.2rem 0 0; }
  .search { display: flex; align-items: center; gap: 0.5rem; padding: 0.4rem 0.75rem; border-radius: var(--radius-sm); border: 1px solid var(--line); background: color-mix(in srgb, var(--surface) 80%, transparent); min-width: 14rem; }
  .search input { background: transparent; border: none; outline: none; color: var(--text); flex: 1; font-size: 0.85rem; }
  .group { display: flex; flex-direction: column; gap: 0.6rem; }
  .gh { display: flex; align-items: baseline; gap: 0.7rem; flex-wrap: wrap; }
  .gh h2 { font-size: 1.05rem; margin: 0; }
  .list { display: grid; grid-template-columns: repeat(auto-fill, minmax(21rem, 1fr)); gap: 0.6rem; }
  .cmd { position: relative; display: flex; flex-direction: column; gap: 0.3rem; text-align: left; padding: 0.75rem 2.2rem 0.75rem 0.9rem; border-radius: var(--radius-sm); border: 1px solid var(--line); transition: border-color 0.15s, transform 0.15s; }
  .cmd:hover { border-color: var(--accent); transform: translateY(-1px); }
  .usage { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 0.88rem; font-weight: 650; color: var(--accent); }
  .does { font-size: 0.83rem; color: var(--text); line-height: 1.35; }
  .meta { display: flex; flex-wrap: wrap; gap: 0.35rem; align-items: center; }
  .alias { font-size: 0.68rem; padding: 0.05rem 0.45rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 8%, transparent); color: var(--muted); }
  .perm { font-family: ui-monospace, monospace; font-size: 0.66rem; color: var(--muted); opacity: 0.85; }
  .copy { position: absolute; top: 0.7rem; right: 0.75rem; color: var(--muted); }
  .tips { padding: 1rem 1.2rem; border-radius: var(--radius-sm); border: 1px solid var(--line); }
  .tips h2 { font-size: 0.95rem; margin: 0 0 0.4rem; }
  .tips ul { margin: 0; padding-left: 1.1rem; display: flex; flex-direction: column; gap: 0.3rem; font-size: 0.83rem; color: var(--muted); }
</style>
