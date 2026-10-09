// In-browser stand-in for the Rust backend, used by `npm run dev` outside
// Tauri and for UI screenshots. Scenario via ?mock=setup|login|main|update.
import { preset } from '@velora/experience';
import launcherPackage from '../../package.json';
import { mockEmit } from './tauri';
import type { Account, Bootstrap, Manifest, PlayerProfile, Settings } from './types';

const mockFlags: Record<string, boolean> = {};
const scenario = new URLSearchParams(location.search).get('mock') ?? 'main';

const settings: Settings = {
  companion: { enabled: true, notifications: true, claimBorders: false, scale: 1, opacity: .88, widgets: [
    {id:'level', enabled:true, x:.02,y:.04}, {id:'balance',enabled:true,x:.76,y:.04},
    {id:'guild',enabled:true,x:.02,y:.8}, {id:'claim',enabled:true,x:.4,y:.04},
    {id:'quests',enabled:true,x:.76,y:.3}, {id:'clock',enabled:false,x:.76,y:.8},
  ] },
  panel_url: scenario === 'setup' ? null : 'https://panel.velora.example',
  selected_instance: null,
  memory_max_mb: 0,
  memory_min_mb: 0,
  custom_resolution: false,
  width: 1280,
  height: 720,
  fullscreen: false,
  after_launch: 'minimize',
  show_console: false,
  java_path: null,
  gc: 'g1',
  jvm_args: '',
  theme_mode: 'server',
  accent: null,
  background_video: true,
  reduce_motion: false,
  glass: null,
  ui_scale: 100,
  concurrent_downloads: 12,
  check_updates: true,
  send_stats: true,
  keybinds_enabled: false,
  keybinds: {},
  vanilla_controls_enabled: false,
  auto_jump: true,
  sensitivity: 0.5,
  show_news: true,
    show_social_sidebar: false,
  instance_game_options: {},
  instance_overrides: {},
};

const manifest: Manifest = {
  api_version: 1,
  panel_version: '0.4.0',
  user: null,
  auth: { panel_accounts: true, registration: 'approval', offline_local: true, yggdrasil_url: 'https://panel.velora.example/api/yggdrasil' },
  branding: {
    name: 'Velora',
    tagline: 'Your worlds, one click away.',
    logo_url: null,
    icon_url: null,
    background: { kind: 'gradient', url: null, blur: 0, dim: 55 },
    colors: { accent: '#6d6af5', accent_2: '#8b88f8', background: '#0d0e12', surface: '#16181e', text: '#ecedf1', muted: '#8b8f9a', success: '#3fb97c', danger: '#e5484d' },
    font: 'Inter',
    radius: 10,
    glass: false,
    version_label: 'Velora',
    news: [
      { id: '1', title: 'Season 4 is live!', body: 'A brand new world with custom terrain, player shops and weekly events. Join now and claim your spot before the land rush.', image_url: null, link: 'https://example.com', date: '2026-09-26', pinned: true, tag: 'Update' },
      { id: '2', title: 'Build contest: Autumn Castles', body: 'Submit your best castle build on the Creative server by October 10th. Top three win VIP for a month.', image_url: null, link: null, date: '2026-09-21', pinned: false, tag: 'Event' },
      { id: '3', title: 'Performance boost', body: 'We switched the modpack to Sodium + Lithium. Expect way higher FPS on older machines.', image_url: null, link: null, date: '2026-09-14', pinned: false, tag: 'Changelog' },
    ],
    links: [
      { label: 'Discord', url: 'https://discord.gg/example', icon: 'discord' },
      { label: 'Website', url: 'https://example.com', icon: 'website' },
      { label: 'Store', url: 'https://example.com/store', icon: 'store' },
    ],
    about: { title: 'Velora Launcher', icon_url: null, body: 'Your worlds, one click away.', links: [] },
    features: { news: true, server_status: true, allow_user_theme: true, allow_advanced_java: true },
    custom_css: '',
  },
  instances: [
    { id: 'survival-smp', name: 'Survival SMP', description: 'Our main survival world — season 4. Claims, shops & weekend events.', icon_url: null, banner_url: null, logo_url: null, mc_version: '1.21.1', loader: 'fabric', loader_version: '0.16.9', revision: 3, server: { name: 'Velora SMP', address: 'play.velora.example', port: 25565, auto_join: true, inject: true }, memory: { min_mb: 2048, max_mb: 6144 }, jvm_args: '', featured: true, source_label: 'Modrinth · Fabulously Optimized 6.2', file_count: 48, total_size: 184_000_000 },
    { id: 'creative', name: 'Creative Build', description: 'Plots, WorldEdit and weekly build contests.', icon_url: null, banner_url: null, logo_url: null, mc_version: '1.20.1', loader: 'vanilla', loader_version: null, revision: 1, server: { name: 'Creative', address: 'creative.velora.example', port: 25565, auto_join: false, inject: true }, memory: { min_mb: 1024, max_mb: 4096 }, jvm_args: '', featured: false, source_label: 'Minecraft 1.20.1', file_count: 0, total_size: 0 },
    { id: 'modded', name: 'Create: Above & Beyond', description: 'Factory building with Create and friends.', icon_url: null, banner_url: null, logo_url: null, mc_version: '1.20.1', loader: 'forge', loader_version: '1.20.1-47.3.0', revision: 2, server: null, memory: { min_mb: 4096, max_mb: 8192 }, jvm_args: '', featured: false, source_label: 'CurseForge · Create: Above & Beyond 1.4', file_count: 212, total_size: 612_000_000 },
  ],
};

