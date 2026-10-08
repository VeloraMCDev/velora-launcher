import { Award, Banknote, Dices, Download, House, Map as MapIcon, Shield, Store, Target, Terminal, Trophy, UserRound, Users } from '@lucide/svelte';
import type { Component } from 'svelte';

export type PlayPage = { id: string; label: string; icon: Component<any>; blurb: string; primary?: boolean; load: () => Promise<{ default: Component<any> }> };

/** Every page of the player panel. `primary` ones sit in the mobile tab bar; the rest live under "More". */
export const PAGES: PlayPage[] = [
  { id: 'home', label: 'Home', icon: House, blurb: 'Your dashboard', primary: true, load: () => import('./pages/Home.svelte') },
  { id: 'experience', label: 'Experience', icon: House, blurb: 'Your experience overview', load: () => import('./pages/Experience.svelte') },
  { id: 'market', label: 'Market', icon: Store, blurb: 'Buy, bid and track your listings', primary: true, load: () => import('./pages/Market.svelte') },
  { id: 'casino', label: 'Casino', icon: Dices, blurb: 'Slots, wheel, plinko, mines', primary: true, load: () => import('./pages/Casino.svelte') },
  { id: 'social', label: 'Friends', icon: Users, blurb: 'Friends, messages and profiles', primary: true, load: () => import('./pages/Social.svelte') },
  { id: 'wallet', label: 'Wallet', icon: Banknote, blurb: 'Balances and transactions', load: () => import('./pages/Wallet.svelte') },
  { id: 'guilds', label: 'Guilds', icon: Shield, blurb: 'Your guild, bank and land', load: () => import('./pages/Guilds.svelte') },
  { id: 'quests', label: 'Quests', icon: Target, blurb: 'Quests, achievements and levels', load: () => import('./pages/Quests.svelte') },
  { id: 'collections', label: 'Collection', icon: Award, blurb: 'Your unlocked cosmetics and titles', load: () => import('./pages/Collections.svelte') },
  { id: 'stats', label: 'Leaderboards', icon: Trophy, blurb: 'Rankings and your stats', load: () => import('./pages/Stats.svelte') },
  { id: 'map', label: 'Live map', icon: MapIcon, blurb: 'See the world and who is on', load: () => import('./pages/LiveMap.svelte') },
  { id: 'launcher', label: 'Launcher', icon: Download, blurb: 'Download and instances', load: () => import('./pages/Launcher.svelte') },
  { id: 'commands', label: 'Commands', icon: Terminal, blurb: 'Every in-game command', load: () => import('./pages/Commands.svelte') },
  { id: 'profile', label: 'Account', icon: UserRound, blurb: 'Skin, cape and sign-in', load: () => import('./pages/Profile.svelte') },
];
