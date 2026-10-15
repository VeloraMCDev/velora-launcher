<script lang="ts">
  import { onMount } from 'svelte';
  let { record, label }: { record: (event: string) => void; label: string } = $props();
  onMount(() => { record('mount:' + label); return () => record('unmount:' + label); });
</script>

<h2>Loaded {label}</h2>
<button>Fixture action {label}</button>
