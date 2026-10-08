import type { Branding } from './branding';
export type { Branding } from './branding';

/** API v1 presentation declarations. Experience policy and presets belong to their owner. */
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
export const defaultExperience = (): Experience => ({ kind: 'generic', branding: null, features: [], navigation: [], widgets: [], modules: {} });
export const enabled = (experience: Experience | undefined, feature: Feature) => experience?.features.includes(feature) ?? false;

/** Host-supplied availability controls pages; manifests cannot add unregistered pages. */
export function navigation(experience: Experience | undefined, defaults: { id: string; label: string }[], available: (page: string) => boolean = () => true) {
  const pages = defaults.filter(p => available(p.id));
  const configured = experience?.navigation ?? [];
  return configured.length
    ? [...configured.filter(p => pages.some(d => d.id === p.id)), ...pages.filter(p => !configured.some(c => c.id === p.id))]
    : pages;
}

/** Text/link fallback does not execute manifest-provided code. */
export const safeLink = (value: unknown) => typeof value === 'string' && /^https?:\/\//i.test(value) ? value : null;
