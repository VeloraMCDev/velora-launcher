<script lang="ts">
  import type { Experience } from './index';
  import Widgets from '../../packages/platform-ui/ExperienceWidgets.svelte';
  import { widgetComponents } from './components';
  let { experience }: { experience?: Experience } = $props();
</script>

<!-- The private composition supplies locally compiled components to the generic SDK host. -->
<Widgets {experience} components={widgetComponents} />
