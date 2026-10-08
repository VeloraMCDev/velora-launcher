<script lang="ts">
  import { Pin, ArrowUpRight } from '@lucide/svelte';
  import { abs } from '../lib/store.svelte';
  import { niceDate } from '../lib/format';
  import { openUrl } from '../lib/tauri';
  import type { NewsItem } from '../lib/types';

  let { news }: { news: NewsItem[] } = $props();
  const sorted = $derived([...news].sort((a, b) => Number(b.pinned) - Number(a.pinned)));
</script>

<section class="news">
  <div class="list">
    {#each sorted as n (n.id)}
      <article class="card glass" class:link={!!n.link}>
        {#if n.image_url}<img src={abs(n.image_url)} alt="" loading="lazy" />{/if}
        <div class="body">
          <div class="meta">
            {#if n.tag}<span class="tag">{n.tag}</span>{/if}
            {#if n.pinned}<Pin size={11} />{/if}
            <span class="spacer"></span>
            <span class="tiny muted">{niceDate(n.date)}</span>
          </div>
          <h3>{n.title}</h3>
          <p class="selectable">{n.body}</p>
          {#if n.link}
            <button class="more ghost sm" onclick={() => n.link && openUrl(n.link)}>Read more <ArrowUpRight size={13} /></button>
          {/if}
        </div>
      </article>
    {/each}
  </div>
</section>

<style>
  .news { display: flex; flex-direction: column; min-height: 0; gap: 0.7rem; }
  .list { display: flex; flex-direction: column; gap: 0.7rem; overflow-y: auto; padding-right: 0.3rem; min-height: 0; padding-bottom: 1rem; }
  .card { overflow: hidden; flex-shrink: 0; background: var(--surface); transition: border-color 0.15s; }
  .card:hover { border-color: var(--line-strong); }
  .card img { width: 100%; aspect-ratio: 16 / 7; object-fit: cover; display: block; }
  .body { padding: 0.85rem 1rem 0.9rem; display: flex; flex-direction: column; gap: 0.35rem; }
  .meta { display: flex; align-items: center; gap: 0.4rem; color: var(--muted); }
  .tag { font-size: 0.72rem; font-weight: 560; color: var(--accent); }
  h3 { font-size: 0.95rem; font-weight: 600; line-height: 1.35; }
  p { font-size: 0.83rem; line-height: 1.55; color: var(--muted); display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; }
  .more { align-self: flex-start; padding-left: 0; color: var(--accent); margin-top: 0.1rem; }
  .more:hover { background: transparent; color: var(--text); }
</style>
