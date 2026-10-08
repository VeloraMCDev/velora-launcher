<script lang="ts">
  import { Search, Check, X, Sparkles, Minus, Plus } from '@lucide/svelte';
  import { ENCHANTS, ENCHANT_CATEGORIES, enchantColor, enchantName, roman, type EnchantInfo } from '../lib/mcData';

  let { value = $bindable({}) }: { value: Record<string, number> } = $props();

  let q = $state('');
  let cat = $state<string>('all');
  const shown = $derived(ENCHANTS.filter((e) => (cat === 'all' || e.cat === cat) && (!q || (e.name + e.blurb).toLowerCase().includes(q.toLowerCase()))));
  const applied = $derived(Object.entries(value));
  const info = (id: string): EnchantInfo | undefined => ENCHANTS.find((e) => e.id === id);

  function toggle(e: EnchantInfo) {
    if (e.id in value) { const { [e.id]: _, ...rest } = value; value = rest; }
    else value = { ...value, [e.id]: e.max };
  }
  // Low levels are what people pick most, so the slider is squared: the first half of the track covers levels 1–65.
  const toPos = (level: number) => Math.sqrt(Math.max(0, level - 1) / 254) * 1000;
  const fromPos = (pos: number) => 1 + Math.round(254 * (pos / 1000) ** 2);
  const setLevel = (id: string, n: number) => (value = { ...value, [id]: Math.max(1, Math.min(255, Math.round(n))) });
  function remove(id: string) { const { [id]: _, ...rest } = value; value = rest; }
</script>

