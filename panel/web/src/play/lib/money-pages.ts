// Plain helpers shared by the Home, Wallet, Market and Stats pages (no runes here).

export const stripColors = (s: string) => (s ?? '').replace(/§./g, '');

/** "minecraft:diamond_sword" -> "Diamond Sword" */
export function prettyItem(id: string): string {
  return (id.split(':').pop() ?? id).split('_').map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
}

export function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
  return h >>> 0;
}

const EMOJI: [RegExp, string][] = [
  [/sword/, '⚔️'], [/pickaxe/, '⛏️'], [/axe/, '🪓'], [/shovel/, '🥄'], [/hoe/, '🌾'], [/bow|arrow/, '🏹'], [/helmet|chestplate|leggings|boots|shield/, '🛡️'],
  [/diamond/, '💎'], [/emerald/, '💚'], [/gold/, '🪙'], [/iron|netherite|copper/, '🔩'], [/apple|golden_apple/, '🍎'], [/bread|cookie|cake|pie/, '🍞'],
  [/potion|bottle/, '🧪'], [/book|enchant/, '📖'], [/elytra/, '🪽'], [/totem/, '🗿'], [/tnt/, '🧨'], [/trident/, '🔱'], [/log|plank|wood/, '🪵'],
  [/stone|cobble|deepslate|ore/, '🪨'], [/dirt|grass|sand/, '🟫'], [/egg|spawn/, '🥚'], [/fish|salmon|cod/, '🐟'], [/beef|pork|mutton|chicken/, '🍖'],
  [/wheat|carrot|potato|seed|melon/, '🌱'], [/star|beacon/, '⭐'], [/redstone/, '🔴'], [/lapis/, '🔵'], [/coal|charcoal/, '⚫'], [/bucket/, '🪣'],
];
export function itemEmoji(id: string): string | null {
  const k = id.split(':').pop() ?? id;
  for (const [re, e] of EMOJI) if (re.test(k)) return e;
  return null;
}

/** A stable pair of hues for an item id, used for the gradient tile. */
export function itemGradient(id: string): string {
  const h = hash(id) % 360;
  return `linear-gradient(145deg, hsl(${h} 62% 46%), hsl(${(h + 48) % 360} 58% 28%))`;
}

export function timeLeft(iso: string | null | undefined, now: number): string {
  if (!iso) return '';
  const s = Math.floor((parseTime(iso) - now) / 1000);
  if (s <= 0) return 'ending…';
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  return d ? `${d}d ${h}h` : h ? `${h}h ${m}m` : m ? `${m}m ${String(s % 60).padStart(2, '0')}s` : `${s}s`;
}

/** The panel stores timestamps as `YYYY-MM-DD HH:MM:SS` (UTC) or RFC3339; handle both. */
export function parseTime(iso: string): number {
  if (!iso) return NaN;
  const t = /^\d{4}-\d\d-\d\d \d\d:\d\d/.test(iso) && !/[zZ+]/.test(iso.slice(10)) ? iso.replace(' ', 'T') + 'Z' : iso;
  return new Date(t).getTime();
}

export function dayLabel(iso: string): string {
  const t = parseTime(iso);
  if (isNaN(t)) return 'Earlier';
  const d = new Date(t), now = new Date();
  const start = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((start(now) - start(d)) / 86400000);
  if (diff <= 0) return 'Today';
  if (diff === 1) return 'Yesterday';
  if (diff < 7) return d.toLocaleDateString(undefined, { weekday: 'long' });
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: d.getFullYear() === now.getFullYear() ? undefined : 'numeric' });
}

export const clock = (iso: string) => { const t = parseTime(iso); return isNaN(t) ? '' : new Date(t).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }); };

export function ago(iso: string | null | undefined): string {
  if (!iso) return 'never';
  const s = Math.max(1, (Date.now() - parseTime(iso)) / 1000);
  if (s < 60) return 'just now';
  if (s < 3600) return `${Math.round(s / 60)}m ago`;
  if (s < 86400) return `${Math.round(s / 3600)}h ago`;
  if (s < 86400 * 30) return `${Math.round(s / 86400)}d ago`;
  return new Date(parseTime(iso)).toLocaleDateString();
}

export function playtime(secs: number | null | undefined): string {
  const s = secs ?? 0;
  if (s < 60) return `${s}s`;
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600), m = Math.floor((s % 3600) / 60);
  if (d) return `${d}d ${h}h`;
  return h ? `${h}h ${m}m` : `${m}m`;
}

export const greeting = () => { const h = new Date().getHours(); return h < 5 ? 'Late night session' : h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening'; };

export const num = (n: number | null | undefined) => (n ?? 0).toLocaleString();
