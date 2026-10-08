<script lang="ts">
  import { Swords, Zap, Heart, Wind, Shield, ShieldCheck, Anchor, Move, Clover, X, Plus } from '@lucide/svelte';
  import { ATTRIBUTES, OPERATIONS, SLOTS, attrInfo, type AttrInfo } from '../lib/mcData';

  type Attr = { attribute: string; amount: number; operation: string; slot: string };
  let { value = $bindable([]) }: { value: Attr[] } = $props();

  const ICONS: Record<string, typeof Swords> = { sword: Swords, zap: Zap, heart: Heart, wind: Wind, shield: Shield, 'shield-check': ShieldCheck, anchor: Anchor, move: Move, clover: Clover };

  function add(a: AttrInfo) {
    if (value.length >= 12) return;
    value = [...value, { attribute: a.id, amount: a.start, operation: 'add', slot: 'mainhand' }];
  }
  function patch(i: number, p: Partial<Attr>) { value = value.map((x, j) => (j === i ? { ...x, ...p } : x)); }
  const remove = (i: number) => (value = value.filter((_, j) => j !== i));

  function range(a: AttrInfo | undefined, op: string) {
    if (op === 'add') return { min: a?.min ?? -100, max: a?.max ?? 100, step: a?.step ?? 0.5 };
    return { min: -1, max: 3, step: 0.05 };
  }
  function shown(a: AttrInfo | undefined, amount: number, op: string) {
    if (op !== 'add') return `${amount >= 0 ? '+' : ''}${Math.round(amount * 100)}%`;
    return `${amount >= 0 ? '+' : ''}${+amount.toFixed(3)}${a?.unit ? ' ' + a.unit : ''}`;
  }
  function switchOp(i: number, op: string) {
    const a = attrInfo(value[i].attribute);
    // keep the number meaningful when the scale changes
    patch(i, { operation: op, amount: op === 'add' ? (a?.start ?? 1) : 0.25 });
  }
</script>

<section class="ap">
  <header><div><b>Attribute bonuses</b> <small>the really overpowered part</small></div><span class="count">{value.length} / 12</span></header>

  {#if value.length}
    <div class="applied">
      {#each value as a, i (i)}
        {@const info = attrInfo(a.attribute)}
        {@const r = range(info, a.operation)}
        {@const Icon = ICONS[info?.icon ?? 'zap'] ?? Zap}
        <div class="row" style:--c={info?.color ?? '#8b88f8'}>
          <div class="top">
            <span class="ico"><Icon size={16} /></span>
            <span class="nm"><b>{info?.name ?? a.attribute}</b><small>{info?.blurb ?? ''}</small></span>
            <span class="val">{shown(info, a.amount, a.operation)}</span>
            <button class="x" aria-label="Remove bonus" onclick={() => remove(i)}><X size={14} /></button>
          </div>
          <input class="fancy" type="range" min={r.min} max={r.max} step={r.step} value={a.amount} style:--p="{((a.amount - r.min) / (r.max - r.min)) * 100}%"
            aria-label="Amount" oninput={(e) => patch(i, { amount: +e.currentTarget.value })} />
          <div class="opts">
            <div class="seg" role="radiogroup" aria-label="How the number applies">
              {#each OPERATIONS as o}<button role="radio" aria-checked={a.operation === o.id} class:on={a.operation === o.id} title={o.hint} onclick={() => switchOp(i, o.id)}>{o.label}</button>{/each}
            </div>
          </div>
          <div class="slots" role="radiogroup" aria-label="Active in slot">
            {#each SLOTS as s}<button class="pill" role="radio" aria-checked={a.slot === s.id} class:on={a.slot === s.id} onclick={() => patch(i, { slot: s.id })}>{s.label}</button>{/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="grid">
    {#each ATTRIBUTES as at (at.id)}
      {@const Icon = ICONS[at.icon] ?? Zap}
      <button class="card" style:--c={at.color} onclick={() => add(at)} disabled={value.length >= 12}>
        <span class="ico"><Icon size={16} /></span>
        <span class="txt"><b>{at.name}</b><small>{at.blurb}</small></span>
        <span class="plus"><Plus size={14} /></span>
      </button>
    {/each}
  </div>
</section>

<style>
  .ap { display: grid; gap: 12px; padding: 14px; border-radius: 14px; background: var(--surface); border: 1px solid var(--line); }
  header { display: flex; align-items: center; justify-content: space-between; }
  header small { color: var(--muted); margin-left: 6px; }
  .count { font-size: 0.75rem; padding: 2px 9px; border-radius: 999px; background: var(--accent-soft); color: var(--accent-2); }
  .applied { display: grid; gap: 9px; }
  .row { --c: var(--accent); display: grid; gap: 10px; padding: 12px; border-radius: 12px; background: linear-gradient(135deg, color-mix(in srgb, var(--c) 13%, var(--surface-2)), var(--surface-2)); border: 1px solid color-mix(in srgb, var(--c) 35%, transparent); }
  .top { display: flex; align-items: center; gap: 10px; }
  .ico { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 9px; background: color-mix(in srgb, var(--c) 22%, transparent); color: var(--c); flex-shrink: 0; }
  .nm { display: grid; min-width: 0; flex: 1; }
  .nm small { color: var(--muted); font-size: 0.72rem; }
  .val { font-family: var(--mono); font-weight: 700; color: var(--c); font-size: 1rem; white-space: nowrap; }
  .x { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; border: 0; background: transparent; color: var(--muted); cursor: pointer; }
  .x:hover { background: var(--surface-3); color: var(--bad); }
  .slots { display: flex; gap: 5px; flex-wrap: wrap; }
  .pill { padding: 4px 11px; border-radius: 999px; border: 1px solid var(--line); background: transparent; color: var(--text-2); font-size: 0.75rem; cursor: pointer; transition: all 0.12s; }
  .pill:hover { border-color: var(--c); }
  .pill.on { background: color-mix(in srgb, var(--c) 24%, transparent); border-color: var(--c); color: var(--text); font-weight: 600; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 7px; }
  .card { --c: var(--accent); display: flex; align-items: center; gap: 10px; text-align: left; padding: 9px 10px; border-radius: 11px; border: 1px solid var(--line); background: var(--surface-2); color: inherit; cursor: pointer; transition: transform 0.12s, border-color 0.12s; }
  .card:hover:not(:disabled) { transform: translateY(-1px); border-color: color-mix(in srgb, var(--c) 65%, transparent); }
  .card:disabled { opacity: 0.4; cursor: not-allowed; }
  .txt { display: grid; min-width: 0; flex: 1; }
  .txt small { color: var(--muted); font-size: 0.7rem; }
  .plus { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 50%; background: color-mix(in srgb, var(--c) 25%, transparent); color: var(--c); }
</style>
