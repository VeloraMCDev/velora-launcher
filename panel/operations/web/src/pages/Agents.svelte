<script lang="ts">
  import { onMount } from 'svelte';
  import { api, downloadInstaller } from '../lib/api';
  import { ask, toast } from '../lib/state.svelte';
  import { bytes, pct, when } from '../lib/format';
  type Report = {platform:string;version:string;uptime:number;cpu:number|null;memory:{total:number;used:number};disks:{name:string;total:number;used:number}[];services:{name:string;kind:string;ok:boolean;ms:number|null;players?:{online:number;max:number}}[]};
  type Agent = {id:string;name:string;online:boolean;created_at:string;received_at:string|null;report:Report|null};
  let agents=$state<Agent[]>([]), name=$state(''), busy=$state(false), error=$state('');
  let enrolled=$state<{id:string;name:string;token:string}|null>(null);
  type Target = {name:string;kind:'minecraft'|'http'|'tcp';host:string;port:number;url:string;virtual_host:string};
  let mode=$state<'linux'|'desktop'>('linux'), targets=$state<Target[]>([]), downloading=$state(false);
  const cfg=$derived(enrolled ? {ops_url:location.origin,id:enrolled.id,token:enrolled.token,interval_seconds:30,
    services:targets.map(t=>t.kind==='http' ? {name:t.name,kind:t.kind,url:t.url} : {name:t.name,kind:t.kind,host:t.host,port:Number(t.port),...(t.virtual_host && t.kind==='minecraft' ? {virtual_host:t.virtual_host} : {})}),disks:[]} : null);
  const configuration=$derived(cfg ? JSON.stringify(cfg,null,2) : '');
  async function load() { try { agents=await api<Agent[]>('/api/agents'); } catch(e) { error=(e as Error).message; } }
  onMount(()=>{ void load(); const timer=setInterval(()=>void load(),15000); return ()=>clearInterval(timer); });
  async function enroll(e:SubmitEvent) { e.preventDefault();busy=true;error='';try { enrolled=await api('/api/agents',{method:'POST',body:{name}});name='';targets=[];await load(); } catch(e) {error=(e as Error).message;} finally {busy=false;} }
  async function revoke(a:Agent) { if (!await ask(`Revoke ${a.name}?`,'Its monitoring credential will stop working. Running services are unaffected.','Revoke',true)) return;try {await api(`/api/agents/${a.id}`,{method:'DELETE'});if(enrolled?.id===a.id)enrolled=null;await load();}catch(e){toast((e as Error).message,'error');} }
  function downloadConfig() { const url=URL.createObjectURL(new Blob([configuration],{type:'application/json'}));const link=document.createElement('a');link.href=url;link.download='velora-monitor-config.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000); }
  function addTarget() { targets.push({name:'',kind:'minecraft',host:mode==='linux'?'127.0.0.1':'host.docker.internal',port:25565,url:mode==='linux'?'http://127.0.0.1:8080/health':'http://host.docker.internal:8080/health',virtual_host:''}); }
  async function install(e:SubmitEvent) {
    e.preventDefault();downloading=true;error='';
    try { const file=await downloadInstaller({configuration:cfg,mode});const url=URL.createObjectURL(file);const a=document.createElement('a');a.href=url;a.download='velora-monitor-install.zip';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000); }
    catch(e) { error=(e as Error).message; } finally {downloading=false;}
  }
