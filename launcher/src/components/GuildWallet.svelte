<script lang="ts">
  import PlayerLink from './PlayerLink.svelte';
  import { onMount,untrack } from 'svelte';
  import {
    Landmark, ArrowDownToLine, ArrowUpFromLine, Wallet, Lock, Server, History, Coins, LoaderCircle, RefreshCw, TrendingUp
  } from '@lucide/svelte';
  import Avatar from './Avatar.svelte';
  import { invoke } from '../lib/tauri';
  import { toast } from '../lib/store.svelte';
  import type { Guild, GuildMember } from '../lib/types';

  type Tx = { id: number; actor_uuid: string; kind: string; amount: number; created_at: string; note?: string };
  type WalletData = { balance: number; my_balance?: number; role?: string; currency_symbol?: string; transactions: Tx[]; upkeep?: {daily_cents:number;arrears_cents:number;grace_days:number;freezes_at:string|null;claims_frozen:boolean}|null };

  let { guild, members, servers, instanceId }: {
    guild: Guild;
    members: GuildMember[];
    servers: Array<{ id: number; name: string; instance_id: string }>;
    instanceId: string;
  } = $props();

  let serverId = $state<number | null>(null);
  let wallet = $state<WalletData | null>(null);
  let loading = $state(false);
  let busy = $state(false);
  let mode = $state<'deposit' | 'withdraw'>('deposit');
  let amount = $state('');
  let shown = $state(0);

  const instanceServers = $derived(servers.filter((s) => s.instance_id === instanceId));
  const symbol = $derived(wallet?.currency_symbol ?? '$');
  const role = $derived(wallet?.role ?? 'member');
  const canWithdraw = $derived(role === 'leader' || role === 'officer');
  const value = $derived(Number(amount));
  const validAmount = $derived(Number.isFinite(value) && value >= 0.01 && Math.abs(value * 100 - Math.round(value * 100)) < 1e-6);
  const limit = $derived(mode === 'deposit' ? wallet?.my_balance : wallet?.balance);
  const overLimit = $derived(validAmount && limit != null && value > limit + 1e-9);
  const afterGuild = $derived(wallet && validAmount ? wallet.balance + (mode === 'deposit' ? value : -value) : null);
  const afterMine = $derived(wallet?.my_balance != null && validAmount ? wallet.my_balance + (mode === 'deposit' ? -value : value) : null);
  // Money the guild receives (deposits, shop and market sales, payments from guilds) vs. spends.
  const isOut = (kind: string) => kind === 'withdraw' || kind === 'purchase' || kind === 'transfer_out' || kind === 'upkeep';
  const verbs: Record<string, string> = {
    deposit: 'deposited', withdraw: 'withdrew', sale: 'sold items', purchase: 'bought', transfer_in: 'received a payment', transfer_out: 'paid a guild', upkeep: 'paid daily upkeep'
  };
  const totals = $derived({
    in: wallet?.transactions.filter((t) => !isOut(t.kind)).reduce((n, t) => n + t.amount, 0) ?? 0,
    out: wallet?.transactions.filter((t) => isOut(t.kind)).reduce((n, t) => n + t.amount, 0) ?? 0
  });
  const quick = [10, 50, 100, 500, 1000];

  const money = (n: number) => `${symbol}${n.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  const who = (uuid: string) => members.find((m) => m.uuid === uuid);
  function ago(iso: string): string {
    const s = (Date.now() - new Date(iso).getTime()) / 1000;
    if (s < 60) return 'just now';
    if (s < 3600) return `${Math.floor(s / 60)}m ago`;
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
    if (s < 86400 * 7) return `${Math.floor(s / 86400)}d ago`;
    return new Date(iso).toLocaleDateString();
  }

  // Count the balance up (or down) smoothly whenever it changes.
  let frame = 0;
  $effect(() => {
    const target = wallet?.balance ?? 0;
    const from = untrack(()=>shown);
    const start = performance.now();
    cancelAnimationFrame(frame);
    if(document.hidden){shown=target;return;}
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / 550);
      shown = from + (target - from) * (1 - Math.pow(1 - t, 3));
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  });

  async function load() {
    if (!serverId) { wallet = null; return; }
    loading = true;
    try {
      wallet = await invoke<WalletData>('get_guild_wallet', { guildId: guild.id, serverId });
      if (!canWithdraw) mode = 'deposit';
    } catch (e: any) {
      toast(typeof e === 'string' ? e : e?.message ?? 'Unable to load the guild wallet', 'error');
    } finally {
      loading = false;
    }
  }

  async function submit() {
    if (!serverId || busy || !validAmount || overLimit) return;
    busy = true;
    try {
      await invoke('transfer_guild_wallet', { guildId: guild.id, serverId, amount: value, withdraw: mode === 'withdraw' });
      toast(mode === 'withdraw' ? `Withdrew ${money(value)} from the treasury` : `Deposited ${money(value)} into the treasury`, 'ok');
      amount = '';
      await load();
    } catch (e: any) {
      toast(typeof e === 'string' ? e : e?.message ?? 'Transfer failed', 'error');
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    if (!instanceServers.some((s) => s.id === serverId)) serverId = instanceServers[0]?.id ?? null;
  });
  $effect(() => { void serverId; void guild.id; load(); });
  onMount(() => () => cancelAnimationFrame(frame));
</script>

<div class="wallet">
  {#if instanceServers.length === 0}
    <div class="empty glass">
      <span class="empty-icon"><Server size={26} /></span>
      <h3>No game server linked</h3>
      <p class="muted">The guild treasury lives on a game server's economy. Ask an admin to link a server to this instance in the panel.</p>
    </div>
  {:else}
    {#if instanceServers.length > 1}
      <div class="servers" role="tablist" aria-label="Game server">
        {#each instanceServers as s (s.id)}
          <button role="tab" aria-selected={serverId === s.id} class="srv" class:on={serverId === s.id} onclick={() => (serverId = s.id)}>
            <Server size={13} /> {s.name}
          </button>
        {/each}
      </div>
    {/if}

    {#if !wallet && loading}
      <div class="empty glass"><LoaderCircle class="spin" size={28} /></div>
    {:else if wallet}
      <div class="cols">
        <div class="left">
          <section class="treasury">
            <div class="glow" aria-hidden="true"></div>
            <div class="t-top">
              <span class="t-label"><Landmark size={14} /> Guild treasury</span>
              <span class="tag">[{guild.tag}]</span>
              <button class="ghost icon sm refresh" onclick={load} aria-label="Refresh wallet" title="Refresh"><RefreshCw size={14} class={loading ? 'spin' : ''} /></button>
            </div>
            <div class="amount">{money(shown)}</div>
            {#if wallet.upkeep}<p class="tiny">Daily upkeep {money(wallet.upkeep.daily_cents/100)} · {wallet.upkeep.grace_days}-day grace period.</p>
              {#if wallet.upkeep.arrears_cents>0}<p class="warn tiny">Unpaid: {money(wallet.upkeep.arrears_cents/100)}. {wallet.upkeep.claims_frozen?'New claims are frozen.':`New claims freeze on ${wallet.upkeep.freezes_at} UTC.`} Deposit funds to settle outstanding bills within a minute. Existing claims remain protected.</p>{/if}
            {/if}
            <div class="t-stats">
              <div><span class="k"><TrendingUp size={12} /> Money in</span><strong class="in">{money(totals.in)}</strong></div>
              <div><span class="k"><ArrowUpFromLine size={12} /> Money out</span><strong class="out">{money(totals.out)}</strong></div>
              <div><span class="k"><Wallet size={12} /> Your wallet</span><strong>{wallet.my_balance != null ? money(wallet.my_balance) : '—'}</strong></div>
            </div>
            <p class="t-note tiny">Totals cover the last {wallet.transactions.length} transfers.</p>
          </section>

          <section class="transfer glass">
            <div class="mode" role="tablist">
              <button role="tab" aria-selected={mode === 'deposit'} class:on={mode === 'deposit'} onclick={() => (mode = 'deposit')}>
                <ArrowDownToLine size={15} /> Deposit
              </button>
              <button
                role="tab"
                aria-selected={mode === 'withdraw'}
                class:on={mode === 'withdraw'}
                disabled={!canWithdraw}
                title={canWithdraw ? '' : 'Only guild leaders and officers can withdraw'}
                onclick={() => (mode = 'withdraw')}
              >
                {#if canWithdraw}<ArrowUpFromLine size={15} />{:else}<Lock size={14} />{/if} Withdraw
              </button>
            </div>

            <label class="field">
              <span>Amount</span>
              <div class="money" class:bad={overLimit}>
                <span class="sym">{symbol}</span>
                <input type="number" inputmode="decimal" min="0.01" step="0.01" placeholder="0.00" bind:value={amount} onkeydown={(e) => e.key === 'Enter' && submit()} />
                {#if limit != null}<button class="max" type="button" onclick={() => (amount = String(Math.floor(limit * 100) / 100))}>Max</button>{/if}
              </div>
            </label>
            <div class="quick">
              {#each quick as q}<button type="button" class="q" onclick={() => (amount = String(q))}>{symbol}{q.toLocaleString()}</button>{/each}
            </div>

            {#if afterGuild != null && !overLimit}
              <div class="preview">
                <span>Treasury <strong>{money(wallet.balance)}</strong> → <strong class="hl">{money(afterGuild)}</strong></span>
                {#if afterMine != null}<span>You <strong>{money(wallet.my_balance!)}</strong> → <strong class="hl">{money(afterMine)}</strong></span>{/if}
              </div>
            {:else if overLimit}
              <p class="warn tiny">{mode === 'deposit' ? "That's more than your wallet holds." : "That's more than the treasury holds."}</p>
            {:else}
              <p class="muted tiny">Use up to two decimals. Transfers are instant and recorded for the whole guild.</p>
            {/if}

            <button class="primary go" disabled={busy || !validAmount || overLimit} onclick={submit}>
              {#if busy}<LoaderCircle class="spin" size={15} />{:else if mode === 'deposit'}<ArrowDownToLine size={15} />{:else}<ArrowUpFromLine size={15} />{/if}
              {mode === 'deposit' ? 'Deposit to treasury' : 'Withdraw to wallet'}
            </button>
          </section>
        </div>

        <section class="history glass">
          <header><h3><History size={16} /> Recent activity</h3><span class="chip">{wallet.transactions.length}</span></header>
          {#if wallet.transactions.length === 0}
            <div class="none">
              <Coins size={28} />
              <p>No transfers yet.</p>
              <p class="tiny muted">Be the first to fund the guild treasury.</p>
            </div>
          {:else}
            <ul>
              {#each wallet.transactions as tx (tx.id)}
                {@const m = who(tx.actor_uuid)}
                <li>
                  <span class="ic" class:out={isOut(tx.kind)}>
                    {#if isOut(tx.kind)}<ArrowUpFromLine size={14} />{:else}<ArrowDownToLine size={14} />{/if}
                  </span>
                  <Avatar uuid={tx.actor_uuid} name={m?.name ?? '?'} size={1.7} />
                  <div class="who">
                    <strong>{#if m}<PlayerLink uuid={tx.actor_uuid} name={m.name} />{:else}Former member{/if}</strong>
                    <span class="tiny muted" title={tx.note}>{verbs[tx.kind] ?? tx.kind}{tx.note ? ` · ${tx.note}` : ''} · {ago(tx.created_at)}</span>
                  </div>
                  <span class="amt" class:out={isOut(tx.kind)}>{isOut(tx.kind) ? '−' : '+'}{money(tx.amount)}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>
    {/if}
  {/if}
</div>

<style>
  .wallet { display: flex; flex-direction: column; gap: 1rem; animation: fade-up 0.25s var(--ease) both; }
  .servers { display: flex; gap: 0.4rem; flex-wrap: wrap; }
  .srv { padding: 0.4rem 0.85rem; font-size: 0.82rem; border-radius: 99rem; color: var(--muted); background: transparent; }
  .srv.on { color: var(--text); background: color-mix(in srgb, var(--accent) 18%, transparent); border-color: color-mix(in srgb, var(--accent) 45%, transparent); }
  .cols { display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr); gap: 1rem; align-items: start; }
  @media (max-width: 980px) { .cols { grid-template-columns: 1fr; } }
  .left { display: flex; flex-direction: column; gap: 1rem; }

  .treasury { position: relative; overflow: hidden; padding: 1.4rem 1.6rem 1.1rem; border-radius: var(--radius); border: 1px solid color-mix(in srgb, var(--accent) 35%, var(--line)); background: linear-gradient(140deg, color-mix(in srgb, var(--accent) 26%, var(--surface)), var(--surface) 70%); }
  .glow { position: absolute; width: 18rem; height: 18rem; right: -6rem; top: -9rem; border-radius: 50%; background: radial-gradient(circle, color-mix(in srgb, var(--accent-2) 45%, transparent), transparent 70%); pointer-events: none; }
  .t-top { position: relative; display: flex; align-items: center; gap: 0.6rem; }
  .t-label { display: inline-flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; color: color-mix(in srgb, var(--text) 75%, transparent); }
  .tag { font-weight: 800; letter-spacing: 0.08em; font-size: 0.78rem; color: var(--accent-2); }
  .refresh { margin-left: auto; }
  .amount { position: relative; margin: 0.7rem 0 1rem; font-size: clamp(2.2rem, 5vw, 3.1rem); font-weight: 750; letter-spacing: -0.03em; font-variant-numeric: tabular-nums; }
  .t-stats { position: relative; display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem; padding-top: 0.9rem; border-top: 1px solid var(--line); }
  .t-stats > div { display: flex; flex-direction: column; gap: 0.2rem; min-width: 0; }
  .k { display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; color: var(--muted); }
  .t-stats strong { font-size: 0.95rem; font-weight: 650; font-variant-numeric: tabular-nums; overflow: hidden; text-overflow: ellipsis; }
  .in { color: var(--success); }
  .out { color: color-mix(in srgb, var(--danger) 75%, white); }
  .t-note { position: relative; margin-top: 0.7rem; color: var(--muted); }

  .transfer { display: flex; flex-direction: column; gap: 0.85rem; padding: 1.1rem 1.25rem 1.25rem; background: var(--surface); }
  .mode { display: grid; grid-template-columns: 1fr 1fr; gap: 0.25rem; padding: 0.25rem; border-radius: var(--radius-sm); background: color-mix(in srgb, var(--bg) 70%, transparent); border: 1px solid var(--line); }
  .mode button { border: 0; background: transparent; color: var(--muted); padding: 0.5rem; }
  .mode button.on { background: color-mix(in srgb, var(--accent) 22%, transparent); color: var(--text); }
  .field { display: flex; flex-direction: column; gap: 0.4rem; font-size: 0.82rem; font-weight: 560; }
  .money { display: flex; align-items: center; gap: 0.5rem; padding: 0 0.5rem 0 0.9rem; border-radius: var(--radius-sm); border: 1px solid var(--line-strong); background: color-mix(in srgb, var(--bg) 70%, transparent); transition: border-color 0.15s; }
  .money:focus-within { border-color: var(--accent); }
  .money.bad { border-color: var(--danger); }
  .sym { color: var(--muted); font-weight: 650; }
  .money input { border: 0; background: transparent; padding: 0.75rem 0; font-size: 1.15rem; font-weight: 650; font-variant-numeric: tabular-nums; }
  .money input:focus { background: transparent; }
  .max { padding: 0.25rem 0.6rem; font-size: 0.74rem; border-radius: 99rem; }
  .quick { display: flex; gap: 0.4rem; flex-wrap: wrap; }
  .q { padding: 0.3rem 0.7rem; font-size: 0.78rem; border-radius: 99rem; color: var(--muted); font-variant-numeric: tabular-nums; }
  .q:hover { color: var(--text); }
  .preview { display: flex; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap; padding: 0.6rem 0.8rem; border-radius: var(--radius-sm); font-size: 0.8rem; color: var(--muted); background: color-mix(in srgb, var(--text) 4%, transparent); }
  .preview strong { color: var(--text); font-variant-numeric: tabular-nums; }
  .preview .hl { color: var(--accent-2); }
  .warn { color: var(--warn); }
  .go { padding: 0.75rem; font-size: 0.95rem; }

  .history { padding: 1.1rem 0.4rem 0.6rem; background: var(--surface); min-height: 18rem; }
  .history header { display: flex; align-items: center; justify-content: space-between; padding: 0 1rem 0.7rem; }
  .history h3 { display: inline-flex; align-items: center; gap: 0.5rem; font-size: 0.98rem; }
  ul { list-style: none; margin: 0; padding: 0 0.4rem; display: flex; flex-direction: column; max-height: 30rem; overflow-y: auto; }
  li { display: grid; grid-template-columns: auto auto 1fr auto; align-items: center; gap: 0.7rem; padding: 0.6rem 0.6rem; border-radius: var(--radius-sm); }
  li:hover { background: color-mix(in srgb, var(--text) 4%, transparent); }
  .ic { width: 1.7rem; height: 1.7rem; display: grid; place-items: center; border-radius: 50%; background: color-mix(in srgb, var(--success) 16%, transparent); color: var(--success); }
  .ic.out { background: color-mix(in srgb, var(--danger) 16%, transparent); color: color-mix(in srgb, var(--danger) 75%, white); }
  .who { display: flex; flex-direction: column; min-width: 0; }
  .who strong { font-size: 0.86rem; font-weight: 620; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .amt { font-weight: 680; font-size: 0.9rem; font-variant-numeric: tabular-nums; color: var(--success); }
  .amt.out { color: color-mix(in srgb, var(--danger) 75%, white); }
  .none { display: flex; flex-direction: column; align-items: center; gap: 0.3rem; padding: 2.5rem 1rem; color: var(--muted); text-align: center; }

  .empty { display: flex; flex-direction: column; align-items: center; gap: 0.6rem; padding: 3rem 1.5rem; text-align: center; background: var(--surface); }
  .empty p { max-width: 28rem; }
  .empty-icon { width: 3.4rem; height: 3.4rem; border-radius: var(--radius); display: grid; place-items: center; background: color-mix(in srgb, var(--accent) 15%, transparent); color: var(--accent-2); }
</style>
