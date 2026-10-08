<script lang="ts">
  import { onMount } from 'svelte';
  import { Crosshair, Skull, EyeOff, Search, Trash2, Shield, Info, Trophy } from '@lucide/svelte';
  import Avatar from './Avatar.svelte';
  import BetInput from './BetInput.svelte';
  import { cget, cpost, money, ago, untilText, type CasinoState } from './casino';
  import { toast, errorText } from './host';

  type Board = { uuid: string; name: string; total: number; count: number; by: string | null; me: boolean };
  type Mine = { id: number; target: string; paid: number; reward: number; at: string; expires_at: string | null };
  type Data = {
    enabled: boolean; board: Board[]; mine: Mine[]; recent: { target: string; killer: string; total: number; at: string }[];
    rules: { min: number; max: number; tax_percent: number; expire_days: number; max_active: number; allow_anonymous: boolean; allow_cancel: boolean };
  };
  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  let data = $state<Data | null>(null);
  let query = $state('');
  let found = $state<{ uuid: string; name: string; online: boolean }[]>([]);
  let target = $state<{ uuid: string; name: string } | null>(null);
  let amount = $state(500);
  let anonymous = $state(false);
  let busy = $state(false);
  let now = $state(Date.now());
  const rules = $derived(data?.rules);
  const reward = $derived(Math.round(amount * (1 - (rules?.tax_percent ?? 0) / 100) * 100) / 100);

  async function load() { try { data = await cget<Data>(st.server.id, '/bounties'); if (data && amount < data.rules.min) amount = data.rules.min; } catch (e) { toast(errorText(e), 'error'); } }
  onMount(() => { void load(); const a = setInterval(load, 20000), b = setInterval(() => (now = Date.now()), 1000); return () => { clearInterval(a); clearInterval(b); }; });

  let t: ReturnType<typeof setTimeout>;
  function search() {
    clearTimeout(t);
    t = setTimeout(async () => {
      try { found = (await cget<{ players: typeof found }>(st.server.id, `/players?q=${encodeURIComponent(query.trim().replace(/[^A-Za-z0-9_]/g, ''))}`)).players.filter((p) => p.uuid !== st.me.uuid); } catch { found = []; }
    }, 200);
  }
  $effect(() => { void query; search(); });

  async function place() {
    if (!target) return;
    busy = true;
    try {
      await cpost<{ balance: number }>(st.server.id, '/bounties', { target: target.name, amount, anonymous: anonymous && !!rules?.allow_anonymous }).then((r) => onbalance(r.balance));
      toast(`Bounty placed on ${target.name}`, 'ok');
      target = null; query = ''; onplayed(); await load();
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }
  async function withdraw(id: number) {
    try { const r = await cpost<{ balance: number; refunded: number }>(st.server.id, `/bounties/${id}/cancel`); onbalance(r.balance); toast(`Withdrawn. ${money(r.refunded)} came back.`, 'ok'); onplayed(); await load(); } catch (e) { toast(errorText(e), 'error'); }
  }
</script>

<div class="wrap">
  {#if st.bounty_on_me > 0}
    <div class="onme"><Skull size={20} /><div><b>There's a {money(st.bounty_on_me)} bounty on your head.</b><span>Anyone who defeats you in PvP collects it. Stay sharp.</span></div></div>
  {/if}

  <div class="cols">
    <section class="panel place">
      <h2><Crosshair size={18} /> Put a price on someone</h2>
      {#if data && !data.enabled}
        <p class="note">Bounties are switched off right now.</p>
      {:else}
        {#if target}
          <div class="chosen"><Avatar name={target.name} uuid={target.uuid} size={2.4} /><b>{target.name}</b><button class="ghost sm" onclick={() => (target = null)}>Change</button></div>
        {:else}
          <label class="search"><Search size={15} /><input placeholder="Find a player…" bind:value={query} aria-label="Find a player" /></label>
          <div class="results">
            {#each found as p (p.uuid)}
              <button class="player" onclick={() => (target = p)}><Avatar name={p.name} uuid={p.uuid} size={1.9} /><span>{p.name}</span>{#if p.online}<i class="on" title="Online"></i>{/if}</button>
            {:else}<p class="note">{query ? 'Nobody by that name has played here.' : 'Type a name, or pick from recent players.'}</p>{/each}
          </div>
        {/if}
        <BetInput bind:value={amount} min={rules?.min ?? 1} max={rules?.max ?? 1000000} {balance} label="Bounty" disabled={!target} />
        <div class="math">
          <span>The reward will be</span><b>{money(reward)}</b>
          <small>{rules?.tax_percent ?? 0}% goes to the house when it's placed.{rules?.expire_days ? ` Uncollected bounties expire after ${rules.expire_days} days and come back to you.` : ''}</small>
        </div>
        {#if rules?.allow_anonymous}<label class="check"><input type="checkbox" bind:checked={anonymous} /> <EyeOff size={14} /> Keep my name hidden</label>{/if}
        <button class="primary go" onclick={place} disabled={busy || !target || (balance != null && balance < amount)}>Place bounty · {money(amount)}</button>
      {/if}
    </section>

    <section class="panel board">
      <h2><Trophy size={18} /> Most wanted</h2>
      {#if data?.board.length}
        <ol>
          {#each data.board as b, i (b.uuid)}
            <li class:me={b.me}>
              <span class="rank">{i + 1}</span>
              <Avatar name={b.name} uuid={b.uuid} size={2.4} />
              <div class="who"><b>{b.name}{b.me ? ' (you)' : ''}</b><small>{b.count} bount{b.count === 1 ? 'y' : 'ies'}{b.by ? ' · ' + b.by : ' · anonymous'}</small></div>
              <span class="total">{money(b.total)}</span>
            </li>
          {/each}
        </ol>
      {:else}<p class="note"><Shield size={14} /> No bounties out. The server is at peace… for now.</p>{/if}
    </section>
  </div>

  <div class="cols">
    <section class="panel">
      <h2>Your bounties</h2>
      {#if data?.mine.length}
        <ul class="list">
          {#each data.mine as m (m.id)}
            <li><div><b>{m.target}</b><small>{money(m.reward)} reward · {ago(m.at, now)}{m.expires_at ? ' · expires in ' + untilText(m.expires_at, now) : ''}</small></div>
              {#if data.rules.allow_cancel}<button class="ghost sm" onclick={() => withdraw(m.id)}><Trash2 size={13} /> Withdraw</button>{/if}</li>
          {/each}
        </ul>
      {:else}<p class="note"><Info size={13} /> You haven't put a bounty on anyone.</p>{/if}
    </section>
    <section class="panel">
      <h2>Recently collected</h2>
      {#if data?.recent.length}
        <ul class="list">{#each data.recent as r}<li><div><b>{r.killer}</b> took down <b>{r.target}</b><small>{ago(r.at, now)}</small></div><span class="amt">+{money(r.total)}</span></li>{/each}</ul>
      {:else}<p class="note">Nobody has collected a bounty yet.</p>{/if}
    </section>
  </div>
</div>

<style>
  .wrap { display: flex; flex-direction: column; gap: 1rem; }
  .cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(22rem, 1fr)); gap: 1rem; align-items: start; }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.85rem; }
  h2 { display: flex; gap: 0.5rem; align-items: center; font-size: 1.02rem; }
  .onme { display: flex; gap: 0.8rem; align-items: center; padding: 0.85rem 1.1rem; border-radius: var(--radius); background: linear-gradient(90deg, color-mix(in srgb, #ef4444 24%, transparent), transparent); border: 1px solid color-mix(in srgb, #ef4444 50%, transparent); color: #fca5a5; }
  .onme div { display: flex; flex-direction: column; } .onme b { color: #fecaca; } .onme span { font-size: 0.82rem; color: var(--muted); }
  .note { font-size: 0.82rem; color: var(--muted); display: flex; gap: 0.35rem; align-items: center; }
  .search { position: relative; display: flex; align-items: center; } .search :global(svg) { position: absolute; left: 0.75rem; color: var(--muted); } .search input { padding-left: 2.2rem; }
  .results { display: grid; grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr)); gap: 0.4rem; max-height: 9.5rem; overflow: auto; }
  .player { display: flex; align-items: center; gap: 0.5rem; padding: 0.4rem 0.55rem; border-radius: var(--radius-sm); border: 1px solid var(--line); background: color-mix(in srgb, var(--text) 4%, transparent); color: var(--text); text-align: left; position: relative; }
  .player:hover { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 14%, transparent); }
  .player span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.86rem; }
  .on { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: var(--success); margin-left: auto; box-shadow: 0 0 6px var(--success); }
  .chosen { display: flex; gap: 0.7rem; align-items: center; padding: 0.55rem 0.8rem; border-radius: var(--radius-sm); background: color-mix(in srgb, #ef4444 12%, transparent); border: 1px solid color-mix(in srgb, #ef4444 40%, transparent); }
  .chosen .ghost { margin-left: auto; }
  .math { display: grid; grid-template-columns: 1fr auto; gap: 0.15rem 0.6rem; align-items: baseline; padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 5%, transparent); font-size: 0.85rem; }
  .math b { font-size: 1.3rem; font-variant-numeric: tabular-nums; color: #fca5a5; } .math small { grid-column: 1 / -1; color: var(--muted); font-size: 0.75rem; }
  .check { display: flex; gap: 0.45rem; align-items: center; font-size: 0.85rem; }
  .go { padding: 0.8rem; font-weight: 700; border: none; border-radius: var(--radius); background: linear-gradient(135deg, #ef4444, #b91c1c); color: #fff; box-shadow: 0 6px 22px -8px #ef4444; }
  .go:hover:not(:disabled) { filter: brightness(1.1); }
  ol { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.4rem; max-height: 26rem; overflow: auto; }
  ol li { display: flex; align-items: center; gap: 0.7rem; padding: 0.5rem 0.7rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 4%, transparent); border: 1px solid transparent; }
  ol li:nth-child(1) { border-color: color-mix(in srgb, #f5d97a 55%, transparent); background: linear-gradient(90deg, color-mix(in srgb, #f5d97a 14%, transparent), transparent); }
  ol li.me { border-color: color-mix(in srgb, #ef4444 55%, transparent); }
  .rank { width: 1.4rem; text-align: center; font-weight: 800; color: var(--muted); } li:first-child .rank { color: #f5d97a; }
  .who { display: flex; flex-direction: column; min-width: 0; flex: 1; } .who b, .who small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } small { color: var(--muted); font-size: 0.75rem; }
  .total { font-weight: 800; font-size: 1.05rem; font-variant-numeric: tabular-nums; color: #fca5a5; }
  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.35rem; }
  .list li { display: flex; justify-content: space-between; align-items: center; gap: 0.8rem; padding: 0.5rem 0.2rem; border-bottom: 1px solid var(--line); } .list li div { display: flex; flex-direction: column; }
  .amt { color: var(--success); font-weight: 700; font-variant-numeric: tabular-nums; }
</style>
