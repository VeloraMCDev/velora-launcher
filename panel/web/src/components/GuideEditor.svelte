<script lang="ts">
  import type { PlayerGuide } from '@velora/experience/branding';
  import { DEFAULT_GUIDE } from '@velora/experience/player-guide';
  import IconPicker from './IconPicker.svelte';
  let { value = $bindable() }: { value?: PlayerGuide | null } = $props();
  function edit() { value = structuredClone(DEFAULT_GUIDE); }
  function move(index: number, direction: number) {
    if (!value) return;
    const next = index + direction;
    if (next < 0 || next >= value.articles.length) return;
    const [article] = value.articles.splice(index, 1); value.articles.splice(next, 0, article);
  }
</script>
<div class="guide-editor">
  <p>Publish modpack details, server rules, gameplay instructions and integration help. Articles render as plain text. Links must use HTTPS.</p>
  {#if !value}<p>Using the built-in Player Guide.</p><button onclick={edit}>Customize Player Guide</button>{:else}
    <label>Guide title<input bind:value={value.title} maxlength="80" /></label>
    <label>Introduction<textarea bind:value={value.introduction} rows="3" maxlength="2000"></textarea></label>
    <label><input type="checkbox" bind:checked={value.show_commands} /> Include built-in command reference and tips</label>
    {#each value.articles as article, i (article.id)}<fieldset><legend>Article {i + 1}</legend>
      <div class="row"><button type="button" disabled={i === 0} onclick={() => move(i, -1)}>Move up</button><button type="button" disabled={i === value!.articles.length - 1} onclick={() => move(i, 1)}>Move down</button><button type="button" onclick={() => value?.articles.splice(i, 1)}>Remove article</button></div>
      <label>Category<input bind:value={article.category} maxlength="80" /></label><label>Title<input bind:value={article.title} maxlength="120" /></label>
      <label>Icon<input bind:value={article.icon} placeholder="book, HTTPS image URL or iconify value" maxlength="2048" /></label><IconPicker onpick={icon => article.icon = icon} />
      <label>Content<textarea bind:value={article.body} rows="7" maxlength="20000"></textarea></label>
      {#each article.links as link, j}<div class="row"><label>Link label<input bind:value={link.label} maxlength="120" /></label><label>HTTPS URL<input bind:value={link.url} maxlength="2048" /></label><button type="button" onclick={() => article.links.splice(j, 1)}>Remove link</button></div>{/each}
      <button type="button" disabled={article.links.length >= 12} onclick={() => article.links.push({ label: '', url: '' })}>Add link</button>
    </fieldset>{/each}
    <div class="row"><button disabled={value.articles.length >= 100} onclick={() => value?.articles.push({ id: crypto.randomUUID(), category: 'Server', title: 'New article', body: '', icon: 'book', links: [] })}>Add article</button><button onclick={() => value = null}>Use built-in guide</button></div>
  {/if}
</div>
<style>.guide-editor { display: grid; gap: 1rem; } label { display: flex; flex-direction: column; gap: .4rem; } fieldset { border: 1px solid var(--line); border-radius: 10px; display: grid; gap: .8rem; padding: 1rem; min-width: 0; } .row { flex-wrap: wrap; } .row label { flex: 1; min-width: 150px; }</style>
