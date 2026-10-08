// Buy orders and contracts are shared by the launcher (Tauri) and the web player panel. Each app tells them how to reach the panel
// API (`/board/{server}{path}`) and how to show a toast, once at start-up, with `setBoardHost`.
export type ToastKind = 'ok' | 'error' | 'info';

export interface BoardHost {
  get<T>(serverId: number, path: string): Promise<T>;
  post<T>(serverId: number, path: string, body: Record<string, unknown>): Promise<T>;
  toast(text: string, kind?: ToastKind): void;
}

let current: BoardHost | null = null;
export const setBoardHost = (host: BoardHost) => { current = host; };
const host = () => { if (!current) throw new Error('The board has no host. Call setBoardHost() first.'); return current; };

export const bget = <T>(serverId: number, path: string) => host().get<T>(serverId, path);
export const bpost = <T>(serverId: number, path: string, body: Record<string, unknown> = {}) => host().post<T>(serverId, path, body);
export const toast = (text: string, kind: ToastKind = 'ok') => host().toast(text, kind);
export const errorText = (e: unknown): string => (typeof e === 'string' ? e : e instanceof Error ? e.message : JSON.stringify(e));

export const money = (v: number | null | undefined) => (v == null ? '—' : '$' + v.toLocaleString(undefined, { minimumFractionDigits: v % 1 === 0 ? 0 : 2, maximumFractionDigits: 2 }));
export function untilText(iso: string | null | undefined, now = Date.now()) {
  if (!iso) return '';
  const s = Math.max(0, Math.floor((new Date(iso).getTime() - now) / 1000));
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d}d ${h}h` : h ? `${h}h ${m}m` : m ? `${m}m ${s % 60}s` : `${s}s`;
}
export function ago(iso: string, now = Date.now()) {
  const s = Math.max(1, (now - new Date(iso).getTime()) / 1000);
  return s < 90 ? 'just now' : s < 3600 ? `${Math.round(s / 60)}m ago` : s < 86400 ? `${Math.round(s / 3600)}h ago` : `${Math.round(s / 86400)}d ago`;
}

export type Order = {
  id: number; buyer_name: string; item_id: string; item_name: string; amount: number; total: number; each: number;
  status: 'open' | 'claimed' | 'filled' | 'cancelled' | 'expired'; claimer_name: string | null; claim_until: string | null;
  created_at: string; expires_at: string; mine: boolean; claimed_by_me: boolean; filler_name: string | null; resolved?: string;
};
export type OrdersBoard = {
  enabled: boolean; balance: number | null; orders: Order[]; history: Order[]; my_open: number; my_claims: number;
  rules: { max_open: number; min_total: number; max_total: number; max_amount: number; expire_hours: number; claim_minutes: number; max_claims: number; fee_percent: number };
};
export type Contract = {
  id: number; kind: 'kill' | 'gather'; target: string; title: string; required: number; progress: number; reward: number; bonus: boolean;
  status: string; expires_at: string; item_name: string | null;
};
export type ContractsBoard = {
  enabled: boolean; balance?: number | null; contracts: Contract[]; recent?: Contract[];
  stats?: { done_today: number; daily_limit: number; limit_reached: boolean; rerolls_left: number; completed: number; earned: number; board_size: number; expire_hours: number };
};

/** Items people commonly ask for, as suggestions in the request form. */
export const COMMON_ITEMS = ['COBBLESTONE', 'STONE', 'OAK_LOG', 'SPRUCE_LOG', 'BIRCH_LOG', 'SAND', 'GRAVEL', 'GLASS', 'COAL', 'IRON_INGOT', 'GOLD_INGOT', 'COPPER_INGOT', 'DIAMOND', 'EMERALD', 'NETHERITE_INGOT', 'REDSTONE', 'LAPIS_LAZULI', 'QUARTZ', 'GLOWSTONE_DUST', 'WHEAT', 'CARROT', 'POTATO', 'SUGAR_CANE', 'BONE', 'STRING', 'GUNPOWDER', 'ENDER_PEARL', 'BLAZE_ROD', 'OBSIDIAN', 'ANCIENT_DEBRIS'];