const frontierBrand = structuredClone(manifest.branding);
frontierBrand.name = 'Velora Frontiers';
frontierBrand.tagline = 'Build settlements. Shape a shared world.';
frontierBrand.colors.accent = '#d6a85f';
frontierBrand.colors.accent_2 = '#91b8a2';
frontierBrand.colors.background = '#101b19';
frontierBrand.news = [];
frontierBrand.links = [];
manifest.instances.push({ ...manifest.instances[2], id: 'frontiers', name: 'Velora Frontiers', description: 'Settlements, citizens, resources and cooperation.', experience: { ...preset('frontiers'), branding: frontierBrand } });

const accounts: Account[] = scenario === 'main' || scenario === 'progress' || scenario === 'update' ? [
  { id: 'a1', kind: 'panel', username: 'Alex_Miner', uuid: '00000000-0000-4000-8000-000000000001', panel_url: 'https://panel.velora.example', role: 'admin' },
  { id: 'a2', kind: 'offline', username: 'Steve', uuid: '5627dd98-e6be-3c21-b8a8-e92344183641', panel_url: null, role: null },
] : [];
let active: string | null = accounts[0]?.id ?? null;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const profile: PlayerProfile = {
  uuid: '00000000-0000-4000-8000-000000000001',
  name: 'Alex_Miner',
  skin_url: null,
  skin_model: 'classic',
  cape: null,
  available_capes: [
    { id: 1, name: 'Founder', url: '' },
    { id: 2, name: 'Season 4', url: '' },
  ],
};
let running = false;

async function fakeLaunch(id: string) {
  const stages: [string, string, number, number][] = [
    ['java', 'Installing Java 21', 48_000_000, 212],
    ['game', 'Downloading Minecraft 1.21.1', 420_000_000, 3412],
    ['loader', 'Installing Fabric 0.16.9', 6_000_000, 14],
    ['files', 'Syncing instance files', 184_000_000, 48],
  ];
  for (const [stage, label, total, files] of stages) {
    mockEmit('launch://progress', { type: 'stage', stage, label });
    for (let i = 1; i <= 20; i++) {
      await sleep(scenario === 'progress' ? 400 : 60);
      mockEmit('launch://progress', { type: 'progress', stage, done: (total * i) / 20, total, files_done: Math.round((files * i) / 20), files_total: files });
    }
  }
  mockEmit('launch://progress', { type: 'stage', stage: 'launching', label: 'Starting Minecraft' });
  await sleep(500);
  running = true;
  mockEmit('launch://state', { instance_id: id, run_id: 'mock-run', state: 'running', message: null });
  const lines = ['[main/INFO]: Loading Minecraft 1.21.1 with Fabric Loader 0.16.9', '[Render thread/INFO]: Setting user: Alex_Miner', '[Render thread/INFO]: Backend library: LWJGL version 3.3.3', '[Render thread/INFO]: Reloading ResourceManager: vanilla, fabric', '[Render thread/INFO]: Sound engine started', '[Render thread/INFO]: Connecting to play.velora.example, 25565'];
  for (const l of lines) {
    await sleep(300);
    mockEmit('game://log', [`[12:00:0${lines.indexOf(l)}] ${l}`]);
  }
}

const wallet = {
  balance: 12840.5, my_balance: 2450.75, role: 'leader', currency_symbol: '$',
  transactions: [
    { id: 5, actor_uuid: 'mock-user-1', kind: 'deposit', amount: 500, created_at: new Date(Date.now() - 12 * 60_000).toISOString() },
    { id: 4, actor_uuid: 'friend-1', kind: 'deposit', amount: 1250.25, created_at: new Date(Date.now() - 3 * 3600_000).toISOString() },
    { id: 3, actor_uuid: 'mock-user-1', kind: 'withdraw', amount: 300, created_at: new Date(Date.now() - 26 * 3600_000).toISOString() },
    { id: 2, actor_uuid: 'friend-2', kind: 'deposit', amount: 75, created_at: new Date(Date.now() - 3 * 86400_000).toISOString() },
    { id: 1, actor_uuid: 'gone-player', kind: 'deposit', amount: 10000, created_at: new Date(Date.now() - 9 * 86400_000).toISOString() }
  ] as Array<{ id: number; actor_uuid: string; kind: 'deposit' | 'withdraw'; amount: number; created_at: string }>
};
const mockWallet = () => JSON.parse(JSON.stringify(wallet));

