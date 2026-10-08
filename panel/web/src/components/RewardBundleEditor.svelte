<script lang="ts">
  import { Coins, Package, KeyRound, Users, Map as MapIcon, Award, MessageSquare, Terminal, Plus, X, Gift, Sparkles } from '@lucide/svelte';
  import { get } from '../lib/api';
  import type { RewardAction } from '../lib/rewards';
  import type { GameServer } from '../lib/types';

  // Everything a quest, achievement or rank milestone hands out besides its XP and title.
  let { actions = $bindable([]) }: { actions: RewardAction[] } = $props();

  let servers = $state<GameServer[]>([]);
  let cosmetics = $state<{ key: string; label: string; type: string }[]>([]);
  $effect(() => { get<{ key: string; label: string; type: string }[]>('/api/admin/cosmetics/templates').then((r) => (cosmetics = Array.isArray(r) ? r : [])).catch(() => {}); });
  let customItems = $state<{ id: string; title: string }[]>([]);
  $effect(() => { get<{ items: { id: string; title: string }[] }>('/api/admin/custom-items').then(r => customItems = r.items).catch(() => {}); });
  $effect(() => { get<GameServer[]>('/api/admin/servers').then((s) => (servers = Array.isArray(s) ? s : [])).catch(() => {}); });

  const kinds = [
    { id: 'money', label: 'Money', icon: Coins, blurb: 'Credit the player’s balance' },
    { id: 'item', label: 'Item', icon: Package, blurb: 'Hand over items in game' },
    { id: 'custom_item', label: 'Custom item', icon: Gift, blurb: 'Give a custom item with its name, model and bonuses' },
    { id: 'unlock', label: 'Cosmetic', icon: Sparkles, blurb: 'Unlock a title, particle, pet, message or modelled cosmetic from the Cosmetics Studio' },
    { id: 'permission', label: 'Permission', icon: KeyRound, blurb: 'Grant an in-game permission (LuckPerms)' },
    { id: 'group', label: 'Group', icon: Users, blurb: 'Add them to a permission group' },
    { id: 'claim_chunks', label: 'Claim chunks', icon: MapIcon, blurb: 'Raise their guild’s land limit' },
    { id: 'badge', label: 'Badge', icon: Award, blurb: 'Show a badge on their profile' },
    { id: 'message', label: 'Message', icon: MessageSquare, blurb: 'Tell them something in game' },
    { id: 'command', label: 'Command', icon: Terminal, blurb: 'Run a console command (the server must allow it)' }
  ] as const;
  const meta = (t: string) => kinds.find((k) => k.id === t)!;
  const items = ['diamond', 'emerald', 'netherite_ingot', 'golden_apple', 'enchanted_golden_apple', 'elytra', 'totem_of_undying', 'experience_bottle',
    'iron_ingot', 'gold_ingot', 'ender_pearl', 'name_tag', 'diamond_pickaxe', 'diamond_sword', 'shulker_box', 'cooked_beef', 'bread', 'torch'];

  let adding = $state(false);
  function add(type: RewardAction['type']) {
    const base: Record<string, RewardAction> = {
      money: { type: 'money', amount: 100 }, item: { type: 'item', item: 'diamond', amount: 1 },
      custom_item: { type: 'custom_item', custom: customItems[0]?.id ?? '', amount: 1 },
      unlock: { type: 'unlock', key: cosmetics[0]?.key ?? '' },
      permission: { type: 'permission', node: '', value: true, minutes: 0 }, group: { type: 'group', group: '' },
      claim_chunks: { type: 'claim_chunks', amount: 4 }, badge: { type: 'badge', badge: '' },
      message: { type: 'message', text: '' }, command: { type: 'command', command: '' }
    };
    actions = [...actions, base[type]];
    adding = false;
  }
  const remove = (i: number) => (actions = actions.filter((_, n) => n !== i));
  const days = (a: RewardAction) => ((a.minutes ?? 0) / 1440);
  const setDays = (a: RewardAction, v: string) => { a.minutes = Math.max(0, Math.round((Number(v) || 0) * 1440)); };
</script>

