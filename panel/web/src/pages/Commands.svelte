<script lang="ts">
  import { onMount } from 'svelte';
  import { Save, Plus, Trash2, Package, Heart, Drumstick, Plane, Archive, Box, X, ChevronDown } from '@lucide/svelte';
  import Toggle from '../components/Toggle.svelte';
  import ItemEditor, { type ItemSpec } from '../components/ItemEditor.svelte';
  import { get, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';

  type KitItem = ItemSpec & { custom?: string };
  type Kit = { id: string; name: string; description: string; cooldown_secs: number; one_time: boolean; groups: string[]; items: KitItem[]; commands: string[] };
  type U = {
    heal: { enabled: boolean; amount: number; cooldown_secs: number };
    feed: { enabled: boolean; amount: number; cooldown_secs: number };
    fly: { enabled: boolean };
    vault: { enabled: boolean; count: number; rows: number; free_count: number };
    echest: { enabled: boolean };
    kits: Kit[];
  };
  let u = $state<U | null>(null);
  let saved = $state('');
  let customs = $state<{ id: string; title: string }[]>([]);
  let open = $state<string | null>(null);
  let detail = $state<string | null>(null);
  let busy = $state(false);
  let groupInput = $state<Record<string, string>>({});

  async function load() {
    try {
      const d = await get<{ settings: U }>('/api/admin/utilities');
      u = d.settings; saved = JSON.stringify(d.settings);
      customs = (await get<{ items: { id: string; title: string }[] }>('/api/admin/custom-items')).items;
    } catch (e) { toastError(e); }
  }
  onMount(load);
  const dirty = $derived(u ? JSON.stringify(u) !== saved : false);

  const cooldownText = (s: number) => (s <= 0 ? 'no cooldown' : s % 86400 === 0 ? `${s / 86400} day${s === 86400 ? '' : 's'}` : s % 3600 === 0 ? `${s / 3600} h` : s % 60 === 0 ? `${s / 60} min` : `${s} s`);

  function addKit() {
    if (!u) return;
    const id = `kit${u.kits.length + 1}`;
    u.kits.push({ id, name: 'New kit', description: '', cooldown_secs: 86400, one_time: false, groups: [], items: [{ item: 'minecraft:bread', amount: 16 }], commands: [] });
    open = id;
  }
  function addGroup(k: Kit) {
    const g = (groupInput[k.id] ?? '').trim().toLowerCase();
    if (g && !k.groups.includes(g)) k.groups.push(g);
    groupInput[k.id] = '';
  }

  async function save() {
    if (!u) return;
    busy = true;
    try {
      const body = $state.snapshot(u);
      const r = await put<{ settings: U }>('/api/admin/utilities', body);
      u = r.settings; saved = JSON.stringify(r.settings); toast('Saved — servers pick it up within seconds');
    } catch (e) { toastError(e); } finally { busy = false; }
  }
</script>

<div class="page wide">
  <header>
    <div>
      <h1>Commands &amp; kits</h1>
      <p>Configure <code>/heal</code>, <code>/feed</code>, <code>/fly</code>, <code>/vault</code>, <code>/echest</code> and <code>/kit</code>. Changes reach every game server within seconds.</p>
    </div>
    <button onclick={save} disabled={!dirty || busy}><Save size={15} /> Save</button>
  </header>

  {#if u}
    <div class="grid">
      <section class="card">
        <h3><Heart size={16} /> /heal</h3>
        <Toggle bind:checked={u.heal.enabled} label="Enabled" />
        <div class="row">
          <label>Health restored<input type="number" min="0" step="1" bind:value={u.heal.amount} /><small>2 = one heart · 0 = full health</small></label>
          <label>Cooldown (seconds)<input type="number" min="0" bind:value={u.heal.cooldown_secs} /><small>{cooldownText(u.heal.cooldown_secs)}</small></label>
        </div>
        <p class="perm">Permission <code>scopenet.command.heal</code> · skip cooldown with <code>scopenet.cooldown.bypass</code></p>
      </section>

      <section class="card">
        <h3><Drumstick size={16} /> /feed</h3>
        <Toggle bind:checked={u.feed.enabled} label="Enabled" />
        <div class="row">
          <label>Hunger restored<input type="number" min="0" max="20" bind:value={u.feed.amount} /><small>20 fills the whole bar</small></label>
          <label>Cooldown (seconds)<input type="number" min="0" bind:value={u.feed.cooldown_secs} /><small>{cooldownText(u.feed.cooldown_secs)}</small></label>
        </div>
        <p class="perm">Permission <code>scopenet.command.feed</code></p>
      </section>

      <section class="card">
        <h3><Plane size={16} /> /fly</h3>
        <Toggle bind:checked={u.fly.enabled} label="Enabled" />
        <p class="perm"><code>free.fly</code> — fly anywhere<br /><code>guild.fly</code> — fly only inside your own guild's claims (flight switches off when you leave)</p>
      </section>

      <section class="card">
        <h3><Archive size={16} /> /vault &amp; /echest</h3>
        <Toggle bind:checked={u.vault.enabled} label="Vaults enabled" help="Portable chests: /vault 1, /vault 2 …" />
        <div class="row">
          <label>Vaults<input type="number" min="1" max="54" bind:value={u.vault.count} /></label>
          <label>Rows each<input type="number" min="1" max="6" bind:value={u.vault.rows} /><small>{u.vault.rows * 9} slots</small></label>
          <label>Free for everyone<input type="number" min="0" max={u.vault.count} bind:value={u.vault.free_count} /></label>
        </div>
        <p class="perm">Vaults after the free ones need <code>scopenet.vault.2</code>, <code>scopenet.vault.3</code> … (give them by rank with LuckPerms).</p>
        <Toggle bind:checked={u.echest.enabled} label="/echest enabled" help="Opens the player's ender chest from anywhere." />
      </section>
    </div>

    <div class="kits-head">
      <h2><Package size={18} /> Kits</h2>
      <button onclick={addKit}><Plus size={14} /> New kit</button>
    </div>
    <p class="hint">Players run <code>/kit &lt;name&gt;</code>. Restrict a kit to LuckPerms groups, give it a cooldown, or make it claimable once.</p>

    {#each u.kits as k (k.id)}
      <section class="card kit">
        <button class="kh" onclick={() => (open = open === k.id ? null : k.id)}>
          <Package size={16} />
          <span><b>{k.name}</b> <code>/kit {k.id}</code></span>
          <small>{k.items.length} item{k.items.length === 1 ? '' : 's'} · {k.one_time ? 'one time' : cooldownText(k.cooldown_secs)} · {k.groups.length ? k.groups.join(', ') : 'everyone'}</small>
          <ChevronDown size={15} class={open === k.id ? 'flip' : ''} />
        </button>
        {#if open === k.id}
          <div class="body">
            <div class="row">
              <label>Command name<input bind:value={k.id} maxlength="24" /></label>
              <label class="grow">Display name<input bind:value={k.name} maxlength="40" /></label>
              <label>Cooldown (seconds)<input type="number" min="0" bind:value={k.cooldown_secs} /><small>{cooldownText(k.cooldown_secs)}</small></label>
            </div>
            <label>Description<input bind:value={k.description} maxlength="200" /></label>
            <Toggle bind:checked={k.one_time} label="One time only" help="Each player can claim this kit once, ever." />

            <div class="block">
              <b>LuckPerms groups</b> <small>empty = everyone can use it</small>
              <div class="chips">
                {#each k.groups as g}<span class="chip">{g}<button class="x" onclick={() => (k.groups = k.groups.filter((x) => x !== g))} aria-label="Remove {g}"><X size={11} /></button></span>{/each}
                <input class="mini" placeholder="add group…" bind:value={groupInput[k.id]} onkeydown={(e) => e.key === 'Enter' && addGroup(k)} onblur={() => addGroup(k)} />
              </div>
            </div>

            <div class="block">
              <div class="bh"><b>Items</b>
                <span class="btns">
                  <button class="ghost" onclick={() => k.items.push({ item: 'minecraft:diamond', amount: 1 })}><Plus size={13} /> Item</button>
                  {#if customs.length}<button class="ghost" onclick={() => k.items.push({ custom: customs[0].id, item: '', amount: 1 })}><Plus size={13} /> Custom item</button>{/if}
                </span>
              </div>
              {#each k.items as it, i (i)}
                <div class="irow">
                  {#if it.custom !== undefined}
                    <select bind:value={it.custom}>{#each customs as c}<option value={c.id}>{c.title} ({c.id})</option>{/each}</select>
                  {:else}
                    <input list="kit-items" bind:value={it.item} />
                    <button class="ghost" onclick={() => (detail = detail === `${k.id}:${i}` ? null : `${k.id}:${i}`)}>Customize</button>
                  {/if}
                  <input type="number" min="1" max="64" bind:value={it.amount} class="amt" />
                  <button class="ghost" onclick={() => (k.items = k.items.filter((_, j) => j !== i))} aria-label="Remove"><X size={14} /></button>
                </div>
                {#if detail === `${k.id}:${i}` && it.custom === undefined}<div class="inner"><ItemEditor bind:spec={k.items[i]} /></div>{/if}
              {/each}
            </div>

            <label>Commands run on claim <small>one per line, {'{player}'} is replaced — e.g. <code>lp user {'{player}'} permission set perk.x true</code></small>
              <textarea rows="2" value={k.commands.join('\n')} oninput={(e) => (k.commands = e.currentTarget.value.split('\n'))}></textarea>
            </label>
            <div><button class="danger" onclick={() => (u!.kits = u!.kits.filter((x) => x !== k))}><Trash2 size={14} /> Delete kit</button></div>
          </div>
        {/if}
      </section>
    {:else}
      <p class="muted">No kits yet.</p>
    {/each}
    <datalist id="kit-items">{#each ['minecraft:bread', 'minecraft:cooked_beef', 'minecraft:diamond', 'minecraft:iron_ingot', 'minecraft:diamond_sword', 'minecraft:diamond_pickaxe', 'minecraft:golden_apple', 'minecraft:torch', 'minecraft:oak_log', 'minecraft:ender_pearl'] as i}<option value={i}></option>{/each}</datalist>
  {/if}
</div>

<style>
  .page { display: flex; flex-direction: column; gap: 14px; }
  header { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; }
  header button, .kits-head button { display: inline-flex; align-items: center; gap: 6px; }
  code { font: 0.78rem ui-monospace, monospace; background: var(--bg-2); padding: 1px 5px; border-radius: 5px; }
  .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px; }
  .card { display: flex; flex-direction: column; gap: 10px; }
  h3 { margin: 0; display: flex; align-items: center; gap: 7px; font-size: 1rem; }
  h2 { margin: 0; display: flex; align-items: center; gap: 8px; font-size: 1.1rem; }
  .kits-head { display: flex; justify-content: space-between; align-items: center; margin-top: 8px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 0.8rem; color: var(--muted); }
  label small { font-weight: 400; }
  .row { display: flex; gap: 10px; align-items: flex-start; flex-wrap: wrap; }
  .row input[type='number'] { width: 8rem; }
  .grow { flex: 1; min-width: 10rem; }
  .perm, .hint { font-size: 0.78rem; color: var(--muted); margin: 0; line-height: 1.5; }
  .kit { padding: 0; overflow: hidden; gap: 0; }
  .kh { display: flex; align-items: center; gap: 10px; width: 100%; text-align: left; border: 0; border-radius: 0; background: transparent; padding: 14px 16px; }
  .kh small { margin-left: auto; color: var(--muted); font-weight: 400; }
  .body { display: flex; flex-direction: column; gap: 12px; padding: 4px 16px 16px; border-top: 1px solid var(--line); padding-top: 14px; }
  .block { display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; border: 1px solid var(--line); border-radius: 10px; background: color-mix(in srgb, var(--bg-2) 60%, transparent); font-size: 0.85rem; }
  .block small { color: var(--muted); font-weight: 400; margin-left: 6px; }
  .bh { display: flex; align-items: center; }
  .btns { margin-left: auto; display: flex; gap: 6px; }
  .btns button, .irow button { display: inline-flex; align-items: center; gap: 4px; padding: 3px 8px; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
  .chip { display: inline-flex; align-items: center; gap: 4px; padding: 2px 4px 2px 10px; border-radius: 99px; font: 0.76rem ui-monospace, monospace; background: color-mix(in srgb, var(--accent) 14%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent); }
  .x { padding: 1px; border-radius: 50%; display: grid; place-items: center; background: transparent; border: 0; }
  .mini { width: 9rem; padding: 3px 8px; }
  .irow { display: flex; gap: 8px; align-items: center; }
  .irow input:not(.amt), .irow select { flex: 1; min-width: 0; }
  .amt { width: 5rem; }
  .inner { padding: 10px; border: 1px dashed var(--line); border-radius: 10px; }
  .muted { color: var(--muted); }
  :global(.flip) { transform: rotate(180deg); }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
</style>
