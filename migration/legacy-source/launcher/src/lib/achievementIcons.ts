// Achievement icon catalogue shared by the admin panel and the launcher.
//
// Achievements store *ids* (`icon_item: "diamond_sword"`, `icon_bg: "obsidian"`,
// `icon_border: "gold"`), not pictures, so every app has to turn them into
// something drawable. This file is that one translation. The launcher keeps an
// identical copy at launcher/src/lib/achievementIcons.ts — change both together.

export interface ItemIcon { id: string; label: string; emoji: string }

export const ITEM_ICONS: ItemIcon[] = [
  { id: 'diamond_pickaxe', label: 'Diamond Pickaxe', emoji: '⛏️' },
  { id: 'wooden_pickaxe', label: 'Wooden Pickaxe', emoji: '⛏️' },
  { id: 'diamond', label: 'Diamond', emoji: '💎' },
  { id: 'emerald', label: 'Emerald', emoji: '🟢' },
  { id: 'gold_ingot', label: 'Gold Ingot', emoji: '🪙' },
  { id: 'iron_sword', label: 'Iron Sword', emoji: '🗡️' },
  { id: 'diamond_sword', label: 'Diamond Sword', emoji: '⚔️' },
  { id: 'netherite_sword', label: 'Netherite Sword', emoji: '⚔️' },
  { id: 'netherite_axe', label: 'Netherite Axe', emoji: '🪓' },
  { id: 'bow', label: 'Bow', emoji: '🏹' },
  { id: 'trident', label: 'Trident', emoji: '🔱' },
  { id: 'shield', label: 'Shield', emoji: '🛡️' },
  { id: 'bricks', label: 'Bricks', emoji: '🧱' },
  { id: 'stone_bricks', label: 'Stone Bricks', emoji: '🧱' },
  { id: 'grass_block', label: 'Grass Block', emoji: '🟩' },
  { id: 'crafting_table', label: 'Crafting Table', emoji: '🪚' },
  { id: 'chest', label: 'Treasure Chest', emoji: '📦' },
  { id: 'beacon', label: 'Beacon', emoji: '✨' },
  { id: 'clock', label: 'Clock', emoji: '🕰️' },
  { id: 'compass', label: 'Compass', emoji: '🧭' },
  { id: 'map', label: 'Map', emoji: '🗺️' },
  { id: 'book', label: 'Book', emoji: '📖' },
  { id: 'enchanted_book', label: 'Enchanted Book', emoji: '📘' },
  { id: 'experience_bottle', label: 'Bottle o’ Enchanting', emoji: '🧪' },
  { id: 'golden_apple', label: 'Golden Apple', emoji: '🍏' },
  { id: 'cake', label: 'Cake', emoji: '🎂' },
  { id: 'elytra', label: 'Elytra', emoji: '🪽' },
  { id: 'ender_eye', label: 'Eye of Ender', emoji: '👁️' },
  { id: 'ender_pearl', label: 'Ender Pearl', emoji: '🔮' },
  { id: 'flint_and_steel', label: 'Flint and Steel', emoji: '🔥' },
  { id: 'blaze_rod', label: 'Blaze Rod', emoji: '🔥' },
  { id: 'heart_of_the_sea', label: 'Heart of the Sea', emoji: '💙' },
  { id: 'nether_star', label: 'Nether Star', emoji: '⭐' },
  { id: 'totem_of_undying', label: 'Totem of Undying', emoji: '🗿' },
  { id: 'dragon_egg', label: 'Dragon Egg', emoji: '🥚' },
  { id: 'dragon_head', label: 'Dragon Head', emoji: '🐉' },
  { id: 'skull', label: 'Skull', emoji: '💀' },
  { id: 'fishing_rod', label: 'Fishing Rod', emoji: '🎣' },
  { id: 'wheat', label: 'Wheat', emoji: '🌾' },
  { id: 'castle', label: 'Castle Flag', emoji: '🚩' },
  { id: 'trophy', label: 'Trophy', emoji: '🏆' },
];

export interface Backdrop { id: string; label: string; css: string }

export const BACKGROUNDS: Backdrop[] = [
  { id: 'stone', label: 'Stone', css: 'radial-gradient(circle, #6b7280 0%, #27272a 100%)' },
  { id: 'deepslate', label: 'Deepslate', css: 'radial-gradient(circle, #27272a 0%, #09090b 100%)' },
  { id: 'obsidian', label: 'Obsidian', css: 'radial-gradient(circle, #2e1065 0%, #0f172a 100%)' },
  { id: 'dirt', label: 'Dirt', css: 'radial-gradient(circle, #92400e 0%, #451a03 100%)' },
  { id: 'bedrock', label: 'Bedrock', css: 'radial-gradient(circle, #3f3f46 0%, #09090b 100%)' },
  { id: 'oak_planks', label: 'Oak Planks', css: 'radial-gradient(circle, #c08a4b 0%, #6b4423 100%)' },
  { id: 'netherrack', label: 'Netherrack', css: 'radial-gradient(circle, #9f1239 0%, #3b0a16 100%)' },
  { id: 'end_stone', label: 'End Stone', css: 'radial-gradient(circle, #e8e2a8 0%, #8a8456 100%)' },
  { id: 'gilded_blackstone', label: 'Gilded Blackstone', css: 'radial-gradient(circle, #a16207 0%, #1c1917 100%)' },
];