// `?stress` in the dev URL makes every name, title and description much longer, to check that no screen lets text spill out of its box.
const STRESS = typeof location !== 'undefined' && new URLSearchParams(location.search).has('stress');
const TEXT_KEYS = /(^|_)(name|title|username|description|motd|message|desc|label|bio|tagline|text|rank|guild)(_|$)/i;
function stress(value: unknown, key = ''): unknown {
  if (Array.isArray(value)) return value.map((v) => stress(v, key));
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, stress(v, k)]));
  if (typeof value === 'string' && TEXT_KEYS.test(key) && /^[A-Za-z][A-Za-z '&:,.\-]{2,60}$/.test(value)) return `${value} With An Extraordinarily Long Unbroken Continuation`;
  return value;
}

export async function mockInvoke(cmd: string, args: Record<string, any>): Promise<unknown> {
  const result = await mockInvokeInner(cmd, args);
  return STRESS ? stress(result) : result;
}

async function mockInvokeInner(cmd: string, args: Record<string, any>): Promise<unknown> {
  await sleep(40);
  switch (cmd) {
    case 'textures_status': {
      const ready = new URLSearchParams(location.search).get('tex') === '1';
      return { ready, count: ready ? 1200 : 0, version: ready ? '1.21.1' : null };
    }
    case 'item_textures': {
      // A little pixel-art gem per name, standing in for the real PNGs.
      const out: Record<string, string> = {};
      for (const n of args.names as string[]) {
        const c = document.createElement('canvas');
        c.width = c.height = 16;
        const g = c.getContext('2d')!;
        const h = [...n].reduce((a, ch) => (a * 31 + ch.charCodeAt(0)) % 360, 5);
        for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) {
          const d = Math.abs(x - 7.5) + Math.abs(y - 7.5);
          if (d < 7) { g.fillStyle = `hsl(${h} 70% ${35 + (d < 3 ? 25 : (x + y) % 3 * 6)}%)`; g.fillRect(x, y, 1, 1); }
        }
        out[n] = c.toDataURL();
      }
      return out;
    }
    case 'mc_sprites': {
      const out: Record<string, string> = {};
      for (const n of args.names as string[]) {
        const c = document.createElement('canvas');
        c.width = c.height = 9;
        const g = c.getContext('2d')!;
        g.fillStyle = n.includes('container') ? '#2a0f12' : n.includes('food') ? '#c8793a' : '#d4262c';
        g.fillRect(1, 1, 7, 6); g.fillRect(2, 7, 5, 1);
        out[n] = c.toDataURL();
      }
      return out;
    }
    case 'get_public_servers':
      return { servers: [{ id: 1, name: 'Survival SMP', instance_id: 'survival-smp', map: true, online: true, players_online: 42, players_max: 100 }] };
    case 'get_map_info':
      // Lets a browser test point the mock at a real panel's map: set window.__mockMapInfo before the app loads.
      if ((window as unknown as { __mockMapInfo?: unknown }).__mockMapInfo) return (window as unknown as { __mockMapInfo: unknown }).__mockMapInfo;
      return { enabled: true, ready: false, message: 'Waiting for the game server to draw its world.', tile_size: 256, max_zoom: 6, tile_base: 'https://panel.example/api/map/1', token: null, token_ttl_secs: 3600, players: 0,
        dimensions: [] };
    case 'get_map_overlay':
      return { players: [], claims: [], pins: [], updated: null };
    case 'get_player_stats':
      return { username: 'Alex_Miner', uuid: '00000000-0000-4000-8000-000000000001', total: null, servers: [], events: [] };
    case 'get_leaderboard':
      return { leaderboard: [], totals: { players: 0, playtime_secs: 0 }, sort: args?.sort ?? 'playtime_secs' };
    case 'get_economy_balances':
      return [{ server_id: 1, server_name: 'Survival SMP', balance: 2450.75, currency_symbol: '$' }];
    case 'get_server_baltop':
      return [{ rank: 1, uuid: '00000000-0000-4000-8000-000000000001', username: 'Alex_Miner', balance: 2450.75 }];
    case 'get_transactions':
      return [{ id: 1, server_id: 1, server_name: 'Survival SMP', from_uuid: '00000000-0000-4000-8000-000000000001', from_name: 'Alex_Miner', to_uuid: 'friend-1', to_name: 'Alex', amount: 150, description: 'Payment to Alex', created_at: '2026-09-29T15:30:00Z' }];
    case 'bootstrap': {
      const b: Bootstrap = {
        app: { version: launcherPackage.version, default_panel_url: null, panel_locked: false, repo: 'https://github.com/scopeddlol/Velora-MC', os: 'windows', arch: 'x86_64', total_ram_mb: 16384, data_dir: 'C:\\Users\\you\\AppData\\Roaming\\net.scopenet.launcher' },
        panel_url: settings.panel_url,
        settings,
        accounts,
        active_account: active,
        manifest: scenario === 'setup' ? null : manifest,
        game_running: running ? [{ run_id: 'mock-run', instance_id: 'survival-smp' }] : [],
      };
      return structuredClone(b);
    }
    case 'refresh_manifest':
    case 'set_panel_url':
      if (cmd === 'set_panel_url') settings.panel_url = args.url;
      return structuredClone(manifest);
    case 'save_settings':
      Object.assign(settings, args.settings);
      return null;
    case 'login_panel':
    case 'add_offline': {
      const a: Account = { id: crypto.randomUUID(), kind: cmd === 'login_panel' ? 'panel' : 'offline', username: args.username, uuid: crypto.randomUUID(), panel_url: null, role: 'player' };
      accounts.push(a);
      active = a.id;
      return a;
    }
    case 'register_panel':
      return { account: null, pending: true };
    case 'account_profile':
    case 'set_cape':
    case 'set_skin_model':
      if (cmd === 'set_cape') profile.cape = profile.available_capes.find((c) => c.id === args.capeId) ?? null;
      if (cmd === 'set_skin_model') profile.skin_model = args.model;
      return structuredClone(profile);
    case 'upload_skin':
      profile.skin_url = 'data:image/png;base64,' + args.data;
      profile.skin_model = args.model;
      return structuredClone(profile);
    case 'delete_skin':
      profile.skin_url = null;
      return structuredClone(profile);
    case 'select_account':
      active = args.id;
      return null;
    case 'launch':
      fakeLaunch(args.instanceId);
      return null;
    case 'kill_game':
      running = false;
      mockEmit('game://exit', { instance_id: 'survival-smp', run_id: args?.runId ?? 'mock-run', code: 0, crashed: false, tail: [], crash_report: null });
      mockEmit('launch://state', { instance_id: 'survival-smp', state: 'idle', message: null });
      return null;
    case 'save_instance_options':
      return { fov: '0.8', gamma: '1.0', graphicsMode: '1', autoJump: 'true' };
    case 'ping_server':
      await sleep(200);
      return { online: true, version: 'Paper 1.21.1', players_online: 42, players_max: 100, sample: ['Alex_Miner', 'jeb_', 'Dinnerbone'], motd: '§b§lVelora §r§7» §fSeason 4 is live!', favicon: null, latency_ms: 23 };
    case 'instance_local':
      return { installed: args.instanceId !== 'modded', revision: 3, size: 1_240_000_000 };
    case 'storage_info':
      return { shared: 2_400_000_000, runtimes: 410_000_000, instances: [['survival-smp', 1_240_000_000], ['creative', 320_000_000]] };
    case 'check_update':
      return scenario === 'update' ? { version: '0.10.0', notes: 'A native Fabric companion, launcher-managed HUD, and panel-hosted launcher updates.', url: 'https://panel.velora.example/api/v1/launcher/updates/mock/setup.exe', size: 24_000_000 } : null;
    case 'detect_java':
      return 21;
    case 'get_my_level':
      return { uuid: 'mock-user-1', level: 14, current_xp: 3450, next_level_xp: 5000, progress_pct: 69, title: 'Veteran Adventurer', badges: ['Beta Pioneer', 'Dragon Slayer'], rewards: [] };
    case 'get_reward_summaries':
      return { 'quest:q1': ['$250.00', 'Diamond x3'], 'quest:q2': ['+4 claim chunks'] };
    case 'get_my_quests':
      return [
        { quest: { id: 'q1', title: 'Deepslate Quarry', description: 'Mine 50 deepslate blocks in the caverns.', quest_type: 'daily', category: 'mining', xp_reward: 120, stat_type: 'mine_deepslate', target_count: 50, icon: 'pickaxe', active: true }, current_count: 35, completed: false, claimed: false, expires_at: '2026-10-01T00:00:00Z' },
        { quest: { id: 'q2', title: 'Night Patrol', description: 'Defeat 15 hostile monsters.', quest_type: 'daily', category: 'combat', xp_reward: 150, stat_type: 'kill_monsters', target_count: 15, icon: 'sword', active: true }, current_count: 15, completed: true, claimed: false, expires_at: '2026-10-01T00:00:00Z' },
        { quest: { id: 'q3', title: 'Territorial Fortress', description: 'Place 250 building blocks within claimed territory.', quest_type: 'weekly', category: 'building', xp_reward: 500, stat_type: 'place_blocks', target_count: 250, icon: 'hammer', active: true }, current_count: 180, completed: false, claimed: false, expires_at: '2026-10-05T00:00:00Z' },
      ];
    case 'claim_quest':
      return { uuid: 'mock-user-1', level: 14, current_xp: 3600, next_level_xp: 5000, progress_pct: 72, title: 'Veteran Adventurer', badges: ['Beta Pioneer'], rewards: [] };
    case 'get_my_achievements':
      return [
        { id: 'ach1', title: 'Stone Age', description: 'Mine your first piece of cobblestone.', category: 'mining', xp_reward: 50, frame_type: 'task', icon_item: '🪨', icon_bg: '#334155', icon_border: '#64748b', unlocked: true, unlocked_at: '2026-09-20T12:00:00Z' },
        { id: 'ach2', title: 'Acquire Hardware', description: 'Smelt an iron ingot in a furnace.', category: 'mining', xp_reward: 100, frame_type: 'task', icon_item: '⛓️', icon_bg: '#475569', icon_border: '#94a3b8', unlocked: true, unlocked_at: '2026-09-21T14:30:00Z' },
        { id: 'ach3', title: 'Diamonds!', description: 'Acquire diamonds from deep underground.', category: 'mining', xp_reward: 350, frame_type: 'goal', icon_item: '💎', icon_bg: '#0284c7', icon_border: '#38bdf8', unlocked: true, unlocked_at: '2026-09-24T18:15:00Z' },
        { id: 'ach4', title: 'Free the End', description: 'Defeat the Ender Dragon in the End dimension.', category: 'combat', xp_reward: 1000, frame_type: 'challenge', icon_item: 'dragon_head', icon_bg: 'end_stone', icon_border: 'purple', unlocked: false, unlocked_at: null },
      ];
    case 'get_guilds':
      return [
        { id: 'g1', instance_id: args?.instanceId ?? 'survival-smp', name: 'Iron Fortress', tag: 'IRON', description: 'Defenders of the realm and builders of grand monuments.', icon_url: null, banner_url: "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='900' height='200'%3E%3Cdefs%3E%3ClinearGradient id='g' x1='0' x2='1'%3E%3Cstop offset='0' stop-color='%23214a7a'/%3E%3Cstop offset='1' stop-color='%236d3a8a'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='900' height='200' fill='url(%23g)'/%3E%3Cpath d='M0 150 L120 90 L220 140 L360 60 L520 150 L700 80 L900 140 V200 H0Z' fill='%23ffffff22'/%3E%3C/svg%3E", leader_uuid: 'mock-user-1', member_count: 5, max_members: 20, claims_count: 8, created_at: '2026-09-01T00:00:00Z' },
        { id: 'g2', instance_id: args?.instanceId ?? 'survival-smp', name: 'Void Walkers', tag: 'VOID', description: 'Exploring the farthest reaches of every dimension.', icon_url: null, banner_url: null, leader_uuid: 'other-user', member_count: 8, max_members: 20, claims_count: 12, created_at: '2026-09-05T00:00:00Z' }
      ];
    case 'kick_guild_member':
    case 'transfer_guild_leader':
      return null;
    case 'casino_get': { const { casinoGet } = await import('./mockCasino'); try { return casinoGet(args?.path as string); } catch (e) { throw e; } }
    case 'board_get': { const { boardGet } = await import('./mockBoard'); return boardGet(args?.path as string); }
    case 'board_post': { const { boardPost } = await import('./mockBoard'); return boardPost(args?.path as string, args?.body as any); }
    case 'casino_post': { const { casinoPost } = await import('./mockCasino'); return casinoPost(args?.path as string, args?.body); }
    case 'market_listings': {
      const now = Date.now();
      return { server: { id: 1, name: 'Survival SMP' }, balance: 1840.5, listings: [
        { id: 12, seller_name: 'Alex', seller_guild: 'IRON', item_id: 'NETHERITE_SWORD', item_name: 'Stormbreaker', amount: 1, price: 2500, created_at: new Date(now - 3600e3).toISOString(), kind: 'auction', ends_at: new Date(now + 5400e3).toISOString(), current_bid: 2750, bidder_name: 'Steve', bid_count: 4, min_next_bid: 2887.5, mine: false, leading: false },
        { id: 11, seller_name: 'Alex_Miner', seller_guild: null, item_id: 'DIAMOND', item_name: 'Diamond', amount: 32, price: 900, created_at: new Date(now - 7200e3).toISOString(), kind: 'buy_now', ends_at: null, current_bid: null, bidder_name: null, bid_count: 0, min_next_bid: 900, mine: true, leading: false },
        { id: 10, seller_name: 'Mia', seller_guild: null, item_id: 'GOLDEN_APPLE', item_name: 'Golden Apple', amount: 8, price: 160, created_at: new Date(now - 9000e3).toISOString(), kind: 'buy_now', ends_at: null, current_bid: null, bidder_name: null, bid_count: 0, min_next_bid: 160, mine: false, leading: false },
        { id: 9, seller_name: 'Steve', seller_guild: null, item_id: 'ELYTRA', item_name: 'Elytra', amount: 1, price: 400, created_at: new Date(now - 20000e3).toISOString(), kind: 'auction', ends_at: new Date(now + 600e3).toISOString(), current_bid: 520, bidder_name: 'Alex_Miner', bid_count: 2, min_next_bid: 546, mine: false, leading: true },
        { id: 8, seller_name: 'Alex', seller_guild: 'IRON', item_id: 'OAK_LOG', item_name: 'Oak Log', amount: 64, price: 45, created_at: new Date(now - 40000e3).toISOString(), kind: 'buy_now', ends_at: null, current_bid: null, bidder_name: null, bid_count: 0, min_next_bid: 45, mine: false, leading: false },
      ] };
    }
    case 'market_mine':
      return { waiting: 1, waiting_items: [{ item_name: 'Elytra', amount: 1, note: 'won' }], history: [
        { kind: 'sold', amount: 640, description: 'Market purchase: 16x Iron Ingot from Alex_Miner', at: new Date(Date.now() - 1800e3).toISOString(), other: 'u1' },
        { kind: 'bought', amount: 160, description: 'Market purchase: 8x Golden Apple from Mia', at: new Date(Date.now() - 86400e3).toISOString(), other: 'u2' },
        { kind: 'bid', amount: 520, description: 'Auction bid: 1x Elytra', at: new Date(Date.now() - 7200e3).toISOString(), other: 'auction' },
      ] };
    case 'market_act':
      return { ok: true, message: args?.action === 'buy' ? 'Bought! It is waiting in your vault.' : args?.action === 'bid' ? 'You lead the auction.' : 'Listing cancelled; it is waiting in your vault.' };
    case 'notifications_list':
      return { unread: mockNotes.filter((n) => !n.read).length, items: mockNotes };
    case 'notifications_mark':
      if (args?.clear) mockNotes.length = 0;
      else mockNotes.forEach((n) => { if (!args?.ids || (args.ids as number[]).includes(n.id)) n.read = true; });
      return null;
    case 'get_my_guild':
      return { id: 'g1', instance_id: args?.instanceId ?? 'survival-smp', name: 'Iron Fortress', tag: 'IRON', motd: 'Raid night is Saturday 8pm — bring potions!', description: 'Defenders of the realm and builders of grand monuments.', icon_url: null, banner_url: "data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='900' height='200'%3E%3Cdefs%3E%3ClinearGradient id='g' x1='0' x2='1'%3E%3Cstop offset='0' stop-color='%23214a7a'/%3E%3Cstop offset='1' stop-color='%236d3a8a'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='900' height='200' fill='url(%23g)'/%3E%3Cpath d='M0 150 L120 90 L220 140 L360 60 L520 150 L700 80 L900 140 V200 H0Z' fill='%23ffffff22'/%3E%3C/svg%3E", leader_uuid: 'mock-user-1', member_count: 5, max_members: 20, claims_count: 8, created_at: '2026-09-01T00:00:00Z' };
    case 'get_my_collections':
      return [
        { key: 'title.veteran', type: 'title', source_type: 'level', source_id: '10', equipped: true, metadata: { label: 'Veteran' } },
        { key: 'badge.pioneer', type: 'badge', source_type: 'achievement', source_id: 'ach1', equipped: false, metadata: { label: 'Beta Pioneer' } },
        { key: 'pet.wolf', type: 'pet', source_type: 'quest', source_id: 'q2', equipped: false, metadata: { label: 'Loyal Wolf' } },
      ];
    case 'equip_collection_item':
      return null;
    case 'get_guild_roles':
    case 'get_guild_join_requests':
    case 'my_guild_invites':
      return [];
    case 'get_guild_claim_flags':
    case 'set_guild_claim_flags': {
      const catalog = [
        ['build', 'Visitors can build', 'Players outside your guild can place and break blocks and use buckets on your land.', 'Visitors', false],
        ['interact', 'Visitors can use doors & buttons', 'Doors, trapdoors, buttons, levers, pressure plates and beds.', 'Visitors', false],
        ['containers', 'Visitors can open chests', 'Chests, barrels, furnaces, hoppers, shulker boxes and similar.', 'Visitors', false],
        ['entry', 'Visitors can walk in', 'Turn this off to keep everyone outside your guild off your land.', 'Visitors', true],
        ['pvp', 'Player vs player', 'Players can hurt each other on your land.', 'Combat', true],
        ['mob_spawning', 'Mobs spawn', 'Mobs spawn naturally, from spawners and from eggs.', 'Mobs', true],
        ['explosions', 'Explosions break blocks', 'TNT, creepers and beds can destroy blocks.', 'World', false],
      ] as const;
      if (cmd === 'set_guild_claim_flags') Object.assign(mockFlags, args?.flags as Record<string, boolean>);
      return { flags: Object.fromEntries(catalog.map(([id, , , , d]) => [id, mockFlags[id] ?? d])), can_edit: true, catalog: catalog.map(([id, label, help, group, d]) => ({ id, label, help, group, default: d, editable: true })) };
    }
    case 'get_my_guild_memberships':
      return [{ id: 'g1', instance_id: args?.instanceId ?? 'survival-smp', name: 'Iron Fortress', tag: 'IRON', role: 'leader', server_count: 2, primary_server_count: 1 }];
    case 'set_primary_guild':
    case 'create_guild_relation':
    case 'respond_guild_relation':
      return null;
    case 'get_guild_relations':
      return [{ id: 1, guild_id: 'g1', other_guild_id: 'g2', relation: 'alliance', status: 'accepted', name: 'Void Walkers', tag: 'VOID' }];
    case 'get_guild_wallet':
      return mockWallet();
    case 'transfer_guild_wallet': {
      const amt = Number(args?.amount);
      if (args?.withdraw) { if (amt > wallet.balance) throw 'Insufficient funds'; wallet.balance -= amt; wallet.my_balance += amt; }
      else { if (amt > wallet.my_balance) throw 'Insufficient funds'; wallet.balance += amt; wallet.my_balance -= amt; }
      wallet.transactions.unshift({ id: Date.now(), actor_uuid: 'mock-user-1', kind: args?.withdraw ? 'withdraw' : 'deposit', amount: amt, created_at: new Date().toISOString() });
      return { balance: wallet.balance };
    }
    case 'get_guild_claims':
      return args?.guildId ? [
        { id: 101, guild_id: 'g1', instance_id: args?.instanceId ?? 'survival-smp', dimension: args?.dimension ?? 'minecraft:overworld', chunk_x: 0, chunk_z: 0, claimed_by_uuid: 'mock-user-1', claimed_at: '2026-09-10T00:00:00Z', guild_name: 'Iron Fortress', guild_tag: 'IRON' },
        { id: 104, guild_id: 'g1', instance_id: args?.instanceId ?? 'survival-smp', dimension: args?.dimension ?? 'minecraft:overworld', chunk_x: 40, chunk_z: -25, claimed_by_uuid: 'mock-user-1', claimed_at: '2026-09-10T00:00:00Z', guild_name: 'Iron Fortress', guild_tag: 'IRON' }
      ] : [
        { id: 101, guild_id: 'g1', instance_id: args?.instanceId ?? 'survival-smp', dimension: args?.dimension ?? 'minecraft:overworld', chunk_x: 0, chunk_z: 0, claimed_by_uuid: 'mock-user-1', claimed_at: '2026-09-10T00:00:00Z', guild_name: 'Iron Fortress', guild_tag: 'IRON' },
        { id: 102, guild_id: 'g1', instance_id: args?.instanceId ?? 'survival-smp', dimension: args?.dimension ?? 'minecraft:overworld', chunk_x: 1, chunk_z: 0, claimed_by_uuid: 'mock-user-1', claimed_at: '2026-09-10T00:00:00Z', guild_name: 'Iron Fortress', guild_tag: 'IRON' },
        { id: 103, guild_id: 'g2', instance_id: args?.instanceId ?? 'survival-smp', dimension: args?.dimension ?? 'minecraft:overworld', chunk_x: -2, chunk_z: 1, claimed_by_uuid: 'other-user', claimed_at: '2026-09-11T00:00:00Z', guild_name: 'Void Walkers', guild_tag: 'VOID' }
      ];
    case 'rename_guild':
      return { ok: true, name: String(args?.name ?? '').trim(), tag: String(args?.tag || 'IRON').toUpperCase() };
    case 'disband_guild':
      return { ok: true, refunded: 0 };
    case 'claim_guild_chunk':
      return { id: Math.floor(Math.random() * 10000), guild_id: 'g1', instance_id: args?.instanceId, dimension: args?.dimension, chunk_x: args?.chunkX, chunk_z: args?.chunkZ, claimed_by_uuid: 'mock-user-1', claimed_at: new Date().toISOString(), guild_name: 'Iron Fortress', guild_tag: 'IRON' };
    case 'unclaim_guild_chunk':
      return null;
    case 'get_guild_members':
      return [
        { guild_id: 'g1', uuid: '00000000-0000-4000-8000-000000000001', name: 'Alex_Miner', role: 'leader', joined_at: '2026-09-01T00:00:00Z', online: true },
        { guild_id: 'g1', uuid: 'friend-1', name: 'Alex', role: 'officer', joined_at: '2026-09-02T00:00:00Z', online: true },
        { guild_id: 'g1', uuid: 'friend-2', name: 'Steve', role: 'member', joined_at: '2026-09-05T00:00:00Z', online: false }
      ];
    case 'get_guild_posts':
      return [
        { id: 'gp1', guild_id: 'g1', author_uuid: 'mock-user-1', author_name: 'Alyssa', title: 'Community Farm Completed!', content: 'The automatic wheat and pumpkin farm at chunk [2, 1] is open to all guild members. Please replenish seeds after harvesting.', pinned: true, created_at: '2026-09-25T16:00:00Z' }
      ];
    case 'create_guild_post':
      return { id: 'gp' + Date.now(), guild_id: args?.guildId, author_uuid: 'mock-user-1', author_name: 'Alyssa', title: args?.title, content: args?.content, pinned: args?.pinned ?? false, created_at: new Date().toISOString() };
    case 'get_friends':
      return [
        { uuid: 'friend-1', username: 'Alex', status: 'accepted', online: true, playing_on: 'Survival SMP', avatar_url: null, unread: 2 },
        { uuid: 'friend-2', username: 'Steve', status: 'accepted', online: false, playing_on: null, avatar_url: null },
        { uuid: 'friend-3', username: 'Alex_Miner', status: 'pending_incoming', online: true, playing_on: null, avatar_url: null }
      ];
    case 'send_friend_request':
    case 'respond_friend_request':
    case 'remove_friend':
      return null;
    case 'get_direct_messages':
      return [
        { id: 1, sender_uuid: args?.friendUuid, recipient_uuid: 'mock-user-1', content: 'Hey! Want to team up and explore the trial chambers?', created_at: '2026-09-28T18:20:00Z', is_read: true },
        { id: 2, sender_uuid: 'mock-user-1', recipient_uuid: args?.friendUuid, content: 'Definitely! Let me grab some potions from base first.', created_at: '2026-09-28T18:22:00Z', is_read: true }
      ];
    case 'send_direct_message':
      return { id: Date.now(), sender_uuid: 'mock-user-1', recipient_uuid: args?.friendUuid, content: args?.content, created_at: new Date().toISOString(), is_read: true };
    case 'get_game_invites':
      return [
        { id: 'inv-1', sender_uuid: 'friend-1', sender_name: 'Alex', recipient_uuid: 'mock-user-1', instance_id: 'survival-smp', instance_name: 'smp.scopenet.gg', server_name: 'Survival SMP', status: 'pending', created_at: '2026-09-28T19:00:00Z' }
      ];
    case 'send_game_invite':
      return { id: 'inv-' + Date.now(), sender_uuid: 'mock-user-1', sender_name: 'Alyssa', recipient_uuid: args?.friendUuid, instance_id: args?.instanceId, instance_name: args?.serverAddress, server_name: args?.serverName, status: 'pending', created_at: new Date().toISOString() };
    case 'respond_game_invite':
      return null;
    case 'get_user_profile':
      return {
        uuid: args?.uuid,
        username: args?.uuid === 'mock-user-1' ? 'Alyssa' : (args?.uuid === 'friend-1' ? 'Alex' : 'Steve'),
        bio: 'Explorer, redstone engineer, and proud founder of the Iron Fortress guild!',
        banner_url: null,
        skin_url: null,
        skin_model: 'classic',
        cape_url: null,
        level_info: { uuid: args?.uuid, level: 14, current_xp: 3450, next_level_xp: 5000, progress_pct: 69, title: 'Veteran Adventurer', badges: ['Beta Pioneer', 'Dragon Slayer'], rewards: [] },
        server_levels: [
          { server_id: 1, server_name: 'Survival SMP', uuid: args?.uuid, level: 9, current_xp: 1200, next_level_xp: 2200, progress_pct: 54, rank_title: 'Knight' }
        ],
        badges: ['Beta Pioneer', 'Dragon Slayer'],
        achievements: [
          { id: 'ach1', title: 'Diamonds!', description: 'Acquire diamonds.', category: 'mining', xp_reward: 350, frame_type: 'goal', icon_item: '💎', icon_bg: '#0284c7', icon_border: '#38bdf8', unlocked: true, unlocked_at: '2026-09-24T18:15:00Z' }
        ],
        posts: [
          { id: 'post-1', user_uuid: args?.uuid, username: 'Alyssa', content: 'Just finished our mountain watchtower! What do you guys think?', image_url: null, likes_count: 7, liked_by_me: true, created_at: '2026-09-27T20:10:00Z' }
        ],
        friends_count: 12,
        stats: { playtime_secs: 74200, joins: 48, deaths: 12, player_kills: 19, mob_kills: 480, blocks_broken: 15400, blocks_placed: 9800, messages: 320, first_seen: '2026-08-01T00:00:00Z', last_seen: '2026-09-28T22:00:00Z' },
        created_at: '2026-08-01T00:00:00Z'
      };
    case 'update_my_profile':
      return {
        uuid: 'mock-user-1',
        username: 'Alyssa',
        bio: args?.bio ?? '',
        banner_url: args?.bannerUrl ?? null,
        skin_url: null,
        skin_model: 'classic',
        cape_url: null,
        level_info: { uuid: 'mock-user-1', level: 14, current_xp: 3450, next_level_xp: 5000, progress_pct: 69, title: 'Veteran Adventurer', badges: ['Beta Pioneer'], rewards: [] },
        server_levels: [],
        badges: ['Beta Pioneer'],
        achievements: [],
        posts: [],
        friends_count: 12,
        stats: null,
        created_at: '2026-08-01T00:00:00Z'
      };
    case 'create_user_post':
      return { id: 'post-' + Date.now(), user_uuid: 'mock-user-1', username: 'Alyssa', content: args?.content, image_url: args?.imageUrl ?? null, likes_count: 0, liked_by_me: false, created_at: new Date().toISOString() };
    case 'like_user_post':
      return true;
    default:
      return null;
  }
}

const mockNotes: { id: number; kind: string; title: string; body: string; link: string | null; created_at: string; read: boolean }[] = [
  { id: 3, kind: 'guild_renamed', title: 'Your guild was renamed', body: 'Iron Fort is now called Iron Fortress [IRON].', link: '/guild', created_at: new Date(Date.now() - 600e3).toISOString(), read: false },
  { id: 2, kind: 'auction_won', title: 'You won an auction', body: 'Diamond Pickaxe for $450.', link: null, created_at: new Date(Date.now() - 86400e3).toISOString(), read: false },
  { id: 1, kind: 'guild_disbanded', title: 'Your guild was disbanded', body: 'Old Guild was disbanded by its leader.', link: null, created_at: new Date(Date.now() - 4 * 86400e3).toISOString(), read: true },
];
