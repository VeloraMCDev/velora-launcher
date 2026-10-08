<script lang="ts">
  import { ShieldCheck, Lock, LoaderCircle } from '@lucide/svelte';
  import { invoke } from '../lib/tauri';
  import { toast } from '../lib/store.svelte';

  let { guildId }: { guildId: string } = $props();

  type Rule = { id: string; label: string; help: string; group: string; default: boolean; editable: boolean };
  type Rules = { flags: Record<string, boolean>; catalog: Rule[]; can_edit: boolean };

  let rules = $state<Rules | null>(null);
  let error = $state('');
  let saving = $state<string | null>(null);

  async function load() {
    try { rules = await invoke<Rules>('get_guild_claim_flags', { guildId }); error = ''; }
    catch (e) { error = String(e); }
  }
  $effect(() => { void guildId; void load(); });

  const groups = $derived.by(() => {
    const out: { name: string; rules: Rule[] }[] = [];
    for (const r of rules?.catalog ?? []) {
      let g = out.find((x) => x.name === r.group);
      if (!g) out.push((g = { name: r.group, rules: [] }));
      g.rules.push(r);
    }
    return out;
  });
  const changed = $derived((rules?.catalog ?? []).filter((r) => rules!.flags[r.id] !== r.default).length);

  async function toggle(rule: Rule) {
    if (!rules || !rules.can_edit || !rule.editable || saving) return;
    const before = rules.flags[rule.id];
    const next = !before;
    saving = rule.id;
    rules.flags[rule.id] = next; // shown at once, undone if the server refuses
    try {
      rules = await invoke<Rules>('set_guild_claim_flags', { guildId, flags: { [rule.id]: next } });
      toast(`${rule.label}: ${next ? 'on' : 'off'}`);
    } catch (e) {
      if (rules) rules.flags[rule.id] = before;
      toast(String(e), 'error');
    } finally { saving = null; }
  }
  async function reset() {
    if (!rules?.can_edit || !confirm('Put every rule back to its default?')) return;
    const flags = Object.fromEntries(rules.catalog.filter((r) => r.editable).map((r) => [r.id, r.default]));
    try { rules = await invoke<Rules>('set_guild_claim_flags', { guildId, flags }); toast('Land rules reset'); }
    catch (e) { toast(String(e), 'error'); }
  }
</script>

<section class="card rules glass">
  <div class="head">
    <h3><ShieldCheck size={16} /> Land rules</h3>
    {#if rules?.can_edit && changed}<button class="ghost sm" onclick={reset}>Reset to defaults</button>{/if}
  </div>
  {#if error}
    <p class="tiny muted">{error}</p>
  {:else if !rules}
    <p class="tiny muted">Loading…</p>
  {:else}
    <p class="tiny muted">
      {#if rules.can_edit}Decide what happens on your guild's land. Members are never held back by the visitor rules. Changes reach the game within seconds.
      {:else}Your guild's land rules. The leader and officers can change them.{/if}
    </p>
    {#each groups as g (g.name)}
      <div class="grp">
        <h4>{g.name}</h4>
        {#each g.rules as r (r.id)}
          <button type="button" class="rule" class:on={rules.flags[r.id]} role="switch" aria-checked={rules.flags[r.id]} disabled={!rules.can_edit || !r.editable} onclick={() => toggle(r)}>
            <span class="txt"><b>{r.label}{#if !r.editable} <Lock size={12} aria-label="Managed by the server" />{/if}</b><small>{r.editable ? r.help : 'The server admins manage this rule.'}</small></span>
            <span class="sw" aria-hidden="true">{#if saving === r.id}<LoaderCircle size={12} class="spin" />{:else}<i></i>{/if}</span>
          </button>
        {/each}
      </div>
    {/each}
  {/if}
</section>

<style>
  .rules { display: flex; flex-direction: column; gap: 0.5rem; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 0.6rem; }
  .head h3 { display: flex; align-items: center; gap: 0.5rem; font-size: 1rem; margin: 0; }
  .grp { display: flex; flex-direction: column; gap: 0.35rem; margin-top: 0.6rem; }
  .grp h4 { margin: 0; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 600; }
  .rule { display: flex; align-items: center; justify-content: space-between; gap: 0.9rem; text-align: left; width: 100%; padding: 0.65rem 0.8rem; border-radius: 0.7rem; border: 1px solid var(--line); background: color-mix(in srgb, var(--surface-2) 70%, transparent); }
  .rule:hover:not(:disabled) { border-color: var(--line-strong); }
  .rule:disabled { cursor: default; opacity: 0.75; }
  .txt { display: flex; flex-direction: column; gap: 0.15rem; min-width: 0; }
  .txt b { font-weight: 600; display: inline-flex; align-items: center; gap: 0.4rem; }
  .txt small { color: var(--muted); font-weight: 400; line-height: 1.4; }
  .sw { position: relative; flex: none; width: 2.4rem; height: 1.35rem; border-radius: 99px; background: var(--surface-3); display: grid; place-items: center; transition: background 0.15s; }
  .sw i { position: absolute; top: 0.19rem; left: 0.19rem; width: 0.97rem; height: 0.97rem; border-radius: 50%; background: #fff; transition: transform 0.15s; }
  .rule.on .sw { background: var(--good); }
  .rule.on .sw i { transform: translateX(1.05rem); }
  .sw :global(.spin) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
