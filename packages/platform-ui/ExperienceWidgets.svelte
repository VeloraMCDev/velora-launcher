<script lang="ts">
  import type { Component } from 'svelte';
  import type { Experience } from './experience';
  import { safeLink } from './experience';
  let { experience, components = {} }: {
    experience?: Experience;
    components?: Record<string, Component<{ experience: Experience; widget: Experience['widgets'][number] }>>;
  } = $props();
</script>

{#if experience?.widgets.length}
  <section class="experience-widgets" aria-label="Experience overview">
    {#each experience.widgets as widget (widget.id)}
      {@const Widget = Object.hasOwn(components, widget.component) ? components[widget.component] : undefined}
      <article class="experience-widget" data-component={widget.component}>
        <h3>{widget.title}</h3>
        {#if widget.body}<p>{widget.body}</p>{/if}
        {#if Widget}
          <Widget {experience} {widget} />
        {:else if widget.component === 'links' && Array.isArray(widget.config.links)}
          {#each widget.config.links as link}
            {@const href = safeLink(link?.url)}
            {#if href}<a {href} target="_blank" rel="noreferrer">{link.label}</a>{/if}
          {/each}
        {/if}
      </article>
    {/each}
  </section>
{/if}

<style>
  .experience-widgets { display: grid; gap: 1rem; }
  .experience-widget { padding: 1.25rem; border: 1px solid var(--line); border-radius: var(--radius, 12px); background: var(--surface); }
  h3 { margin: 0 0 .6rem; } p { color: var(--muted); line-height: 1.6; margin: .3rem 0; }
  a { display: inline-block; color: var(--accent); margin: .5rem 1rem .5rem 0; }
</style>
