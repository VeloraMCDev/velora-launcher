<script lang="ts">
  import { Lock, ShieldCheck, LoaderCircle } from '@lucide/svelte';
  import { get, put } from '../../lib/api';
  import { toast, toastError } from '../../lib/toast.svelte';

  let { guildId }: { guildId: string } = $props();

  type Rule = { id: string; label: string; help: string; group: string; default: boolean; editable: boolean };
  type Rules = { flags: Record<string, boolean>; catalog: Rule[]; can_edit: boolean };

  let rules = $state<Rules | null>(null);
  let error = $state('');
  let saving = $state<string | null>(null);

  async function load() {
    try {
      rules = await get<Rules>(`/api/v1/guilds/${guildId}/claim-flags`);
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
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
    const next = !rules.flags[rule.id];
    saving = rule.id;
    const before = rules.flags[rule.id];
    rules.flags[rule.id] = next; // show it at once, undo if the server says no
    try {
      rules = await put<Rules>(`/api/v1/guilds/${guildId}/claim-flags`, { flags: { [rule.id]: next } });
      toast(`${rule.label}: ${next ? 'on' : 'off'}`);
    } catch (e) {
      if (rules) rules.flags[rule.id] = before;
      toastError(e);
    } finally {
      saving = null;
    }
  }
  async function reset() {
    if (!rules?.can_edit || !confirm('Put every rule back to its default?')) return;
    const flags = Object.fromEntries(rules.catalog.filter((r) => r.editable).map((r) => [r.id, r.default]));
    try {
      rules = await put<Rules>(`/api/v1/guilds/${guildId}/claim-flags`, { flags });
      toast('Land rules reset');
    } catch (e) {
      toastError(e);
    }
  }
</script>

<section class="pl-card rules">
  <div class="pl-card-head">
    <h2><ShieldCheck size={16} /> Land rules</h2>
    {#if rules?.can_edit && changed}<button class="more" onclick={reset}>Reset to defaults</button>{/if}
  </div>
  {#if error}
    <p class="sub">{error}</p>
  {:else if !rules}
    <div class="pl-skel" style="height:140px"></div>
  {:else}
    <p class="sub intro">
      {#if rules.can_edit}Decide what happens on your guild's land. Your members are never held back by the visitor rules. Changes reach the game within seconds.
      {:else}These are your guild's land rules. The leader and officers can change them.{/if}
    </p>
    {#each groups as g (g.name)}
      <div class="grp">
        <h3>{g.name}</h3>
        {#each g.rules as r (r.id)}
          <button type="button" class="rule" role="switch" aria-checked={rules.flags[r.id]} disabled={!rules.can_edit || !r.editable} class:on={rules.flags[r.id]} onclick={() => toggle(r)}>
            <span class="txt"><b>{r.label}{#if !r.editable} <Lock size={12} aria-label="Managed by the server" />{/if}</b><small>{r.editable ? r.help : 'The server admins manage this rule.'}</small></span>
            <span class="sw" aria-hidden="true">{#if saving === r.id}<LoaderCircle size={12} class="spin" />{:else}<i></i>{/if}</span>
          </button>
        {/each}
      </div>
    {/each}
  {/if}
</section>

<style>
  .rules .intro { margin-bottom: 8px; }
  .grp { display: flex; flex-direction: column; gap: 6px; margin-top: 14px; }
  .grp h3 { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--muted); font-weight: 600; }
  .rule { display: flex; align-items: center; justify-content: space-between; gap: 14px; text-align: left; width: 100%; padding: 11px 13px; border-radius: 14px; border: 1px solid var(--pl-line); background: var(--pl-glass-2); }
  .rule:hover:not(:disabled) { border-color: var(--line-strong); }
  .rule:disabled { cursor: default; opacity: 0.75; }
  .txt { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .txt b { font-weight: 600; display: inline-flex; align-items: center; gap: 6px; }
  .txt small { color: var(--muted); font-weight: 400; line-height: 1.4; }
  .sw { position: relative; flex: none; width: 38px; height: 22px; border-radius: 99px; background: var(--surface-3); display: grid; place-items: center; transition: background 0.15s; }
  .sw i { position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: #fff; transition: transform 0.15s; }
  .rule.on .sw { background: var(--good); }
  .rule.on .sw i { transform: translateX(16px); }
  .sw :global(.spin) { animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
