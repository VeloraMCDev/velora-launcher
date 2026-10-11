<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { bytes, when } from '../lib/format';
  type Item = {id:string;name:string;source:'release'|'build';version:string;product:string;bytes:number;created_at:string;prerelease:boolean;digest:string|null;expired:boolean;expires_at:string|null;download:string;url:string;commit:string|null};
  type Catalog = {items:Item[];errors:string[];page:number;has_more:boolean;checked:string;token_set:boolean};
  let data = $state<Catalog | null>(null), busy = $state(false), error = $state(''), search = $state(''), source = $state('all');
  const visible = $derived((data?.items ?? []).filter(i => (source === 'all' || i.source === source) && `${i.name} ${i.product} ${i.version} ${i.commit ?? ''}`.toLowerCase().includes(search.toLowerCase())));
  async function load(more=false) {
    if (busy) return; busy=true; error='';
    try {
      const next = await api<Catalog>(`/api/artifacts?page=${more ? (data?.page ?? 0)+1 : 1}`);
      if (more && data) { next.items = [...new Map([...data.items,...next.items].map(i=>[i.id,i])).values()]; next.errors=[...data.errors,...next.errors]; }
      data=next;
    } catch(e) { error=(e as Error).message; } finally { busy=false; }
  }
  onMount(() => { void load(); const timer=setInterval(()=>{ if (data?.page === 1) void load(); },60000); return ()=>clearInterval(timer); });
</script>
<div class="head"><div><h1>Builds & downloads</h1><p class="muted">Velora mods, plugins, extensions, installers and other published build output.</p></div><button disabled={busy} onclick={()=>load()}>Refresh</button></div>
<div class="card intro"><p>Release files remain available on GitHub. Successful main build archives include unreleased builds and expire according to their workflow retention. Downloading does not approve or deploy a build.</p><p class="tiny muted">Workflow archives download as ZIP files. Verify the release signature or checksum before installation. Container images remain under Updates.</p><a class="btn sm" href="/api/agents/download">Download monitoring agent</a></div>
{#if error}<p role="alert" class="bad">{error}</p>{/if}
{#each data?.errors ?? [] as e}<p role="alert" class="bad">{e}</p>{/each}
{#if data && !data.token_set}<p class="muted">Add a GitHub token with Actions read permission in Settings to download workflow archives and avoid anonymous API limits.</p>{/if}
<div class="row filters"><input aria-label="Search builds" placeholder="Search product, file, version or commit…" bind:value={search} /><select aria-label="Build source" bind:value={source}><option value="all">All output</option><option value="release">Published releases</option><option value="build">Workflow builds</option></select></div>
<div class="card flush"><div class="table-scroll"><table><thead><tr><th>File / product</th><th>Version / build</th><th>Availability</th><th>Size</th><th></th></tr></thead><tbody>
{#each visible as i (i.id)}<tr><td><b>{i.name}</b><div class="tiny muted">{i.product}</div>{#if i.digest}<div class="tiny mono digest" title={i.digest}>{i.digest}</div>{/if}</td><td>{i.version}<div class="tiny muted">{when(i.created_at)}{#if i.commit} · {i.commit.slice(0,7)}{/if}</div>{#if i.prerelease}<span class="badge warn">Prerelease</span>{/if}</td><td><span class="badge {i.expired ? 'warn' : 'ok'}">{i.expired ? 'Expired' : i.source === 'release' ? 'Released' : 'Built on main'}</span>{#if i.expires_at}<div class="tiny muted">Expires {when(i.expires_at)}</div>{/if}</td><td>{bytes(i.bytes)}</td><td><div class="row"><a class="btn sm ghost" href={i.url} target="_blank" rel="noreferrer">Details</a>{#if !i.expired}<a class="btn sm primary" href={i.download}>Download{i.source === 'build' ? ' ZIP' : ''}</a>{/if}</div></td></tr>
{:else}<tr><td colspan="5" class="empty">{busy ? 'Loading build output…' : 'No matching downloadable output on this page.'}</td></tr>{/each}
</tbody></table></div></div>
{#if data?.has_more}<button class="more" disabled={busy} onclick={()=>load(true)}>Load older output</button>{/if}
{#if data}<p class="tiny muted">Checked {when(data.checked)}. Catalog results are cached for up to five minutes. Publication and packaging workflows remain manual.</p>{/if}
<style>.head{display:flex;justify-content:space-between;gap:1rem;margin-bottom:20px}.intro{display:flex;flex-direction:column;gap:12px;margin-bottom:18px;align-items:flex-start}.filters{margin:18px 0}.filters input{flex:1;min-width:0}.digest{max-width:300px;overflow:hidden;text-overflow:ellipsis}.more{margin:16px 0}td{vertical-align:top}@media(max-width:600px){.filters{flex-wrap:wrap}.filters input{width:100%}}</style>
