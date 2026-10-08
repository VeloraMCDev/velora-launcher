<script lang="ts">
  import type { Experience } from './index';
  let { experience }: { experience: Experience; widget: Experience['widgets'][number] } = $props();
  type Summary = { name: string; citizens?: number; amount?: number };
  const rows = (value: unknown): Summary[] => Array.isArray(value)
    ? value.filter(row => row && typeof row === 'object' && typeof row.name === 'string') : [];
  const frontiers = $derived(experience.modules.frontiers as { settlements?: unknown; resources?: unknown } | undefined);
</script>
<div class="frontier-grid">
  <section><h4>Settlements</h4>
    {#each rows(frontiers?.settlements) as settlement}<p><strong>{settlement.name}</strong> · {settlement.citizens ?? 0} citizens</p>
    {:else}<p>Your settlements will appear here as your frontier grows.</p>{/each}
  </section>
  <section><h4>Resources</h4>
    {#each rows(frontiers?.resources) as resource}<p><strong>{resource.name}</strong> · {resource.amount ?? 0}</p>
    {:else}<p>Your gathered resources will appear here.</p>{/each}
  </section>
</div>
<style>
  .frontier-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1.5rem; margin-top: 1rem; }
  h4 { margin: 0 0 .6rem; } p { color: var(--muted); line-height: 1.6; margin: .3rem 0; }
</style>
