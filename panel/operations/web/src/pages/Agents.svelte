<script lang="ts">
  import { onMount } from 'svelte';
  import { api, downloadInstaller } from '../lib/api';
  import { ask, toast } from '../lib/state.svelte';
  import { bytes, pct, when } from '../lib/format';
  import ServiceTargets from '../components/ServiceTargets.svelte';
  import { editedServices, loadHostConfiguration, serviceTargets, updateHostConfiguration, type Target, type HostConfiguration } from '../lib/agent-editor';
  type Report = {platform:string;version:string;uptime:number;cpu:number|null;memory:{total:number;used:number};disks:{name:string;total:number;used:number}[];services:{name:string;kind:string;ok:boolean;ms:number|null;players?:{online:number;max:number}}[]};
  type Agent = {id:string;name:string;online:boolean;created_at:string;received_at:string|null;report:Report|null};
  let agents=$state<Agent[]>([]), name=$state(''), busy=$state(false), error=$state('');
  let enrolled=$state<{id:string;name:string;token:string}|null>(null);
  let editing=$state<Agent|null>(null), editName=$state(''), editError=$state(''), saving=$state(false);
  let imported=$state<HostConfiguration|null>(null), editTargets=$state<Target[]>([]), savedConfiguration=$state(false);
  let mode=$state<'linux'|'desktop'>('linux'), targets=$state<Target[]>([]), downloading=$state(false);
  type Image = {image:string;commit:string;run:number};
  let images=$state<Image[]>([]), image=$state(''), imageError=$state('');
  async function loadImages() { try {const catalog=await api<{images:Image[];error:string|null}>('/api/agents/images');images=catalog.images;image=images[0]?.image ?? '';imageError=catalog.error ?? '';}catch{imageError='Image discovery unavailable; local build remains available.';} }
  const cfg=$derived(enrolled ? {ops_url:location.origin,id:enrolled.id,token:enrolled.token,interval_seconds:30,
    services:targets.map(t=>t.kind==='http' ? {name:t.name,kind:t.kind,url:t.url} : {name:t.name,kind:t.kind,host:t.host,port:Number(t.port),...(t.virtual_host && t.kind==='minecraft' ? {virtual_host:t.virtual_host} : {})}),disks:[]} : null);
  const configuration=$derived(cfg ? JSON.stringify(cfg,null,2) : '');
  async function load() { try { agents=await api<Agent[]>('/api/agents'); } catch(e) { error=(e as Error).message; } }
  onMount(()=>{ void load();void loadImages(); const timer=setInterval(()=>void load(),15000); return ()=>clearInterval(timer); });
  async function enroll(e:SubmitEvent) { e.preventDefault();busy=true;error='';try { enrolled=await api('/api/agents',{method:'POST',body:{name}});name='';targets=[];await load(); } catch(e) {error=(e as Error).message;} finally {busy=false;} }
  async function revoke(a:Agent) { if (!await ask(`Revoke ${a.name}?`,'Its monitoring credential will stop working. Running services are unaffected.','Revoke',true)) return;try {await api(`/api/agents/${a.id}`,{method:'DELETE'});if(enrolled?.id===a.id)enrolled=null;await load();}catch(e){toast((e as Error).message,'error');} }
  function downloadConfig() { const url=URL.createObjectURL(new Blob([configuration],{type:'application/json'}));const link=document.createElement('a');link.href=url;link.download='velora-monitor-config.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000); }
  function edit(a:Agent) {editing=a;editName=a.name;editError='';imported=null;editTargets=[];savedConfiguration=false;}
  function closeEdit() {editing=null;imported=null;editTargets=[];}
  async function importConfiguration(event:Event) {
    imported=null;editTargets=[];editError='';savedConfiguration=false;
    const input=event.target as HTMLInputElement, file=input.files?.[0], selected=editing?.id;
    try {if(file && selected) {if(file.size>512*1024)throw Error('Configuration is too large');const c=loadHostConfiguration(await file.text(),selected);if(editing?.id!==selected)return;editTargets=serviceTargets(c.services);imported=c;}}
    catch(e){if(editing?.id===selected)editError=(e as Error).message;} finally {input.value='';}
  }
  async function saveHost(event:SubmitEvent) {
    event.preventDefault();if(!editing)return;saving=true;editError='';savedConfiguration=false;
    try {
      const next=imported ? updateHostConfiguration(imported,editTargets) : null;
      await api(`/api/agents/${editing.id}`,{method:'PATCH',body:{name:editName}});
      await load();
      if(next) {const url=URL.createObjectURL(new Blob([JSON.stringify(next,null,2)+'\n'],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='velora-monitor-config.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);savedConfiguration=true;imported=next;}
      else {closeEdit();toast('Host name saved','ok');}
    } catch(e){editError=(e as Error).message;} finally {saving=false;}
  }
  async function install(e:SubmitEvent) {
    e.preventDefault();downloading=true;error='';
    try { if(cfg)editedServices(targets);const file=await downloadInstaller({configuration:cfg,mode,image:image || null});const url=URL.createObjectURL(file);const a=document.createElement('a');a.href=url;a.download='velora-monitor-install.zip';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000); }
    catch(e) { error=(e as Error).message; } finally {downloading=false;}
  }
</script>
<div class="head"><h1>Monitoring agents</h1><p class="muted">Host metrics and selected game-server, Calagopus and service health checks.</p></div>
<div class="card setup"><h2>Connect a host with Docker</h2><p>Enroll a host, choose its services, and download one installation bundle. Extract it on the host and start it with Docker Compose. Docker handles Node and restarts the agent after reboot.</p><form class="row" onsubmit={enroll}><input required maxlength="80" aria-label="Host name" placeholder="Host name" bind:value={name}/><button class="primary" disabled={busy || enrolled !== null}>Enroll host</button></form><p class="tiny muted">Requires Docker with Compose. Reports use HTTPS; the agent exposes no inbound ports and needs no Docker socket.</p></div>
{#if error}<p role="alert" class="bad">{error}</p>{/if}
{#if editing}<section class="card credential" aria-label="Edit host"><h2>Edit {editing.name}</h2>
<form class="installer" onsubmit={saveHost} oninput={()=>savedConfiguration=false}>
  <label>Host name<input required maxlength="80" bind:value={editName}/></label>
  <h3>Monitored services</h3>
  <p class="small muted">Load this host's current configuration to edit its services. The file stays in your browser; its enrollment and access headers are preserved.</p>
  <label>Current host configuration<input type="file" accept=".json,application/json" onchange={importConfiguration} disabled={saving}/></label>
  {#if imported}<ServiceTargets bind:targets={editTargets}/><p class="tiny muted">Existing HTTP probe headers, Ops access headers and disk checks are retained. The new configuration takes effect after you apply it on the host.</p>{/if}
  {#if editError}<p role="alert" class="bad">{editError}</p>{/if}
  <div class="row"><button class="primary" disabled={saving}>{imported?'Save name & download configuration':'Save host name'}</button><button type="button" disabled={saving} onclick={closeEdit}>Close</button></div>
  {#if savedConfiguration}<div role="status"><p>Host name saved. Replace the host's existing configuration with the downloaded file, then run:</p><pre>sudo chown 1000:1000 velora-monitor-config.json
sudo chmod 600 velora-monitor-config.json
docker compose up -d --force-recreate monitor</pre><p class="tiny muted">The current metrics stay visible until the agent reports with the new service configuration. Docker Desktop users can omit the sudo commands.</p></div>{/if}
</form></section>{/if}
{#if enrolled}<div class="card credential"><h2>Install on {enrolled.name}</h2><p>The bundle contains this host's one-time credential. Keep it private; revoke and enroll again if lost.</p>
<form class="installer" onsubmit={install}>
  <label>Agent image<select bind:value={image}>{#each images as release}<option value={release.image}>Published amd64 image · {release.commit.slice(0,7)} · run {release.run}</option>{/each}<option value="">Build locally from bundled source</option></select></label>
  {#if imageError}<p class="tiny muted">{imageError}</p>{/if}
  <p class="tiny muted">Published images are pinned to an exact digest. Choose a local build for ARM hosts without amd64 emulation. Agent upgrades require a new image selection; enrollment stays the same.</p>
  <label>Docker host<select bind:value={mode}><option value="linux">Linux Docker Engine</option><option value="desktop">Docker Desktop (Windows / macOS)</option></select></label>
  <p class="tiny muted">{mode==='linux' ? 'Local services can use 127.0.0.1 through host networking.' : 'Use host.docker.internal to reach host services. Resource metrics describe Docker’s Linux VM; use the standalone agent for native Windows/macOS metrics.'}</p>
  <h3>Services to monitor</h3>
  <ServiceTargets bind:targets {mode}/>
  <p class="tiny muted">Without services, the agent reports resource metrics only. Choose backend ports to monitor individual servers. Configure authenticated probes, SSO service headers or disk mounts in the extracted configuration if needed.</p>
  <button class="primary" disabled={downloading}>Download Docker bundle</button>
</form>
<h3>Start the agent</h3><p>Extract the ZIP into its own folder on the host. Open a terminal in that folder and run:</p>
<pre>{(mode==='linux' ? 'sudo chown 1000:1000 velora-monitor-config.json\nsudo chmod 600 velora-monitor-config.json\n' : '') + (image ? 'docker compose pull\ndocker compose up -d' : 'docker compose up -d --build')}</pre>
<p class="tiny muted">Check <code>docker compose logs --tail=50 monitor</code> and wait for Online here. To stop it, run <code>docker compose down</code>. Docker must be configured to start at boot.</p>
<details><summary>Standalone Node installation / configuration</summary><pre>{configuration}</pre><div class="row"><button onclick={downloadConfig}>Download configuration</button><a class="btn" href="/api/agents/download">Download Node agent</a></div><p class="tiny muted">With Node 24 installed: <code>node velora-monitor-agent.mjs velora-monitor-config.json</code>.</p></details>
<button onclick={()=>enrolled=null}>I saved the bundle</button></div>{/if}
<div class="grid g2">
{#each agents as a (a.id)}<section class="card"><div class="card-head"><h2>{a.name}</h2><span class="spacer"></span><span class="badge {a.online ? 'ok' : a.received_at ? 'bad' : 'warn'}">{a.online ? 'Online' : a.received_at ? 'Offline' : 'Awaiting first report'}</span></div><p class="tiny muted">{a.received_at ? `Last report ${when(a.received_at)}` : 'Install the agent with its enrollment configuration.'}</p>
<p class="tiny muted">Agent ID: <code>{a.id}</code></p>
{#if a.report}<p class="small metrics">{a.report.platform} · CPU {pct(a.report.cpu)} · Memory {bytes(a.report.memory.used)} / {bytes(a.report.memory.total)}{!a.online ? ' · Last known metrics' : ''}</p>{#each a.report.disks as d}<p class="tiny muted">{d.name}: {bytes(d.used)} / {bytes(d.total)}</p>{/each}<table><thead><tr><th>Service</th><th>Status</th><th>Latency / players</th></tr></thead><tbody>{#each a.report.services as s}<tr><td>{s.name}<div class="tiny faint">{s.kind}</div></td><td><span class="badge {!a.online ? 'warn' : s.ok ? 'ok' : 'bad'}">{!a.online ? 'Stale' : s.ok ? 'Healthy' : 'Unreachable'}</span></td><td>{s.ms ?? '—'} ms{#if s.players}<div>{s.players.online} / {s.players.max} players</div>{/if}</td></tr>{:else}<tr><td colspan="3" class="muted">No service probes configured on this host.</td></tr>{/each}</tbody></table>{/if}
<div class="row"><button class="sm" disabled={saving} onclick={()=>edit(a)}>Edit host</button><button class="sm ghost revoke" onclick={()=>revoke(a)}>Revoke agent</button></div></section>
{:else}<div class="card empty">No hosts enrolled yet. Use agents for hosts and services you choose to monitor.</div>{/each}
</div>
<style>.head{margin-bottom:20px}.setup,.credential,.installer{display:flex;flex-direction:column;gap:14px}.setup,.credential{margin-bottom:20px}.setup form{flex-wrap:wrap}.setup input{flex:1;min-width:150px}.credential{border-color:var(--accent);min-width:0}.installer label{display:flex;flex-direction:column;gap:8px}pre{overflow:auto;padding:14px;background:var(--bg-2);max-width:100%}details summary{cursor:pointer}details pre{margin:12px 0}.metrics{margin:14px 0}.revoke{margin-top:16px}code{overflow-wrap:anywhere}</style>
