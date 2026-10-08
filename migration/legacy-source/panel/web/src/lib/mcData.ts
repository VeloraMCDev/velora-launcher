// Catalogues behind the pickers: enchantments, attributes, equipment slots and base items.

export type EnchantCategory = 'melee' | 'armor' | 'tools' | 'bows' | 'trident' | 'fishing' | 'general';
export interface EnchantInfo { id: string; name: string; max: number; cat: EnchantCategory; blurb: string }

export const ENCHANT_CATEGORIES: { id: EnchantCategory | 'all'; label: string; color: string }[] = [
  { id: 'all', label: 'All', color: '#9aa0b4' },
  { id: 'melee', label: 'Melee', color: '#ff6b6b' },
  { id: 'armor', label: 'Armor', color: '#5aa9ff' },
  { id: 'tools', label: 'Tools', color: '#ffb84d' },
  { id: 'bows', label: 'Bows', color: '#7bd88f' },
  { id: 'trident', label: 'Trident', color: '#4dd6d0' },
  { id: 'fishing', label: 'Fishing', color: '#6fb3ff' },
  { id: 'general', label: 'General', color: '#c792ea' },
];
export const enchantColor = (c: string) => ENCHANT_CATEGORIES.find((x) => x.id === c)?.color ?? '#9aa0b4';

export const ENCHANTS: EnchantInfo[] = [
  { id: 'sharpness', name: 'Sharpness', max: 5, cat: 'melee', blurb: 'More damage to every mob' },
  { id: 'smite', name: 'Smite', max: 5, cat: 'melee', blurb: 'More damage to undead' },
  { id: 'bane_of_arthropods', name: 'Bane of Arthropods', max: 5, cat: 'melee', blurb: 'More damage to spiders and bugs' },
  { id: 'knockback', name: 'Knockback', max: 2, cat: 'melee', blurb: 'Launches targets away' },
  { id: 'fire_aspect', name: 'Fire Aspect', max: 2, cat: 'melee', blurb: 'Sets targets on fire' },
  { id: 'looting', name: 'Looting', max: 3, cat: 'melee', blurb: 'More drops from mobs' },
  { id: 'sweeping_edge', name: 'Sweeping Edge', max: 3, cat: 'melee', blurb: 'Stronger sweep attacks' },
  { id: 'density', name: 'Density', max: 5, cat: 'melee', blurb: 'Mace: more damage per block fallen' },
  { id: 'breach', name: 'Breach', max: 4, cat: 'melee', blurb: 'Mace: cuts through armor' },
  { id: 'wind_burst', name: 'Wind Burst', max: 3, cat: 'melee', blurb: 'Mace: bounces you upward' },
  { id: 'protection', name: 'Protection', max: 4, cat: 'armor', blurb: 'Less damage from everything' },
  { id: 'fire_protection', name: 'Fire Protection', max: 4, cat: 'armor', blurb: 'Less fire damage' },
  { id: 'blast_protection', name: 'Blast Protection', max: 4, cat: 'armor', blurb: 'Less explosion damage' },
  { id: 'projectile_protection', name: 'Projectile Protection', max: 4, cat: 'armor', blurb: 'Less arrow damage' },
  { id: 'feather_falling', name: 'Feather Falling', max: 4, cat: 'armor', blurb: 'Boots: softer landings' },
  { id: 'respiration', name: 'Respiration', max: 3, cat: 'armor', blurb: 'Helmet: breathe longer underwater' },
  { id: 'aqua_affinity', name: 'Aqua Affinity', max: 1, cat: 'armor', blurb: 'Helmet: mine fast underwater' },
  { id: 'thorns', name: 'Thorns', max: 3, cat: 'armor', blurb: 'Hurts attackers' },
  { id: 'depth_strider', name: 'Depth Strider', max: 3, cat: 'armor', blurb: 'Boots: swim faster' },
  { id: 'frost_walker', name: 'Frost Walker', max: 2, cat: 'armor', blurb: 'Boots: freeze water underfoot' },
  { id: 'soul_speed', name: 'Soul Speed', max: 3, cat: 'armor', blurb: 'Boots: run fast on soul sand' },
  { id: 'swift_sneak', name: 'Swift Sneak', max: 3, cat: 'armor', blurb: 'Leggings: sneak faster' },
  { id: 'efficiency', name: 'Efficiency', max: 5, cat: 'tools', blurb: 'Mine faster' },
  { id: 'silk_touch', name: 'Silk Touch', max: 1, cat: 'tools', blurb: 'Mine blocks as they are' },
  { id: 'fortune', name: 'Fortune', max: 3, cat: 'tools', blurb: 'More drops from ores' },
  { id: 'power', name: 'Power', max: 5, cat: 'bows', blurb: 'Arrows hit harder' },
  { id: 'punch', name: 'Punch', max: 2, cat: 'bows', blurb: 'Arrows knock back' },
  { id: 'flame', name: 'Flame', max: 1, cat: 'bows', blurb: 'Arrows set targets on fire' },
  { id: 'infinity', name: 'Infinity', max: 1, cat: 'bows', blurb: 'Never run out of arrows' },
  { id: 'multishot', name: 'Multishot', max: 1, cat: 'bows', blurb: 'Crossbow: three arrows at once' },
  { id: 'quick_charge', name: 'Quick Charge', max: 3, cat: 'bows', blurb: 'Crossbow: reload faster' },
  { id: 'piercing', name: 'Piercing', max: 4, cat: 'bows', blurb: 'Crossbow: arrows pass through' },
  { id: 'loyalty', name: 'Loyalty', max: 3, cat: 'trident', blurb: 'Returns when thrown' },
  { id: 'riptide', name: 'Riptide', max: 3, cat: 'trident', blurb: 'Launches you through water' },
  { id: 'channeling', name: 'Channeling', max: 1, cat: 'trident', blurb: 'Calls lightning in storms' },
  { id: 'impaling', name: 'Impaling', max: 5, cat: 'trident', blurb: 'More damage to sea creatures' },
  { id: 'luck_of_the_sea', name: 'Luck of the Sea', max: 3, cat: 'fishing', blurb: 'Better catches' },
  { id: 'lure', name: 'Lure', max: 3, cat: 'fishing', blurb: 'Fish bite sooner' },
  { id: 'unbreaking', name: 'Unbreaking', max: 3, cat: 'general', blurb: 'Lasts longer' },
  { id: 'mending', name: 'Mending', max: 1, cat: 'general', blurb: 'Repairs with experience' },
  { id: 'vanishing_curse', name: 'Curse of Vanishing', max: 1, cat: 'general', blurb: 'Disappears on death' },
  { id: 'binding_curse', name: 'Curse of Binding', max: 1, cat: 'general', blurb: 'Cannot be taken off' },
];

