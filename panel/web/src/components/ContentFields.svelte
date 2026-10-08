<script lang="ts">
  import { Plus, X } from '@lucide/svelte';
  import Slide from './Slide.svelte';
  import Toggle from './Toggle.svelte';
  import DropEditor from './DropEditor.svelte';
  import McLoreEditor from './McLoreEditor.svelte';

  // `v` holds the kind-specific numbers of one thing (the same keys the API stores).
  let { kind, v = $bindable({}), title = '' }: { kind: string; v: Record<string, any>; title?: string } = $props();

  const set = (k: string, val: unknown) => (v = { ...v, [k]: val });
  const num = (k: string, d: number) => (typeof v[k] === 'number' ? (v[k] as number) : d);
  const hb = $derived((v.hitbox ?? {}) as { width?: number; height?: number });

  const MOBS: [string, string][] = [
    ['ZOMBIE', 'Zombie'], ['SKELETON', 'Skeleton'], ['SPIDER', 'Spider'], ['CREEPER', 'Creeper'], ['WITHER_SKELETON', 'Wither skeleton'], ['PIGLIN_BRUTE', 'Piglin brute'],
    ['HUSK', 'Husk'], ['STRAY', 'Stray'], ['BLAZE', 'Blaze'], ['VINDICATOR', 'Vindicator'], ['ENDERMAN', 'Enderman'], ['IRON_GOLEM', 'Iron golem'],
    ['COW', 'Cow'], ['PIG', 'Pig'], ['SHEEP', 'Sheep'], ['VILLAGER', 'Villager'], ['WOLF', 'Wolf'], ['HORSE', 'Horse'],
  ];
  const MOUNTS: [string, string, string][] = [['horse', 'Horse', 'Fast and agile'], ['donkey', 'Donkey', 'Steady and a bit slower'], ['mule', 'Mule', 'Sturdy, in between']];
  const GROWTH: [number, string][] = [[60, '1 min'], [300, '5 min'], [600, '10 min'], [1800, '30 min'], [3600, '1 hour'], [21600, '6 hours'], [86400, '1 day']];
  const hardnessText = (n: number) => (n === 0 ? 'instant' : n < 1 ? `${n.toFixed(2)} s` : n <= 3 ? `${+n.toFixed(1)} s — like dirt or wood` : n <= 10 ? `${+n.toFixed(1)} s — like stone` : n <= 40 ? `${Math.round(n)} s — like obsidian` : `${Math.round(n)} s`);
  const lines = (k: string) => (Array.isArray(v[k]) ? (v[k] as string[]) : []);
  function setLines(k: string, l: string[]) { set(k, l); }
  const addCmd = () => set('commands', [...lines('commands'), '']);
  const cmdTxt = (i: number, s: string) => set('commands', lines('commands').map((c, j) => (j === i ? s : c)));
  const delCmd = (i: number) => set('commands', lines('commands').filter((_, j) => j !== i));
</script>

