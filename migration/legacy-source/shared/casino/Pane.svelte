<script lang="ts">
  // One table of the casino. It tells everything inside whether it is the one on screen, so keyboard shortcuts only reach it then.
  import { setContext, type Snippet } from 'svelte';
  import { ACTIVE } from './hotkeys';
  let { active, children }: { active: boolean; children: Snippet } = $props();
  setContext(ACTIVE, () => active);
</script>
{@render children()}