export const ROMAN = ['', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'];
export const roman = (n: number) => (n >= 1 && n <= 10 ? ROMAN[n] : String(n));
export const enchantName = (id: string) => ENCHANTS.find((e) => e.id === id)?.name ?? id.replace(/^minecraft:/, '').replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());

export type AttrOp = 'add' | 'multiply_base' | 'multiply_total';
export interface AttrInfo { id: string; name: string; blurb: string; color: string; min: number; max: number; step: number; start: number; unit: string; icon: string }
export const ATTRIBUTES: AttrInfo[] = [
  { id: 'generic.attack_damage', name: 'Attack damage', blurb: 'Hits harder', color: '#ff6b6b', min: -10, max: 100, step: 0.5, start: 8, unit: 'dmg', icon: 'sword' },
  { id: 'generic.attack_speed', name: 'Attack speed', blurb: 'Swings faster', color: '#ffb84d', min: -3, max: 10, step: 0.1, start: 1, unit: '', icon: 'zap' },
  { id: 'generic.max_health', name: 'Max health', blurb: 'More hearts', color: '#ff5d8f', min: -20, max: 100, step: 1, start: 4, unit: 'hp', icon: 'heart' },
  { id: 'generic.movement_speed', name: 'Movement speed', blurb: 'Runs faster', color: '#5ad1ff', min: -0.1, max: 0.5, step: 0.005, start: 0.02, unit: '', icon: 'wind' },
  { id: 'generic.armor', name: 'Armor', blurb: 'Takes less damage', color: '#9aa7c7', min: -10, max: 30, step: 1, start: 4, unit: 'pts', icon: 'shield' },
  { id: 'generic.armor_toughness', name: 'Armor toughness', blurb: 'Armor holds up against big hits', color: '#7d8fb3', min: 0, max: 20, step: 0.5, start: 2, unit: '', icon: 'shield-check' },
  { id: 'generic.knockback_resistance', name: 'Knockback resistance', blurb: 'Harder to push around', color: '#8b8fa5', min: 0, max: 1, step: 0.05, start: 0.3, unit: '', icon: 'anchor' },
  { id: 'generic.attack_knockback', name: 'Attack knockback', blurb: 'Pushes targets away', color: '#e68a4d', min: 0, max: 10, step: 0.5, start: 1, unit: '', icon: 'move' },
  { id: 'generic.luck', name: 'Luck', blurb: 'Better loot', color: '#5fd68a', min: -10, max: 10, step: 0.5, start: 1, unit: '', icon: 'clover' },
];
export const attrInfo = (id: string) => ATTRIBUTES.find((a) => a.id === id);

