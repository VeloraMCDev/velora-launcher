<script lang="ts">
  import OwnedLogin from '../../../../packages/panel-ui/Login.svelte';
  import { platform } from '../lib/api';
  import { setToken } from '../lib/session.svelte';
  let props: {brandName: string; logo: string | null; registration?: 'closed' | 'approval' | 'open'; app?: boolean} = $props();
</script>

<OwnedLogin {...props} client={platform} onToken={setToken} />
