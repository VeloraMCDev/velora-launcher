<script lang="ts">
  import { onMount } from 'svelte';
  import { Award, Check, RefreshCw, Sparkles } from '@lucide/svelte';
  import { invoke } from '../lib/tauri';
  import { toast } from '../lib/store.svelte';

  type Unlock = { key: string; type: string; source_type: string; source_id: string; equipped: boolean; metadata: Record<string, unknown> };
  let unlocks = $state<Unlock[]>([]);
  let loading = $state(false);

  async function load() { loading = true; try { unlocks = (await invoke<Unlock[] | null>('get_my_collections')) ?? []; } catch (e) { toast(String(e), 'error'); } finally { loading = false; } }
  async function equip(item: Unlock) {
    try { await invoke('equip_collection_item', { unlockKey: item.key, equipped: !item.equipped }); await load(); }
    catch (e) { toast(String(e), 'error'); }
  }
  const title = (item: Unlock) => String(item.metadata.label ?? item.metadata.name ?? item.key);
  onMount(load);
</script>

<div class="page">
  <header><div><h1><Award size={22} /> Collection</h1><p>Titles, badges and cosmetics unlocked on your account.</p></div><button class="ghost icon" onclick={load} aria-label="Refresh collection" title="Refresh"><RefreshCw size={16} /></button></header>
  {#if loading && !unlocks.length}<div class="loading"><span class="spin">◌</span></div>
  {:else if !unlocks.length}<div class="empty"><Sparkles size={24} /><span>No collection unlocks yet</span></div>
  {:else}<div class="list">{#each unlocks as item (item.key)}<article><span class="icon"><Award size={18} /></span><span class="content"><strong>{title(item)}</strong><small>{item.type.replaceAll('_', ' ')} · {item.source_type}</small></span><button class:primary={item.equipped} class="equip" onclick={() => equip(item)}>{#if item.equipped}<Check size={15} /> Equipped{:else}Equip{/if}</button></article>{/each}</div>{/if}
</div>

<style>
  .page{height:100%;overflow:auto;padding:1.6rem 2.2rem;display:flex;flex-direction:column;gap:1rem}header{display:flex;justify-content:space-between;align-items:center;border-bottom:1px solid var(--line);padding-bottom:1rem}h1{display:flex;align-items:center;gap:.6rem;margin:0;font-size:1.35rem}header p{margin:.35rem 0 0;color:var(--muted);font-size:.88rem}.list{display:flex;flex-direction:column}article{display:flex;align-items:center;gap:.8rem;padding:.9rem 0;border-bottom:1px solid var(--line)}.icon{width:2.4rem;height:2.4rem;display:grid;place-items:center;background:color-mix(in srgb,var(--accent) 14%,transparent);color:var(--accent);flex:none}.content{display:flex;flex-direction:column;gap:.2rem;min-width:0;flex:1}.content small{color:var(--muted);text-transform:capitalize}.equip{display:inline-flex;align-items:center;gap:.35rem}.empty,.loading{display:flex;align-items:center;justify-content:center;gap:.6rem;min-height:12rem;color:var(--muted)}.spin{animation:spin 1s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}@media(max-width:650px){.page{padding:1rem}.equip{font-size:.78rem}}
</style>
