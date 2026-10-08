<script lang="ts">
  import { Package, Clock, CheckCircle, XCircle, User, Server, Trash2, RefreshCw, Search, Filter } from '@lucide/svelte/icons';
  import { get, post, del } from '../lib/api';
  import { toast } from '../lib/toast.svelte';
  import Empty from '../components/Empty.svelte';

  /** One row of `GET /api/admin/reward-deliveries`. */
  type ApiDelivery = {
    id: number;
    player: string;
    kind: string;
    server_id: number | null;
    summary: string;
    source: string;
    created_at: string;
    delivered_at: string | null;
    attempts: number;
    error: string | null;
  };

  type RewardDelivery = {
    id: number;
    username: string;
    server_id: number | null;
    server_name: string;
    reward_type: string;
    summary: string;
    status: 'pending' | 'delivered' | 'failed';
    created_at: string;
    delivered_at: string | null;
    error_message: string | null;
    retry_count: number;
  };

  /** The game gives up after this many attempts (see `rewards::poll`). */
  const MAX_ATTEMPTS = 8;

  const toDelivery = (d: ApiDelivery): RewardDelivery => ({
    id: d.id,
    username: d.player,
    server_id: d.server_id,
    server_name: d.server_id == null ? 'Any server' : (serverNames.get(d.server_id) ?? `Server #${d.server_id}`),
    reward_type: d.source ? `${d.kind} · ${d.source}` : d.kind,
    summary: d.summary,
    status: d.delivered_at ? 'delivered' : d.error || d.attempts >= MAX_ATTEMPTS ? 'failed' : 'pending',
    created_at: d.created_at,
    delivered_at: d.delivered_at,
    error_message: d.error,
    retry_count: d.attempts,
  });

  let deliveries = $state<RewardDelivery[]>([]);
  let loading = $state(false);
  let filterStatus = $state<'all' | 'pending' | 'delivered' | 'failed'>('all');
  let filterServer = $state<number | 'all'>('all');
  let search = $state('');
  let servers = $state<{ id: number; name: string }[]>([]);
  const serverNames = $derived(new Map(servers.map((s) => [s.id, s.name])));

  async function loadDeliveries() {
    loading = true;
    try {
      const [rows, srv] = await Promise.all([
        get<ApiDelivery[]>('/api/admin/reward-deliveries?limit=300'),
        get<{ id: number; name: string }[] | { servers: { id: number; name: string }[] }>('/api/admin/servers').catch(() => []),
      ]);
      servers = (Array.isArray(srv) ? srv : srv.servers ?? []).map((s) => ({ id: s.id, name: s.name }));
      deliveries = rows.map(toDelivery);
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      loading = false;
    }
  }

  async function retryDelivery(id: number) {
    try {
      await post(`/api/admin/reward-deliveries/${id}/retry`, {});
      toast('Delivery queued for retry');
      await loadDeliveries();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function deleteDelivery(id: number) {
    if (!confirm('Delete this delivery record? This cannot be undone.')) return;
    
    try {
      await del(`/api/admin/reward-deliveries/${id}`);
      toast('Delivery deleted');
      await loadDeliveries();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  async function clearCompleted() {
    if (!confirm('Clear all delivered rewards from the queue?')) return;
    
    try {
      await post('/api/admin/reward-deliveries/clear-delivered', {});
      toast('Delivered rewards cleared');
      await loadDeliveries();
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  const filtered = $derived(
    deliveries.filter(d => {
      if (filterStatus !== 'all' && d.status !== filterStatus) return false;
      if (filterServer !== 'all' && d.server_id !== filterServer) return false;
      if (search.trim()) {
        const s = search.toLowerCase();
        return d.username.toLowerCase().includes(s) || d.reward_type.toLowerCase().includes(s) || d.summary.toLowerCase().includes(s);
      }
      return true;
    })
  );

  const stats = $derived({
    total: deliveries.length,
    pending: deliveries.filter(d => d.status === 'pending').length,
    delivered: deliveries.filter(d => d.status === 'delivered').length,
    failed: deliveries.filter(d => d.status === 'failed').length,
  });

  function statusColor(status: string): string {
    if (status === 'delivered') return 'var(--good)';
    if (status === 'failed') return 'var(--bad)';
    return 'var(--warn)';
  }

  $effect(() => {
    void loadDeliveries();
  });
</script>

<div class="page">
  <header>
    <div>
      <h1><Package size={22} /> Reward Delivery Queue</h1>
      <p>Monitor and manage pending reward deliveries to players.</p>
    </div>
    <button class="ghost" onclick={clearCompleted}><Trash2 size={16} /> Clear delivered</button>
  </header>

  <div class="stats-row">
    <div class="stat-card">
      <div class="stat-icon pending"><Clock size={20} /></div>
      <div class="stat-content">
        <strong>{stats.pending}</strong>
        <small>Pending</small>
      </div>
    </div>
    <div class="stat-card">
      <div class="stat-icon delivered"><CheckCircle size={20} /></div>
      <div class="stat-content">
        <strong>{stats.delivered}</strong>
        <small>Delivered</small>
      </div>
    </div>
    <div class="stat-card">
      <div class="stat-icon failed"><XCircle size={20} /></div>
      <div class="stat-content">
        <strong>{stats.failed}</strong>
        <small>Failed</small>
      </div>
    </div>
    <div class="stat-card">
      <div class="stat-icon total"><Package size={20} /></div>
      <div class="stat-content">
        <strong>{stats.total}</strong>
        <small>Total</small>
      </div>
    </div>
  </div>

  <div class="controls">
    <div class="search-box">
      <Search size={16} />
      <input type="text" bind:value={search} placeholder="Search by player or reward type..." />
    </div>
    
    <select bind:value={filterStatus}>
      <option value="all">All statuses</option>
      <option value="pending">Pending</option>
      <option value="delivered">Delivered</option>
      <option value="failed">Failed</option>
    </select>

    {#if servers.length > 1}
      <select bind:value={filterServer}>
        <option value="all">All servers</option>
        {#each servers as server}
          <option value={server.id}>{server.name}</option>
        {/each}
      </select>
    {/if}

    <button class="ghost icon" onclick={loadDeliveries} title="Refresh"><RefreshCw size={16} /></button>
  </div>

  {#if loading && !deliveries.length}
    <div class="loading"><span class="spin">◌</span></div>
  {:else if !filtered.length}
    <Empty icon={Package} title="No deliveries" text={search.trim() || filterStatus !== 'all' ? 'No matching deliveries found.' : 'All rewards have been delivered.'} />
  {:else}
    <div class="delivery-list">
      {#each filtered as delivery (delivery.id)}
        <article class="delivery-card" data-status={delivery.status}>
          <div class="delivery-status" style="background: {statusColor(delivery.status)}20; color: {statusColor(delivery.status)}">
            {#if delivery.status === 'pending'}
              <Clock size={18} />
            {:else if delivery.status === 'delivered'}
              <CheckCircle size={18} />
            {:else}
              <XCircle size={18} />
            {/if}
          </div>

          <div class="delivery-content">
            <div class="delivery-header">
              <div class="delivery-player">
                <User size={14} />
                <strong>{delivery.username}</strong>
              </div>
              <div class="delivery-server">
                <Server size={13} />
                <span>{delivery.server_name}</span>
              </div>
            </div>

            <div class="delivery-reward">
              <strong>{delivery.summary || delivery.reward_type}</strong>
              <small class="reward-type">{delivery.reward_type}</small>
            </div>

            <div class="delivery-meta">
              <time>{new Date(delivery.created_at).toLocaleString()}</time>
              {#if delivery.delivered_at}
                <span class="delivered-time">→ {new Date(delivery.delivered_at).toLocaleString()}</span>
              {/if}
              {#if delivery.retry_count > 0}
                <span class="retry-badge">{delivery.retry_count} retr{delivery.retry_count === 1 ? 'y' : 'ies'}</span>
              {/if}
            </div>

            {#if delivery.error_message}
              <div class="error-box">
                <XCircle size={14} />
                <span>{delivery.error_message}</span>
              </div>
            {/if}
          </div>

          <div class="delivery-actions">
            {#if delivery.status !== 'delivered'}
              <button class="ghost icon tiny" onclick={() => retryDelivery(delivery.id)} title="Retry"><RefreshCw size={14} /></button>
            {/if}
            <button class="ghost icon tiny" onclick={() => deleteDelivery(delivery.id)} title="Delete"><Trash2 size={14} /></button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .page { padding: 1.6rem 2.2rem; max-width: 1200px; margin: 0 auto; }
  header { display: flex; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--line); }
  h1 { display: flex; align-items: center; gap: 10px; margin: 0; font-size: 1.4rem; }
  header p { margin: 6px 0 0; color: var(--muted); font-size: .88rem; }

  .stats-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 20px; }
  .stat-card { display: flex; align-items: center; gap: 12px; padding: 14px 16px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  .stat-icon { width: 42px; height: 42px; border-radius: 10px; display: grid; place-items: center; flex: none; }
  .stat-icon.pending { background: #f9731620; color: #f97316; }
  .stat-icon.delivered { background: #4ade8020; color: #4ade80; }
  .stat-icon.failed { background: #ef444420; color: #ef4444; }
  .stat-icon.total { background: var(--accent-soft); color: var(--accent-2); }
  .stat-content { display: flex; flex-direction: column; gap: 2px; }
  .stat-content strong { font-size: 1.3rem; font-weight: 700; line-height: 1; }
  .stat-content small { font-size: .78rem; color: var(--muted); }

  .controls { display: flex; gap: 10px; margin-bottom: 18px; align-items: stretch; }
  .controls select { width: auto; min-width: 150px; }
  .search-box { flex: 1; display: flex; align-items: center; gap: 10px; padding: 0 12px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; }
  .search-box input { flex: 1; min-width: 0; background: none; border: none; padding: 10px 0; font-size: .88rem; }
  select { padding: 10px 12px; background: var(--bg-2); border: 1px solid var(--line); border-radius: 10px; font-size: .88rem; color: var(--text); }

  .delivery-list { display: flex; flex-direction: column; gap: 10px; }
  .delivery-card { display: flex; align-items: center; gap: 14px; padding: 14px 16px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  .delivery-card[data-status="failed"] { border-color: #ef444440; }
  
  .delivery-status { width: 44px; height: 44px; border-radius: 11px; display: grid; place-items: center; flex: none; }
  
  .delivery-content { flex: 1; display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .delivery-header { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .delivery-player { display: flex; align-items: center; gap: 6px; font-size: .88rem; font-weight: 550; }
  .delivery-server { display: flex; align-items: center; gap: 5px; font-size: .78rem; color: var(--muted); }
  
  .delivery-reward { display: flex; align-items: baseline; gap: 8px; }
  .delivery-reward strong { font-size: .95rem; font-weight: 600; }
  .reward-type { font-size: .76rem; color: var(--muted); text-transform: uppercase; letter-spacing: .02em; }
  
  .delivery-meta { display: flex; align-items: center; gap: 10px; font-size: .76rem; color: var(--muted); flex-wrap: wrap; }
  .delivered-time { color: var(--good); }
  .retry-badge { padding: 2px 6px; border-radius: 5px; background: var(--warn)20; color: var(--warn); font-weight: 550; }
  
  .error-box { display: flex; align-items: center; gap: 6px; padding: 8px 10px; background: #ef444410; border: 1px solid #ef444430; border-radius: 8px; font-size: .8rem; color: #ef4444; }
  
  .delivery-actions { display: flex; gap: 4px; flex: none; }
  .tiny { padding: 6px 8px; }

  .loading { display: flex; justify-content: center; padding: 60px 0; color: var(--muted); }
  .spin { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  @media (max-width: 768px) {
    .page { padding: 1rem; }
    .stats-row { grid-template-columns: 1fr 1fr; }
    .controls { flex-direction: column; }
    .delivery-card { flex-direction: column; align-items: flex-start; }
    .delivery-actions { align-self: flex-end; }
  }
</style>
