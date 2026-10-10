<script lang="ts">
  import { Copy, TriangleAlert, Download } from '@lucide/svelte';
  import { copy } from '../lib/api';
  import { toast } from '../lib/toast.svelte';

  let { token, panelUrl }: { token: string; panelUrl: string } = $props();

  const snippet = $derived(`panel-url=${panelUrl}\ntoken=${token}`);
  const flag = $derived(`-javaagent:authlib-injector.jar=${panelUrl}/api/yggdrasil`);
  const startCommand = $derived(`java ${flag} -jar fabric-server-launch.jar nogui`);

  async function copyText(t: string) {
    await copy(t);
    toast('Copied');
  }
</script>

<div class="warn"><TriangleAlert size={16} /> Copy the token now — it won't be shown again.</div>
<div class="copyline"><code>{token}</code><button class="ghost icon sm" aria-label="Copy token" onclick={() => copyText(token)}><Copy size={14} /></button></div>

<ol class="steps">
  <li>
    <strong>Install Velora Core on the server</strong>
    <span class="muted small">Velora Core runs on Fabric 1.20.1 with Fabric API. Put <code>scopenet-fabric-1.20.1-*.jar</code> (Velora Core Server) and Fabric API in the server's <code>mods</code> folder. Players add <code>scopenet-client-fabric-1.20.1-*.jar</code> (Velora Core Client) and Fabric API to their own <code>mods</code> folder for the map, vaults and in-game hub.</span>
    <a class="dl" href="https://github.com/VeloraMCDev/velora-launcher/releases/latest" target="_blank" rel="noreferrer"><Download size={14} /> Download from Releases</a>
  </li>
  <li>
    <strong>Add the token</strong>
    <span class="muted small">Start the server once. Velora Core creates the file below with every option explained. Set these two lines, then restart or run <code>/scopenet reload</code>.</span>
    <span class="muted tiny"><code>config/scopenet.properties</code></span>
    <div class="code"><pre>{snippet}</pre><button class="ghost icon sm" aria-label="Copy config" onclick={() => copyText(snippet)}><Copy size={14} /></button></div>
  </li>
  <li>
    <strong>Turn on the panel's sign-in</strong>
    <span class="muted small">Download <a href="/api/v1/launcher/authlib-injector.jar">authlib-injector.jar</a> next to the server jar, keep <code>online-mode=true</code>, and add this flag before <code>-jar</code> in the start command. Players then join with their Velora account.</span>
    <div class="copyline"><code>{flag}</code><button class="ghost icon sm" aria-label="Copy start flag" onclick={() => copyText(flag)}><Copy size={14} /></button></div>
    <details class="example">
      <summary>Full start command example</summary>
      <div class="copyline"><code>{startCommand}</code><button class="ghost icon sm" aria-label="Copy start command" onclick={() => copyText(startCommand)}><Copy size={14} /></button></div>
    </details>
  </li>
  <li>
    <strong>Check it worked</strong>
    <span class="muted small">Join the server, then run <code>/scopenet status</code>. It shows the panel address, whether the token is set and which modules are on. The full guide is in the repository at <code>docs/VELORA_CORE_SETUP.md</code>.</span>
  </li>
</ol>

<style>
  .warn { display: flex; align-items: center; gap: 8px; color: #fcd34d; font-size: 0.85rem; }
  .copyline, .code { display: flex; align-items: flex-start; gap: 6px; background: var(--bg-2); border: 1px solid var(--line); border-radius: var(--radius-sm); padding: 4px 4px 4px 12px; }
  .copyline { align-items: center; }
  .copyline code { flex: 1; overflow-x: auto; white-space: nowrap; padding: 8px 0; font-size: 0.82rem; }
  .code pre { flex: 1; margin: 0; padding: 8px 0; font-family: var(--mono); font-size: 0.8rem; overflow-x: auto; }
  .steps { margin: 4px 0 0; padding-left: 20px; display: flex; flex-direction: column; gap: 16px; }
  .steps li { display: flex; flex-direction: column; gap: 8px; }
  .steps li::marker { color: var(--muted); }
  .dl { display: inline-flex; align-items: center; gap: 6px; font-size: 0.85rem; align-self: flex-start; }
  .example summary { cursor: pointer; font-size: 0.82rem; color: var(--muted); margin-bottom: 6px; }
</style>
