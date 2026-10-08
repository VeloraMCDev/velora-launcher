// Metadata for the Content Studio: what each kind is, which frameworks bring it, and the shapes the API returns.
import type { Component } from 'svelte';
import { Pickaxe, Swords, Shield, Apple, Package, Box, Archive, Armchair, UserRound, Car, Sprout, Skull, Wand2, FileArchive, Hammer, Cuboid } from '@lucide/svelte';

export type KindId = 'tool' | 'weapon' | 'armor' | 'food' | 'item' | 'block' | 'chest' | 'decoration' | 'npc' | 'vehicle' | 'crop' | 'mob';
export interface KindMeta { id: KindId; label: string; plural: string; blurb: string; color: string; icon: Component<any>; store: 'item' | 'content' }

export const KINDS: KindMeta[] = [
  { id: 'weapon', label: 'Weapon', plural: 'Weapons', blurb: 'Swords, bows, staves — anything you swing or shoot', color: '#ff6b6b', icon: Swords, store: 'item' },
  { id: 'tool', label: 'Tool', plural: 'Tools', blurb: 'Pickaxes, axes, hammers, rods', color: '#ffb84d', icon: Pickaxe, store: 'item' },
  { id: 'armor', label: 'Armor', plural: 'Armor', blurb: 'Helmets, chestplates, boots, shields', color: '#5aa9ff', icon: Shield, store: 'item' },
  { id: 'food', label: 'Food', plural: 'Food', blurb: 'Things players can eat', color: '#f0a35e', icon: Apple, store: 'item' },
  { id: 'item', label: 'Other item', plural: 'Other items', blurb: 'Keys, gems, currency, quest items', color: '#c792ea', icon: Package, store: 'item' },
  { id: 'block', label: 'Block', plural: 'Blocks', blurb: 'Solid blocks players can place and mine', color: '#8bd17c', icon: Box, store: 'content' },
  { id: 'chest', label: 'Chest', plural: 'Chests', blurb: 'Storage with its own look', color: '#e0b04a', icon: Archive, store: 'content' },
  { id: 'decoration', label: 'Decoration', plural: 'Decorations', blurb: 'Furniture, lamps, statues — some you can sit on', color: '#4dd6d0', icon: Armchair, store: 'content' },
  { id: 'npc', label: 'NPC', plural: 'NPCs', blurb: 'Characters that talk and run commands', color: '#6fb3ff', icon: UserRound, store: 'content' },
  { id: 'vehicle', label: 'Vehicle', plural: 'Vehicles', blurb: 'Something players ride', color: '#ff8fcf', icon: Car, store: 'content' },
  { id: 'crop', label: 'Crop', plural: 'Crops', blurb: 'Plants that grow in stages and can be harvested', color: '#7bd88f', icon: Sprout, store: 'content' },
  { id: 'mob', label: 'Mob', plural: 'Mobs', blurb: 'Monsters and animals with their own stats', color: '#e5484d', icon: Skull, store: 'content' },
];
export const kindMeta = (id: string): KindMeta => KINDS.find((k) => k.id === id) ?? KINDS[4];
export const ITEM_KIND_IDS = KINDS.filter((k) => k.store === 'item').map((k) => k.id);

export interface FrameworkMeta { id: string; label: string; color: string; blurb: string; zip: string; brings: KindId[] }
export const FRAMEWORKS: FrameworkMeta[] = [
  { id: 'itemsadder', label: 'ItemsAdder', color: '#4da3ff', blurb: 'Items, blocks, furniture and food from ItemsAdder packs', zip: 'Zip plugins/ItemsAdder/contents (or one pack folder inside it).', brings: ['weapon', 'tool', 'armor', 'food', 'item', 'block', 'chest', 'decoration', 'vehicle', 'crop'] },
  { id: 'nexo', label: 'Nexo', color: '#b06bff', blurb: 'Nexo items, custom blocks and furniture', zip: 'Zip plugins/Nexo/items and plugins/Nexo/pack together.', brings: ['weapon', 'tool', 'armor', 'food', 'item', 'block', 'chest', 'decoration', 'crop'] },
  { id: 'oraxen', label: 'Oraxen', color: '#3ddc97', blurb: 'Oraxen items, note-block blocks and furniture', zip: 'Zip plugins/Oraxen/items and plugins/Oraxen/pack together.', brings: ['weapon', 'tool', 'armor', 'food', 'item', 'block', 'chest', 'decoration', 'crop'] },
  { id: 'craftengine', label: 'CraftEngine', color: '#ffa94d', blurb: 'CraftEngine items, blocks and furniture', zip: 'Zip plugins/CraftEngine/resources (the folders holding configuration/ and resourcepack/).', brings: ['weapon', 'tool', 'armor', 'food', 'item', 'block', 'decoration'] },
  { id: 'modelengine', label: 'ModelEngine', color: '#ff6fb1', blurb: 'Blockbench models (.bbmodel) for NPCs, vehicles and decorations', zip: 'Zip plugins/ModelEngine/blueprints (the .bbmodel files).', brings: ['npc', 'vehicle', 'decoration', 'mob'] },
  { id: 'mythicmobs', label: 'MythicMobs', color: '#ff5d5d', blurb: 'Mob names, health, damage and drops (with their ModelEngine model)', zip: 'Zip plugins/MythicMobs/Mobs, plus the .bbmodel files they use.', brings: ['mob', 'npc'] },
];
export const frameworkMeta = (id: string): FrameworkMeta | undefined => FRAMEWORKS.find((f) => f.id === id);

export interface ViewBundle { mode: 'elements' | 'flat'; elements?: unknown[]; textures?: Record<string, string | null>; layers?: string[] }
export interface Analyzed {
  key: string; framework: string; id: string; kind: KindId; title: string; name: string | null; lore: string[]; material: string; cmd: number | null;
  texture: string | null; model: string | null; scale: number; extras: Record<string, any>; warnings: string[]; missing: string[];
  preview: string | null; exists: boolean; store: 'item' | 'content'; view?: ViewBundle;
}
export interface Analysis {
  detected: { framework: string; label: string; entries: number }[];
  entries: Analyzed[]; notes: string[]; skipped: string[]; files: number; files_replaced: number;
}
export interface LibraryItem { id: string; kind: KindId; title: string; source: string; store: 'item' | 'content'; name: string | null; item: string | null; preview: string | null; has_look: boolean }

export const SOURCE_LABEL: Record<string, string> = { manual: 'Hand-made', ...Object.fromEntries(FRAMEWORKS.map((f) => [f.id, f.label])) };
export const ICONS_FALLBACK = { Wand2, FileArchive, Hammer, Cuboid };