export const OPERATIONS: { id: AttrOp; label: string; hint: string }[] = [
  { id: 'add', label: 'Flat', hint: 'Adds the number as is' },
  { id: 'multiply_base', label: '% of base', hint: 'Adds a percentage of the base value' },
  { id: 'multiply_total', label: '% of total', hint: 'Multiplies the final value' },
];

export const SLOTS: { id: string; label: string }[] = [
  { id: 'mainhand', label: 'Main hand' }, { id: 'offhand', label: 'Off hand' }, { id: 'head', label: 'Head' },
  { id: 'chest', label: 'Chest' }, { id: 'legs', label: 'Legs' }, { id: 'feet', label: 'Feet' }, { id: 'any', label: 'Anywhere' },
];

export interface BaseGroup { id: string; label: string; items: string[] }
const mk = (...ids: string[]) => ids.map((i) => 'minecraft:' + i);
export const BASE_GROUPS: BaseGroup[] = [
  { id: 'weapons', label: 'Weapons', items: mk('wooden_sword', 'stone_sword', 'iron_sword', 'golden_sword', 'diamond_sword', 'netherite_sword', 'bow', 'crossbow', 'trident', 'mace') },
  { id: 'tools', label: 'Tools', items: mk('wooden_pickaxe', 'stone_pickaxe', 'iron_pickaxe', 'golden_pickaxe', 'diamond_pickaxe', 'netherite_pickaxe', 'iron_axe', 'diamond_axe', 'netherite_axe', 'iron_shovel', 'diamond_shovel', 'netherite_shovel', 'iron_hoe', 'diamond_hoe', 'netherite_hoe', 'fishing_rod', 'shears', 'flint_and_steel') },
  { id: 'armor', label: 'Armor', items: mk('leather_helmet', 'chainmail_helmet', 'iron_helmet', 'golden_helmet', 'diamond_helmet', 'netherite_helmet', 'iron_chestplate', 'diamond_chestplate', 'netherite_chestplate', 'iron_leggings', 'diamond_leggings', 'netherite_leggings', 'iron_boots', 'diamond_boots', 'netherite_boots', 'turtle_helmet', 'elytra', 'shield') },
  { id: 'food', label: 'Food', items: mk('apple', 'golden_apple', 'bread', 'cooked_beef', 'cooked_chicken', 'cooked_porkchop', 'carrot', 'baked_potato', 'cookie', 'melon_slice', 'pumpkin_pie', 'mushroom_stew') },
  { id: 'misc', label: 'Everything else', items: mk('paper', 'stick', 'nether_star', 'diamond', 'emerald', 'gold_ingot', 'iron_ingot', 'blaze_rod', 'ender_pearl', 'totem_of_undying', 'compass', 'clock', 'book', 'enchanted_book', 'player_head', 'amethyst_shard', 'echo_shard', 'heart_of_the_sea', 'phantom_membrane', 'spyglass') },
];
export const itemLabel = (id: string) => id.replace(/^minecraft:/, '').replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