</script>
<div class="head"><h1>Monitoring agents</h1><p class="muted">Host metrics and selected game-server, Calagopus and service health checks.</p></div>
<div class="card setup"><h2>Connect a host with Docker</h2><p>Enroll a host, choose its services, and download one installation bundle. Extract it on the host and start it with Docker Compose. Docker handles Node and restarts the agent after reboot.</p><form class="row" onsubmit={enroll}><input required maxlength="80" aria-label="Host name" placeholder="Host name" bind:value={name}/><button class="primary" disabled={busy || enrolled !== null}>Enroll host</button></form><p class="tiny muted">Requires Docker with Compose. Reports use HTTPS; the agent exposes no inbound ports and needs no Docker socket.</p></div>
{#if error}<p role="alert" class="bad">{error}</p>{/if}
{#if enrolled}<div class="card credential"><h2>Install on {enrolled.name}</h2><p>The bundle contains this host's one-time credential. Keep it private; revoke and enroll again if lost.</p>
<form class="installer" onsubmit={install}>
  <label>Docker host<select bind:value={mode}><option value="linux">Linux Docker Engine</option><option value="desktop">Docker Desktop (Windows / macOS)</option></select></label>
  <p class="tiny muted">{mode==='linux' ? 'Local services can use 127.0.0.1 through host networking.' : 'Use host.docker.internal to reach host services. Resource metrics describe Docker’s Linux VM; use the standalone agent for native Windows/macOS metrics.'}</p>
  <h3>Services to monitor</h3>
  {#each targets as t,i}<div class="target">
    <div class="row"><input required maxlength="80" aria-label={`Service ${i+1} name`} placeholder="Service name" bind:value={t.name}/><select aria-label={`Service ${i+1} type`} bind:value={t.kind}><option value="minecraft">Minecraft Java</option><option value="http">HTTP / Calagopus</option><option value="tcp">TCP port</option></select><button type="button" class="ghost sm" onclick={()=>targets.splice(i,1)}>Remove</button></div>
    {#if t.kind==='http'}<input required type="url" aria-label={`Service ${i+1} URL`} placeholder="Health endpoint URL" bind:value={t.url}/>
    {:else}<div class="row"><input required aria-label={`Service ${i+1} host`} placeholder="Host or IP" bind:value={t.host}/><input required type="number" min="1" max="65535" aria-label={`Service ${i+1} port`} bind:value={t.port}/></div>{#if t.kind==='minecraft'}<input aria-label={`Service ${i+1} virtual hostname`} placeholder="Virtual hostname (optional, e.g. smp.velora.scopedd.lol)" bind:value={t.virtual_host}/>{/if}{/if}
  </div>{/each}
  <button type="button" disabled={targets.length>=64} onclick={addTarget}>Add service</button>
  <p class="tiny muted">Without services, the agent reports resource metrics only. Choose backend ports to monitor individual servers. Configure authenticated probes, SSO service headers or disk mounts in the extracted configuration if needed.</p>
  <button class="primary" disabled={downloading}>Download Docker bundle</button>
</form>
<h3>Start the agent</h3><p>Extract the ZIP into its own folder on the host. Open a terminal in that folder and run:</p>
<pre>{mode==='linux' ? 'sudo chown 1000:1000 velora-monitor-config.json\nsudo chmod 600 velora-monitor-config.json\ndocker compose up -d --build' : 'docker compose up -d --build'}</pre>
<p class="tiny muted">Check <code>docker compose logs --tail=50 monitor</code> and wait for Online here. To stop it, run <code>docker compose down</code>. Docker must be configured to start at boot.</p>
<details><summary>Standalone Node installation / configuration</summary><pre>{configuration}</pre><div class="row"><button onclick={downloadConfig}>Download configuration</button><a class="btn" href="/api/agents/download">Download Node agent</a></div><p class="tiny muted">With Node 24 installed: <code>node velora-monitor-agent.mjs velora-monitor-config.json</code>.</p></details>
<button onclick={()=>enrolled=null}>I saved the bundle</button></div>{/if}
<div class="grid g2">
{#each agents as a (a.id)}<section class="card"><div class="card-head"><h2>{a.name}</h2><span class="spacer"></span><span class="badge {a.online ? 'ok' : a.received_at ? 'bad' : 'warn'}">{a.online ? 'Online' : a.received_at ? 'Offline' : 'Awaiting first report'}</span></div><p class="tiny muted">{a.received_at ? `Last report ${when(a.received_at)}` : 'Install the agent with its enrollment configuration.'}</p>
{#if a.report}<p class="small metrics">{a.report.platform} · CPU {pct(a.report.cpu)} · Memory {bytes(a.report.memory.used)} / {bytes(a.report.memory.total)}{!a.online ? ' · Last known metrics' : ''}</p>{#each a.report.disks as d}<p class="tiny muted">{d.name}: {bytes(d.used)} / {bytes(d.total)}</p>{/each}<table><thead><tr><th>Service</th><th>Status</th><th>Latency / players</th></tr></thead><tbody>{#each a.report.services as s}<tr><td>{s.name}<div class="tiny faint">{s.kind}</div></td><td><span class="badge {!a.online ? 'warn' : s.ok ? 'ok' : 'bad'}">{!a.online ? 'Stale' : s.ok ? 'Healthy' : 'Unreachable'}</span></td><td>{s.ms ?? '—'} ms{#if s.players}<div>{s.players.online} / {s.players.max} players</div>{/if}</td></tr>{:else}<tr><td colspan="3" class="muted">No service probes configured on this host.</td></tr>{/each}</tbody></table>{/if}
<button class="sm ghost revoke" onclick={()=>revoke(a)}>Revoke agent</button></section>
{:else}<div class="card empty">No hosts enrolled yet. Use agents for hosts and services you choose to monitor.</div>{/each}
</div>
<style>.head{margin-bottom:20px}.setup,.credential,.installer{display:flex;flex-direction:column;gap:14px}.setup,.credential{margin-bottom:20px}.setup form,.target .row{flex-wrap:wrap}.setup input,.target input{flex:1;min-width:150px}.credential{border-color:var(--accent);min-width:0}.installer label{display:flex;flex-direction:column;gap:8px}.target{display:flex;flex-direction:column;gap:10px;padding:14px;border:1px solid var(--border);border-radius:8px}pre{overflow:auto;padding:14px;background:var(--bg-2);max-width:100%}details summary{cursor:pointer}details pre{margin:12px 0}.metrics{margin:14px 0}.revoke{margin-top:16px}</style>
