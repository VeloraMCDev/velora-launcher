<script lang="ts">
  import { onMount } from 'svelte';
  import { Target, Sparkles, Users, Save, RotateCcw, Search, Shuffle, Shield, Pin, TrendingUp, Plus, Minus, Equal, ArrowUpToLine, Eraser, Info, Coins, Table2, Wand2 } from '@lucide/svelte';
  import Avatar from '../components/Avatar.svelte';
  import Modal from '../components/Modal.svelte';
  import { del, get, post, put } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import type { PlayerProgress, ProgressionSettings, ProgressionView } from '../lib/types';

  const DEFAULTS: ProgressionSettings = {
    daily_quest_limit: 5,
    weekly_quest_limit: 3,
    quest_rotation: 'per_player',
    level_base: 100,
    level_exponent: 1.5,
    max_level: 0,
    global_xp_multiplier: 1,
    server_xp_multiplier: 1,
    xp_rates: { playtime_per_hour: 600, player_kill: 50, mob_kill: 15, block_broken: 0.2, block_placed: 0.2, message: 2 },
    rules: { starting_balance: 1000, guild_base_claims: 16, guild_claims_per_member: 0, guild_claims_per_level: 0, guild_max_members: 0, market_max_listings: 0,
      auto_money: { enabled: true, scale: 1, quest_daily: 0.35, quest_weekly: 0.5, quest_other: 0.4, achievement: 0.6, level_base: 20, level_exponent: 1.4, milestone: 3 } },
  };

  let tab = $state<'quests' | 'xp' | 'money' | 'players'>('quests');
  let saved = $state<ProgressionSettings | null>(null);
  let s = $state<ProgressionSettings>(structuredClone(DEFAULTS));
  let pool = $state({ daily: { enabled: 0, pinned: 0 }, weekly: { enabled: 0, pinned: 0 } });
  let saving = $state(false);

  const dirty = $derived(saved != null && JSON.stringify(saved) !== JSON.stringify(s));
  const curveChanged = $derived(saved != null && (saved.level_base !== s.level_base || saved.level_exponent !== s.level_exponent || saved.max_level !== s.max_level));

  async function load() {
    try {
      const v = await get<ProgressionView>('/api/admin/progression');
      saved = structuredClone(v.settings);
      s = structuredClone(v.settings);
      pool = v.quest_pool;
    } catch (e) {
      toastError(e);
    }
  }

  async function save() {
    saving = true;
    try {
      const v = await put<ProgressionView & { recalculated: number }>('/api/admin/progression', s);
      saved = structuredClone(v.settings);
      s = structuredClone(v.settings);
      pool = v.quest_pool;
      toast(v.recalculated ? `Saved — ${v.recalculated} player levels recalculated` : 'Progression settings saved');
    } catch (e) {
      toastError(e);
    } finally {
      saving = false;
    }
  }

  const revert = () => saved && (s = structuredClone(saved));

  // ---- curve helpers (mirror the server's maths for live previews) ----
  const xpFor = (lvl: number, base = s.level_base, exp = s.level_exponent) => (lvl <= 1 ? 0 : Math.round(base * Math.pow(lvl, exp)));
  function levelFrom(xp: number): number {
    const cap = s.max_level > 0 ? s.max_level : 10000;
    let l = 1;
    while (l < cap && xp >= xpFor(l + 1)) l++;
    return l;
  }
  const sampleLevels = $derived([5, 10, 25, 50, 100].filter((l) => !s.max_level || l <= s.max_level));
  const chart = $derived.by(() => {
    const top = Math.min(s.max_level > 0 ? s.max_level : 60, 60);
    const pts = Array.from({ length: top - 1 }, (_, i) => ({ l: i + 2, xp: xpFor(i + 2) }));
    const maxXp = Math.max(1, ...pts.map((p) => p.xp));
    const path = pts.map((p, i) => `${i ? 'L' : 'M'}${(((p.l - 2) / Math.max(1, top - 2)) * 280 + 10).toFixed(1)},${(100 - (p.xp / maxXp) * 88).toFixed(1)}`).join(' ');
    return { path, top, maxXp };
  });

  // ---- default money (mirrors the server so the preview moves as you type) ----
  const round5 = (x: number) => Math.max(5, Math.round(x / 5) * 5);
  const am = $derived(s.rules.auto_money);
  const questPay = (rate: number, xp: number) => round5(xp * rate * am.scale);
  const levelPay = (l: number) => round5(am.level_base * Math.pow(l, am.level_exponent) * am.scale);
  const milestonePay = (l: number) => round5(levelPay(l) * am.milestone);
  const moneySamples = $derived({
    daily: [80, 150, 250, 450].map((xp) => ({ xp, pay: questPay(am.quest_daily, xp) })),
    weekly: [600, 1000, 1800, 2500].map((xp) => ({ xp, pay: questPay(am.quest_weekly, xp) })),
    achievements: [100, 500, 1500, 5000].map((xp) => ({ xp, pay: round5(xp * am.achievement * am.scale) })),
    levels: [2, 5, 10, 25, 50, 100].filter((l) => !s.max_level || l <= s.max_level).map((l) => ({ l, pay: levelPay(l), milestone: milestonePay(l) })),
  });
  let applying_money = $state(false);
  async function applyMoney() {
    applying_money = true;
    try {
      const r = await post<{ written: number }>('/api/admin/progression/auto-money/apply');
      toast(r.written ? `Updated the money on ${r.written} rewards` : 'Every reward already pays the current rates');
    } catch (e) {
      toastError(e);
    } finally {
      applying_money = false;
    }
  }
  const usd = (v: number) => `$${v.toLocaleString()}`;

  const n = (v: number) => Math.round(v).toLocaleString();
  const per = (r: number) => (r > 0 && r < 1 ? `1 XP per ${Math.round(1 / r)}` : `${r} XP each`);

  const quests = $derived([
    { key: 'daily_quest_limit' as const, label: 'Daily quests', hint: 'Reset every day at 00:00 UTC', pool: pool.daily, icon: Target },
    { key: 'weekly_quest_limit' as const, label: 'Weekly quests', hint: 'Reset every Monday at 00:00 UTC', pool: pool.weekly, icon: TrendingUp },
  ]);

  // ---- players ----
  let q = $state('');
  let players = $state<PlayerProgress[]>([]);
  let total = $state(0);
  let loadingPlayers = $state(false);
  let timer: ReturnType<typeof setTimeout>;
  async function loadPlayers() {
    loadingPlayers = true;
    try {
      const r = await get<{ players: PlayerProgress[]; total: number }>(`/api/admin/progression/players?q=${encodeURIComponent(q)}&limit=40`);
      players = r.players;
      total = r.total;
    } catch (e) {
      toastError(e);
    } finally {
      loadingPlayers = false;
    }
  }
  $effect(() => {
    void q;
    clearTimeout(timer);
    timer = setTimeout(() => tab === 'players' && loadPlayers(), 250);
    return () => clearTimeout(timer);
  });
  $effect(() => { if (tab === 'players' && !players.length) loadPlayers(); });

  type Detail = {
    uuid: string; name: string; title: string | null; max_level: number;
    global: { xp: number; level: number; level_xp: number; level_span: number; progress_pct: number };
    servers: { server_id: number; server_name: string; xp: number; level: number; progress_pct: number; rank_name: string | null }[];
  };
  let detail = $state<Detail | null>(null);
  let open = $state(false);
  let scope = $state('global');
  let mode = $state<'add' | 'remove' | 'set_xp' | 'set_level' | 'reset'>('add');
  let amount = $state(500);
  let reason = $state('');
  let applying = $state(false);
  type PlayerUnlock = { key: string; type: string; source_type: string; source_id: string; equipped: boolean; metadata: Record<string, unknown> };
  let unlocks = $state<PlayerUnlock[]>([]);
  let grantDraft = $state({ key: '', type: 'title', label: '' });
  let collectionBusy = $state(false);
  type Template = { key: string; type: string; label: string; metadata: Record<string, unknown> };
  let templates = $state<Template[]>([]);
  $effect(() => { get<Template[]>('/api/admin/cosmetics/templates').then((r) => (templates = Array.isArray(r) ? r : [])).catch(() => {}); });
  let picked = $state('');
  async function grantTemplate() {
    const t = templates.find((x) => x.key === picked);
    if (!detail || !t) return;
    collectionBusy = true;
    try {
      await post('/api/admin/collections/grant', { uuid: detail.uuid, unlock_key: t.key, unlock_type: t.type, metadata: { ...t.metadata, label: t.label } });
      picked = '';
      await loadUnlocks(detail.uuid);
      toast(`Granted ${t.label}`);
    } catch (e) { toastError(e); }
    finally { collectionBusy = false; }
  }

  async function inspect(p: PlayerProgress) {
    try {
      detail = await get<Detail>(`/api/admin/progression/players/${p.uuid}`);
      await loadUnlocks(p.uuid);
      scope = 'global';
      mode = 'add';
      amount = 500;
      reason = '';
      open = true;
    } catch (e) {
      toastError(e);
    }
  }

  async function loadUnlocks(uuid: string) {
    const response = await get<{ unlocks: PlayerUnlock[] }>(`/api/admin/players/${uuid}/collections`);
    unlocks = response.unlocks;
  }

  async function grantUnlock() {
    if (!detail || !grantDraft.key.trim()) return;
    collectionBusy = true;
    try {
      await post('/api/admin/collections/grant', {
        uuid: detail.uuid,
        unlock_key: grantDraft.key.trim(),
        unlock_type: grantDraft.type,
        metadata: grantDraft.label.trim() ? { label: grantDraft.label.trim() } : {},
      });
      grantDraft = { key: '', type: 'title', label: '' };
      await loadUnlocks(detail.uuid);
      toast('Collection unlock granted');
    } catch (e) { toastError(e); }
    finally { collectionBusy = false; }
  }

  async function revokeUnlock(item: PlayerUnlock) {
    if (!detail || !confirm(`Revoke ${item.key} from ${detail.name}?`)) return;
    collectionBusy = true;
    try {
      await del(`/api/admin/collections/${detail.uuid}/${encodeURIComponent(item.key)}`);
      await loadUnlocks(detail.uuid);
      toast('Collection unlock revoked');
    } catch (e) { toastError(e); }
    finally { collectionBusy = false; }
  }

  const currentXp = $derived(!detail ? 0 : scope === 'global' ? detail.global.xp : detail.servers.find((x) => String(x.server_id) === scope)?.xp ?? 0);
  const nextXp = $derived(
    mode === 'add' ? currentXp + amount
    : mode === 'remove' ? Math.max(0, currentXp - amount)
    : mode === 'set_xp' ? amount
    : mode === 'set_level' ? xpFor(amount)
    : 0
  );

  async function apply() {
    if (!detail) return;
    applying = true;
    try {
      const body: Record<string, unknown> = { scope: scope === 'global' ? 'global' : 'server', mode, amount: mode === 'reset' ? 0 : Number(amount), reason };
      if (scope !== 'global') body.server_id = Number(scope);
      detail = await post<Detail>(`/api/admin/progression/players/${detail.uuid}/adjust`, body);
      toast('XP updated');
      loadPlayers();
    } catch (e) {
      toastError(e);
    } finally {
      applying = false;
    }
  }

  const modes = [
    { id: 'add', label: 'Give XP', icon: Plus },
    { id: 'remove', label: 'Take XP', icon: Minus },
    { id: 'set_xp', label: 'Set XP', icon: Equal },
    { id: 'set_level', label: 'Set level', icon: ArrowUpToLine },
    { id: 'reset', label: 'Reset', icon: Eraser },
  ] as const;

  onMount(load);
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Progression</h1>
      <p>Decide how many quests players get, how fast they earn XP, how steep levels are — and fix any player's XP by hand.</p>
    </div>
  </div>

  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === 'quests'} class:on={tab === 'quests'} onclick={() => (tab = 'quests')}><Target size={15} /> Quest limits</button>
    <button role="tab" aria-selected={tab === 'xp'} class:on={tab === 'xp'} onclick={() => (tab = 'xp')}><Sparkles size={15} /> XP &amp; levels</button>
    <button role="tab" aria-selected={tab === 'money'} class:on={tab === 'money'} onclick={() => (tab = 'money')}><Coins size={15} /> Money rewards</button>
    <button role="tab" aria-selected={tab === 'players'} class:on={tab === 'players'} onclick={() => (tab = 'players')}><Users size={15} /> Players</button>
  </div>

  {#if tab === 'quests'}
    <div class="cols">
      {#each quests as c (c.key)}
        <section class="card quota">
          <div class="q-head">
            <span class="ic"><c.icon size={18} /></span>
            <div><h2>{c.label}</h2><p class="muted small">{c.hint}</p></div>
          </div>
          <div class="big">
            <button class="step" aria-label="Fewer" onclick={() => (s[c.key] = Math.max(0, s[c.key] - 1))}><Minus size={16} /></button>
            <div class="num">
              <input type="number" min="0" max="100" bind:value={s[c.key]} aria-label={c.label + ' per player'} />
              <span class="muted small">{s[c.key] === 0 ? 'every quest' : s[c.key] === 1 ? 'quest per player' : 'quests per player'}</span>
            </div>
            <button class="step" aria-label="More" onclick={() => (s[c.key] = Math.min(100, s[c.key] + 1))}><Plus size={16} /></button>
          </div>
          <input class="range" type="range" min="0" max={Math.max(20, Math.min(100, c.pool.enabled))} bind:value={s[c.key]} aria-label="Limit slider" />
          <div class="pool">
            <span><strong>{c.pool.enabled}</strong> enabled in the pool</span>
            <span class="pin"><Pin size={12} /> {c.pool.pinned} pinned (always included)</span>
          </div>
          {#if s[c.key] > 0 && s[c.key] < c.pool.pinned}
            <p class="warn small"><Info size={13} /> Only {s[c.key]} of the {c.pool.pinned} pinned quests fit — unpin some or raise the limit.</p>
          {/if}
        </section>
      {/each}
    </div>

    <section class="card">
      <div class="section-title"><h2><Shuffle size={16} /> Rotation</h2></div>
      <div class="choice">
        <button class="opt" class:on={s.quest_rotation === 'per_player'} onclick={() => (s.quest_rotation = 'per_player')}>
          <Users size={17} /><span><strong>Per player</strong><small>Everyone draws their own random set each day/week.</small></span>
        </button>
        <button class="opt" class:on={s.quest_rotation === 'shared'} onclick={() => (s.quest_rotation = 'shared')}>
          <Shield size={17} /><span><strong>Shared</strong><small>Everyone gets the same set — good for community goals.</small></span>
        </button>
      </div>
      <p class="muted small note">
        A player's set is fixed the first time they open their quests in a period, so edits here never swap out quests they've started.
        Raising a limit adds quests, lowering it trims the list. Pin quests on the <a href="#/quests">Quests</a> page to always include them.
      </p>
    </section>
  {:else if tab === 'xp'}
    <div class="cols">
      <section class="card">
        <div class="section-title"><h2><TrendingUp size={16} /> Level curve</h2></div>
        <p class="muted small">XP needed for level <em>L</em> is <code>round(base × L<sup>exponent</sup>)</code>.</p>
        <div class="fields3">
          <label class="field">Base<input type="number" min="10" max="100000" step="10" bind:value={s.level_base} /></label>
          <label class="field">Exponent<input type="number" min="1" max="3" step="0.05" bind:value={s.level_exponent} /></label>
          <label class="field">Max level<input type="number" min="0" max="10000" bind:value={s.max_level} /><span class="tiny muted">0 = no cap</span></label>
        </div>
        <svg class="chart" viewBox="0 0 300 110" role="img" aria-label="XP required per level">
          <path d="M10,100 H290" class="axis" />
          <path d={chart.path} class="line" />
        </svg>
        <div class="chart-legend tiny muted"><span>Level 2</span><span>Level {chart.top} · {n(chart.maxXp)} XP</span></div>
        <ul class="samples">
          {#each sampleLevels as l}<li><span class="muted">Level {l}</span><strong>{n(xpFor(l))} XP</strong></li>{/each}
        </ul>
        {#if curveChanged}<p class="warn small"><Info size={13} /> Saving recalculates every player's stored level from their XP. Nobody loses XP.</p>{/if}
      </section>

      <section class="card">
        <div class="section-title"><h2><Sparkles size={16} /> XP earned from play</h2></div>
        <div class="rates">
          <label class="field"><span class="lab">Online time <span class="unit">XP / hour</span></span><input type="number" min="0" step="10" bind:value={s.xp_rates.playtime_per_hour} /></label>
          <label class="field"><span class="lab">Player kill <span class="unit">XP</span></span><input type="number" min="0" step="1" bind:value={s.xp_rates.player_kill} /></label>
          <label class="field"><span class="lab">Mob kill <span class="unit">XP</span></span><input type="number" min="0" step="1" bind:value={s.xp_rates.mob_kill} /></label>
          <label class="field"><span class="lab">Block broken <span class="unit">{per(s.xp_rates.block_broken)}</span></span><input type="number" min="0" step="0.05" bind:value={s.xp_rates.block_broken} /></label>
          <label class="field"><span class="lab">Block placed <span class="unit">{per(s.xp_rates.block_placed)}</span></span><input type="number" min="0" step="0.05" bind:value={s.xp_rates.block_placed} /></label>
          <label class="field"><span class="lab">Chat message <span class="unit">XP</span></span><input type="number" min="0" step="1" bind:value={s.xp_rates.message} /></label>
        </div>
        <div class="section-title mult"><h2>Multipliers</h2></div>
        <div class="fields2">
          <label class="field">Global XP ×<input type="number" min="0" max="100" step="0.1" bind:value={s.global_xp_multiplier} /></label>
          <label class="field">Server XP ×<input type="number" min="0" max="100" step="0.1" bind:value={s.server_xp_multiplier} /></label>
        </div>
        <p class="muted small">These stack with the multipliers in each server's plugin/mod config (server levels default to 1.5× there). Quest and achievement XP is set per quest/achievement.</p>
      </section>

      <section class="card">
        <div class="section-title"><h2><Info size={16} /> Economy &amp; guild rules</h2></div>
        <p class="muted small">Apply on every server. Home limits, cooldowns and RTP are set per server in the plugin/mod config, and ranks can raise home limits with permission nodes (velora.homes.10).</p>
        <div class="rates">
          <label class="field"><span class="lab">Starting balance <span class="unit">$</span></span><input type="number" min="0" step="50" bind:value={s.rules.starting_balance} /></label>
          <label class="field"><span class="lab">Guild claims at creation <span class="unit">chunks</span></span><input type="number" min="0" step="1" bind:value={s.rules.guild_base_claims} /></label>
          <label class="field"><span class="lab">Extra claims per member <span class="unit">chunks</span></span><input type="number" min="0" step="1" bind:value={s.rules.guild_claims_per_member} /></label>
          <label class="field"><span class="lab">Extra claims per guild level <span class="unit">chunks</span></span><input type="number" min="0" step="1" bind:value={s.rules.guild_claims_per_level} /></label>
          <label class="field"><span class="lab">Guild member limit <span class="unit">0 = none</span></span><input type="number" min="0" step="1" bind:value={s.rules.guild_max_members} /></label>
          <label class="field"><span class="lab">Market listings per player <span class="unit">0 = none</span></span><input type="number" min="0" step="1" bind:value={s.rules.market_max_listings} /></label>
        </div>
      </section>
    </div>
  {:else if tab === 'money'}
    <div class="cols">
      <section class="card">
        <div class="section-title"><h2><Coins size={16} /> Default money rewards</h2></div>
        <p class="muted small">Every quest, achievement and rank pays money scaled by how hard it is: a quest or achievement by its XP, a level by how high it is. Amounts are always flat numbers ending in 5 or 0. Anything you set yourself on a quest, achievement or reward is never overwritten.</p>
        <label class="onoff"><input type="checkbox" bind:checked={s.rules.auto_money.enabled} /> <span><strong>Pay default money</strong><small class="muted">Off removes the automatic amounts again.</small></span></label>
        <div class="rates">
          <label class="field"><span class="lab">Overall scale <span class="unit">×</span></span><input type="number" min="0.1" max="20" step="0.1" bind:value={s.rules.auto_money.scale} /></label>
          <label class="field"><span class="lab">Daily quest <span class="unit">$ per XP</span></span><input type="number" min="0" step="0.05" bind:value={s.rules.auto_money.quest_daily} /></label>
          <label class="field"><span class="lab">Weekly quest <span class="unit">$ per XP</span></span><input type="number" min="0" step="0.05" bind:value={s.rules.auto_money.quest_weekly} /></label>
          <label class="field"><span class="lab">Other quests <span class="unit">$ per XP</span></span><input type="number" min="0" step="0.05" bind:value={s.rules.auto_money.quest_other} /></label>
          <label class="field"><span class="lab">Achievement <span class="unit">$ per XP</span></span><input type="number" min="0" step="0.05" bind:value={s.rules.auto_money.achievement} /></label>
          <label class="field"><span class="lab">Rank milestone <span class="unit">× the level's pay</span></span><input type="number" min="0" step="0.5" bind:value={s.rules.auto_money.milestone} /></label>
          <label class="field"><span class="lab">Level pay base <span class="unit">$</span></span><input type="number" min="0" step="5" bind:value={s.rules.auto_money.level_base} /></label>
          <label class="field"><span class="lab">Level pay steepness <span class="unit">exponent</span></span><input type="number" min="0.5" max="3" step="0.05" bind:value={s.rules.auto_money.level_exponent} /></label>
        </div>
        <div class="money-actions">
          <button class="ghost" disabled={applying_money || dirty} onclick={applyMoney} title={dirty ? 'Save first' : ''}><Wand2 size={15} /> {applying_money ? 'Applying…' : 'Apply to everything now'}</button>
          <a class="btn ghost" href="#/bulk"><Table2 size={15} /> Open the bulk editor</a>
        </div>
        <p class="muted small note">Saving applies the new rates straight away, and a background job keeps newly added quests and achievements covered.</p>
      </section>

      <section class="card">
        <div class="section-title"><h2><Sparkles size={16} /> What that pays</h2></div>
        {#each [{ label: 'Daily quests', rows: moneySamples.daily }, { label: 'Weekly quests', rows: moneySamples.weekly }, { label: 'Achievements', rows: moneySamples.achievements }] as g}
          <h3 class="mh">{g.label}</h3>
          <ul class="samples">
            {#each g.rows as r}<li><span class="muted">{n(r.xp)} XP</span><strong>{usd(r.pay)}</strong></li>{/each}
          </ul>
        {/each}
        <h3 class="mh">Reaching a level</h3>
        <ul class="samples">
          {#each moneySamples.levels as r}<li><span class="muted">Level {r.l}</span><strong>{usd(r.pay)}</strong><small class="muted">+ {usd(r.milestone)} milestone</small></li>{/each}
        </ul>
      </section>
    </div>
  {:else}
    <section class="card">
      <div class="search"><Search size={16} /><input type="search" placeholder="Search players by name…" bind:value={q} /><span class="muted small">{total.toLocaleString()} players</span></div>
      {#if loadingPlayers && !players.length}
        <div class="skeleton"></div>
      {:else if !players.length}
        <p class="muted">No players match.</p>
      {:else}
        <div class="plist">
          {#each players as p (p.uuid)}
            <button class="prow" onclick={() => inspect(p)}>
              <Avatar name={p.name} uuid={p.uuid} size={34} />
              <span class="pname"><strong>{p.name}</strong>{#if p.title}<small class="muted">{p.title}</small>{/if}</span>
              <span class="pbar"><i style:width="{p.progress_pct}%"></i></span>
              <span class="plvl">Lv {p.global_level}</span>
              <span class="pxp muted">{n(p.global_xp)} XP</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

{#if dirty}
  <div class="savebar" role="status">
    <span>You have unsaved progression changes.</span>
    <button class="ghost" onclick={revert}><RotateCcw size={15} /> Discard</button>
    <button class="primary" disabled={saving} onclick={save}><Save size={15} /> {saving ? 'Saving…' : 'Save changes'}</button>
  </div>
{/if}

<Modal bind:open title={detail ? `Adjust ${detail.name}` : 'Adjust XP'} width={560}>
  {#if detail}
    <div class="who">
      <Avatar name={detail.name} uuid={detail.uuid} size={44} />
      <div>
        <strong>{detail.name}</strong>
        <span class="muted small">Global level {detail.global.level} · {n(detail.global.xp)} XP</span>
        <div class="pbar wide"><i style:width="{detail.global.progress_pct}%"></i></div>
      </div>
    </div>

    <label class="field">Level track
      <select bind:value={scope}>
        <option value="global">Global level (all servers)</option>
        {#each detail.servers as sv}<option value={String(sv.server_id)}>{sv.server_name} — level {sv.level} · {n(sv.xp)} XP</option>{/each}
      </select>
    </label>

    <div class="modes" role="tablist">
      {#each modes as m}
        <button role="tab" aria-selected={mode === m.id} class="mode" class:on={mode === m.id} onclick={() => (mode = m.id)}><m.icon size={14} /> {m.label}</button>
      {/each}
    </div>

    {#if mode !== 'reset'}
      <label class="field">{mode === 'set_level' ? 'Level' : 'XP amount'}
        <input type="number" min={mode === 'set_level' ? 1 : 0} bind:value={amount} />
      </label>
    {/if}
    <label class="field">Reason <span class="tiny muted">(kept in the activity log)</span><input maxlength="120" bind:value={reason} placeholder="e.g. event prize, rollback after bug" /></label>

    <div class="preview">
      <span>{n(currentXp)} XP <small class="muted">(level {levelFrom(currentXp)})</small></span>
      <span class="arrow">→</span>
      <strong>{n(nextXp)} XP <small>(level {levelFrom(nextXp)})</small></strong>
    </div>

    <section class="unlock-admin">
      <h3>Collection unlocks</h3>
      {#if unlocks.length}
        <div class="unlock-list">
          {#each unlocks as item (item.key)}
            <div class="unlock-row">
              <span><strong>{String(item.metadata.label ?? item.key)}</strong><small>{item.type.replaceAll('_', ' ')} · {item.source_type}</small></span>
              <button class="ghost icon" title="Revoke unlock" aria-label="Revoke {item.key}" disabled={collectionBusy} onclick={() => revokeUnlock(item)}>×</button>
            </div>
          {/each}
        </div>
      {:else}<p class="muted small">No unlocks granted.</p>{/if}
      <div class="grant-row">
        <select aria-label="Cosmetic to grant" bind:value={picked}>
          <option value="">Grant from the Cosmetics Studio…</option>
          {#each templates as t}<option value={t.key}>{t.label} · {t.type.replaceAll('_', ' ')}</option>{/each}
        </select>
        <button class="primary" disabled={collectionBusy || !picked} onclick={grantTemplate}>Grant</button>
      </div>
      <details class="manual"><summary>Grant by raw key instead</summary>
      <div class="grant-row">
        <input aria-label="Unlock key" bind:value={grantDraft.key} placeholder="unlock_key" maxlength="80" />
        <select aria-label="Unlock type" bind:value={grantDraft.type}>
          <option value="title">Title</option><option value="badge">Badge</option><option value="cosmetic">Cosmetic</option>
          <option value="particle">Particle</option><option value="pet">Pet</option><option value="join_message">Join message</option><option value="leave_message">Leave message</option><option value="showcase">Showcase</option>
        </select>
        <input aria-label="Display label" bind:value={grantDraft.label} placeholder="Display label" maxlength="80" />
        <button class="primary" disabled={collectionBusy || !grantDraft.key.trim()} onclick={grantUnlock}>Grant</button>
      </div>
      </details>
    </section>
  {/if}
  {#snippet footer()}
    <button class="ghost" onclick={() => (open = false)}>Close</button>
    <button class={mode === 'reset' || mode === 'remove' ? 'danger' : 'primary'} disabled={applying || (mode !== 'reset' && !(amount >= (mode === 'set_level' ? 1 : 0)))} onclick={apply}>
      {applying ? 'Applying…' : 'Apply'}
    </button>
  {/snippet}
</Modal>

<style>
  .page { padding-bottom: 90px; }
  .tabs { display: inline-flex; gap: 4px; padding: 4px; border-radius: 12px; background: var(--bg-2); margin-bottom: 18px; }
  .tabs button { border: 0; background: transparent; padding: 8px 16px; border-radius: 9px; color: var(--muted); gap: 8px; }
  .tabs button.on { background: var(--accent-soft); color: var(--text); }
  .cols { display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 16px; margin-bottom: 16px; align-items: start; }
  .quota { display: flex; flex-direction: column; gap: 14px; }
  .q-head { display: flex; gap: 12px; align-items: center; }
  .q-head h2 { margin: 0; font-size: 1.05rem; }
  .q-head p { margin: 2px 0 0; }
  .ic { width: 38px; height: 38px; border-radius: 11px; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent-2); }
  .big { display: flex; align-items: center; justify-content: center; gap: 18px; padding: 6px 0; }
  .step { width: 40px; height: 40px; padding: 0; border-radius: 50%; }
  .num { display: flex; flex-direction: column; align-items: center; min-width: 120px; }
  .num input { width: 100px; text-align: center; font-size: 2.6rem; font-weight: 700; letter-spacing: -0.03em; background: transparent; border: 0; padding: 0; font-variant-numeric: tabular-nums; -moz-appearance: textfield; appearance: textfield; }
  .num input::-webkit-outer-spin-button, .num input::-webkit-inner-spin-button { -webkit-appearance: none; }
  .range { width: 100%; accent-color: var(--accent); }
  .pool { display: flex; justify-content: space-between; flex-wrap: wrap; gap: 8px; font-size: 0.85rem; color: var(--text-2); padding-top: 10px; border-top: 1px solid var(--line); }
  .pin { display: inline-flex; align-items: center; gap: 5px; color: var(--muted); }
  .warn { display: flex; gap: 7px; align-items: center; color: #fcd34d; margin: 0; }
  .choice { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 10px; margin: 12px 0; }
  .opt { justify-content: flex-start; gap: 12px; text-align: left; white-space: normal; padding: 12px 14px; background: var(--bg-2); }
  .opt span { display: flex; flex-direction: column; gap: 2px; }
  .opt small { color: var(--muted); font-weight: 400; }
  .opt.on { border-color: color-mix(in srgb, var(--accent) 60%, transparent); background: var(--accent-soft); }
  .onoff { display: flex; gap: 10px; align-items: center; padding: 10px 12px; border-radius: 12px; background: var(--bg-2); margin: 12px 0 4px; cursor: pointer; }
  .onoff span { display: flex; flex-direction: column; }
  .onoff input { width: 18px; height: 18px; accent-color: var(--accent); }
  .money-actions { display: flex; flex-wrap: wrap; gap: 8px; margin: 14px 0 8px; }
  .mh { margin: 14px 0 0; font-size: 0.8rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.06em; }
  .note { line-height: 1.55; margin: 0; }
  .unlock-admin { border-top: 1px solid var(--line); padding-top: 12px; margin-top: 4px; }
  .unlock-admin h3 { margin: 0 0 8px; font-size: .92rem; }
  .unlock-list { max-height: 150px; overflow: auto; }
  .unlock-row,.grant-row { display: flex; align-items: center; gap: 8px; padding: 6px 0; }
  .unlock-row > span { display: flex; flex: 1; min-width: 0; flex-direction: column; }
  .unlock-row small { color: var(--muted); text-transform: capitalize; }
  .grant-row { display: grid; grid-template-columns: 1fr 1fr 1fr auto; margin-top: 8px; }
  @media(max-width:650px) { .grant-row { grid-template-columns: 1fr 1fr; } }
  .fields3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin: 12px 0; }
  .fields2 { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; margin: 10px 0; }
  .rates { display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; margin-top: 12px; }
  .field { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; color: var(--text-2); font-weight: 500; }
  .lab { display: flex; justify-content: space-between; gap: 8px; }
  .unit { font-weight: 400; color: var(--muted); font-size: 0.75rem; }
  .mult { margin-top: 18px; }
  .chart { width: 100%; height: 110px; margin-top: 6px; }
  .axis { stroke: var(--line); stroke-width: 1; fill: none; }
  .line { stroke: var(--accent-2); stroke-width: 2.2; fill: none; stroke-linejoin: round; stroke-linecap: round; }
  .chart-legend { display: flex; justify-content: space-between; }
  .samples { list-style: none; margin: 12px 0 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 8px; }
  .samples li { display: flex; flex-direction: column; padding: 9px 12px; border-radius: 10px; background: var(--bg-2); font-size: 0.85rem; }
  .search { display: flex; align-items: center; gap: 10px; padding: 0 12px; border-radius: 12px; background: var(--bg-2); margin-bottom: 14px; }
  .search input { border: 0; background: transparent; }
  .search span { white-space: nowrap; }
  .plist { display: flex; flex-direction: column; gap: 4px; }
  .prow { display: grid; grid-template-columns: 34px minmax(120px, 1.3fr) minmax(80px, 2fr) 56px 100px; align-items: center; gap: 14px; padding: 9px 12px; border-radius: 12px; background: transparent; border-color: transparent; text-align: left; }
  .prow:hover { background: var(--bg-2); }
  .pname { display: flex; flex-direction: column; min-width: 0; }
  .pname small { font-size: 0.75rem; }
  .pbar { height: 6px; border-radius: 99px; background: var(--bg-2); overflow: hidden; }
  .pbar.wide { margin-top: 6px; width: 260px; max-width: 100%; }
  .pbar i { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--accent), var(--accent-2)); }
  .plvl { font-weight: 700; color: #fcd34d; }
  .pxp { text-align: right; font-variant-numeric: tabular-nums; font-size: 0.85rem; }
  .skeleton { height: 220px; border-radius: 12px; background: var(--bg-2); }
  .who { display: flex; gap: 14px; align-items: center; }
  .who div { display: flex; flex-direction: column; }
  .modes { display: flex; flex-wrap: wrap; gap: 6px; }
  .mode { padding: 7px 12px; border-radius: 99px; font-size: 0.83rem; background: var(--bg-2); gap: 6px; }
  .mode.on { border-color: color-mix(in srgb, var(--accent) 60%, transparent); background: var(--accent-soft); }
  .preview { display: flex; align-items: center; justify-content: center; gap: 14px; padding: 14px; border-radius: 12px; background: var(--bg-2); font-variant-numeric: tabular-nums; }
  .preview strong { color: var(--accent-2); }
  .arrow { color: var(--muted); }
  .savebar { position: fixed; left: 50%; bottom: 22px; transform: translateX(-50%); z-index: 40; display: flex; align-items: center; gap: 12px; padding: 10px 12px 10px 18px; border-radius: 14px; background: var(--surface); border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent); box-shadow: 0 14px 40px -12px rgba(0, 0, 0, 0.6); animation: rise 0.2s ease; }
  @keyframes rise { from { opacity: 0; transform: translate(-50%, 10px); } }
  @media (max-width: 720px) { .fields3, .rates { grid-template-columns: 1fr; } .prow { grid-template-columns: 34px 1fr 56px; } .pbar, .pxp { display: none; } .savebar { left: 12px; right: 12px; transform: none; flex-wrap: wrap; } }
</style>
