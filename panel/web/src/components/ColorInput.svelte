<script lang="ts">
  let { value = $bindable('#000000'), label }: { value: string; label: string } = $props();
</script>

<label class="color-input">
  <input type="color" bind:value />
  <span class="col">
    <span class="lbl">{label}</span>
    <input class="hex mono" type="text" bind:value maxlength="9" spellcheck="false" />
  </span>
</label>

<style>
  .color-input { display: flex; gap: 10px; align-items: center; background: var(--bg-2); border: 1px solid var(--line); padding: 8px; border-radius: 12px; }
  .col { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .lbl { font-size: 0.78rem; color: var(--muted); }
  .hex { border: none; background: none; padding: 0; font-size: 0.85rem; width: 90px; }
  .hex:focus { background: none; }
</style>
