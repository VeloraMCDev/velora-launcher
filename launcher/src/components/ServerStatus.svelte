<script lang="ts">
  import { Users, Signal, Copy, Check } from '@lucide/svelte';
  import { parseMotd } from '../lib/motd';
  import { invoke } from '../lib/tauri';
  import type { ServerEntry, ServerStatus } from '../lib/types';

  let { server }: { server: ServerEntry } = $props();
  let status = $state<ServerStatus | null>(null);
  let failed = $state(false);
  let copied = $state(false);

  $effect(() => {
    const { address, port } = server;
    status = null;
    failed = false;
    let alive = true;
    const load = () =>
      invoke<ServerStatus>('ping_server', { host: address, port })
        .then((s) => alive && ((status = s), (failed = false)))
        .catch(() => alive && (failed = true));
    load();
    const t = setInterval(() => !document.hidden && load(), 30_000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  });

  const addr = $derived(server.port === 25565 ? server.address : `${server.address}:${server.port}`);
  const bars = $derived(!status ? 0 : status.latency_ms < 60 ? 4 : status.latency_ms < 120 ? 3 : status.latency_ms < 250 ? 2 : 1);

  function copy() {
    navigator.clipboard.writeText(addr);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="server glass">
  {#if status?.favicon}<img src={status.favicon} alt="" class="fav" />{:else}<span class="fav ph" class:on={status}></span>{/if}
  <div class="info">
    <div class="row name">
      <strong>{server.name || server.address}</strong>
      <span class="state" class:online={status} class:off={failed}>{status ? 'Online' : failed ? 'Offline' : 'Checking…'}</span>
    </div>
    {#if status}
      <div class="motd">{#each parseMotd(status.motd.split('\n')[0]) as s}<span style:color={s.color} class:b={s.bold} class:i={s.italic}>{s.text}</span>{/each}</div>
    {:else}
      <button class="addr" onclick={copy}>{addr} {#if copied}<Check size={12} />{:else}<Copy size={12} />{/if}</button>
    {/if}
  </div>
  {#if status}
    <div class="stats">
      <span class="players" title={status.sample.join(', ')}><Users size={14} /> <b>{status.players_online}</b><span class="muted">/{status.players_max}</span></span>
      <span class="ping" title="{status.latency_ms} ms">
        <span class="sig">{#each [1, 2, 3, 4] as i}<i class:lit={i <= bars} style:height="{i * 25}%"></i>{/each}</span>
        {status.latency_ms}ms
      </span>
    </div>
  {/if}
</div>

<style>
  .server { display: flex; align-items: center; gap: 0.75rem; padding: 0.5rem 0.9rem 0.5rem 0.5rem; min-width: 18rem; max-width: 28rem; height: 2.75rem; background: var(--surface); }
  .fav { width: 1.8rem; height: 1.8rem; border-radius: calc(var(--radius-sm) - 2px); image-rendering: pixelated; flex-shrink: 0; }
  .ph { background: color-mix(in srgb, var(--text) 8%, transparent); }
  .ph.on { background: color-mix(in srgb, var(--accent) 30%, transparent); }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.05rem; }
  .name { gap: 0.5rem; }
  .name strong { min-width: 0; flex: 0 1 auto; font-size: 0.85rem; font-weight: 560; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .state { font-size: 0.72rem; font-weight: 520; color: var(--muted); display: inline-flex; align-items: center; gap: 0.3rem; }
  .state::before { content: ''; width: 0.45rem; height: 0.45rem; border-radius: 50%; background: currentColor; }
  .state.online { color: var(--success); }
    .state.off { color: var(--danger); }
  .motd { font-size: 0.75rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: color-mix(in srgb, var(--text) 75%, transparent); }
  .motd .b { font-weight: 700; }
  .motd .i { font-style: italic; }
  .addr { all: unset; cursor: pointer; font-size: 0.8rem; color: var(--muted); display: inline-flex; align-items: center; gap: 0.35rem; font-family: var(--mono); }
  .addr:hover { color: var(--text); }
  .stats { display: flex; flex-direction: column; align-items: flex-end; gap: 0.05rem; font-size: 0.78rem; font-variant-numeric: tabular-nums; }
  .players { display: inline-flex; align-items: center; gap: 0.3rem; }
  .players :global(svg) { color: var(--muted); }
  .ping { display: inline-flex; align-items: center; gap: 0.35rem; color: var(--muted); font-size: 0.72rem; }
  .sig { display: inline-flex; align-items: flex-end; gap: 0.1rem; height: 0.7rem; }
  .sig i { width: 0.18rem; background: color-mix(in srgb, var(--text) 20%, transparent); border-radius: 1px; }
  .sig i.lit { background: var(--success); }
</style>