<div class="cf">
  {#if kind === 'block'}
    <Slide label="Breaking time" min={0} max={60} step={0.25} color="#8bd17c" format={hardnessText} bind:value={() => num('hardness', 1.5), (n) => set('hardness', n)} hint="How long mining it takes without special tools" />
    <Slide label="Light" min={0} max={15} step={1} color="#ffd95a" format={(n) => (n === 0 ? 'no light' : `level ${n}`)} bind:value={() => num('light', 0), (n) => set('light', n)} />
    <DropEditor title="Drops" empty="Drops itself" bind:value={() => (v.drops ?? []) as any[], (d) => set('drops', d)} />
  {:else if kind === 'chest'}
    <div class="rows">
      <Slide label="Rows" min={1} max={6} step={1} color="#e0b04a" format={(n) => `${n} row${n > 1 ? 's' : ''} · ${n * 9} slots`} bind:value={() => num('rows', 3), (n) => set('rows', n)} />
      <div class="slots" aria-hidden="true">{#each Array(num('rows', 3) * 9) as _}<span></span>{/each}</div>
    </div>
    <label class="txt">Window title<input value={v.title ?? title} oninput={(e) => set('title', e.currentTarget.value)} maxlength="40" placeholder={title || 'Chest'} /></label>
  {:else if kind === 'decoration'}
    <Toggle bind:checked={() => !!v.seat, (b) => set('seat', b)} label="Players can sit on it" help="Right-click to sit, like a chair or bench" />
    <Toggle bind:checked={() => !!v.solid, (b) => set('solid', b)} label="Solid" help="Blocks movement like a block would" />
    <div class="two">
      <Slide label="Hitbox width" min={0.1} max={4} step={0.1} color="#4dd6d0" format={(n) => `${n.toFixed(1)} blocks`} bind:value={() => hb.width ?? 1, (n) => set('hitbox', { ...hb, width: n })} />
      <Slide label="Hitbox height" min={0.1} max={4} step={0.1} color="#4dd6d0" format={(n) => `${n.toFixed(1)} blocks`} bind:value={() => hb.height ?? 1, (n) => set('hitbox', { ...hb, height: n })} />
    </div>
  {:else if kind === 'npc'}
    <Toggle bind:checked={() => v.name_visible !== false, (b) => set('name_visible', b)} label="Show name above head" />
    <McLoreEditor name={title} max={8} bind:lines={() => lines('messages'), (l) => setLines('messages', l)} />
    <div class="cmds">
      <div class="h"><b>Commands on click</b><small>run as the console. <code>{'{player}'}</code> is the player who clicked</small></div>
      {#each lines('commands') as c, i (i)}
        <div class="cmd"><span>/</span><input value={c} oninput={(e) => cmdTxt(i, e.currentTarget.value)} placeholder="give {'{player}'} diamond 1" maxlength="200" /><button aria-label="Remove command" onclick={() => delCmd(i)}><X size={14} /></button></div>
      {/each}
      <button class="add" onclick={addCmd} disabled={lines('commands').length >= 8}><Plus size={14} /> Add a command</button>
    </div>
    <div class="two">
      <Slide label="Hitbox width" min={0.1} max={4} step={0.1} color="#6fb3ff" format={(n) => `${n.toFixed(1)} blocks`} bind:value={() => hb.width ?? 0.8, (n) => set('hitbox', { ...hb, width: n })} />
      <Slide label="Hitbox height" min={0.1} max={6} step={0.1} color="#6fb3ff" format={(n) => `${n.toFixed(1)} blocks`} bind:value={() => hb.height ?? 1.9, (n) => set('hitbox', { ...hb, height: n })} />
    </div>
  {:else if kind === 'vehicle'}
    <div class="tiles" role="radiogroup" aria-label="What it rides on">
      {#each MOUNTS as [id, name, blurb]}
        <button role="radio" aria-checked={(v.mount ?? 'horse') === id} class="tile" class:on={(v.mount ?? 'horse') === id} onclick={() => set('mount', id)}><b>{name}</b><small>{blurb}</small></button>
      {/each}
    </div>
    <Slide label="Speed" min={0.05} max={1} step={0.01} color="#ff8fcf" format={(n) => `${Math.round(n * 100)}%`} bind:value={() => num('speed', 0.25), (n) => set('speed', n)} hint="Steered with the movement keys, like riding a saddled horse" />
  {:else if kind === 'crop'}
    <div class="growth">
      <span class="l">Growth time</span>
      <div class="seg" role="radiogroup" aria-label="Growth time">
        {#each GROWTH as [secs, label]}<button role="radio" aria-checked={num('growth_seconds', 600) === secs} class:on={num('growth_seconds', 600) === secs} onclick={() => set('growth_seconds', secs)}>{label}</button>{/each}
      </div>
    </div>
    <Toggle bind:checked={() => v.replant !== false, (b) => set('replant', b)} label="Replants itself" help="Goes back to the first stage after harvest" />
    <DropEditor title="Harvest drops" empty="Drops nothing" bind:value={() => (v.drops ?? []) as any[], (d) => set('drops', d)} />
  {:else if kind === 'mob'}
    <div class="mobs" role="radiogroup" aria-label="Base creature">
      {#each MOBS as [id, name]}<button role="radio" aria-checked={(v.entity ?? 'ZOMBIE') === id} class="mob" class:on={(v.entity ?? 'ZOMBIE') === id} onclick={() => set('entity', id)}>{name}</button>{/each}
    </div>
    <div class="two">
      <Slide label="Health" min={1} max={500} step={1} color="#ff5d8f" format={(n) => `${n} hp · ${n / 2} hearts`} bind:value={() => num('health', 20), (n) => set('health', n)} />
      <Slide label="Damage" min={0} max={100} step={0.5} color="#ff6b6b" bind:value={() => num('damage', 2), (n) => set('damage', n)} />
      <Slide label="Armor" min={0} max={30} step={1} color="#9aa7c7" bind:value={() => num('armor', 0), (n) => set('armor', n)} />
      <Slide label="Speed" min={0} max={1} step={0.01} color="#5ad1ff" format={(n) => (n === 0 ? 'normal' : `${Math.round(n * 100)}%`)} bind:value={() => num('speed', 0), (n) => set('speed', n)} />
    </div>
    <DropEditor title="Drops" empty="No extra drops" bind:value={() => (v.drops ?? []) as any[], (d) => set('drops', d)} />
  {/if}
</div>

<style>
  .cf { display: grid; gap: 16px; }
  .two { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 16px; }
  .rows { display: grid; gap: 10px; }
  .slots { display: grid; grid-template-columns: repeat(9, 1fr); gap: 3px; max-width: 280px; }
  .slots span { aspect-ratio: 1; border-radius: 3px; background: #8b8b8b; box-shadow: inset 1.5px 1.5px 0 #373737, inset -1.5px -1.5px 0 #fff8; }
  .txt { display: grid; gap: 4px; font-size: 0.8rem; color: var(--muted); max-width: 20rem; }
  .cmds { display: grid; gap: 8px; }
  .cmds .h { display: grid; gap: 2px; }
  .cmds small { color: var(--muted); }
  .cmd { display: flex; align-items: center; gap: 6px; padding: 4px 6px 4px 12px; border-radius: 10px; background: var(--surface-2); border: 1px solid var(--line); }
  .cmd span { color: var(--muted); font-family: var(--mono); }
  .cmd input { border: 0; background: none; flex: 1; font-family: var(--mono); font-size: 0.85rem; outline: none; padding: 6px 0; }
  .cmd button { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 7px; border: 0; background: transparent; color: var(--muted); cursor: pointer; }
  .cmd button:hover { background: var(--surface-3); color: var(--bad); }
  .add { justify-self: start; display: inline-flex; align-items: center; gap: 6px; padding: 7px 13px; border-radius: 9px; border: 1px dashed var(--line-strong); background: transparent; color: var(--text-2); cursor: pointer; }
  .add:hover:not(:disabled) { border-color: var(--accent); color: var(--accent-2); }
  .tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 8px; }
  .tile { display: grid; gap: 2px; text-align: left; padding: 11px 13px; border-radius: 12px; border: 1px solid var(--line); background: var(--surface-2); color: inherit; cursor: pointer; transition: border-color 0.12s, background 0.12s; }
  .tile small { color: var(--muted); }
  .tile:hover { border-color: var(--accent); }
  .tile.on { border-color: var(--accent); background: var(--accent-soft); }
  .growth { display: grid; gap: 7px; }
  .l { font-size: 0.82rem; color: var(--text-2); }
  .mobs { display: flex; gap: 6px; flex-wrap: wrap; }
  .mob { padding: 6px 13px; border-radius: 999px; border: 1px solid var(--line); background: var(--surface-2); color: var(--text-2); font-size: 0.8rem; cursor: pointer; transition: all 0.12s; }
  .mob:hover { border-color: #e5484d; }
  .mob.on { background: color-mix(in srgb, #e5484d 20%, transparent); border-color: #e5484d; color: var(--text); font-weight: 600; }
</style>