export interface Rim { id: string; label: string; color: string }

export const BORDERS: Rim[] = [
  { id: 'bronze', label: 'Bronze', color: '#b45309' },
  { id: 'silver', label: 'Silver', color: '#94a3b8' },
  { id: 'gold', label: 'Gold', color: '#eab308' },
  { id: 'diamond', label: 'Diamond', color: '#06b6d4' },
  { id: 'purple', label: 'Amethyst / Purple', color: '#c084fc' },
  { id: 'netherite', label: 'Netherite', color: '#475569' },
];

export const FRAMES = [
  { id: 'task', label: 'Task (Normal)', bannerTitle: 'Advancement Made!', bannerColor: '#55ff55' },
  { id: 'goal', label: 'Goal (Rounded)', bannerTitle: 'Goal Reached!', bannerColor: '#55ffff' },
  { id: 'challenge', label: 'Challenge (Spiked & Glowing)', bannerTitle: 'Challenge Complete!', bannerColor: '#ff55ff' },
] as const;

// Ids that aren't in the catalogue (added through the API, or from an older
// data set) still get a sensible picture from their name.
const KEYWORDS: [RegExp, string][] = [
  [/pickaxe|shovel|hoe/, '⛏️'], [/axe/, '🪓'], [/sword|dagger/, '⚔️'], [/bow|arrow/, '🏹'], [/trident/, '🔱'],
  [/shield/, '🛡️'], [/helmet|chestplate|leggings|boots|armor/, '🛡️'], [/apple|carrot|bread|food|steak/, '🍏'],
  [/potion|bottle/, '🧪'], [/book/, '📖'], [/ore|ingot|nugget/, '🪙'], [/diamond|emerald|amethyst|gem/, '💎'],
  [/star|beacon/, '✨'], [/egg/, '🥚'], [/head|skull/, '💀'], [/block|brick|stone|planks|log/, '🧱'],
  [/fire|flame|blaze|lava/, '🔥'], [/fish|rod/, '🎣'], [/clock|watch/, '🕰️'], [/map|compass/, '🧭'],
];

const isCss = (v: string) => /^(#|rgb|hsl|linear-gradient|radial-gradient|var\()/i.test(v.trim());
const isId = (v: string) => /^[a-z0-9_]+$/.test(v);

/** Library icons are stored as `iconify:<pack>:<name>[@rrggbb]`; the panel serves each as an SVG. */
export const ICONIFY = /^iconify:([a-z0-9-]+):([a-z0-9_-]+)(?:@([0-9a-f]{3}|[0-9a-f]{6}))?$/i;
export const iconifyValue = (pack: string, name: string, color?: string | null) => `iconify:${pack}:${name}${color ? '@' + color.replace('#', '') : ''}`;

export const imageIcon = (value?: string | null): string | null => {
  const v = value?.trim() ?? '';
  const lib = ICONIFY.exec(v);
  if (lib) return `/api/v1/icons/${lib[1]}/${lib[2]}.svg${lib[3] ? '?color=' + lib[3] : ''}`;
  return /^(https:\/\/|\/uploads\/)/i.test(v) && /\.png(?:[?#].*)?$/i.test(v) ? v : null;
};

export interface ResolvedIcon { image: string | null; emoji: string; background: string; borderColor: string; glow: string }

export function resolveIcon(a: { icon_item?: string | null; icon_bg?: string | null; icon_border?: string | null }): ResolvedIcon {
  const item = (a.icon_item ?? '').trim().replace(/^minecraft:/, '');
  const image = imageIcon(item);
  let emoji = '🏆';
  if (item) {
    const known = ITEM_ICONS.find((i) => i.id === item);
    if (known) emoji = known.emoji;
    else if (!isId(item) && [...item].length <= 4) emoji = item;
    else emoji = KEYWORDS.find(([re]) => re.test(item))?.[1] ?? '🏆';
  }

  const bgRaw = (a.icon_bg ?? '').trim();
  const background = BACKGROUNDS.find((b) => b.id === bgRaw)?.css ?? (bgRaw && isCss(bgRaw) ? bgRaw : BACKGROUNDS[1].css);

  const rimRaw = (a.icon_border ?? '').trim();
  const borderColor = BORDERS.find((b) => b.id === rimRaw)?.color ?? (rimRaw && isCss(rimRaw) ? rimRaw : '#eab308');
  return { image, emoji, background, borderColor, glow: `0 0 12px ${borderColor}99` };
}
