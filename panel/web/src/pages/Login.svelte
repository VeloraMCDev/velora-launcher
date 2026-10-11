<script lang="ts">
  import InterfaceIcon from "@velora/experience/InterfaceIcon.svelte";
  import OwnedLogin from '../../../../packages/panel-ui/Login.svelte';
  import { platform } from '../lib/api';
  import { setToken } from '../lib/session.svelte';
  let props: {brandName: string; logo: string | null; registration?: 'closed' | 'approval' | 'open'; app?: boolean} = $props();
</script>

<OwnedLogin iconRenderer={InterfaceIcon} {...props} client={platform} onToken={setToken} />
