<script lang="ts">
  import type { Target } from '../lib/agent-editor';
  let {targets=$bindable(),mode='linux'}=$props<{targets:Target[];mode?:'linux'|'desktop'}>();
  function add() {targets.push({name:'',kind:'minecraft',host:mode==='linux'?'127.0.0.1':'host.docker.internal',port:25565,url:mode==='linux'?'http://127.0.0.1:8080/health':'http://host.docker.internal:8080/health',virtual_host:''});}
</script>
{#each targets as t,i}<div class="target">
  <div class="row"><input required maxlength="80" aria-label={`Service ${i+1} name`} placeholder="Service name" bind:value={t.name}/><select aria-label={`Service ${i+1} type`} bind:value={t.kind}><option value="minecraft">Minecraft Java</option><option value="http">HTTP / Calagopus</option><option value="tcp">TCP port</option></select><button type="button" class="ghost sm" onclick={()=>targets.splice(i,1)}>Remove</button></div>
  {#if t.kind==='http'}<input required type="url" aria-label={`Service ${i+1} URL`} placeholder="Health endpoint URL" bind:value={t.url}/>
  {:else}<div class="row"><input required aria-label={`Service ${i+1} host`} placeholder="Host or IP" bind:value={t.host}/><input required type="number" min="1" max="65535" aria-label={`Service ${i+1} port`} bind:value={t.port}/></div>{#if t.kind==='minecraft'}<input aria-label={`Service ${i+1} virtual hostname`} placeholder="Virtual hostname (optional)" bind:value={t.virtual_host}/>{/if}{/if}
</div>{/each}
<button type="button" disabled={targets.length>=64} onclick={add}>Add service</button>
<style>.target{display:flex;flex-direction:column;gap:10px;padding:14px;border:1px solid var(--border);border-radius:8px}.row{flex-wrap:wrap}input{flex:1;min-width:150px}</style>
