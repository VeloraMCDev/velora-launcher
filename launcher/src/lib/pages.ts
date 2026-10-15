import type { Component } from 'svelte';
import { PageLoads } from './page-loads';

const loaders = {
  settings: () => import('../pages/Settings.svelte'),
  stats: () => import('../pages/Stats.svelte'),
  quests: () => import('../pages/Quests.svelte'),
  guilds: () => import('../pages/Guilds.svelte'),
  social: () => import('../pages/Social.svelte'),
  commands: () => import('../pages/Commands.svelte'),
  collections: () => import('../pages/Collections.svelte'),
  market: () => import('../pages/Market.svelte'),
  casino: () => import('../pages/Casino.svelte'),
};

export type LazyPageName = keyof typeof loaders;
const pages = new PageLoads<{ default: Component }>();

export function loadPage(name: LazyPageName) {
  return pages.load(name, loaders[name]);
}
