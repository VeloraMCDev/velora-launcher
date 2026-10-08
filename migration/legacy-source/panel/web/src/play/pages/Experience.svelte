<script lang="ts">
  import ExperienceWidgets from '@scopenet/experience/ExperienceWidgets.svelte';
  import { play } from '../store.svelte';
  import { route } from '../../lib/router.svelte';
  const instance = $derived(play.manifest?.instances.find(i => i.id === route.instanceId));
</script>
<div class="pl-page">
  <h1>{instance?.name}</h1>
  <p class="muted">{instance?.description}</p>
  <ExperienceWidgets experience={instance?.experience} />
</div>
