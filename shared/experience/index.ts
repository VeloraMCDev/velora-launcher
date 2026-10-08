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

export function preset(kind: 'smp' | 'frontiers'): Experience {
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
