<script lang="ts">
  import { Plus, X } from '@lucide/svelte';
  import ColorInput from './ColorInput.svelte';
  import Toggle from './Toggle.svelte';
  import type { EmbedStyle } from '../lib/types';

  let { style = $bindable(), placeholders = [], toggleLabel = 'Send this message' }: { style: EmbedStyle; placeholders?: string[]; toggleLabel?: string } = $props();

  let last: HTMLTextAreaElement | HTMLInputElement | null = null;
  const remember = (e: Event) => { last = e.currentTarget as HTMLInputElement; };
  // Clicking a placeholder inserts it at the cursor of the field you were typing in.
  function insert(name: string) {
    const token = `{${name}}`;
    if (!last) { style.description = (style.description ?? '') + token; return; }
    const el = last, start = el.selectionStart ?? el.value.length, end = el.selectionEnd ?? start;
    el.value = el.value.slice(0, start) + token + el.value.slice(end);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.focus();
    el.setSelectionRange(start + token.length, start + token.length);
  }
  const addField = () => (style.fields = [...(style.fields ?? []), { name: '', value: '', inline: true }]);
  const removeField = (i: number) => (style.fields = style.fields.filter((_, n) => n !== i));
</script>

<div class="editor">
  <Toggle bind:checked={style.enabled} label={toggleLabel} />

  {#if placeholders.length}
    <div class="ph" aria-label="Placeholders">
      <span class="lbl">Placeholders <small>click to insert</small></span>
      <div>{#each placeholders as p}<button type="button" class="chip" onclick={() => insert(p)}>{`{${p}}`}</button>{/each}</div>
    </div>
  {/if}

  <div class="grid">
    <label class="field wide">Message text <small>above the box, mentions work here (&lt;@&amp;roleid&gt;)</small><input bind:value={style.content} onfocus={remember} placeholder="Optional" /></label>
    <label class="field wide">Title<input bind:value={style.title} onfocus={remember} maxlength="256" /></label>
    <label class="field wide">Description<textarea rows="4" bind:value={style.description} onfocus={remember} maxlength="4000"></textarea></label>
    <ColorInput bind:value={style.color} label="Colour" />
    <label class="field">Title link<input bind:value={style.url} onfocus={remember} placeholder="https://…" /></label>
    <label class="field">Small picture (right)<input bind:value={style.thumbnail} onfocus={remember} placeholder="https://… or {'{avatar}'}" /></label>
    <label class="field">Big picture (bottom)<input bind:value={style.image} onfocus={remember} placeholder="https://…" /></label>
    <label class="field">Author name<input bind:value={style.author_name} onfocus={remember} /></label>
    <label class="field">Author icon<input bind:value={style.author_icon} onfocus={remember} placeholder="https://…" /></label>
    <label class="field">Footer<input bind:value={style.footer} onfocus={remember} /></label>
    <label class="field">Footer icon<input bind:value={style.footer_icon} onfocus={remember} placeholder="https://…" /></label>
    <label class="field">Sender name<input bind:value={style.username} onfocus={remember} placeholder="Velora" /></label>
    <label class="field">Sender picture<input bind:value={style.avatar_url} onfocus={remember} placeholder="https://…" /></label>
  </div>
  <Toggle bind:checked={style.timestamp} label="Show the time" />

  <div class="fields">
    <div class="fhead"><span class="lbl">Fields</span><button type="button" class="ghost sm" onclick={addField}><Plus size={14} /> Add field</button></div>
    {#each style.fields ?? [] as f, i}
      <div class="frow">
        <input bind:value={f.name} onfocus={remember} placeholder="Name" />
        <input bind:value={f.value} onfocus={remember} placeholder="Value" />
        <label class="inl"><input type="checkbox" bind:checked={f.inline} /> Side by side</label>
        <button type="button" class="ghost icon" aria-label="Remove field" onclick={() => removeField(i)}><X size={14} /></button>
      </div>
    {/each}
  </div>
</div>

<style>
  .editor { display: flex; flex-direction: column; gap: 14px; }
  .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; }
  .field.wide { grid-column: 1 / -1; }
  .field small, .ph small { color: var(--muted); font-weight: 400; }
  .ph { display: flex; flex-direction: column; gap: 6px; }
  .ph > div { display: flex; flex-wrap: wrap; gap: 6px; }
  .lbl { font-size: 0.82rem; font-weight: 600; }
  .chip { padding: 3px 9px; border-radius: 999px; font: 500 0.76rem ui-monospace, monospace; background: color-mix(in srgb, var(--accent) 14%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); }
  .fhead { display: flex; align-items: center; justify-content: space-between; }
  .fields { display: flex; flex-direction: column; gap: 8px; }
  .frow { display: grid; grid-template-columns: 1fr 1.6fr auto auto; gap: 8px; align-items: center; }
  .inl { display: flex; align-items: center; gap: 6px; font-size: 0.78rem; white-space: nowrap; }
  @media (max-width: 760px) { .grid { grid-template-columns: 1fr; } .frow { grid-template-columns: 1fr; } }
</style>
