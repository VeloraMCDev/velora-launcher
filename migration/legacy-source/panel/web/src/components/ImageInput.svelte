<script lang="ts">
  import { ImageUp, Trash2, LoaderCircle } from '@lucide/svelte';
  import { uploadMedia } from '../lib/api';
  import { toastError } from '../lib/toast.svelte';

  let { value = $bindable(null), label, accept = 'image/*', help, video = false, previewValue = undefined }: {
    value: string | null; label: string; accept?: string; help?: string; video?: boolean; previewValue?: string | null;
  } = $props();

  let busy = $state(false);
  let input: HTMLInputElement;
  const isVideo = $derived(!!value && /\.(mp4|webm)$/i.test(value));
  const preview = $derived(previewValue === undefined ? value : previewValue);

  async function pick(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    busy = true;
    try {
      value = await uploadMedia(file);
    } catch (err) {
      toastError(err);
    } finally {
      busy = false;
      input.value = '';
    }
  }
</script>

<div class="field image-input">
  <span class="lbl">{label}</span>
  <div class="row">
    <button type="button" class="preview" onclick={() => input.click()} aria-label="Upload {label}">
      {#if busy}
        <LoaderCircle class="spin" size={20} />
      {:else if value && isVideo}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video src={value} muted autoplay loop playsinline></video>
      {:else if preview}
        <img src={preview} alt="" />
      {:else}
        <ImageUp size={20} />
      {/if}
    </button>
    <div class="col grow">
      <input type="text" placeholder="https://… or upload" bind:value />
      {#if help}<span class="help">{help}</span>{/if}
    </div>
    {#if value}
      <button type="button" class="ghost icon" onclick={() => (value = null)} aria-label="Remove"><Trash2 size={16} /></button>
    {/if}
  </div>
  <input bind:this={input} type="file" accept={video ? 'image/*,video/mp4,video/webm' : accept} hidden onchange={pick} />
</div>

<style>
  .image-input { display: flex; flex-direction: column; gap: 7px; }
  .lbl { font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .help { color: var(--muted); font-size: 0.78rem; }
  .grow { flex: 1; gap: 4px; }
  .preview {
    width: 56px; height: 56px; padding: 0; border-radius: 12px; overflow: hidden; flex-shrink: 0;
    background: repeating-conic-gradient(#1b1e2e 0 25%, #151826 0 50%) 50% / 12px 12px; border: 1px dashed var(--line-strong);
    color: var(--muted);
  }
  .preview img, .preview video { width: 100%; height: 100%; object-fit: cover; }
</style>
