<script lang="ts">
  import { iconUrl, VAULT_ITEMS } from '@velora/experience/icons.svelte';
  import { UI_ICON_NAMES, UI_ITEMS } from '@velora/experience/ui-icons';
  import { NAV_ITEMS } from '@velora/experience/nav-icons';
  import IconPicker from './IconPicker.svelte';
  let { value = $bindable() }: { value?: Record<string, string> } = $props();
  let query = $state('');
  let kind = $state('nav');
  let customKey = $state('');
  let customValue = $state('');
  const spriteEntries = ['hud/heart/full', 'hud/heart/half', 'hud/heart/container', 'hud/food_full', 'hud/food_half', 'hud/food_empty'].map(name => ({ key: 'sprite:' + name, name, item: name.replace(/^hud\//, '').replaceAll('/', '_') }));
  const entries = $derived(kind === 'sprite' ? spriteEntries : kind === 'nav' ? Object.keys(NAV_ITEMS).map(name => ({ key: 'nav:' + name, name, item: NAV_ITEMS[name] })) : kind === 'ui' ? UI_ICON_NAMES.map(name => ({ key: 'ui:' + name, name, item: UI_ITEMS[name] })) : Object.keys(VAULT_ITEMS).map(name => ({ key: 'item:' + name, name, item: name })));
  const matches = $derived(entries.filter(e => `${e.name} ${e.item ?? ''}`.toLowerCase().includes(query.toLowerCase())));
  let limit = $state(30);
  function set(key: string, icon: string) {
    const next = { ...value };
    if (icon.trim()) next[key] = icon.trim(); else delete next[key];
    value = next;
  }
</script>
<div class="icons-editor">
  <p>Customize navigation, interface symbols and Minecraft item icons across the launcher, Admin Panel and player apps. Item overrides also apply to inventories, rewards and collections. Leave an override empty to use the default.</p>
  <p>Use a Minecraft item ID, an HTTPS image URL, an uploaded image path or the icon library. Minecraft defaults load from <a href="https://ccvaults.com/" target="_blank" rel="noopener noreferrer">ccVaults / ccLeaf</a>; artwork retains its original terms.</p>
  <div class="row"><label>Icon category<select bind:value={kind} onchange={() => limit = 30}><option value="nav">Navigation</option><option value="ui">Interface symbols</option><option value="item">Minecraft assets</option><option value="sprite">HUD sprites</option></select></label><label>Search icons<input bind:value={query} oninput={() => limit = 30} placeholder="Search name or item" /></label></div>
  <p>{matches.length} icons · {Object.keys(value ?? {}).length} overrides</p>
  {#each matches.slice(0, limit) as entry (entry.key)}<div class="entry">
    <div><b>{entry.name}</b><small>{entry.key} · Default: {entry.item ?? 'line icon'}</small></div>
    {#if iconUrl(value?.[entry.key] || entry.item)}<img src={iconUrl(value?.[entry.key] || entry.item)!} alt="" width="28" height="28" />{/if}
    <input aria-label={entry.name + ' icon override'} value={value?.[entry.key] ?? ''} oninput={e => set(entry.key, e.currentTarget.value)} placeholder={entry.item ?? 'Default line icon'} maxlength="2048" />
    <IconPicker label="Pick icon" onpick={icon => set(entry.key, icon)} /><button type="button" disabled={!value?.[entry.key]} onclick={() => set(entry.key, '')}>Reset</button>
  </div>{/each}
  {#if limit < matches.length}<button type="button" onclick={() => limit += 30}>Show more icons</button>{/if}
  <fieldset><legend>Custom icon override</legend><p>Use this for a modded item or another icon key. Examples: item:modid:item_name or ui:Search.</p><label>Icon key<input bind:value={customKey} placeholder="item:modid:item_name" maxlength="128" /></label><label>Icon value<input bind:value={customValue} placeholder="Minecraft ID, HTTPS image or iconify value" maxlength="2048" /></label><button type="button" disabled={!/^(nav|ui|item|sprite):[a-z0-9_:/-]+$/i.test(customKey) || !customValue.trim()} onclick={() => { set(customKey, customValue); customKey = ''; customValue = ''; }}>Add override</button></fieldset>
  {#if Object.keys(value ?? {}).length}<details><summary>Saved overrides</summary>{#each Object.entries(value ?? {}) as [key, icon]}<div class="entry"><b>{key}</b><span>{icon}</span><button type="button" onclick={() => set(key, '')}>Remove</button></div>{/each}</details>{/if}
</div>
<style>.icons-editor { display: grid; gap: .8rem; } label { display: grid; gap: .4rem; } .row { flex-wrap: wrap; } .entry { display: flex; align-items: center; gap: .6rem; flex-wrap: wrap; padding: .7rem; border: 1px solid var(--line); border-radius: 8px; } .entry div { flex: 1 1 160px; } small { display: block; color: var(--muted); } .entry input { flex: 1 1 200px; min-width: 120px; } img { object-fit: contain; image-rendering: pixelated; }</style>
