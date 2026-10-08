<script lang="ts">
  import { Link2, LoaderCircle, ArrowRight } from '@lucide/svelte';
  import { connect } from '../lib/store.svelte';
  import { errorText } from '../lib/tauri';

  let url = $state('');
  let busy = $state(false);
  let error = $state('');

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      await connect(url);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="center">
  <form class="card glass" onsubmit={submit}>
    <div class="icon"><Link2 size={26} /></div>
    <h1>Connect to your server</h1>
    <p class="muted">Enter the panel address your community gave you. You only need to do this once.</p>
    <input bind:value={url} placeholder="panel.yourserver.com" autocomplete="url" required />
    {#if error}<div class="error selectable">{error}</div>{/if}
    <button class="primary big" disabled={busy || !url.trim()}>
      {#if busy}<LoaderCircle class="spin" size={18} />{:else}<ArrowRight size={18} />{/if} Connect
    </button>
  </form>
</div>

<style>
  .center { height: 100%; display: grid; place-items: center; padding: 2rem; }
  .card { width: 100%; max-width: 27rem; padding: 2rem; display: flex; flex-direction: column; gap: 0.9rem; background: var(--surface); animation: fade 0.3s ease; }
  .icon { width: 2.8rem; height: 2.8rem; border-radius: var(--radius); display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent); }
  h1 { font-size: 1.35rem; font-weight: 600; }
  p { line-height: 1.5; font-size: 0.93rem; }
  .big { padding: 0.85rem; font-size: 1rem; }
  .error { font-size: 0.87rem; padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); color: color-mix(in srgb, var(--danger) 60%, white); background: color-mix(in srgb, var(--danger) 12%, transparent); }
</style>
