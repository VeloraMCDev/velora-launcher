<script lang="ts">
  import { Copy, TriangleAlert, Download } from '@lucide/svelte';
  import { copy } from '../lib/api';
  import { toast } from '../lib/toast.svelte';

  let { token, panelUrl }: { token: string; panelUrl: string } = $props();
  let tab = $state<'paper' | 'mod'>('paper');

  const snippet = $derived(
    tab === 'paper' ? `panel-url: "${panelUrl}"\ntoken: "${token}"` : `panel-url=${panelUrl}\ntoken=${token}`,
  );

  async function copyText(t: string) {
    await copy(t);
    toast('Copied');
  }
</script>

<div class="warn"><TriangleAlert size={16} /> Copy the token now — it won't be shown again.</div>
<div class="copyline"><code>{token}</code><button class="ghost icon sm" aria-label="Copy token" onclick={() => copyText(token)}><Copy size={14} /></button></div>

<ol class="steps">
  <li>
    <strong>Install the Velora integration</strong>
    <span class="muted small">Use scopenet-paper for Paper, Purpur or Spigot. Fabric and Forge need the JAR for the exact Minecraft version: 1.20.1, 1.21.1, 26.1.2, 26.2 or 26.3.</span>
    <a class="dl" href="https://github.com/VeloraMCDev/velora-launcher/releases/latest" target="_blank" rel="noreferrer"><Download size={14} /> Download from Releases</a>
  </li>
  <li>
    <strong>Configure it</strong>
    <div class="segmented">
      <button class:active={tab === 'paper'} onclick={() => (tab = 'paper')}>Paper</button>
      <button class:active={tab === 'mod'} onclick={() => (tab = 'mod')}>Fabric / Forge</button>
    </div>
    <span class="muted tiny"><code>{tab === 'paper' ? 'plugins/SCOPENET/config.yml' : 'config/scopenet.properties'}</code></span>
    <div class="code"><pre>{snippet}</pre><button class="ghost icon sm" aria-label="Copy config" onclick={() => copyText(snippet)}><Copy size={14} /></button></div>
  </li>
  <li>
    <strong>Use the panel's sign-in</strong>
    <span class="muted small">Add this flag before <code>-jar</code> in the start command and keep <code>online-mode=true</code>. Download <a href="/api/v1/launcher/authlib-injector.jar">authlib-injector.jar</a> next to the server jar.</span>
    <div class="copyline"><code>-javaagent:authlib-injector.jar={panelUrl}/api/yggdrasil</code><button class="ghost icon sm" aria-label="Copy start flag" onclick={() => copyText(`-javaagent:authlib-injector.jar=${panelUrl}/api/yggdrasil`)}><Copy size={14} /></button></div>
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
  .segmented { align-self: flex-start; }
  .dl { display: inline-flex; align-items: center; gap: 6px; font-size: 0.85rem; align-self: flex-start; }
</style>
