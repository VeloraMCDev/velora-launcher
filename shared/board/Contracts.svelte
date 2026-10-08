<script lang="ts">
  import { onMount } from 'svelte';
  import { Swords, Package, Flame, Clock, RefreshCw, Trophy, Info, CheckCircle2 } from '@lucide/svelte';
  import { bget, bpost, errorText, money, toast, untilText, type Contract, type ContractsBoard } from './host';

  let { serverId, onbalance }: { serverId: number; onbalance?: (v: number | null) => void } = $props();
  let board = $state<ContractsBoard | null>(null);
  let error = $state('');
  let busy = $state(false);
  let now = $state(Date.now());

  async function load() {
    try { board = await bget<ContractsBoard>(serverId, '/contracts'); if (board.balance !== undefined) onbalance?.(board.balance); error = ''; } catch (e) { error = errorText(e); }
  }
  onMount(() => { void load(); const t = setInterval(() => (now = Date.now()), 1000); const p = setInterval(() => void load(), 15000); return () => { clearInterval(t); clearInterval(p); }; });
  $effect(() => { void serverId; board = null; void load(); });

  async function swap(c: Contract) {
    busy = true;
    try { board = await bpost<ContractsBoard>(serverId, `/contracts/${c.id}/abandon`); toast('Swapped for a new contract.', 'ok'); } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }
  const pct = (c: Contract) => Math.min(100, Math.round((c.progress / c.required) * 100));
  const s = $derived(board?.stats);
</script>

