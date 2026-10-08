<script lang="ts">
  import OwnedSkinView from '../../../../packages/platform-skin/SkinView.svelte';
  let props: {skin: string | null; cape?: string | null; slim?: boolean; side?: 'front' | 'back'; scale?: number} = $props();
</script>

<OwnedSkinView {...props} />
