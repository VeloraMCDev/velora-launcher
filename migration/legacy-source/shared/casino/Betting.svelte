<script lang="ts">
  import { onMount } from 'svelte';
  import { Swords, Plus, Search, Clock, Lock, Check, X as XIcon, Ban } from '@lucide/svelte';
  import Avatar from './Avatar.svelte';
  import BetInput from './BetInput.svelte';
  import { cget, cpost, money, mult, untilText, ago, type CasinoState } from './casino';
  import { toast, errorText } from './host';

  type Market = {
    id: number; subject_uuid: string; subject: string; creator: string; metric: string; metric_label: string; threshold: number; progress: number;
    locks_at: string; ends_at: string; status: string; outcome: string | null; yes_pool: number; no_pool: number; bettors: number;
    odds_yes: number; odds_no: number; mine: { side: string; stake: number; payout: number | null }[]; mine_creator: boolean;
  };
  type Data = {
    enabled: boolean; open: Market[]; done: Market[]; can_create: boolean; my_open: number;
    rules: { metrics: { id: string; label: string }[]; windows: number[]; lock_minutes: number; min_stake: number; max_stake: number; rake_percent: number; max_open: number; max_threshold: number; creators: string };
  };
  let { st, balance, onbalance, onplayed }: { st: CasinoState; balance: number | null; onbalance: (v: number) => void; onplayed: () => void } = $props();
  let data = $state<Data | null>(null);
  let now = $state(Date.now());
  let openId = $state<number | null>(null);
  let stake = $state(50);
  let creating = $state(false);
  let busy = $state(false);
  // new-bet form
  let query = $state('');
  let found = $state<{ uuid: string; name: string; online: boolean }[]>([]);
  let subject = $state<{ uuid: string; name: string } | null>(null);
  let metric = $state('player_kills');
  let threshold = $state(3);
  let windowMin = $state(60);

  async function load() { try { data = await cget<Data>(st.server.id, '/markets'); if (data) { if (!data.rules.metrics.some((m) => m.id === metric)) metric = data.rules.metrics[0]?.id ?? metric; if (!data.rules.windows.includes(windowMin)) windowMin = data.rules.windows[0]; if (stake < data.rules.min_stake) stake = data.rules.min_stake; } } catch (e) { toast(errorText(e), 'error'); } }
  onMount(() => { void load(); const a = setInterval(load, 15000), b = setInterval(() => (now = Date.now()), 1000); return () => { clearInterval(a); clearInterval(b); }; });
  let timer: ReturnType<typeof setTimeout>;
  $effect(() => { const q = query; clearTimeout(timer); timer = setTimeout(async () => { try { found = (await cget<{ players: typeof found }>(st.server.id, `/players?q=${encodeURIComponent(q.trim().replace(/[^A-Za-z0-9_]/g, ''))}`)).players; } catch { found = []; } }, 200); });

  const windowText = (m: number) => (m < 60 ? `${m} minutes` : m < 1440 ? `${m / 60} hour${m === 60 ? '' : 's'}` : `${m / 1440} day${m === 1440 ? '' : 's'}`);
  const closed = (m: Market) => new Date(m.locks_at).getTime() <= now;
  const mineSide = (m: Market) => m.mine[0]?.side ?? null;
  const share = (m: Market) => { const t = m.yes_pool + m.no_pool; return t ? (m.yes_pool / t) * 100 : 50; };

  async function create() {
    if (!subject) return;
    busy = true;
    try {
      await cpost(st.server.id, '/markets', { subject: subject.name, metric, threshold, window_minutes: windowMin });
      toast('Bet opened. Anyone can back Yes or No now.', 'ok');
      creating = false; subject = null; query = ''; await load();
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }
  async function place(m: Market, side: 'yes' | 'no') {
    busy = true;
    try {
      const r = await cpost<{ balance: number }>(st.server.id, `/markets/${m.id}/bet`, { side, stake });
      onbalance(r.balance); onplayed();
      toast(`${money(stake)} on ${side === 'yes' ? 'Yes' : 'No'}`, 'ok');
      await load();
    } catch (e) { toast(errorText(e), 'error'); } finally { busy = false; }
  }
  async function cancel(m: Market) {
    try { await cpost(st.server.id, `/markets/${m.id}/cancel`); toast('Bet cancelled, stakes returned', 'ok'); await load(); } catch (e) { toast(errorText(e), 'error'); }
  }
  const result = (m: Market) => (m.status === 'void' ? 'Refunded' : m.outcome === 'yes' ? 'Yes won' : 'No won');
</script>

<div class="wrap">
  <header class="intro">
    <div>
      <h2><Swords size={20} /> Bet on players</h2>
      <p>Will someone reach a target before time runs out? Back <b>Yes</b> or <b>No</b>. The winning side splits the whole pool{data ? ` (the house keeps ${data.rules.rake_percent}%)` : ''}. If nobody takes the other side, everyone is refunded.</p>
    </div>
    {#if data?.enabled && data.can_create}<button class="primary" onclick={() => (creating = !creating)}><Plus size={15} /> {creating ? 'Close' : 'Open a bet'}</button>{/if}
  </header>

  {#if data && !data.enabled}<p class="note">Betting is switched off right now.</p>{/if}

  {#if creating && data}
    <section class="panel create">
      <h3>New bet</h3>
      <div class="step">
        <span class="lbl">1 · Who?</span>
        {#if subject}
          <div class="chosen"><Avatar name={subject.name} uuid={subject.uuid} size={2.2} /><b>{subject.name}</b><button class="ghost sm" onclick={() => (subject = null)}>Change</button></div>
        {:else}
          <label class="search"><Search size={15} /><input placeholder="Find a player…" bind:value={query} /></label>
          <div class="results">{#each found.filter((p) => p.uuid !== st.me.uuid) as p (p.uuid)}<button class="player" onclick={() => (subject = p)}><Avatar name={p.name} uuid={p.uuid} size={1.8} /><span>{p.name}</span>{#if p.online}<i class="on"></i>{/if}</button>{/each}</div>
        {/if}
      </div>
      <div class="step">
        <span class="lbl">2 · What will they do?</span>
        <div class="chips">{#each data.rules.metrics as m}<button class:on={metric === m.id} onclick={() => (metric = m.id)}>{m.label}</button>{/each}</div>
        <div class="num">
          <button class="ghost sm" onclick={() => (threshold = Math.max(1, threshold - 1))} aria-label="Fewer">−</button>
          <input type="number" min="1" max={data.rules.max_threshold} bind:value={threshold} aria-label="Target" />
          <button class="ghost sm" onclick={() => (threshold = Math.min(data!.rules.max_threshold, threshold + 1))} aria-label="More">+</button>
          <span class="muted">or more in the time limit</span>
        </div>
      </div>
      <div class="step">
        <span class="lbl">3 · How long?</span>
        <div class="chips">{#each data.rules.windows as w}<button class:on={windowMin === w} onclick={() => (windowMin = w)}>{windowText(w)}</button>{/each}</div>
      </div>
      <p class="sentence">{#if subject}<b>{subject.name}</b>{:else}<i>Someone</i>{/if} will get <b>{threshold}</b> {data.rules.metrics.find((m) => m.id === metric)?.label ?? ''} in <b>{windowText(windowMin)}</b>.</p>
      <button class="primary" onclick={create} disabled={busy || !subject}>Open this bet</button>
    </section>
  {/if}

  <section>
    <h3 class="sec">Open bets <span class="count">{data?.open.length ?? 0}</span></h3>
    <div class="cards">
      {#each data?.open ?? [] as m (m.id)}
        {@const lock = closed(m)}
        <article class="bet" class:expanded={openId === m.id}>
          <div class="head">
            <Avatar name={m.subject} uuid={m.subject_uuid} size={2.6} />
            <div class="title"><span><b>{m.subject}</b> reaches <b>{m.threshold}</b> {m.metric_label}?</span><small>opened by {m.creator} · {m.bettors} bet{m.bettors === 1 ? '' : 's'}</small></div>
            <span class="time" class:lock>{#if lock}<Lock size={13} /> Betting closed · result in {untilText(m.ends_at, now)}{:else}<Clock size={13} /> {untilText(m.locks_at, now)} to bet{/if}</span>
          </div>
          <div class="prog" title="{m.progress} of {m.threshold} so far"><i style="width:{Math.min(100, (m.progress / m.threshold) * 100)}%"></i><span>{m.progress} / {m.threshold}</span></div>
          <div class="pool">
            <div class="split"><i class="y" style="width:{share(m)}%"></i><i class="n" style="width:{100 - share(m)}%"></i></div>
            <div class="legend"><span class="y">Yes · {money(m.yes_pool)} {#if m.odds_yes}<b>{mult(m.odds_yes)}</b>{/if}</span><span class="n">{#if m.odds_no}<b>{mult(m.odds_no)}</b>{/if} {money(m.no_pool)} · No</span></div>
          </div>
          {#if m.mine.length}<p class="mine">Your bet: <b>{money(m.mine.reduce((a, b) => a + b.stake, 0))}</b> on <b>{mineSide(m) === 'yes' ? 'Yes' : 'No'}</b></p>{/if}
          {#if openId === m.id && !lock && m.subject_uuid !== st.me.uuid}
            <div class="place">
              <BetInput bind:value={stake} min={data!.rules.min_stake} max={data!.rules.max_stake} {balance} label="Stake" disabled={busy} />
              <div class="sides">
                <button class="yes" disabled={busy || mineSide(m) === 'no'} onclick={() => place(m, 'yes')}><Check size={15} /> Back Yes</button>
                <button class="no" disabled={busy || mineSide(m) === 'yes'} onclick={() => place(m, 'no')}><XIcon size={15} /> Back No</button>
              </div>
              {#if mineSide(m)}<small class="muted">You can add to your bet, but not switch sides.</small>{/if}
            </div>
          {/if}
          <div class="foot">
            {#if m.subject_uuid === st.me.uuid}<span class="muted">You can't bet on yourself.</span>
            {:else if !lock}<button class="ghost sm" onclick={() => (openId = openId === m.id ? null : m.id)}>{openId === m.id ? 'Hide' : mineSide(m) ? 'Add to your bet' : 'Place a bet'}</button>{/if}
            {#if (m.mine_creator || st.me.admin) && m.bettors === 0}<button class="ghost sm" onclick={() => cancel(m)}><Ban size={13} /> Cancel</button>{/if}
          </div>
        </article>
      {:else}<p class="note">No open bets. {data?.can_create ? 'Open one and see who dares.' : 'Check back soon.'}</p>{/each}
    </div>
  </section>

  {#if data?.done.length}
    <section>
      <h3 class="sec">Finished</h3>
      <ul class="done">
        {#each data.done as m (m.id)}
          {@const won = m.mine.some((b) => (b.payout ?? 0) > b.stake)}
          <li><Avatar name={m.subject} uuid={m.subject_uuid} size={1.8} />
            <span class="what"><b>{m.subject}</b> {m.metric_label} {m.threshold}+<small>{ago(m.ends_at, now)} · reached {m.progress}</small></span>
            <span class="res" class:void={m.status === 'void'}>{result(m)}</span>
            {#if m.mine.length}<span class="you" class:win={won}>{won ? 'You won ' + money(m.mine.reduce((a, b) => a + (b.payout ?? 0), 0)) : m.status === 'void' ? 'Stake returned' : 'You lost'}</span>{/if}</li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .wrap { display: flex; flex-direction: column; gap: 1.1rem; }
  .intro { display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start; flex-wrap: wrap; }
  .intro h2 { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.2rem; } .intro p { color: var(--muted); font-size: 0.88rem; max-width: 46rem; }
  .panel { padding: 1.1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.9rem; }
  .step { display: flex; flex-direction: column; gap: 0.45rem; } .lbl { font-size: 0.78rem; font-weight: 700; color: var(--accent-2); text-transform: uppercase; letter-spacing: 0.06em; }
  .search { position: relative; display: flex; align-items: center; } .search :global(svg) { position: absolute; left: 0.75rem; color: var(--muted); } .search input { padding-left: 2.2rem; }
  .results { display: grid; grid-template-columns: repeat(auto-fill, minmax(9rem, 1fr)); gap: 0.4rem; max-height: 8rem; overflow: auto; }
  .player { display: flex; align-items: center; gap: 0.5rem; padding: 0.4rem 0.55rem; border-radius: var(--radius-sm); border: 1px solid var(--line); background: color-mix(in srgb, var(--text) 4%, transparent); color: var(--text); text-align: left; }
  .player:hover { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 14%, transparent); } .player span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.86rem; }
  .on { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: var(--success); margin-left: auto; box-shadow: 0 0 6px var(--success); }
  .chosen { display: flex; gap: 0.7rem; align-items: center; padding: 0.5rem 0.8rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--accent) 12%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent); } .chosen .ghost { margin-left: auto; }
  .chips { display: flex; flex-wrap: wrap; gap: 0.4rem; }
  .chips button { padding: 0.35rem 0.9rem; border-radius: 99rem; border: 1px solid var(--line-strong); background: color-mix(in srgb, var(--text) 5%, transparent); color: var(--text); font-size: 0.85rem; text-transform: capitalize; }
  .chips button:hover { border-color: var(--accent); } .chips button.on { background: var(--accent); border-color: var(--accent); box-shadow: 0 3px 14px -4px var(--accent); }
  .num { display: flex; gap: 0.5rem; align-items: center; } .num input { width: 6rem; text-align: center; font-size: 1.2rem; font-weight: 700; } .muted { color: var(--muted); font-size: 0.82rem; }
  .sentence { padding: 0.7rem 0.9rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 5%, transparent); font-size: 0.95rem; }
  .sec { display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.6rem; font-size: 1rem; } .count { font-size: 0.75rem; padding: 0.05rem 0.55rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 10%, transparent); }
  .note { color: var(--muted); font-size: 0.88rem; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(24rem, 1fr)); gap: 0.9rem; align-items: start; }
  .bet { padding: 1rem; border-radius: var(--radius); background: var(--panel, var(--surface)); border: 1px solid var(--line); backdrop-filter: var(--blur); display: flex; flex-direction: column; gap: 0.7rem; transition: border-color 0.15s, transform 0.15s; }
  .bet:hover { border-color: var(--line-strong); } .bet.expanded { border-color: color-mix(in srgb, var(--accent) 55%, transparent); }
  .head { display: flex; gap: 0.7rem; align-items: center; flex-wrap: wrap; } .title { flex: 1; min-width: 12rem; display: flex; flex-direction: column; font-size: 0.95rem; } .title small { color: var(--muted); font-size: 0.74rem; }
  .time { font-size: 0.75rem; color: var(--muted); display: inline-flex; gap: 0.3rem; align-items: center; } .time.lock { color: var(--warn); }
  .prog { position: relative; height: 1.15rem; border-radius: 99rem; background: color-mix(in srgb, var(--text) 8%, transparent); overflow: hidden; }
  .prog i { position: absolute; inset: 0 auto 0 0; background: linear-gradient(90deg, #6366f1, #38bdf8); border-radius: 99rem; transition: width 0.5s; } .prog span { position: relative; z-index: 1; display: block; text-align: center; font-size: 0.7rem; font-weight: 700; line-height: 1.15rem; text-shadow: 0 1px 2px #000a; }
  .split { display: flex; height: 0.55rem; border-radius: 99rem; overflow: hidden; background: color-mix(in srgb, var(--text) 8%, transparent); } .split .y { background: #22c55e; transition: width 0.4s; } .split .n { background: #ef4444; transition: width 0.4s; }
  .legend { display: flex; justify-content: space-between; font-size: 0.78rem; margin-top: 0.3rem; } .legend .y { color: #4ade80; } .legend .n { color: #f87171; } .legend b { color: var(--text); margin: 0 0.15rem; }
  .mine { font-size: 0.82rem; padding: 0.4rem 0.7rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--accent) 12%, transparent); }
  .place { display: flex; flex-direction: column; gap: 0.7rem; padding-top: 0.3rem; border-top: 1px dashed var(--line-strong); }
  .sides { display: grid; grid-template-columns: 1fr 1fr; gap: 0.5rem; } .sides button { padding: 0.7rem; font-weight: 700; border-radius: var(--radius); border: none; color: #fff; display: inline-flex; justify-content: center; gap: 0.4rem; align-items: center; }
  .sides .yes { background: linear-gradient(135deg, #22c55e, #15803d); } .sides .no { background: linear-gradient(135deg, #ef4444, #b91c1c); } .sides button:hover:not(:disabled) { filter: brightness(1.1); }
  .foot { display: flex; gap: 0.5rem; justify-content: flex-end; align-items: center; }
  .done { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.3rem; }
  .done li { display: flex; align-items: center; gap: 0.8rem; padding: 0.5rem 0.8rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--text) 4%, transparent); flex-wrap: wrap; }
  .what { flex: 1; min-width: 10rem; display: flex; flex-direction: column; font-size: 0.9rem; } .what small { color: var(--muted); font-size: 0.74rem; }
  .res { font-weight: 700; font-size: 0.82rem; padding: 0.15rem 0.7rem; border-radius: 99rem; background: color-mix(in srgb, var(--accent) 22%, transparent); } .res.void { background: color-mix(in srgb, var(--text) 10%, transparent); color: var(--muted); }
  .you { font-size: 0.82rem; color: #f87171; } .you.win { color: #4ade80; font-weight: 700; }
</style>
