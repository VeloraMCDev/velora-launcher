<script lang="ts">
  let { checked = $bindable(false), label, help, disabled = false, onchange }: { checked: boolean; label: string; help?: string; disabled?: boolean; onchange?: () => void } = $props();
</script>

<label class="toggle" class:disabled>
  <span class="text">
    <span class="label">{label}</span>
    {#if help}<span class="help">{help}</span>{/if}
  </span>
  <input type="checkbox" role="switch" bind:checked {disabled} {onchange} />
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
</label>

<style>
  .toggle { display: flex; align-items: center; gap: 16px; cursor: pointer; padding: 4px 0; }
  .toggle.disabled { opacity: 0.5; cursor: not-allowed; }
  .text { flex: 1; display: flex; flex-direction: column; gap: 2px; }
  .label { font-weight: 550; font-size: 0.92rem; }
  .help { color: var(--muted); font-size: 0.8rem; }
  input { position: absolute; opacity: 0; pointer-events: none; }
  .track { width: 42px; height: 24px; border-radius: 99px; background: var(--surface-3); position: relative; transition: background 0.2s; flex-shrink: 0; border: 1px solid var(--line-strong); }
  .thumb { position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: #cfd3f5; transition: transform 0.2s cubic-bezier(0.3, 1.4, 0.6, 1), background 0.2s; }
  input:checked + .track { background: var(--accent); border-color: transparent; }
  input:checked + .track .thumb { transform: translateX(18px); background: white; }
  input:focus-visible + .track { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
