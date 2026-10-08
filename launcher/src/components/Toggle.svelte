<script lang="ts">
  let { checked = $bindable(false), label, help, disabled = false, onchange }: { checked: boolean; label: string; help?: string; disabled?: boolean; onchange?: () => void } = $props();
</script>

<label class="toggle" class:disabled>
  <span class="text"><span class="label">{label}</span>{#if help}<span class="help">{help}</span>{/if}</span>
  <input type="checkbox" role="switch" bind:checked {disabled} onchange={() => onchange?.()} />
  <span class="track" aria-hidden="true"><span class="thumb"></span></span>
</label>

<style>
  .toggle { display: flex; align-items: center; gap: 1rem; cursor: pointer; padding: 0.35rem 0; }
  .disabled { opacity: 0.5; cursor: not-allowed; }
  .text { flex: 1; display: flex; flex-direction: column; gap: 0.15rem; }
  .label { font-weight: 560; font-size: 0.93rem; }
  input { position: absolute; opacity: 0; pointer-events: none; }
  .track { width: 2.75rem; height: 1.55rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 14%, transparent); position: relative; transition: background 0.2s; flex-shrink: 0; }
  .thumb { position: absolute; top: 0.18rem; left: 0.18rem; width: 1.19rem; height: 1.19rem; border-radius: 50%; background: color-mix(in srgb, var(--text) 80%, transparent); transition: transform 0.25s cubic-bezier(0.3, 1.4, 0.6, 1), background 0.2s; }
  input:checked + .track { background: var(--accent); }
  input:checked + .track .thumb { transform: translateX(1.2rem); background: white; }
  input:focus-visible + .track { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