<div class="contracts">
  {#if error && !board}<div class="banner err" role="alert">{error}</div>{/if}
  {#if board && !board.enabled}<div class="banner">Contracts are switched off on this server.</div>{/if}

  {#if board?.enabled}
    <div class="stats">
      <div><span>Finished today</span><b>{s?.done_today ?? 0}{#if s && s.daily_limit > 0}<small> / {s.daily_limit}</small>{/if}</b></div>
      <div><span>Swaps left today</span><b>{s?.rerolls_left ?? 0}</b></div>
      <div><span>All time</span><b>{s?.completed ?? 0}</b></div>
      <div><span>Earned</span><b class="g">{money(s?.earned ?? 0)}</b></div>
    </div>

    <section class="panel how">
      <h2><Info size={16} /> How contracts work</h2>
      <p>Your board is yours alone: a handful of random jobs that pay real money. <b>Kill</b> contracts count on their own as you play. <b>Resource</b> contracts need the actual items: gather them, then run <code>/contracts submit</code> in game to hand them in (they're used up). Finishing contracts makes room for new ones{#if s && s.daily_limit > 0}, up to {s.daily_limit} a day{/if}.</p>
    </section>

    {#if s?.limit_reached}<div class="banner">You've finished today's limit of {s.daily_limit} contracts. Fresh ones arrive tomorrow (UTC).</div>{/if}

    <div class="grid">
      {#each board.contracts as c (c.id)}
        <article class="card" class:hot={c.bonus} class:done={c.progress >= c.required}>
          {#if c.bonus}<span class="hotbadge"><Flame size={12} /> Hot contract · extra pay</span>{/if}
          <div class="head">
            <span class="ico {c.kind}">{#if c.kind === 'kill'}<Swords size={22} />{:else}<Package size={22} />{/if}</span>
            <div class="t"><b>{c.title}</b><small>{c.kind === 'kill' ? 'Counts automatically as you play' : 'Hand in with /contracts submit'}</small></div>
          </div>
          <div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax={c.required} aria-valuenow={c.progress}><i style="width:{pct(c)}%"></i><span>{c.progress.toLocaleString()} / {c.required.toLocaleString()}</span></div>
          <div class="foot">
            <span class="reward"><Trophy size={14} /> {money(c.reward)}</span>
            <span class="time"><Clock size={12} /> {untilText(c.expires_at, now)}</span>
            <button class="ghost" onclick={() => swap(c)} disabled={busy || (s?.rerolls_left ?? 0) <= 0} title="Throw this one away for a fresh contract"><RefreshCw size={13} /> Swap</button>
          </div>
        </article>
      {:else}
        <p class="muted">{s?.limit_reached ? 'All done for today. Come back tomorrow!' : 'Generating your contracts…'}</p>
      {/each}
    </div>

    {#if board.recent?.length}
      <section class="panel">
        <h2><CheckCircle2 size={16} /> Recently finished</h2>
        <ul class="hist">{#each board.recent as c (c.id)}<li><span>{c.title}</span><b class="g">+{money(c.reward)}</b></li>{/each}</ul>
      </section>
    {/if}
  {/if}
</div>

<style>
  .contracts { display: flex; flex-direction: column; gap: 1rem; }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.7rem; }
  h2 { display: flex; gap: 0.45rem; align-items: center; font-size: 1rem; } .how p { margin: 0; font-size: 0.9rem; color: color-mix(in srgb, var(--text) 82%, transparent); line-height: 1.5; }
  code { font-family: var(--mono, monospace); background: color-mix(in srgb, var(--text) 10%, transparent); padding: 0.05rem 0.4rem; border-radius: 0.35rem; font-size: 0.85em; }
  .stats { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.6rem; } .stats div { padding: 0.6rem 0.9rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 5%, transparent); display: flex; flex-direction: column; min-width: 0; }
  .stats span { font-size: 0.68rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; } .stats b { font-size: 1.25rem; font-variant-numeric: tabular-nums; } .stats small { font-weight: 500; color: var(--muted); font-size: 0.8rem; } .g { color: #4ade80; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(19rem, 1fr)); gap: 0.8rem; }
  .card { position: relative; display: flex; flex-direction: column; gap: 0.8rem; padding: 1rem; border-radius: var(--radius); background: linear-gradient(160deg, color-mix(in srgb, var(--accent) 10%, var(--surface)), var(--surface)); border: 1px solid var(--line-strong); transition: transform 0.15s var(--ease, ease), box-shadow 0.15s; }
  .card:hover { transform: translateY(-2px); box-shadow: 0 14px 34px -18px var(--accent); }
  .card.hot { border-color: #f59e0b88; background: linear-gradient(160deg, #f59e0b1f, var(--surface)); box-shadow: 0 0 40px -22px #f59e0b; } .card.done { border-color: #22c55e88; }
  .hotbadge { position: absolute; top: -0.55rem; right: 0.8rem; display: inline-flex; gap: 0.25rem; align-items: center; font-size: 0.68rem; font-weight: 800; padding: 0.15rem 0.6rem; border-radius: 99rem; background: linear-gradient(135deg, #f59e0b, #ec4899); color: #fff; }
  .head { display: flex; gap: 0.75rem; align-items: center; } .t { display: flex; flex-direction: column; min-width: 0; } .t b { font-size: 1.02rem; } .t small { color: var(--muted); font-size: 0.74rem; }
  .ico { display: grid; place-items: center; width: 2.8rem; height: 2.8rem; border-radius: 0.8rem; flex: none; } .ico.kill { background: #ef444430; color: #f87171; } .ico.gather { background: #38bdf830; color: #7dd3fc; }
  .prog { position: relative; height: 1.3rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 10%, transparent); overflow: hidden; } .prog i { position: absolute; inset: 0 auto 0 0; background: linear-gradient(90deg, #22c55e, #4ade80); border-radius: 99rem; transition: width 0.6s var(--ease, ease); }
  .prog span { position: relative; display: block; text-align: center; font-size: 0.74rem; font-weight: 700; line-height: 1.3rem; text-shadow: 0 1px 2px #0008; font-variant-numeric: tabular-nums; }
  .foot { display: flex; align-items: center; gap: 0.7rem; } .reward { display: inline-flex; gap: 0.3rem; align-items: center; font-weight: 800; color: #fbbf24; font-size: 1.05rem; } .time { display: inline-flex; gap: 0.25rem; align-items: center; color: var(--muted); font-size: 0.76rem; margin-right: auto; }
  .ghost { display: inline-flex; gap: 0.3rem; align-items: center; padding: 0.35rem 0.7rem; font-size: 0.78rem; }
  .hist { list-style: none; margin: 0; padding: 0; } .hist li { display: flex; justify-content: space-between; padding: 0.4rem 0; border-bottom: 1px solid var(--line); font-size: 0.86rem; }
  .muted { color: var(--muted); font-size: 0.88rem; } .banner { padding: 0.7rem 1rem; border-radius: var(--radius-sm); font-size: 0.88rem; background: color-mix(in srgb, var(--text) 8%, transparent); } .banner.err { border: 1px solid var(--danger, #e5484d); }
  @media (max-width: 640px) { .stats { grid-template-columns: repeat(2, 1fr); } .grid { grid-template-columns: minmax(0, 1fr); } }
</style>
