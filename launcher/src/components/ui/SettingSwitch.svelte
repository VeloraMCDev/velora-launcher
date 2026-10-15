<script lang="ts">
  let { checked = $bindable(false), label, help, disabled = false, onchange }: {
    checked: boolean; label: string; help?: string; disabled?: boolean; onchange?: () => void;
  } = $props();
  const identifier = $props.id();
</script>

<div class="setting-switch" class:disabled>
  <div class="text">
    <label for={identifier}>{label}</label>
    {#if help}<p id={`${identifier}-help`} class="help">{help}</p>{/if}
  </div>
  <div class="control">
    <input id={identifier} type="checkbox" role="switch" bind:checked {disabled}
      aria-describedby={help ? `${identifier}-help` : undefined} onchange={() => onchange?.()} />
    <span class="track" aria-hidden="true"><span class="thumb"></span></span>
  </div>
</div>

<style>
  .setting-switch { display: flex; align-items: center; gap: 1rem; padding: 0.35rem 0; }
  .text { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  label { font-weight: 560; font-size: 0.93rem; cursor: pointer; }
  .help { margin-top: 0.15rem; }
  .disabled { opacity: var(--state-disabled-opacity); }
  .control { position: relative; flex-shrink: 0; width: 2.75rem; height: 2.75rem; display: grid; place-items: center; }
  input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; z-index: 1; }
  input:disabled { cursor: not-allowed; }
  .track { width: 2.75rem; height: 1.55rem; border-radius: 99rem; background: var(--state-neutral-surface); border: 1px solid var(--line-strong); position: relative; }
  .thumb { position: absolute; top: 0.15rem; left: 0.15rem; width: 1.19rem; height: 1.19rem; border-radius: 50%; background: var(--text); transition: transform 0.2s var(--ease); }
  input:checked + .track { background: var(--accent); }
  input:checked + .track .thumb { transform: translateX(1.2rem); background: white; }
  input:focus-visible + .track { outline: 2px solid var(--state-focus-ring); outline-offset: 3px; }
</style>
