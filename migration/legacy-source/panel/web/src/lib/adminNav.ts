import { PLATFORM_PAGES, pageEnabled, type Experience } from '@scopenet/experience';
import {
  Activity, Banknote, Boxes, ClipboardList, CalendarClock, Dice5, Flag, Globe, LayoutDashboard, Mail, MessageSquare, MessagesSquare, Palette, Server, Settings, Shield,
  ShieldCheck, SlidersHorizontal, Sparkles, Table2, Target, Terminal, Trophy, Users, Wand2, CalendarDays, Key, Shirt, Link2, Package, Monitor,
} from '@lucide/svelte';
import type { Component } from 'svelte';

export type AdminItem = { id: string; label: string; icon: Component<any>; hint: string };
export type AdminGroup = { id: string; label: string; items: AdminItem[] };

/** The admin panel's pages, grouped the way an admin thinks about them. The route name is the item id. */
export const ADMIN_GROUPS: AdminGroup[] = [
  { id: 'overview', label: 'Overview', items: [
    { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard, hint: 'Launches, players and server health' },
    { id: 'activity', label: 'Activity', icon: Activity, hint: 'Recent sign-ins and admin actions' },
  ] },
  { id: 'launcher', label: 'Launcher & site', items: [
    { id: 'instances', label: 'Instances', icon: Boxes, hint: 'Versions and modpacks players can play' },
    { id: 'branding', label: 'Launcher design', icon: Palette, hint: 'Name, colours, news and background' },
    { id: 'landing-builder', label: 'Landing page', icon: Globe, hint: 'Your public website' },
    { id: 'capes', label: 'Capes', icon: Flag, hint: 'Capes players can wear' },
  ] },
  { id: 'community', label: 'Community', items: [
    { id: 'users', label: 'Players', icon: Users, hint: 'Accounts, roles and groups' },
    { id: 'guilds', label: 'Guilds & claims', icon: Shield, hint: 'Guilds, land and join rules' },
    { id: 'events', label: 'Events', icon: CalendarDays, hint: 'Community events and objectives' },
    { id: 'discord', label: 'Discord', icon: MessageSquare, hint: 'Bot, roles and announcements' },
    { id: 'emails', label: 'Emails', icon: Mail, hint: 'Templates and mailing' },
  ] },
  { id: 'game', label: 'Game', items: [
    { id: 'servers', label: 'Servers', icon: Server, hint: 'Connected game servers and the map' },
    { id: 'quests', label: 'Quests', icon: Target, hint: 'Daily and weekly quests' },
    { id: 'quest-chains', label: 'Quest Chains', icon: Link2, hint: 'Multi-step quest sequences' },
    { id: 'achievements', label: 'Achievements', icon: Trophy, hint: 'Achievements and icons' },
    { id: 'leveling', label: 'Leveling & XP', icon: Sparkles, hint: 'Levels, ranks and rewards' },
    { id: 'reward-queue', label: 'Reward Queue', icon: Package, hint: 'Inspect and retry failed deliveries' },
    { id: 'progression', label: 'Progression', icon: SlidersHorizontal, hint: 'XP sources, limits and starting balance' },
    { id: 'cosmetics', label: 'Cosmetics Studio', icon: Shirt, hint: 'Create pets, particles, titles and custom messages' },
    { id: 'bulk', label: 'Bulk editor', icon: Table2, hint: 'Edit XP and money for many quests, achievements and rewards at once' },
    { id: 'items', label: 'Content Studio', icon: Wand2, hint: 'Custom items, blocks and models' },
    { id: 'commands', label: 'Commands & kits', icon: Terminal, hint: 'Everyday commands, kits and warps' },
    { id: 'companion', label: 'Companion Builder', icon: Monitor, hint: 'In-game overlay widgets' },
    { id: 'admin-claims', label: 'Admin claims', icon: ShieldCheck, hint: 'Protected server land' },
    { id: 'chat', label: 'Chat format', icon: MessagesSquare, hint: 'Prefixes and chat layout' },
  ] },
  { id: 'economy', label: 'Economy', items: [
    { id: 'economy', label: 'Economy', icon: Banknote, hint: 'Stats, balances and the market' },
    { id: 'casino', label: 'Casino', icon: Dice5, hint: 'Odds, limits, bounties and bets' },
    { id: 'board', label: 'Orders & contracts', icon: ClipboardList, hint: 'Buy orders and generated contracts' },
  ] },
  { id: 'system', label: 'System', items: [
    { id: 'luckperms', label: 'LuckPerms', icon: Key, hint: 'Group mappings and permission sync' },
    { id: 'tasks', label: 'Scheduled tasks', icon: CalendarClock, hint: 'Background jobs' },
    { id: 'settings', label: 'Settings', icon: Settings, hint: 'Sign-ups, downloads and integrations' },
  ] },
];

export const ADMIN_ITEMS = ADMIN_GROUPS.flatMap((g) => g.items.map((i) => ({ ...i, group: g })));
export const findAdminItem = (route: string) => ADMIN_ITEMS.find((i) => i.id === route) ?? null;

export function adminGroups(instanceId: string | null, experience?: Experience): AdminGroup[] {
  const shared = ADMIN_GROUPS.flatMap(g => g.items).filter(i => PLATFORM_PAGES.has(i.id));
  if (!instanceId) return [{ id: 'platform', label: 'SCOPENET platform', items: shared }];
  return [
    { id: 'control', label: 'Instance control center', items: [
      { id: 'control', label: 'Overview', icon: LayoutDashboard, hint: 'This experience and its connected systems' },
      { id: 'dashboard', label: 'Analytics', icon: Activity, hint: 'Player activity and health for this instance' },
      { id: 'experience', label: 'Experience design', icon: Palette, hint: 'Identity, modules, navigation and widgets' },
      { id: 'installation', label: 'Version & modpack', icon: Boxes, hint: 'Installation files and access' },
    ] },
    ...ADMIN_GROUPS.map(g => ({ ...g, items: g.items.filter(i => !PLATFORM_PAGES.has(i.id) && i.id !== 'dashboard' && pageEnabled(experience, i.id)) })).filter(g => g.items.length),
    { id: 'platform', label: 'SCOPENET platform', items: shared },
  ];
}
