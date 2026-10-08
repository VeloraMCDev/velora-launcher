<script lang="ts">
  import { Minus, Plus } from '@lucide/svelte';
  import { money } from './casino';
  import { betPresets, hotkeys, stepBet } from './hotkeys';
  let { value = $bindable(10), min, max, balance, disabled = false, label = 'Bet' }: { value: number; min: number; max: number; balance: number | null; disabled?: boolean; label?: string } = $props();
  const cap = $derived(Math.max(min, Math.min(max, balance ?? max)));
  const clamp = (v: number) => Math.round(Math.min(cap, Math.max(min, v)) * 100) / 100;
  const set = (v: number) => { if (!disabled) value = clamp(v); };
  const presets = $derived(betPresets(min, cap));
  const short = (v: number) => (v >= 1_000_000 ? `$${+(v / 1_000_000).toFixed(1)}M` : v >= 1000 ? `$${+(v / 1000).toFixed(1)}K` : `$${v}`);
  // The slider moves on a curve so small bets are easy to pick and large ones still reachable.
  const toSlider = (v: number) => (cap > min ? Math.sqrt((Math.min(v, cap) - min) / (cap - min)) * 100 : 100);
  const fromSlider = (p: number) => clamp(min + Math.pow(p / 100, 2) * (cap - min));

  // Same keys at every table: arrows or + / - step the bet, [ halves it, ] doubles it, M is the maximum, N the minimum, 1 to 5 the chips.
  hotkeys(() => ({
    arrowup: () => set(stepBet(value, 1, min, cap)), '+': () => set(stepBet(value, 1, min, cap)), '=': () => set(stepBet(value, 1, min, cap)),
    arrowdown: () => set(stepBet(value, -1, min, cap)), '-': () => set(stepBet(value, -1, min, cap)), _: () => set(stepBet(value, -1, min, cap)),
    '[': () => set(value / 2), ']': () => set(value * 2), m: () => set(cap), n: () => set(min),
    ...Object.fromEntries(presets.slice(0, 5).map((p, i) => [String(i + 1), () => set(p)])),
  }));
</script>

<div class="bet" class:off={disabled}>
  <div class="top"><span>{label}</span><small>Balance {money(balance)}</small></div>
  <div class="amount">
    <button type="button" class="step" {disabled} onclick={() => set(stepBet(value, -1, min, cap))} aria-label="Lower the bet" title="Lower (↓)"><Minus size={16} /></button>
    <label class="field">
      <span class="sym">$</span>
      <input type="number" {min} max={cap} step="1" {disabled} bind:value onchange={() => (value = clamp(Number(value) || min))} aria-label={label} />
    </label>
    <button type="button" class="step" {disabled} onclick={() => set(stepBet(value, 1, min, cap))} aria-label="Raise the bet" title="Raise (↑)"><Plus size={16} /></button>
  </div>
  {#if presets.length}
    <div class="chips" role="group" aria-label="Common bets" style="--n:{presets.length}">
      {#each presets as p, i (p)}
        <button type="button" class="chip" class:on={Math.abs(value - p) < 0.005} {disabled} onclick={() => set(p)} title="Press {i + 1}"><b>{short(p)}</b><kbd>{i + 1}</kbd></button>
      {/each}
    </div>
  {/if}
  <div class="quick">
    <button type="button" {disabled} onclick={() => set(min)} title="Minimum (N)">Min</button>
    <button type="button" {disabled} onclick={() => set(value / 2)} title="Halve ([)">½</button>
    <button type="button" {disabled} onclick={() => set(value * 2)} title="Double (])">2×</button>
    <button type="button" {disabled} onclick={() => set(cap)} title="Maximum (M)">Max</button>
  </div>
  <input class="fancy" type="range" min="0" max="100" step="0.5" {disabled} value={toSlider(value)} oninput={(e) => (value = fromSlider(Number(e.currentTarget.value)))} style="--p:{toSlider(value)}%" aria-label="{label} amount" />
  <div class="range"><span>{money(min)}</span><span>{money(cap)}</span></div>
</div>

<style>
  .bet { display: flex; flex-direction: column; gap: 0.6rem; }
  .bet.off { opacity: 0.6; pointer-events: none; }
  .top { display: flex; justify-content: space-between; align-items: baseline; font-size: 0.78rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  .top small { font-weight: 600; letter-spacing: 0; text-transform: none; }
  .amount { display: grid; grid-template-columns: 2.6rem 1fr 2.6rem; gap: 0.4rem; align-items: stretch; }
  .step { padding: 0; border-radius: 0.75rem; display: grid; place-items: center; }
  .field { position: relative; display: block; }
  .field .sym { position: absolute; left: 0.9rem; top: 50%; transform: translateY(-50%); color: var(--gold, #f5c451); font-weight: 800; font-size: 1.1rem; }
  .field input { width: 100%; height: 100%; padding: 0.55rem 0.5rem 0.55rem 2rem; font-size: 1.45rem; font-weight: 800; text-align: center; letter-spacing: -0.01em; font-variant-numeric: tabular-nums; border-radius: 0.75rem; background: color-mix(in srgb, #000 28%, var(--surface, #16181e)); border-color: color-mix(in srgb, var(--gold, #f5c451) 30%, var(--line-strong)); -moz-appearance: textfield; appearance: textfield; }
  .field input::-webkit-outer-spin-button, .field input::-webkit-inner-spin-button { appearance: none; margin: 0; }
  .chips { display: grid; grid-template-columns: repeat(var(--n, 5), minmax(0, 1fr)); gap: 0.35rem; }
  .chip { position: relative; padding: 0.6rem 0 0.55rem; border-radius: 99rem; min-height: 0; font-size: 0.78rem; font-variant-numeric: tabular-nums; background: radial-gradient(circle at 50% 22%, color-mix(in srgb, var(--gold, #f5c451) 22%, transparent), transparent 70%), color-mix(in srgb, #000 22%, var(--surface-2, #1c1e25)); border: 2px dashed color-mix(in srgb, var(--gold, #f5c451) 45%, transparent); }
  .chip b { font-weight: 800; }
  .chip kbd { position: absolute; top: -0.35rem; right: 0.2rem; font: 700 0.6rem/1 var(--mono, monospace); padding: 0.14rem 0.28rem; border-radius: 0.3rem; background: var(--surface-3, #25272f); color: var(--muted); border: 1px solid var(--line-strong); }
  .chip:hover:not(:disabled) { border-color: var(--gold, #f5c451); transform: translateY(-2px) rotate(-2deg); }
  .chip.on { background: radial-gradient(circle at 50% 22%, color-mix(in srgb, var(--gold, #f5c451) 55%, transparent), transparent 75%), color-mix(in srgb, #000 10%, var(--surface-2, #1c1e25)); border: 2px solid var(--gold, #f5c451); box-shadow: 0 0 18px -4px var(--gold, #f5c451), inset 0 0 12px color-mix(in srgb, var(--gold, #f5c451) 25%, transparent); color: #fff; }
  .quick { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.35rem; }
  .quick button { padding: 0.4rem 0; font-size: 0.8rem; min-height: 0; border-radius: 0.6rem; }
  .range { display: flex; justify-content: space-between; font-size: 0.7rem; color: var(--muted); margin-top: -0.25rem; }
  input.fancy { height: 1.2rem; background: linear-gradient(90deg, var(--gold, var(--accent)) var(--p), color-mix(in srgb, var(--text) 12%, transparent) var(--p)) center / 100% 0.3rem no-repeat; }
</style>
