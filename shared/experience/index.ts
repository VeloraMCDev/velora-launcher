import type { Branding } from './branding';
export type { Branding } from './branding';

/** Shared capability contract and presentation registry for every platform surface. */
export const FEATURES = ['economy', 'casino', 'guilds', 'progression', 'quests', 'achievements', 'collections', 'maps', 'content', 'commands', 'events', 'companion'] as const;
export type Feature = typeof FEATURES[number];
export type Experience = {
  kind: string;
  branding: Branding | null;
  features: Feature[];
  navigation: { id: string; label: string }[];
  widgets: { id: string; component: string; title: string; body: string; config: Record<string, unknown> }[];
  modules: Record<string, unknown>;
};
export const defaultExperience = (): Experience => ({ kind: 'smp', branding: null, features: [...FEATURES], navigation: [], widgets: [], modules: {} });
export const enabled = (experience: Experience | undefined, feature: Feature) => (experience?.features ?? FEATURES).includes(feature);

export const PAGE_FEATURES: Record<string, Feature> = {
  market: 'economy', wallet: 'economy', economy: 'economy', casino: 'casino', board: 'economy',
  guilds: 'guilds', 'admin-claims': 'guilds', leveling: 'progression', progression: 'progression', stats: 'progression', leaderboards: 'progression',
  quests: 'quests', 'quest-chains': 'quests', achievements: 'achievements', collections: 'collections', cosmetics: 'collections',
  map: 'maps', items: 'content', commands: 'commands', chat: 'commands', events: 'events', companion: 'companion',
  bulk: 'progression', 'reward-queue': 'progression', luckperms: 'progression',
};
export const pageEnabled = (experience: Experience | undefined, page: string) => !PAGE_FEATURES[page] || enabled(experience, PAGE_FEATURES[page]);
export const PLATFORM_PAGES = new Set(['instances', 'users', 'activity', 'branding', 'landing-builder', 'capes', 'emails', 'settings']);

export const CORE_MODULES = ['map', 'economy', 'vaults', 'casino', 'analytics', 'factions', 'permissions_chat'] as const;
export type CoreModule = typeof CORE_MODULES[number];
export type CorePolicy = {
  schema: 1; modules: Record<CoreModule, boolean>; currency: 'dollars'; starting_balance_cents: number;
  map_layers: Record<'players' | 'claims' | 'spawn' | 'warps' | 'homes' | 'shops', boolean>;
  virtual_market_fee_bps: number; faction_creation_cents: number; faction_base_claims: number; faction_max_members: number;
  upkeep_chunk_cents: number; upkeep_member_cents: number; upkeep_grace_days: number; rivalry_cooldown_minutes: number;
  faction_outpost_limit: number; upgrade_claims_chunks: number; upgrade_claims_cents: number; upgrade_claims_max: number;
  upgrade_members_slots: number; upgrade_members_cents: number; upgrade_members_max: number;
  upgrade_outposts_cents: number; upgrade_outposts_max: number; faction_vault_cents: number; faction_vault_max: number;
  vault_price_cents: number; casino_abandon_hours: number; analytics_retention_days: number;
  shop_requires_claim: boolean; shop_promotion_cents_per_day: number;
};
export const corePolicy = (): CorePolicy => ({ schema: 1,
  modules: { map: true, economy: true, vaults: true, casino: true, analytics: true, factions: true, permissions_chat: true },
  map_layers: { players: true, claims: true, spawn: true, warps: true, homes: true, shops: true },
  currency: 'dollars', starting_balance_cents: 100_000, virtual_market_fee_bps: 1_000,
  faction_creation_cents: 75_000, faction_base_claims: 16, faction_max_members: 8,
  upkeep_chunk_cents: 200, upkeep_member_cents: 100, upkeep_grace_days: 3, rivalry_cooldown_minutes: 60,
  faction_outpost_limit: 1, upgrade_claims_chunks: 8, upgrade_claims_cents: 50_000, upgrade_claims_max: 10,
  upgrade_members_slots: 2, upgrade_members_cents: 75_000, upgrade_members_max: 6,
  upgrade_outposts_cents: 100_000, upgrade_outposts_max: 3, faction_vault_cents: 150_000, faction_vault_max: 3,
  vault_price_cents: 250_000, casino_abandon_hours: 24, analytics_retention_days: 90,
  shop_requires_claim: true, shop_promotion_cents_per_day: 5_000 });
export function coreFeatures(policy: CorePolicy): Feature[] {
  const features: Feature[] = [];
  for (const [module, feature] of [['map', 'maps'], ['economy', 'economy'], ['casino', 'casino'], ['analytics', 'progression'], ['factions', 'guilds']] as const) {
    if (policy.modules[module]) features.push(feature);
  }
  if (policy.modules.vaults || policy.modules.permissions_chat) features.push('commands');
  return features;
}
export function preset(kind: 'velora-smp' | 'smp' | 'frontiers'): Experience {
  if (kind === 'velora-smp') {
    const policy = corePolicy();
    return { kind, branding: null, features: coreFeatures(policy), navigation: [{ id: 'guilds', label: 'Factions' }],
      widgets: [], modules: { velora_core: policy } };
  }
  if (kind === 'smp') return defaultExperience();
  return {
    kind, branding: null, features: ['maps', 'content', 'events', 'companion'],
    navigation: [{ id: 'home', label: 'Frontier overview' }, { id: 'experience', label: 'Settlements & resources' }],
    widgets: [{ id: 'frontiers', component: 'frontiers', title: 'Build your frontier', body: 'Plan settlements, coordinate resources, and grow together.', config: {} }],
    modules: { frontiers: { settlements: [], resources: [] } },
  };
}

/** Declarative navigation never supplies executable code or remote components. */
export function navigation(experience: Experience | undefined, defaults: { id: string; label: string }[]) {
  const pages = defaults.filter(p => pageEnabled(experience, p.id));
  const configured = experience?.navigation ?? [];
  return configured.length
    ? [...configured.filter(p => pages.some(d => d.id === p.id)), ...pages.filter(p => !configured.some(c => c.id === p.id))]
    : pages;
}
