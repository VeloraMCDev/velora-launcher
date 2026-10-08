// Types and helpers shared by the Casino page and its games.
import { hostGet, hostPost } from './host';
import { cprefs } from './prefs.svelte';

export type Seg = { label: string; value: number; weight: number; color: string };
export type Sym = { id: string; label: string; weight: number; pay: number; color: string };

export type CasinoState = {
  enabled: boolean;
  server: { id: number; name: string };
  me: { uuid: string; name: string; admin: boolean };
  balance: number | null;
  config: {
    enabled: boolean; max_payout: number; daily_loss_limit: number; feed_min_win: number;
    slots: { enabled: boolean; min_bet: number; max_bet: number; pair_pay: number; symbols: Sym[] };
    wheel: { enabled: boolean; min_bet: number; max_bet: number; segments: Seg[] };
    daily: { enabled: boolean; spins_per_day: number; segments: Seg[] };
    plinko: { enabled: boolean; min_bet: number; max_bet: number; min_rows: number; max_rows: number; rtp: number; risks: string[] };
    mines: { enabled: boolean; min_bet: number; max_bet: number; size: number; min_mines: number; max_mines: number; house_edge: number; max_multiplier: number };
    blackjack: { enabled: boolean; min_bet: number; max_bet: number; blackjack_pay: number; dealer_hits_soft_17: boolean };
    crash: { enabled: boolean; min_bet: number; max_bet: number; house_edge: number; max_multiplier: number };
    dice: { enabled: boolean; min_bet: number; max_bet: number; house_edge: number; min_chance: number; max_chance: number };
    coinflip: { enabled: boolean; min_bet: number; max_bet: number; payout: number };
    double: { enabled: boolean; win_chance: number; max_streak: number; offer_minutes: number };
    chaos: { enabled: boolean; surge_chance: number; curse_chance: number };
    bounties: { enabled: boolean };
    betting: { enabled: boolean };
  };
  plinko_tables: Record<string, Record<string, number[]>>;
  rtp: { slots: number; slots_hit: number; wheel: number; plinko: number; mines: number; blackjack: number; crash: number; dice: number; coinflip: number; double: number; chaos: number };
  free: { per_day: number; used: number; left: number; resets_at: string };
  mines: MinesGame | null;
  crash: CrashGame | null;
  blackjack: BlackjackGame | null;
  double: DoubleOffer | null;
  lost_today: number;
  bounty_on_me: number;
  feed: { name: string; game: string; bet: number; payout: number; at: string }[];
};

export type MinesGame = {
  id: number; bet: number; size: number; mines: number; revealed: number[]; status: 'active' | 'lost' | 'cashed';
  multiplier: number; next_multiplier: number; cashout: number; layout?: number[]; hit?: number;
};

/** A chance to gamble the winnings of the round that just paid: win and they double, lose and they are gone. */
export type DoubleOffer = { id: number; stake: number; payout: number; streak: number; max_streak: number; win_chance: number; expires_at: string };

export type CrashGame = {
  id: number; bet: number; status: 'active' | 'cashed' | 'busted'; auto: number | null; elapsed_ms: number; multiplier: number; rate: number;
  crash_point?: number; payout?: number;
};

export type BlackjackGame = {
  id: number; bet: number; status: 'active' | 'done'; player: number[]; player_total: number; soft: boolean; dealer: number[]; dealer_total: number;
  hidden: number; doubled: boolean; can_double: boolean; payout?: number; outcome?: 'blackjack' | 'win' | 'push' | 'lose' | 'bust';
};

/** What a round that can pay Double or Nothing returns alongside its result. */
export type WithDouble = { double: DoubleOffer | null };

/** The twist the server rolled for a win: a lucky surge or a curse. */
export type Twist = { kind: 'surge' | 'curse'; x: number; table: number };

/** A playing card id (0..51) as { rank text, suit symbol, red }. */
export function card(id: number) {
  const r = id % 13, suit = Math.floor(id / 13);
  return { rank: ['A', '2', '3', '4', '5', '6', '7', '8', '9', '10', 'J', 'Q', 'K'][r], suit: ['♠', '♥', '♦', '♣'][suit], red: suit === 1 || suit === 2 };
}

export type Round = { game: string; bet: number; multiplier: number; payout: number; profit: number; balance: number; result: any };

export const cget = <T>(serverId: number, path = '') => hostGet<T>(serverId, path);
// Chaos mode is the player's choice, round by round: the instant games carry the switch with the bet.
const CHAOS_GAMES = new Set(['/slots', '/wheel', '/plinko', '/dice', '/coinflip']);
export const cpost = <T>(serverId: number, path: string, body: Record<string, unknown> = {}) =>
  hostPost<T>(serverId, path, CHAOS_GAMES.has(path) ? { ...body, chaos: cprefs.chaos } : body);

export const money = (v: number | null | undefined, digits = 2) =>
  v == null ? '—' : '$' + v.toLocaleString(undefined, { minimumFractionDigits: v % 1 === 0 && digits === 2 ? 0 : digits, maximumFractionDigits: digits });
export const mult = (v: number) => (v >= 100 ? Math.round(v).toLocaleString() : v >= 10 ? v.toFixed(1).replace(/\.0$/, '') : v.toFixed(2).replace(/0$/, '').replace(/\.0$/, '')) + '×';

/** Colour for a multiplier: dull below 1×, green for wins, hot for big ones. */
export const multColor = (m: number) => (m < 1 ? '#64748b' : m < 2 ? '#38bdf8' : m < 5 ? '#22c55e' : m < 20 ? '#eab308' : m < 100 ? '#f97316' : '#ec4899');

export const GLYPH: Record<string, string> = { cherry: '🍒', lemon: '🍋', bell: '🔔', clover: '🍀', gem: '💎', seven: '7️⃣', star: '⭐', crown: '👑', coin: '🪙', heart: '❤️', diamond: '💠' };

export function untilText(iso: string, now = Date.now()) {
  const s = Math.max(0, Math.floor((new Date(iso).getTime() - now) / 1000));
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d}d ${h}h` : h ? `${h}h ${m}m` : m ? `${m}m ${s % 60}s` : `${s}s`;
}
export function ago(iso: string, now = Date.now()) {
  const s = Math.max(1, (now - new Date(iso).getTime()) / 1000);
  return s < 90 ? 'just now' : s < 3600 ? `${Math.round(s / 60)}m ago` : s < 86400 ? `${Math.round(s / 3600)}h ago` : `${Math.round(s / 86400)}d ago`;
}

/** Resolve after `ms` (animations wait on the server's answer, which already happened). */
export const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** Angle (degrees, clockwise from the top) at the middle of segment `i`, for slices sized by weight. */
export function segmentCenter(segments: { weight: number }[], i: number) {
  const total = segments.reduce((a, s) => a + s.weight, 0);
  const before = segments.slice(0, i).reduce((a, s) => a + s.weight, 0);
  return ((before + segments[i].weight / 2) / total) * 360;
}

/** Which slice sits at `deg` (clockwise from the top, any turn) on a wheel whose slices are sized by weight. */
export function segmentAt(segments: { weight: number }[], deg: number) {
  const total = segments.reduce((a, s) => a + s.weight, 0);
  const at = ((((deg % 360) + 360) % 360) / 360) * total;
  let acc = 0;
  for (let i = 0; i < segments.length; i++) { acc += segments[i].weight; if (at < acc) return i; }
  return segments.length - 1;
}
