<script lang="ts">
  import { Plus, Trash2 } from '@lucide/svelte';
  // Rows of {label, value, weight, color}. `valueLabel` says what the value means (a multiplier or an amount).
  let { segments = $bindable(), valueLabel, step = 0.5, odds = [] }: { segments: { label: string; value: number; weight: number; color: string }[]; valueLabel: string; step?: number; odds?: number[] } = $props();
  const PALETTE = ['#475569', '#64748b', '#0ea5e9', '#22c55e', '#eab308', '#f97316', '#ef4444', '#a855f7', '#ec4899', '#14b8a6'];
  function add() { if (segments.length < 16) segments.push({ label: 'New', value: 1, weight: 5, color: PALETTE[segments.length % PALETTE.length] }); }
</script>

<div class="seg-ed">
  <div class="head"><span></span><span>Label</span><span>{valueLabel}</span><span>Weight</span><span>Chance</span><span></span></div>
  {#each segments as s, i}
    <div class="row">
      <input type="color" bind:value={s.color} aria-label="Colour" />
      <input bind:value={s.label} maxlength="12" aria-label="Label" />
      <input type="number" min="0" {step} bind:value={s.value} aria-label={valueLabel} />
      <input type="number" min="0.01" step="0.1" bind:value={s.weight} aria-label="Weight" />
      <span class="odds">{odds[i] != null ? (odds[i] * 100).toFixed(odds[i] < 0.01 ? 2 : 1) + '%' : '—'}</span>
      <button type="button" class="ghost icon" onclick={() => segments.splice(i, 1)} disabled={segments.length <= 2} aria-label="Remove"><Trash2 size={14} /></button>
    </div>
  {/each}
  <button type="button" class="ghost add" onclick={add} disabled={segments.length >= 16}><Plus size={14} /> Add segment</button>
</div>

<style>
  .seg-ed { display: flex; flex-direction: column; gap: 6px; }
  .head, .row { display: grid; grid-template-columns: 34px 1fr 90px 80px 56px 34px; gap: 8px; align-items: center; }
  .head { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); padding: 0 2px; }
  input[type='color'] { width: 34px; height: 32px; padding: 2px; border-radius: 8px; }
  input { min-width: 0; }
  .odds { font-variant-numeric: tabular-nums; color: var(--text-2); font-size: 0.85rem; text-align: right; }
  .icon { padding: 6px; }
  .add { align-self: flex-start; display: inline-flex; gap: 6px; align-items: center; }
</style>
