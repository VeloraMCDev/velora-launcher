<script lang="ts">
  import type { ComponentProps } from 'svelte';
  import Viewer from '../../packages/platform-map/MapViewer.svelte';
  let props: ComponentProps<typeof Viewer> = $props();
</script>

<!-- Preserve host-supplied player/toolbar snippets, URLs and map state. -->
<Viewer {...props} />
