<script lang="ts">
  // A Discord-style rendering of a webhook message (the exact JSON the panel would send).
  let { body }: { body: any } = $props();
  const e = $derived(body?.embeds?.[0] ?? null);

  const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  // Discord's light markdown, applied to already-escaped text.
  function md(s: string = ''): string {
    return esc(s)
      .replace(/```([\s\S]*?)```/g, '<pre>$1</pre>')
      .replace(/`([^`\n]+)`/g, '<code>$1</code>')
      .replace(/\*\*([^*\n]+)\*\*/g, '<b>$1</b>')
      .replace(/(^|[^*])\*([^*\n]+)\*/g, '$1<i>$2</i>')
      .replace(/__([^_\n]+)__/g, '<u>$1</u>')
      .replace(/&lt;t:(\d+):[a-zA-Z]&gt;/g, (_m, t) => `<span class="time">${new Date(Number(t) * 1000).toLocaleString()}</span>`)
      .replace(/\n/g, '<br>');
  }
  const color = $derived(e?.color != null ? '#' + Number(e.color).toString(16).padStart(6, '0') : '#202225');
</script>

<div class="msg">
  <div class="avatar">{#if body?.avatar_url}<img src={body.avatar_url} alt="" />{:else}<span>S</span>{/if}</div>
  <div class="main">
    <div class="who"><strong>{body?.username ?? 'SCOPENET'}</strong><span class="bot">APP</span><span class="when">Today</span></div>
    {#if body?.content}<div class="content">{@html md(body.content)}</div>{/if}
    {#if e}
      <div class="embed" style:border-left-color={color}>
        <div class="grid" class:has-thumb={!!e.thumbnail}>
          <div class="text">
            {#if e.author}<div class="author">{#if e.author.icon_url}<img src={e.author.icon_url} alt="" />{/if}{e.author.name}</div>{/if}
            {#if e.title}<div class="title">{e.title}</div>{/if}
            {#if e.description}<div class="desc">{@html md(e.description)}</div>{/if}
            {#if e.fields?.length}
              <div class="fields">
                {#each e.fields as f}<div class="field" class:inline={f.inline}><div class="fname">{f.name}</div><div class="fval">{@html md(f.value)}</div></div>{/each}
              </div>
            {/if}
          </div>
          {#if e.thumbnail}<img class="thumb" src={e.thumbnail.url} alt="" />{/if}
        </div>
        {#if e.image}<img class="image" src={e.image.url} alt="" />{/if}
        {#if e.footer || e.timestamp}
          <div class="footer">{#if e.footer?.icon_url}<img src={e.footer.icon_url} alt="" />{/if}{e.footer?.text ?? ''}{#if e.footer && e.timestamp} • {/if}{#if e.timestamp}{new Date(e.timestamp).toLocaleString()}{/if}</div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .msg { display: flex; gap: 12px; padding: 14px; border-radius: 12px; background: #313338; color: #dbdee1; font-family: 'gg sans', 'Segoe UI', system-ui, sans-serif; font-size: 0.9rem; line-height: 1.4; }
  .avatar { width: 40px; height: 40px; border-radius: 50%; background: #5865f2; overflow: hidden; flex-shrink: 0; display: grid; place-items: center; color: #fff; font-weight: 700; }
  .avatar img { width: 100%; height: 100%; object-fit: cover; }
  .main { flex: 1; min-width: 0; }
  .who { display: flex; align-items: center; gap: 6px; }
  .who strong { color: #f2f3f5; }
  .bot { background: #5865f2; color: #fff; font-size: 0.6rem; font-weight: 700; padding: 1px 5px; border-radius: 4px; }
  .when { color: #949ba4; font-size: 0.72rem; }
  .content { margin: 2px 0 4px; }
  .embed { margin-top: 6px; max-width: 520px; padding: 10px 14px 12px 12px; border-left: 4px solid #202225; border-radius: 4px; background: #2b2d31; }
  .grid { display: grid; gap: 12px; grid-template-columns: 1fr; }
  .grid.has-thumb { grid-template-columns: 1fr 80px; }
  .author { display: flex; align-items: center; gap: 8px; font-size: 0.82rem; font-weight: 600; margin-bottom: 4px; color: #f2f3f5; }
  .author img, .footer img { width: 22px; height: 22px; border-radius: 50%; }
  .title { font-weight: 700; color: #f2f3f5; font-size: 1rem; margin-bottom: 4px; }
  .desc { font-size: 0.88rem; }
  .fields { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px 12px; margin-top: 8px; }
  .field { grid-column: 1 / -1; }
  .field.inline { grid-column: auto; }
  .fname { font-weight: 700; font-size: 0.82rem; color: #f2f3f5; margin-bottom: 2px; }
  .fval { font-size: 0.85rem; }
  .thumb { width: 80px; height: 80px; border-radius: 6px; object-fit: cover; justify-self: end; }
  .image { margin-top: 10px; max-width: 100%; border-radius: 6px; display: block; }
  .footer { display: flex; align-items: center; gap: 8px; margin-top: 10px; font-size: 0.72rem; color: #b5bac1; }
  :global(.msg b) { color: #fff; }
  :global(.msg code) { background: #1e1f22; padding: 1px 4px; border-radius: 4px; font-size: 0.82rem; }
  :global(.msg pre) { background: #1e1f22; padding: 8px; border-radius: 6px; margin: 4px 0; white-space: pre-wrap; }
  :global(.msg .time) { background: #4e5058; border-radius: 3px; padding: 0 4px; }
</style>