<section class="bundle">
  <header>
    <span class="head"><Gift size={16} /> Extra rewards <small>{actions.length ? `${actions.length} attached` : 'none yet'}</small></span>
    <button type="button" class="ghost sm" onclick={() => (adding = !adding)}><Plus size={14} /> Add reward</button>
  </header>

  {#if adding}
    <div class="menu">
      {#each kinds as k}
        {@const Icon = k.icon}
        <button type="button" class="kind" onclick={() => add(k.id)}><Icon size={16} /><strong>{k.label}</strong><small>{k.blurb}</small></button>
      {/each}
    </div>
  {/if}

  {#each actions as a, i}
    {@const Icon = meta(a.type).icon}
    <div class="row">
      <span class="ic"><Icon size={16} /></span>
      <div class="fields">
        <strong class="kind-name">{meta(a.type).label}</strong>
        {#if a.type === 'money'}
          <label>Amount<input type="number" min="0.01" step="0.01" bind:value={a.amount} /></label>
        {:else if a.type === 'item'}
          <label class="wide">Item<input list="reward-items" bind:value={a.item} placeholder="diamond or modid:item" /></label>
          <label>How many<input type="number" min="1" max="6400" bind:value={a.amount} /></label>
        {:else if a.type === 'custom_item'}
          <label class="wide">Custom item<select bind:value={a.custom}><option value="">Choose an item</option>{#each customItems as item}<option value={item.id}>{item.title} ({item.id})</option>{/each}</select></label>
          <label>How many<input type="number" min="1" max="6400" bind:value={a.amount} /></label>
        {:else if a.type === 'unlock'}
          <label class="wide">Cosmetic
            <select bind:value={a.key}>
              <option value="">Choose a cosmetic</option>
              {#each cosmetics as c}<option value={c.key}>{c.label} · {c.type.replaceAll('_', ' ')}</option>{/each}
            </select>
          </label>
          {#if !cosmetics.length}<small class="empty">No cosmetics yet. Create one in the Cosmetics Studio first.</small>{/if}
        {:else if a.type === 'permission'}
          <label class="wide">Permission node<input bind:value={a.node} placeholder="essentials.fly" /></label>
          <label>For (days, 0 = forever)<input type="number" min="0" step="1" value={days(a)} oninput={(e) => setDays(a, e.currentTarget.value)} /></label>
        {:else if a.type === 'group'}
          <label class="wide">Group<input bind:value={a.group} placeholder="vip" /></label>
        {:else if a.type === 'claim_chunks'}
          <label>Extra chunks<input type="number" min="1" max="10000" bind:value={a.amount} /></label>
        {:else if a.type === 'badge'}
          <label class="wide">Badge name<input bind:value={a.badge} maxlength="40" placeholder="Pathfinder" /></label>
        {:else if a.type === 'message'}
          <label class="wide">Message<input bind:value={a.text} maxlength="200" placeholder="Thanks for playing!" /></label>
        {:else if a.type === 'command'}
          <label class="wide">Console command <small>({'{player}'} and {'{uuid}'} are filled in)</small><input bind:value={a.command} placeholder="give {'{player}'} minecraft:cake 1" /></label>
        {/if}
        {#if ['money', 'item', 'custom_item', 'permission', 'group', 'command', 'message'].includes(a.type) && servers.length > 1}
          <label>Where
            <select value={a.server_id ?? ''} onchange={(e) => (a.server_id = e.currentTarget.value ? Number(e.currentTarget.value) : null)}>
              <option value="">{a.type === 'money' ? 'Every economy' : 'Wherever they are online'}</option>
              {#each servers as s}<option value={s.id}>{s.name}</option>{/each}
            </select>
          </label>
        {/if}
      </div>
      <button type="button" class="ghost icon" onclick={() => remove(i)} aria-label="Remove reward"><X size={15} /></button>
    </div>
  {/each}
  <datalist id="reward-items">{#each items as it}<option value={it}></option>{/each}</datalist>
  {#if !actions.length && !adding}<p class="empty">Give money, items, permissions, claim chunks and more when this is earned.</p>{/if}
</section>

<style>
  .bundle { display: flex; flex-direction: column; gap: 0.6rem; padding: 0.9rem; border: 1px solid var(--line, #ffffff1a); border-radius: 14px; background: color-mix(in srgb, var(--accent, #8b6cff) 5%, transparent); }
  header { display: flex; align-items: center; justify-content: space-between; gap: 0.6rem; }
  .head { display: inline-flex; align-items: center; gap: 0.45rem; font-weight: 600; }
  .head small { color: var(--muted); font-weight: 400; margin-left: 0.3rem; }
  .menu { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 0.5rem; }
  .kind { display: grid; grid-template-columns: auto 1fr; gap: 0.1rem 0.6rem; align-items: center; text-align: left; padding: 0.6rem 0.7rem; border-radius: 12px; background: var(--surface, #ffffff0d); border: 1px solid var(--line, #ffffff1a); }
  .kind:hover { border-color: var(--accent); }
  .kind small { grid-column: 2; color: var(--muted); font-size: 0.72rem; line-height: 1.3; }
  .row { display: flex; align-items: flex-start; gap: 0.6rem; padding: 0.65rem; border-radius: 12px; background: var(--surface, #ffffff0d); border: 1px solid var(--line, #ffffff14); }
  .ic { display: grid; place-items: center; width: 2rem; height: 2rem; border-radius: 10px; background: color-mix(in srgb, var(--accent, #8b6cff) 20%, transparent); color: var(--accent, #8b6cff); flex-shrink: 0; }
  .fields { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 0.5rem 0.7rem; flex: 1; min-width: 0; }
  .kind-name { flex-basis: 100%; font-size: 0.85rem; }
  label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.74rem; color: var(--muted); min-width: 7.5rem; }
  label.wide { flex: 1; min-width: 12rem; }
  .empty { margin: 0; color: var(--muted); font-size: 0.82rem; }
  .sm { display: inline-flex; align-items: center; gap: 0.3rem; }
</style>
