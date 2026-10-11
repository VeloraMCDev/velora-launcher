<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { ask, toast } from '../lib/state.svelte';
  import { bytes, pct, when } from '../lib/format';
  type Report = {platform:string;version:string;uptime:number;cpu:number|null;memory:{total:number;used:number};disks:{name:string;total:number;used:number}[];services:{name:string;kind:string;ok:boolean;ms:number|null;players?:{online:number;max:number}}[]};
  type Agent = {id:string;name:string;online:boolean;created_at:string;received_at:string|null;report:Report|null};
  let agents=$state<Agent[]>([]), name=$state(''), busy=$state(false), error=$state('');
  let enrolled=$state<{id:string;name:string;token:string}|null>(null);
  const configuration=$derived(enrolled ? JSON.stringify({ops_url:location.origin,id:enrolled.id,token:enrolled.token,interval_seconds:30,services:[],disks:[]},null,2) : '');
  async function load() { try { agents=await api<Agent[]>('/api/agents'); } catch(e) { error=(e as Error).message; } }
  onMount(()=>{ void load(); const timer=setInterval(()=>void load(),15000); return ()=>clearInterval(timer); });
  async function enroll(e:SubmitEvent) { e.preventDefault();busy=true;error='';try { enrolled=await api('/api/agents',{method:'POST',body:{name}});name='';await load(); } catch(e) {error=(e as Error).message;} finally {busy=false;} }
  async function revoke(a:Agent) { if (!await ask(`Revoke ${a.name}?`,'Its monitoring credential will stop working. Running services are unaffected.','Revoke',true)) return;try {await api(`/api/agents/${a.id}`,{method:'DELETE'});if(enrolled?.id===a.id)enrolled=null;await load();}catch(e){toast((e as Error).message,'error');} }
  function downloadConfig() { const url=URL.createObjectURL(new Blob([configuration],{type:'application/json'}));const link=document.createElement('a');link.href=url;link.download='velora-monitor-config.json';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000); }
</script>
<div class="head"><h1>Monitoring agents</h1><p class="muted">Host metrics and selected game-server, Calagopus and service health checks.</p></div>
<div class="card setup"><h2>Connect a host</h2><p>Install Node 24 and the monitoring agent on the host. Enroll it here, download its configuration, and add the services to monitor in that local file. This agent needs no Docker socket or administrator privileges.</p><form class="row" onsubmit={enroll}><input required maxlength="80" aria-label="Host name" placeholder="Host name" bind:value={name}/><button class="primary" disabled={busy}>Enroll host</button><a class="btn" href="/api/agents/download">Download agent</a></form><p class="tiny muted">Run <code>node velora-monitor-agent.mjs /path/to/velora-monitor-config.json</code>. Reports use HTTPS and an independent monitoring credential. No remote commands or deployments are exposed.</p></div>
{#if error}<p role="alert" class="bad">{error}</p>{/if}
{#if enrolled}<div class="card credential"><h2>{enrolled.name}: save this configuration</h2><p>This credential is shown once. Store the file privately; revoke and enroll again if it is lost.</p><pre>{configuration}</pre><div class="row"><button onclick={downloadConfig}>Download configuration</button><button onclick={()=>enrolled=null}>I saved it</button></div></div>{/if}
<div class="grid g2">
{#each agents as a (a.id)}<section class="card"><div class="card-head"><h2>{a.name}</h2><span class="spacer"></span><span class="badge {a.online ? 'ok' : a.received_at ? 'bad' : 'warn'}">{a.online ? 'Online' : a.received_at ? 'Offline' : 'Awaiting first report'}</span></div><p class="tiny muted">{a.received_at ? `Last report ${when(a.received_at)}` : 'Install the agent with its enrollment configuration.'}</p>
{#if a.report}<p class="small metrics">{a.report.platform} · CPU {pct(a.report.cpu)} · Memory {bytes(a.report.memory.used)} / {bytes(a.report.memory.total)}{!a.online ? ' · Last known metrics' : ''}</p>{#each a.report.disks as d}<p class="tiny muted">{d.name}: {bytes(d.used)} / {bytes(d.total)}</p>{/each}<table><thead><tr><th>Service</th><th>Status</th><th>Latency / players</th></tr></thead><tbody>{#each a.report.services as s}<tr><td>{s.name}<div class="tiny faint">{s.kind}</div></td><td><span class="badge {!a.online ? 'warn' : s.ok ? 'ok' : 'bad'}">{!a.online ? 'Stale' : s.ok ? 'Healthy' : 'Unreachable'}</span></td><td>{s.ms ?? '—'} ms{#if s.players}<div>{s.players.online} / {s.players.max} players</div>{/if}</td></tr>{:else}<tr><td colspan="3" class="muted">No service probes configured on this host.</td></tr>{/each}</tbody></table>{/if}
<button class="sm ghost revoke" onclick={()=>revoke(a)}>Revoke agent</button></section>
{:else}<div class="card empty">No hosts enrolled yet. Use agents for hosts and services you choose to monitor.</div>{/each}
</div>
<style>.head{margin-bottom:20px}.setup{display:flex;flex-direction:column;gap:14px;margin-bottom:20px}.setup form{flex-wrap:wrap}.setup input{flex:1;min-width:180px}.credential{margin-bottom:20px;border-color:var(--accent)}pre{overflow:auto;padding:14px;background:var(--bg-2)}.metrics{margin:14px 0}.revoke{margin-top:16px}</style>