<section class="ep">
  <header>
    <div><b>Enchantments</b> <small>levels go up to 255 — vanilla limits don't apply</small></div>
    <span class="count">{applied.length} applied</span>
  </header>

  {#if applied.length}
    <div class="applied">
      {#each applied as [id, level] (id)}
        {@const e = info(id)}
        {@const c = enchantColor(e?.cat ?? 'general')}
        {@const max = e?.max ?? 5}
        <div class="row" style:--c={c}>
          <div class="top">
            <span class="dot"></span>
            <b>{enchantName(id)}</b>
            <span class="lvl">{roman(level)}</span>
            {#if level > max}<span class="beyond" title="Vanilla maximum is {roman(max)}"><Sparkles size={11} /> beyond vanilla</span>{/if}
            <button class="x" aria-label="Remove {enchantName(id)}" onclick={() => remove(id)}><X size={14} /></button>
          </div>
          <div class="slide">
            <button class="step" aria-label="Lower" onclick={() => setLevel(id, level - 1)}><Minus size={13} /></button>
            <div class="track">
              <input class="fancy" type="range" min="0" max="1000" step="1" value={toPos(level)} style:--p="{toPos(level) / 10}%" aria-label="{enchantName(id)} level"
                oninput={(ev) => setLevel(id, fromPos(+ev.currentTarget.value))} />
              <span class="vmark" style:left="{toPos(max) / 10}%" title="Vanilla maximum"></span>
            </div>
            <button class="step" aria-label="Raise" onclick={() => setLevel(id, level + 1)}><Plus size={13} /></button>
            <input class="num" type="number" min="1" max="255" value={level} aria-label="Level number" onchange={(ev) => setLevel(id, +ev.currentTarget.value)} />
          </div>
          <div class="quick">
            <button onclick={() => setLevel(id, max)}>Vanilla max ({roman(max)})</button>
            <button onclick={() => setLevel(id, 10)}>X</button>
            <button onclick={() => setLevel(id, 50)}>50</button>
            <button onclick={() => setLevel(id, 255)}>255</button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="find">
    <label class="search"><Search size={14} /><input bind:value={q} placeholder="Search {ENCHANTS.length} enchantments…" /></label>
    <div class="cats">
      {#each ENCHANT_CATEGORIES as c}
        <button class="cat" class:on={cat === c.id} style:--c={c.color} onclick={() => (cat = c.id)}>{c.label}</button>
      {/each}
    </div>
  </div>
  <div class="grid">
    {#each shown as e (e.id)}
      {@const on = e.id in value}
      <button class="card" class:on style:--c={enchantColor(e.cat)} onclick={() => toggle(e)} aria-pressed={on}>
        <span class="bar"></span>
        <span class="txt"><b>{e.name}</b><small>{e.blurb}</small></span>
        <span class="max">{roman(e.max)}</span>
        <span class="tick">{#if on}<Check size={13} />{/if}</span>
      </button>
    {/each}
    {#if !shown.length}<p class="empty">No enchantment matches “{q}”.</p>{/if}
  </div>
</section>

<style>
  .ep { display: grid; gap: 12px; padding: 14px; border-radius: 14px; background: var(--surface); border: 1px solid var(--line); }
  header { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  header small { color: var(--muted); margin-left: 6px; }
  .count { font-size: 0.75rem; padding: 2px 9px; border-radius: 999px; background: var(--accent-soft); color: var(--accent-2); }
  .applied { display: grid; gap: 8px; }
  .row { --c: var(--accent); padding: 11px 12px; border-radius: 12px; background: linear-gradient(135deg, color-mix(in srgb, var(--c) 14%, var(--surface-2)), var(--surface-2)); border: 1px solid color-mix(in srgb, var(--c) 35%, transparent); display: grid; gap: 9px; }
  .top { display: flex; align-items: center; gap: 8px; }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--c); box-shadow: 0 0 10px var(--c); }
  .lvl { font-family: var(--mono); font-weight: 700; color: var(--c); }
  .beyond { display: inline-flex; align-items: center; gap: 4px; font-size: 0.68rem; padding: 2px 8px; border-radius: 999px; background: color-mix(in srgb, var(--c) 22%, transparent); color: var(--c); }
  .x { margin-left: auto; display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; border: 0; background: transparent; color: var(--muted); cursor: pointer; }
  .x:hover { background: var(--surface-3); color: var(--bad); }
  .slide { display: grid; grid-template-columns: auto 1fr auto 64px; gap: 8px; align-items: center; }
  .step { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 8px; border: 1px solid var(--line-strong); background: var(--surface-3); color: var(--text-2); cursor: pointer; }
  .step:hover { color: var(--text); border-color: var(--c); }
  .track { position: relative; padding: 6px 0; }
  .vmark { position: absolute; top: 2px; width: 2px; height: 22px; background: #fff7; border-radius: 2px; pointer-events: none; }
  .num { text-align: center; padding: 5px; font-family: var(--mono); }
  .quick { display: flex; gap: 6px; flex-wrap: wrap; }
  .quick button { padding: 3px 10px; border-radius: 999px; border: 1px solid var(--line-strong); background: transparent; color: var(--text-2); font-size: 0.72rem; cursor: pointer; }
  .quick button:hover { border-color: var(--c); color: var(--c); }
  .find { display: grid; gap: 8px; }
  .search { display: flex; align-items: center; gap: 8px; padding: 0 11px; border-radius: 10px; background: var(--surface-2); border: 1px solid var(--line); }
  .search input { border: 0; background: none; padding: 9px 0; outline: none; flex: 1; color: inherit; }
  .cats { display: flex; gap: 6px; flex-wrap: wrap; }
  .cat { padding: 4px 12px; border-radius: 999px; border: 1px solid var(--line); background: transparent; color: var(--text-2); font-size: 0.78rem; cursor: pointer; transition: all 0.12s; }
  .cat:hover { border-color: var(--c); }
  .cat.on { background: color-mix(in srgb, var(--c) 22%, transparent); border-color: var(--c); color: var(--c); font-weight: 600; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(215px, 1fr)); gap: 7px; max-height: 340px; overflow: auto; grid-auto-rows: min-content; align-content: start; padding: 2px; }
  .card { --c: var(--accent); position: relative; display: flex; align-items: center; gap: 10px; text-align: left; padding: 9px 10px 9px 14px; border-radius: 11px; border: 1px solid var(--line); background: var(--surface-2); color: inherit; cursor: pointer; transition: transform 0.12s, border-color 0.12s, background 0.12s; overflow: hidden; }
  .card .bar { position: absolute; left: 0; top: 0; bottom: 0; width: 4px; background: var(--c); opacity: 0.7; }
  .card:hover { transform: translateY(-1px); border-color: color-mix(in srgb, var(--c) 60%, transparent); }
  .card.on { background: color-mix(in srgb, var(--c) 16%, var(--surface-2)); border-color: var(--c); }
  .txt { display: grid; min-width: 0; flex: 1; }
  .txt small { color: var(--muted); font-size: 0.7rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .max { font-family: var(--mono); font-size: 0.7rem; color: var(--muted); }
  .tick { width: 18px; height: 18px; border-radius: 50%; display: grid; place-items: center; background: var(--c); color: #fff; opacity: 0; }
  .card.on .tick { opacity: 1; }
  .empty { color: var(--muted); grid-column: 1 / -1; }
</style>
