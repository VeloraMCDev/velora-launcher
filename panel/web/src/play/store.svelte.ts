import { route } from '../lib/router.svelte';
import type { Experience } from '@velora/experience';
import { get } from '../lib/api';

export type PlayServer = {
  id: number; name: string; instance_id: string | null; map: boolean; online: boolean;
  players_online: number; players_max: number; software?: string | null; mc_version?: string | null; tps?: number | null; last_seen?: string | null;
};

const SERVER_KEY = 'scopenet.play.server';
const read = (): number | null => { try { const v = localStorage.getItem(SERVER_KEY); return v ? Number(v) : null; } catch { return null; } };

/** State every player-panel page can read. Pages import `play` and call the helpers; nothing here is per-page. */
export type Manifest = {
  panel_version: string;
  branding: {
    name: string; tagline: string; logo_url: string | null; icon_url: string | null; version_label: string;
    colors: { accent: string; accent_2: string; background: string; surface: string; text: string; muted: string; success: string; danger: string };
    news: { id: string; title: string; body: string; date?: string | null; image_url?: string | null; link?: string | null; pinned?: boolean; tag?: string | null }[];
    links: { label: string; url: string; icon?: string }[];
    about?: { title?: string; body?: string };
  };
  auth: { panel_accounts: boolean; registration: 'closed' | 'approval' | 'open' };
  instances: { experience?: Experience; id: string; name: string; description: string; mc_version: string; loader: string; icon_url?: string | null; banner_url?: string | null; logo_url?: string | null; featured: boolean; source_label: string; file_count: number; total_size: number; server?: { name?: string; address: string } | null }[];
};

export const play = $state({
  manifest: null as Manifest | null,
  servers: [] as PlayServer[],
  serverId: read() as number | null,
  /** Balance on the selected server, null until the player has one. */
  balance: null as number | null,
  loaded: false,
  /** Bumped by pages after an action that changes money, so the header balance refetches. */
  tick: 0,
});

export const currentServer = (): PlayServer | null => play.servers.find((s) => s.id === play.serverId) ?? null;

export async function loadManifest() {
  try { play.manifest = await get<Manifest>('/api/v1/launcher/manifest'); } catch { /* branding falls back to defaults */ }
}

export async function loadServers() {
  const instanceId = route.instanceId;
  if (!instanceId) return;
  try {
    const r = await get<{ servers: PlayServer[] }>('/api/v1/servers/public');
    if (route.instanceId !== instanceId) return;
    play.servers = r.servers.filter(s => s.instance_id === instanceId);
    if (!play.servers.some((s) => s.id === play.serverId)) play.serverId = play.servers[0]?.id ?? null;
  } catch { /* the panel is unreachable; pages show their own errors */ }
  play.loaded = true;
  await refreshBalance();
}

export function selectServer(id: number) {
  play.serverId = id;
  try { localStorage.setItem(SERVER_KEY, String(id)); } catch { /* storage unavailable */ }
  void refreshBalance();
}

export async function refreshBalance() {
  if (play.serverId == null) { play.balance = null; return; }
  const id = play.serverId;
  const instanceId = route.instanceId;
  try {
    const r = await get<{ balance: number | null }>(`/api/v1/economy/balance/${id}`);
    if (id === play.serverId && instanceId === route.instanceId) play.balance = r.balance;
  } catch { /* leave the last known value */ }
}

/** Call after buying, selling, betting or anything else that moves money. */
export function balanceChanged(known?: number | null) {
  if (typeof known === 'number') play.balance = known;
  else void refreshBalance();
  play.tick++;
}

export const money = (v: number | null | undefined, digits = 2) =>
  v == null ? '—' : '$' + v.toLocaleString(undefined, { minimumFractionDigits: v % 1 === 0 ? 0 : digits, maximumFractionDigits: digits });

export function compact(v: number): string {
  const a = Math.abs(v);
  if (a >= 1e9) return (v / 1e9).toFixed(1).replace(/\.0$/, '') + 'B';
  if (a >= 1e6) return (v / 1e6).toFixed(1).replace(/\.0$/, '') + 'M';
  if (a >= 1e4) return (v / 1e3).toFixed(1).replace(/\.0$/, '') + 'k';
  return v.toLocaleString(undefined, { maximumFractionDigits: 2 });
}
