#!/usr/bin/env node
// Fills a fresh SCOPENET panel with believable demo data through its real HTTP APIs.
//
//   node scripts/demo/seed.mjs                # seed (run.sh does this)
//   node scripts/demo/seed.mjs --heartbeat    # keep the two demo game servers "online" (sync every 30 s)
//
// Plain Node 22 ESM, no dependencies. Required steps abort with a clear message; optional ones (skins, ...) warn and go on.
import { randomUUID } from 'node:crypto';
import { deflateSync } from 'node:zlib';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const BASE = (process.env.DEMO_BASE ?? 'http://127.0.0.1:18080').replace(/\/$/, '');
const DATA = process.env.DEMO_DATA ?? join(HERE, '.data');
const ADMIN = { username: 'admin', password: 'demo-admin-pass' };
const PASS = 'demo-pass-1234';
const MAIN = 'Alex_Miner';

// ---------------------------------------------------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------------------------------------------------

class ApiError extends Error {}

async function api(method, path, { token, body, form } = {}) {
  const headers = {};
  if (token) headers.authorization = `Bearer ${token}`;
  let payload;
  if (form) payload = form;
  else if (body !== undefined) {
    headers['content-type'] = 'application/json';
    payload = JSON.stringify(body);
  }
  let res;
  try {
    res = await fetch(BASE + path, { method, headers, body: payload });
  } catch (e) {
    throw new ApiError(`${method} ${path}: cannot reach the panel at ${BASE} (${e.cause?.code ?? e.message})`);
  }
  const text = await res.text();
  let data;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    data = text;
  }
  if (!res.ok) throw new ApiError(`${method} ${path} -> ${res.status} ${typeof data === 'string' ? data : JSON.stringify(data)}`);
  return data;
}

let warnings = 0;
const log = (m) => console.log(`  ${m}`);
const step = (m) => console.log(`\n== ${m}`);
function warn(m) {
  warnings++;
  console.warn(`  [warn] ${m}`);
}
/** Optional work: warn and carry on when it fails. */
async function optional(name, fn) {
  try {
    return await fn();
  } catch (e) {
    warn(`${name}: ${e.message}`);
    return undefined;
  }
}
/** Like `optional`, silent - for bulk loops where individual failures are expected (e.g. a bid that is too low). */
async function quiet(fn) {
  try {
    return await fn();
  } catch {
    return undefined;
  }
}

const rnd = (a, b) => a + Math.random() * (b - a);
const rint = (a, b) => Math.floor(rnd(a, b + 1));
const pick = (arr) => arr[Math.floor(Math.random() * arr.length)];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const round2 = (n) => Math.round(n * 100) / 100;

// ---------------------------------------------------------------------------------------------------------------------
// demo cast
// ---------------------------------------------------------------------------------------------------------------------

// Survival stats: [hours, joins, deaths, player kills, mob kills, blocks broken, blocks placed, chat messages]
// level = the global level we want the player to show; balance = target balance on Survival SMP.
const PLAYERS = [
  { n: MAIN, level: 17, bal: 12480.5, start: 18400, s1: [74, 96, 41, 9, 1860, 52300, 9800, 1240], s2: [11, 15, 6200, 1900, 120], skin: ['#c68642', '#3b82f6', '#4b5563', '#3b2a1a'], model: 'classic', groups: ['VIP'],
    bio: 'Digging since 2019. Diamonds are a state of mind. Co-founder of Iron Pickaxe Co. - always happy to trade ore for builds.', accent: '#2dd4a7', badge: 'Founder', feat: 'ach_miner_49er' },
  { n: 'Steve_Builds', level: 23, bal: 31250, s1: [118, 140, 22, 2, 640, 18200, 61400, 2210], s2: [54, 70, 88000, 21000, 900], skin: ['#e0ac69', '#f59e0b', '#374151', '#7c4a1d'], model: 'classic', groups: ['Builder', 'VIP'],
    bio: 'Medieval castles, cozy villages and way too many stairs. DM me for commissions.', accent: '#f59e0b', badge: 'Master Builder', feat: 'ach_master_builder' },
  { n: 'CreeperSlayer', level: 25, bal: 18900, s1: [133, 160, 88, 64, 5120, 21300, 7200, 1870], s2: [], skin: ['#8d5524', '#16a34a', '#1f2937', '#111827'], model: 'classic', groups: ['VIP'],
    bio: 'Ender Dragon: defeated. Wither: defeated. My own creeper farm: still exploding.', accent: '#22c55e', badge: 'Dragon Slayer', feat: 'ach_monster_hunter' },
  { n: 'LunaMC', level: 20, bal: 9640.25, s1: [96, 120, 30, 5, 1900, 16000, 22800, 3300], s2: [22, 31, 30000, 7000, 800], skin: ['#f1c27d', '#a855f7', '#1e1b4b', '#6b21a8'], model: 'slim', groups: ['Staff'],
    bio: 'Community helper and professional farm optimiser. Ask me anything - or come to the Sunday market.', accent: '#a855f7', badge: 'Helper', feat: 'ach_chatty' },
  { n: 'Redstone_Rae', level: 15, bal: 7120, s1: [61, 77, 25, 1, 700, 14100, 28400, 640], s2: [33, 41, 41000, 9000, 300], skin: ['#ffdbac', '#dc2626', '#292524', '#b91c1c'], model: 'slim', groups: ['Builder'],
    bio: 'Redstone engineer. If it can be automated, it will be. Sorters, farms, and one slightly illegal clock.', accent: '#ef4444', badge: null, feat: 'ach_architect' },
  { n: 'DiamondDave', level: 13, bal: 4380.75, s1: [48, 63, 36, 3, 1500, 39800, 4200, 410], s2: [], skin: ['#c68642', '#06b6d4', '#0e7490', '#1c1917'], model: 'classic', groups: [],
    bio: 'Strip-mining at Y -59 since breakfast.', accent: '#06b6d4', badge: null, feat: 'ach_miner_49er' },
  { n: 'NetherNinja', level: 28, bal: 48210, s1: [142, 171, 97, 41, 6600, 28000, 8800, 1450], s2: [], skin: ['#8d5524', '#7f1d1d', '#0f0f0f', '#0a0a0a'], model: 'classic', groups: ['VIP'],
    bio: 'Fortress runs, bastion raids, zero regrets. Top of the leaderboard and not planning to leave.', accent: '#ef4444', badge: 'Nether Lord', feat: 'ach_nether_walker' },
  { n: 'PixelPaige', level: 11, bal: 3290, s1: [39, 52, 9, 0, 210, 6200, 33100, 980], s2: [71, 90, 120000, 34000, 1500], skin: ['#f1c27d', '#ec4899', '#312e81', '#fbbf24'], model: 'slim', groups: ['Builder'],
    bio: 'Pixel art, mega builds, and the Creative server regular. Come see the mural!', accent: '#ec4899', badge: null, feat: 'ach_architect' },
  { n: 'TNT_Tommy', level: 9, bal: 1860, s1: [31, 49, 66, 12, 1100, 14800, 2900, 560], s2: [6, 9, 3100, 800, 40], skin: ['#e0ac69', '#b91c1c', '#44403c', '#78350f'], model: 'classic', groups: [],
    bio: 'It was supposed to be a small explosion.', accent: '#f97316', badge: null, feat: 'ach_first_blood' },
  { n: 'EnderEmma', level: 12, bal: 5675.5, s1: [52, 70, 28, 7, 2300, 9100, 5400, 1190], s2: [14, 20, 9000, 2000, 210], skin: ['#ffdbac', '#7c3aed', '#18181b', '#4c1d95'], model: 'slim', groups: [],
    bio: 'Shulkers are friends. Endermen are... complicated.', accent: '#8b5cf6', badge: null, feat: 'ach_first_blood' },
  { n: 'FarmerFinn', level: 8, bal: 2140, s1: [27, 38, 8, 0, 450, 17600, 11200, 330], s2: [], skin: ['#e0ac69', '#84cc16', '#854d0e', '#a16207'], model: 'classic', groups: [],
    bio: 'Wheat, carrots, potatoes. Sells in bulk at fair prices.', accent: '#84cc16', badge: null, feat: 'ach_miner_49er' },
  { n: 'ObsidianOwen', level: 16, bal: 8890, s1: [69, 90, 52, 22, 2600, 23400, 6100, 520], s2: [], skin: ['#8d5524', '#4c1d95', '#1e1b4b', '#0c0a09'], model: 'classic', groups: [],
    bio: 'Portal builder. If you need a nether hub, ask. I have flint and steel.', accent: '#6366f1', badge: null, feat: 'ach_duelist' },
  { n: 'BlazeRunner', level: 14, bal: 6015.25, s1: [55, 71, 74, 28, 3000, 12600, 3300, 480], s2: [], skin: ['#c68642', '#f97316', '#451a03', '#facc15'], model: 'classic', groups: [],
    bio: 'Fast hands, faster elytra. Duels at spawn on weekends.', accent: '#f97316', badge: null, feat: 'ach_duelist' },
  { n: 'MossyMax', level: 6, bal: 975, s1: [17, 24, 14, 1, 260, 5200, 2100, 190], s2: [12, 16, 5400, 1200, 60], skin: ['#f1c27d', '#65a30d', '#3f6212', '#365314'], model: 'classic', groups: [],
    bio: 'New-ish. Learning redstone one disaster at a time.', accent: '#65a30d', badge: null, feat: 'ach_first_steps' },
  { n: 'Cobble_Cleo', level: 10, bal: 3760.5, s1: [44, 58, 11, 0, 340, 19800, 14600, 770], s2: [9, 14, 7200, 1700, 90], skin: ['#ffdbac', '#64748b', '#334155', '#92400e'], model: 'slim', groups: [],
    bio: 'Cobblestone generators and the occasional bridge. Honest prices at the Sunday market.', accent: '#64748b', badge: null, feat: 'ach_miner_49er' },
  { n: 'Axolotl_Ava', level: 3, bal: 410, s1: [8, 11, 4, 0, 70, 1500, 900, 85], s2: [5, 7, 2600, 400, 30], skin: ['#f1c27d', '#f9a8d4', '#fb7185', '#f472b6'], model: 'slim', groups: [],
    bio: 'Just joined. Hi! Looking for a guild to build with.', accent: '#f472b6', badge: null, feat: null },
];
const P = Object.fromEntries(PLAYERS.map((p) => [p.n, p]));
const names = PLAYERS.map((p) => p.n);

const state = { admin: null, servers: {}, instances: {} };
const tok = (n) => P[n].token;

// Game-server API (token authenticated, like the Paper plugin / Fabric mod).
const game = (sid, path, body) => api('POST', `/api/server/v1/${path}`, { token: state.servers[sid].token, body });
const op = () => `demo-${randomUUID()}`;
const ago = (min) => Date.now() - min * 60_000;

// ---------------------------------------------------------------------------------------------------------------------
// skin generator (64x64 PNG, no dependencies)
// ---------------------------------------------------------------------------------------------------------------------

const CRC = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, 'latin1');
  data.copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
}
const rgb = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
function shade([r, g, b], f) {
  return [r, g, b].map((v) => Math.max(0, Math.min(255, Math.round(v * f))));
}

/** A simple but recognisable Minecraft skin: face, hair, shirt, trousers and shoes on the standard 64x64 layout. */
function skinPng([skinHex, shirtHex, pantsHex, hairHex]) {
  const W = 64;
  const px = new Uint8Array(W * W * 4); // fully transparent
  const rect = (x, y, w, h, c, a = 255) => {
    for (let j = y; j < y + h; j++)
      for (let i = x; i < x + w; i++) {
        const o = (j * W + i) * 4;
        px[o] = c[0]; px[o + 1] = c[1]; px[o + 2] = c[2]; px[o + 3] = a;
      }
  };
  const dot = (x, y, c) => rect(x, y, 1, 1, c);
  const skin = rgb(skinHex), shirt = rgb(shirtHex), pants = rgb(pantsHex), hair = rgb(hairHex);
  // Head (0,0)-(32,16): all faces skin; top and back hair, fringe on the sides.
  rect(0, 8, 32, 8, skin);
  rect(8, 0, 16, 8, hair); // top + bottom
  rect(24, 8, 8, 8, hair); // back of head
  rect(0, 8, 32, 2, hair); // fringe all round
  rect(0, 10, 8, 3, shade(hair, 0.9)); rect(16, 10, 8, 3, shade(hair, 0.9)); // side burns
  // Face (front = x 8..16, y 8..16).
  const white = [245, 245, 250], iris = shade(skin, 0.35);
  dot(9, 12, white); dot(10, 12, iris); dot(13, 12, iris); dot(14, 12, white);
  rect(11, 14, 2, 1, shade(skin, 0.7)); // mouth
  dot(11, 13, shade(skin, 0.85)); dot(12, 13, shade(skin, 0.85)); // nose
  // Body (front 20,20 8x12; sides, back, top).
  rect(16, 16, 24, 16, shirt);
  rect(16, 16, 24, 4, shade(shirt, 1.12)); // collar/top
  rect(20, 20, 8, 1, shade(skin, 0.95)); dot(23, 21, shade(skin, 0.95)); dot(24, 21, shade(skin, 0.95)); // neckline
  // Arms: sleeves then skin.
  rect(40, 16, 16, 16, skin); rect(40, 16, 16, 6, shirt); rect(40, 16, 16, 4, shade(shirt, 1.12));
  rect(32, 48, 16, 16, skin); rect(32, 48, 16, 6, shirt); rect(32, 48, 16, 4, shade(shirt, 1.12));
  // Legs and shoes.
  rect(0, 16, 16, 16, pants); rect(0, 30, 16, 2, [38, 32, 30]); rect(4, 16, 8, 4, shade(pants, 1.1));
  rect(16, 48, 16, 16, pants); rect(16, 62, 16, 2, [38, 32, 30]); rect(20, 48, 8, 4, shade(pants, 1.1));
  // Belt line.
  rect(20, 31, 8, 1, shade(pants, 0.8)); rect(16, 31, 24, 1, shade(pants, 0.85));

  const raw = Buffer.alloc((W * 4 + 1) * W);
  for (let y = 0; y < W; y++) {
    raw[y * (W * 4 + 1)] = 0;
    Buffer.from(px.buffer, y * W * 4, W * 4).copy(raw, y * (W * 4 + 1) + 1);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(W, 4); ihdr[8] = 8; ihdr[9] = 6;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw, { level: 9 })), chunk('IEND', Buffer.alloc(0))]);
}

// ---------------------------------------------------------------------------------------------------------------------
// seed
// ---------------------------------------------------------------------------------------------------------------------

async function setupBranding() {
  step('Branding, settings, landing page, progression');
  const t = state.admin;
  const b = await api('GET', '/api/admin/branding', { token: t });
  Object.assign(b, {
    name: 'SCOPENET',
    tagline: 'Build together. Play fair. Stay curious.',
    version_label: 'SCOPENET 1.21',
    font: 'Outfit',
    radius: 12,
    glass: true,
    colors: { ...b.colors, accent: '#2dd4a7', accent_2: '#38bdf8', background: '#0a0e13', surface: '#131a22', text: '#e8eef5', muted: '#8a97a8', success: '#34d399', danger: '#f87171' },
    news: [
      { id: 'season-4', title: 'Season 4 starts this Saturday', body: 'A fresh Survival world, new biomes from 1.21 and a bigger market district. Guild claims reset at 18:00 UTC - plan your chunks!', image_url: null, link: null, date: '2026-09-27', pinned: true, tag: 'Event' },
      { id: 'auction-house', title: 'The Auction House is open', body: 'List items for up to a week, bid from the launcher and collect wins from the vault in game. 5% of every sale goes into the community build fund.', image_url: null, link: null, date: '2026-09-21', pinned: false, tag: 'Update' },
      { id: 'casino', title: 'Casino night: double XP on daily spins', body: 'Slots, Wheel, Plinko and Mines are live. Play responsibly - it is play money only, and every bet is logged.', image_url: null, link: null, date: '2026-09-18', pinned: false, tag: 'Fun' },
      { id: 'build-contest', title: 'Build contest: Floating Islands', body: 'Submit your build on Creative Build before the 12th. Winners get capes and a spot on the leaderboard wall.', image_url: null, link: null, date: '2026-09-10', pinned: false, tag: 'Contest' },
    ],
    links: [
      { label: 'Discord', url: 'https://discord.gg/scopenet-demo', icon: 'discord' },
      { label: 'Website', url: 'https://scopenet.example', icon: 'website' },
      { label: 'YouTube', url: 'https://youtube.com/@scopenet-demo', icon: 'youtube' },
      { label: 'GitHub', url: 'https://github.com/scopenet-demo', icon: 'github' },
    ],
    about: { title: 'About SCOPENET', icon_url: null, body: 'SCOPENET is a friendly, player-run Minecraft community: a Survival SMP with guilds, land claims and a player economy, plus a Creative server for big builds.', links: [{ label: 'Rules', url: 'https://scopenet.example/rules', icon: 'website' }] },
  });
  await api('PUT', '/api/admin/branding', { token: t, body: b });
  log('branding saved');

  const s = await api('GET', '/api/admin/settings', { token: t });
  s.auth = { ...s.auth, registration: 'open' };
  await api('PUT', '/api/admin/settings', { token: t, body: s });
  log('sign-ups: open');

  await optional('landing page', async () => {
    const l = await api('GET', '/api/admin/landing', { token: t });
    Object.assign(l, {
      enabled: true,
      brand_name: 'SCOPENET',
      tagline: 'Build together. Play fair. Stay curious.',
      hero_title: 'A Minecraft community that feels like home',
      hero_subtitle: 'Survival SMP with guilds, land claims and a living player economy - plus a Creative server for your biggest builds. One launcher, one account.',
      hero_cta_text: 'Download the launcher',
      server_ip: 'play.scopenet.example',
      server_port: 25565,
    });
    l.theme = { ...l.theme, accent: '#2dd4a7', background: '#0a0e13', surface: '#131a22', footer_text: 'SCOPENET - build together, play fair, stay curious.' };
    await api('PUT', '/api/admin/landing', { token: t, body: l });
    log('landing page enabled');
  });

  // Gentler XP rates so a few dozen hours of play land on believable levels.
  const prog = await api('GET', '/api/admin/progression', { token: t });
  prog.settings.xp_rates = { playtime_per_hour: 40, player_kill: 8, mob_kill: 1, block_broken: 0.02, block_placed: 0.02, message: 0.2 };
  await api('PUT', '/api/admin/progression', { token: t, body: prog.settings });
  state.curve = (await api('GET', '/api/admin/progression', { token: t })).curve;
  log('xp rates tuned');

  // Pin a fixed set of quests so every player sees the same, progress-friendly list (action quests aren't completed by bulk stats).
  const pins = ['q_d_mine_diamond', 'q_d_mob_creepers', 'q_d_farm_wood', 'q_d_build_planks', 'q_d_mine_iron', 'q_w_mine_gold_grand', 'q_w_mob_skeletons_grand', 'q_w_farm_crops_grand'];
  const quests = await api('GET', '/api/admin/quests', { token: t });
  for (const id of pins) {
    const q = quests.find((x) => x.id === id);
    if (!q) throw new Error(`quest ${id} missing from the seeded quest list`);
    await api('PUT', `/api/admin/quests/${id}`, { token: t, body: { title: q.title, description: q.description, period: q.period, category: q.category, target_stat: q.target_stat, target_count: q.target_count, xp_reward: q.xp_reward, icon: q.icon, enabled: q.enabled, pinned: true } });
  }
  log(`${pins.length} quests pinned`);
}

async function setupInstances() {
  step('Instances');
  const t = state.admin;
  const make = async (key, body) => {
    const inst = await api('POST', '/api/admin/instances', { token: t, body });
    state.instances[key] = inst.id;
    log(`${inst.name} (${inst.id}) ${inst.loader} ${inst.mc_version}`);
  };
  await make('survival', {
    name: 'SCOPENET Survival', description: 'The main Survival SMP: guilds, land claims, player market and quests. Fabric with a few quality-of-life mods.',
    mc_version: '1.21.4', loader: 'fabric', visibility: 'public', featured: true, sort: 1, memory: { min_mb: 2048, max_mb: 6144 },
    server: { name: 'Survival SMP', address: 'play.scopenet.example', port: 25565, auto_join: false, inject: true },
  });
  await make('creative', {
    name: 'Creative Build', description: 'Plots, WorldEdit and a very patient moderation team. Vanilla client - no mods required.',
    mc_version: '1.21.4', loader: 'vanilla', visibility: 'public', featured: false, sort: 2, memory: { min_mb: 1024, max_mb: 4096 },
    server: { name: 'Creative Build', address: 'build.scopenet.example', port: 25566, auto_join: false, inject: true },
  });
}

async function setupUsers() {
  step('Players');
  const t = state.admin;
  for (const g of [['VIP', '#f59e0b'], ['Builder', '#38bdf8'], ['Staff', '#34d399']]) await optional(`group ${g[0]}`, () => api('POST', '/api/admin/groups', { token: t, body: { name: g[0], color: g[1] } }));
  for (const p of PLAYERS) {
    const u = await api('POST', '/api/admin/users', { token: t, body: { username: p.n, password: PASS, email: `${p.n.toLowerCase()}@scopenet.example`, groups: p.groups } });
    p.id = u.id;
    p.uuid = u.uuid;
    p.token = (await api('POST', '/api/v1/auth/login', { body: { username: p.n, password: PASS } })).token;
  }
  log(`${PLAYERS.length} accounts created (password ${PASS})`);

  // The "sign-ups are open" path, too: one account registers itself.
  await optional('self-registration', async () => {
    const r = await api('POST', '/api/v1/auth/register', { body: { username: 'Fresh_Spawn', password: PASS } });
    if (!r.token) throw new Error('no token returned');
    log('Fresh_Spawn registered through /auth/register');
  });

  let skins = 0;
  for (const p of PLAYERS) {
    await optional(`skin for ${p.n}`, async () => {
      const form = new FormData();
      form.set('model', p.model);
      form.set('file', new Blob([skinPng(p.skin)], { type: 'image/png' }), 'skin.png');
      await api('POST', `/api/admin/users/${p.id}/skin`, { token: t, form });
      skins++;
    });
  }
  log(`${skins} skins uploaded`);
}

async function setupServers() {
  step('Game servers');
  const defs = [
    { key: 'survival', name: 'Survival SMP', max: 60, software: 'Paper 1.21.4-232', ver: '1.21.4' },
    { key: 'creative', name: 'Creative Build', max: 30, software: 'Paper 1.21.4-232', ver: '1.21.4' },
  ];
  for (const d of defs) {
    const r = await api('POST', '/api/admin/servers', { token: state.admin, body: { name: d.name, instance_id: state.instances[d.key], access: 'all', map_enabled: true } });
    const sid = r.server.id;
    state.servers[sid] = { id: sid, key: d.key, name: d.name, token: r.token, online: [] };
    await game(sid, 'hello', { software: d.software, mc_version: d.ver, plugin_version: '1.4.0', online_mode: true, max_players: d.max });
    log(`server ${sid}: ${d.name} (instance ${state.instances[d.key]})`);
  }
}
const S1 = () => Object.values(state.servers).find((s) => s.key === 'survival').id;
const S2 = () => Object.values(state.servers).find((s) => s.key === 'creative').id;

async function sync(sid, extra = {}) {
  const online = (state.servers[sid].online ?? []).map((n) => ({ uuid: P[n].uuid, name: n }));
  return game(sid, 'sync', { batch_id: randomUUID(), tps: round2(rnd(19.1, 20)), online, stats: [], events: [], features: { leveling_enabled: true, quests_enabled: true, achievements_enabled: true }, ...extra });
}

async function seedStats() {
  step('Stats, events, online players');
  const s1 = S1(), s2 = S2();
  const stat1 = PLAYERS.map((p) => {
    const [h, joins, deaths, pk, mk, br, pl, msg] = p.s1;
    return { uuid: p.uuid, name: p.n, playtime_secs: h * 3600 + rint(0, 3000), joins, deaths, player_kills: pk, mob_kills: mk, blocks_broken: br, blocks_placed: pl, messages: msg };
  });
  const stat2 = PLAYERS.filter((p) => p.s2.length).map((p) => {
    const [h, joins, pl, br, msg] = p.s2;
    return { uuid: p.uuid, name: p.n, playtime_secs: h * 3600 + rint(0, 3000), joins, deaths: Math.round(joins / 6), player_kills: 0, mob_kills: Math.round(h * 2), blocks_broken: br, blocks_placed: pl, messages: msg };
  });
  state.servers[s1].online = ['Alex_Miner', 'Steve_Builds', 'LunaMC', 'Redstone_Rae', 'NetherNinja', 'Cobble_Cleo', 'ObsidianOwen'];
  state.servers[s2].online = ['PixelPaige', 'EnderEmma', 'TNT_Tommy'];

  const ev = (n, kind, detail, minAgo) => ({ uuid: P[n].uuid, name: n, kind, detail, at: ago(minAgo) });
  const events1 = [
    { kind: 'start', detail: null, at: ago(2300) },
    ev('NetherNinja', 'join', null, 1500), ev('CreeperSlayer', 'join', null, 1430), ev('CreeperSlayer', 'advancement', 'Hot Tourist Destination', 1400),
    ev('Steve_Builds', 'join', null, 960), ev('LunaMC', 'join', null, 900), ev('LunaMC', 'chat', 'Sunday market is open at spawn, bring your shulkers!', 880),
    ev('Cobble_Cleo', 'join', null, 700), ev('DiamondDave', 'join', null, 640), ev('DiamondDave', 'advancement', 'Diamonds!', 610),
    ev('Alex_Miner', 'join', null, 420), ev('Alex_Miner', 'advancement', 'Cover Me with Diamonds', 400), ev('Alex_Miner', 'chat', 'anyone want to trade for a netherite pickaxe? check the auction house', 395),
    ev('TNT_Tommy', 'death', 'TNT_Tommy blew up', 380), ev('BlazeRunner', 'kill', 'BlazeRunner slew TNT_Tommy', 375), ev('TNT_Tommy', 'chat', 'rematch?', 372),
    ev('Redstone_Rae', 'join', null, 330), ev('Redstone_Rae', 'advancement', 'Wireless Redstone', 300),
    ev('ObsidianOwen', 'join', null, 250), ev('DiamondDave', 'death', 'DiamondDave fell from a high place', 240), ev('DiamondDave', 'quit', null, 235),
    ev('Cobble_Cleo', 'chat', 'Cobble generator upgrades are live - 64 stacks for 40$', 200), ev('Steve_Builds', 'advancement', 'Postmortal', 190),
    ev('NetherNinja', 'death', 'NetherNinja was blown up by Creeper', 160), ev('NetherNinja', 'kill', 'NetherNinja slew Wither Skeleton', 150),
    ev('Alex_Miner', 'death', 'Alex_Miner tried to swim in lava', 120), ev('Alex_Miner', 'chat', 'ouch. again.', 119),
    ev('LunaMC', 'advancement', 'Local Brewery', 95), ev('EnderEmma', 'quit', null, 80), ev('Steve_Builds', 'chat', 'castle wall is finally done!', 60), ev('CreeperSlayer', 'quit', null, 45),
    ev('Alex_Miner', 'advancement', 'Serious Dedication', 25), ev('LunaMC', 'chat', 'gg Alex', 20), ev('NetherNinja', 'advancement', 'Spooky Scary Skeleton', 10),
  ];
  const events2 = [
    { kind: 'start', detail: null, at: ago(2300) },
    ev('PixelPaige', 'join', null, 700), ev('PixelPaige', 'chat', 'the floating islands plot is looking great', 650), ev('Steve_Builds', 'join', null, 520), ev('Steve_Builds', 'quit', null, 340),
    ev('Redstone_Rae', 'join', null, 300), ev('Redstone_Rae', 'quit', null, 210), ev('EnderEmma', 'join', null, 120), ev('TNT_Tommy', 'join', null, 70), ev('PixelPaige', 'chat', 'contest entries close on the 12th!', 35),
  ];
  // Fixed milestones that unlock achievements.
  const milestones = ['NetherNinja', 'CreeperSlayer', 'Alex_Miner', 'ObsidianOwen', 'BlazeRunner', 'EnderEmma', 'Steve_Builds', 'Redstone_Rae', 'LunaMC'].map((n) => ({ uuid: P[n].uuid, name: n, kind: 'nether', detail: 'entered the Nether', at: ago(3000 - rint(0, 1000)) }));
  for (const n of ['NetherNinja', 'CreeperSlayer']) milestones.push({ uuid: P[n].uuid, name: n, kind: 'ender_dragon', detail: 'defeated the Ender Dragon', at: ago(2000) });

  const half = (a) => a.map((s) => ({ ...s, playtime_secs: Math.floor(s.playtime_secs / 2), joins: Math.floor(s.joins / 2), deaths: Math.floor(s.deaths / 2), player_kills: Math.floor(s.player_kills / 2), mob_kills: Math.floor(s.mob_kills / 2), blocks_broken: Math.floor(s.blocks_broken / 2), blocks_placed: Math.floor(s.blocks_placed / 2), messages: Math.floor(s.messages / 2) }));
  const rest = (a, h) => a.map((s, i) => ({ ...s, playtime_secs: s.playtime_secs - h[i].playtime_secs, joins: s.joins - h[i].joins, deaths: s.deaths - h[i].deaths, player_kills: s.player_kills - h[i].player_kills, mob_kills: s.mob_kills - h[i].mob_kills, blocks_broken: s.blocks_broken - h[i].blocks_broken, blocks_placed: s.blocks_placed - h[i].blocks_placed, messages: s.messages - h[i].messages }));
  const a1 = half(stat1), a2 = half(stat2);
  await sync(s1, { stats: a1, events: [...milestones] });
  await sync(s1, { stats: rest(stat1, a1), events: events1 });
  await sync(s2, { stats: a2 });
  await sync(s2, { stats: rest(stat2, a2), events: events2 });
  log(`Survival: ${stat1.length} players with stats, ${events1.length + milestones.length} events; Creative: ${stat2.length} players, ${events2.length} events`);
}

/** Turns a quest target like `action:block_broken:*_LOG|*_WOOD` into one concrete in-game action string. */
function concreteAction(target) {
  let rest = target.replace(/^action:/, '');
  let dim = 'minecraft:overworld';
  const at = rest.indexOf('@');
  if (at >= 0) { dim = rest.slice(at + 1); rest = rest.slice(0, at); }
  const [kind, mats] = rest.split(':');
  let mat = mats.split('|')[0];
  if (mat === '*') mat = 'STONE';
  else if (mat.startsWith('*')) mat = 'OAK' + mat.slice(1);
  else if (mat.endsWith('*')) mat = mat.slice(0, -1) + 'STONE';
  return `${kind}:${mat}@${dim}`;
}

async function seedQuests() {
  step('Quests');
  const s1 = S1();
  // Alex: a designed day. [done, claim?] per quest id.
  const alexPlan = {
    q_d_mine_diamond: [4, true], q_d_farm_wood: [32, false], q_d_mob_creepers: [3, false], q_d_build_planks: [20, false], q_d_mine_iron: [9, false],
    q_w_mine_gold_grand: [41, false], q_w_mob_skeletons_grand: [40, false], q_w_farm_crops_grand: [120, false],
  };
  const plans = {};
  for (const p of PLAYERS) {
    const mine = await api('GET', '/api/v1/quests/my', { token: p.token });
    const list = mine.map((x) => x.quest);
    plans[p.n] = list.map((q) => {
      let done;
      let claim = false;
      if (p.n === MAIN) [done, claim] = alexPlan[q.id] ?? [0, false];
      else {
        const f = pick([0, 0, 0.25, 0.5, 0.75, 1, 1]);
        done = Math.round(q.target_count * f);
        claim = f === 1 && Math.random() < 0.6;
      }
      return { q, done, claim };
    });
  }
  const events = [];
  for (const p of PLAYERS)
    for (const { q, done } of plans[p.n])
      if (done > 0 && q.target_stat.startsWith('action:')) events.push({ uuid: p.uuid, name: p.n, kind: 'action', detail: `${concreteAction(q.target_stat)} +${done}`, at: ago(rint(5, 600)) });
  // Playing time and block counts already moved other quests' stats; action events only matter for the pinned set.
  await sync(s1, { events });
  let claimed = 0;
  for (const p of PLAYERS)
    for (const { q, claim } of plans[p.n])
      if (claim) {
        const r = await quiet(() => api('POST', `/api/v1/quests/${q.id}/claim`, { token: p.token }));
        if (r) claimed++;
      }
  log(`${events.length} quest progress events, ${claimed} rewards claimed`);
}

async function seedLevels() {
  step('Levels');
  const curve = Object.fromEntries(state.curve.map((c) => [c.level, c.xp]));
  curve[1] = 0;
  for (const p of PLAYERS) {
    const lo = curve[p.level], hi = curve[p.level + 1] ?? lo + 700;
    const xp = Math.round(lo + (hi - lo) * (p.n === MAIN ? 0.42 : rnd(0.1, 0.8)));
    await api('POST', `/api/admin/progression/players/${p.uuid}/adjust`, { token: state.admin, body: { scope: 'global', mode: 'set_xp', amount: xp, reason: 'demo seed' } });
    await quiet(() => api('POST', `/api/admin/progression/players/${p.uuid}/adjust`, { token: state.admin, body: { scope: 'server', server_id: S1(), mode: 'set_xp', amount: Math.round(xp * 0.9), reason: 'demo seed' } }));
  }
  const me = await api('GET', '/api/v1/levels/me', { token: tok(MAIN) });
  log(`${MAIN}: level ${me.global_level ?? me.level}, ${me.global_xp} xp`);
}

async function seedEconomy() {
  step('Economy');
  const s1 = S1(), s2 = S2();
  const flavour = ['Daily mining payout', 'Shop sale', 'Quest reward', 'Event prize', 'Trade settlement'];
  for (const p of PLAYERS) {
    // Start 1000 (server rule) -> target, in a few believable steps.
    let remaining = (p.start ?? p.bal) - 1000;
    const steps = p.n === MAIN ? 5 : rint(2, 3);
    for (let i = 0; i < steps; i++) {
      const last = i === steps - 1;
      const part = last ? remaining : round2(remaining * rnd(0.25, 0.5));
      remaining = round2(remaining - part);
      if (Math.abs(part) < 0.01) continue;
      await game(s1, 'economy/adjust', { uuid: p.uuid, username: p.n, delta: round2(part), operation_id: op(), description: part < 0 ? 'Shop purchase' : flavour[i % flavour.length] });
    }
    if (p.s2.length) await game(s2, 'economy/adjust', { uuid: p.uuid, username: p.n, delta: round2(rnd(300, 3200)), operation_id: op(), description: 'Build contest prize' });
  }
  // Player-to-player payments.
  const pays = [['Alex_Miner', 'LunaMC', 250, 'Thanks for the potions'], ['Steve_Builds', 'Alex_Miner', 1200, 'Stone brick order'], ['Alex_Miner', 'Cobble_Cleo', 320, 'Cobble delivery'], ['NetherNinja', 'BlazeRunner', 900, 'Blaze rods'],
    ['LunaMC', 'Alex_Miner', 80, 'Beer money'], ['Redstone_Rae', 'Alex_Miner', 450, 'Redstone for the sorter']];
  for (const [f, t2, amount, description] of pays)
    await quiet(() => game(s1, 'economy/pay', { from_uuid: P[f].uuid, from_name: f, to_uuid: P[t2].uuid, to_name: t2, amount, description }));
  log(`${PLAYERS.length} balances set on Survival SMP, ${pays.length} player payments`);
}

async function seedGuilds() {
  step('Guilds');
  const s1 = S1();
  const inst = state.instances.survival;
  const defs = [
    { name: 'Iron Pickaxe Co.', tag: 'IRON', leader: MAIN, officer: ['Steve_Builds'], members: ['Redstone_Rae', 'Cobble_Cleo', 'FarmerFinn', 'MossyMax', 'DiamondDave'], origin: [0, 0, 4, 3],
      desc: 'Miners, builders and tinkerers. We pool ore, share farms and throw a build night every Saturday. New members welcome - just ask!', motd: 'Build night Saturday 20:00 at the guild hall.', bank: 6480 },
    { name: 'Nether Nomads', tag: 'NNOM', leader: 'NetherNinja', officer: ['BlazeRunner'], members: ['ObsidianOwen', 'EnderEmma'], origin: [-9, 5, 3, 3], desc: 'Fortress raiders and portal engineers. We live below Y=32 and above lava.', motd: 'Bastion run tonight.', bank: 9230.5 },
    { name: 'Pixel Pioneers', tag: 'PIXL', leader: 'PixelPaige', officer: [], members: ['LunaMC', 'TNT_Tommy'], origin: [6, -6, 3, 3], desc: 'Art, aesthetics and the occasional explosion. Mega-builds are a team sport.', motd: 'Mural phase 2 this weekend.', bank: 2100 },
    { name: 'Creeper Hunters', tag: 'HUNT', leader: 'CreeperSlayer', officer: [], members: [], origin: [-4, -10, 3, 2], desc: 'Hunting everything that goes hiss in the night.', motd: 'Full moon tonight - bring a shield.', bank: 1480 },
  ];
  state.guilds = {};
  for (const g of defs) {
    const made = await api('POST', '/api/v1/guilds', { token: tok(g.leader), body: { instance_id: inst, name: g.name, tag: g.tag, description: g.desc, motd: g.motd } });
    const id = made.id;
    state.guilds[g.tag] = id;
    await api('PUT', `/api/v1/guilds/${id}`, { token: tok(g.leader), body: { description: g.desc, motd: g.motd } });
    for (const m of [...g.officer, ...g.members]) await api('POST', `/api/v1/guilds/${id}/members`, { token: tok(g.leader), body: { username: m } });
    for (const o of g.officer) await api('PUT', `/api/v1/guilds/${id}/members/${P[o].uuid}/role`, { token: tok(g.leader), body: { role: 'officer' } });
    // Claimed chunks: a w x h block at the guild's origin.
    const [ox, oz, w, h] = g.origin;
    let claims = 0;
    for (let x = 0; x < w; x++) for (let z = 0; z < h; z++) {
      const r = await quiet(() => api('POST', `/api/v1/guilds/${id}/claims`, { token: tok(g.leader), body: { server_id: s1, chunk_x: ox + x, chunk_z: oz + z } }));
      if (r) claims++;
    }
    log(`${g.name} [${g.tag}]: ${1 + g.officer.length + g.members.length} members, ${claims} chunks`);
  }
  const iron = state.guilds.IRON;
  await optional('IRON custom role', async () => {
    await api('POST', `/api/v1/guilds/${iron}/roles`, { token: tok(MAIN), body: { name: 'Engineer', priority: 5, can_invite: false, can_kick: false, can_claim: true, can_post: true, can_manage: false } });
    await api('PUT', `/api/v1/guilds/${iron}/members/${P.Redstone_Rae.uuid}/role`, { token: tok(MAIN), body: { role: 'Engineer' } });
  });
  // Bank: member deposits, shop income and one officer withdrawal, per guild up to its target balance.
  const memberDeposits = { IRON: [['Alex_Miner', 1500], ['Steve_Builds', 1200], ['Redstone_Rae', 600], ['Cobble_Cleo', 400], ['FarmerFinn', 180], ['DiamondDave', 250]], NNOM: [['NetherNinja', 4000], ['BlazeRunner', 800], ['ObsidianOwen', 500]], PIXL: [['PixelPaige', 900], ['LunaMC', 500]], HUNT: [['CreeperSlayer', 600]] };
  for (const g of defs) {
    let have = 0;
    for (const [n, amount] of memberDeposits[g.tag]) {
      const r = await quiet(() => game(s1, 'guilds/bank/transfer', { operation_id: op(), uuid: P[n].uuid, username: n, amount, action: 'deposit' }));
      if (r) have += amount;
    }
    const credit = round2(g.bank - have + (g.tag === 'IRON' ? 500 : 0));
    if (credit > 0) await quiet(() => game(s1, 'guilds/bank/credit', { operation_id: op(), uuid: P[g.leader].uuid, username: g.leader, amount: credit, description: g.tag === 'IRON' ? '128x Iron Ingot (market sale)' : 'Shop sales' }));
    if (g.tag === 'IRON') await quiet(() => game(s1, 'guilds/bank/transfer', { operation_id: op(), uuid: P.Alex_Miner.uuid, username: 'Alex_Miner', amount: 500, action: 'withdraw' }));
  }
  // Guild posts.
  const posts = [
    ['Alex_Miner', iron, 'Build night this Saturday', 'We are finishing the storage hall at 20:00. Bring stone bricks and spare shulker boxes - Steve has the blueprints.'],
    ['Steve_Builds', iron, 'Hall roof is done!', 'The roof is on and the lanterns are in. Next up: the sorting wall. Rae, can you wire it?'],
    ['Redstone_Rae', iron, 'Auto-sorter design v2', 'New design handles 6 item types per column and is 30% smaller. Schematic is in the chest by the entrance.'],
    ['NetherNinja', state.guilds.NNOM, 'Bastion run tonight', 'Gold gear only, bring fire resistance. We meet at the portal at 21:00.'],
    ['PixelPaige', state.guilds.PIXL, 'Mural phase 2', 'Phase 1 is complete! For phase 2 we need 4 more people on the palette team.'],
  ];
  for (const [who, gid, title, content] of posts) await optional(`guild post "${title}"`, () => api('POST', `/api/v1/guilds/${gid}/posts`, { token: tok(who), body: { title, content } }));
  // A pending join request and an invite so Alex has something to answer.
  await optional('join request', () => api('POST', `/api/v1/guilds/${iron}/requests`, { token: tok('Axolotl_Ava'), body: { message: "Hi! I'm new and love mining. Can I join?" } }));
  await optional('guild invite', () => api('POST', `/api/v1/guilds/${state.guilds.NNOM}/invites`, { token: tok('NetherNinja'), body: { uuid: P.Axolotl_Ava.uuid } }));
}

async function seedSocial() {
  step('Social');
  // Profiles
  for (const p of PLAYERS) {
    await optional(`profile ${p.n}`, () => api('PUT', '/api/v1/profiles/me', { token: p.token, body: { bio: p.bio, accent_color: p.accent, custom_badge: p.badge ?? undefined, featured_achievement_id: p.feat ?? undefined } }));
  }
  // Friendships: [requester, recipient, accepted?]
  const fr = [
    ['Steve_Builds', MAIN, true], [MAIN, 'LunaMC', true], ['Redstone_Rae', MAIN, true], [MAIN, 'Cobble_Cleo', true], ['FarmerFinn', MAIN, true], [MAIN, 'DiamondDave', true], ['MossyMax', MAIN, true],
    ['EnderEmma', MAIN, false], ['Axolotl_Ava', MAIN, false], [MAIN, 'PixelPaige', false],
    ['Steve_Builds', 'LunaMC', true], ['Steve_Builds', 'Redstone_Rae', true], ['PixelPaige', 'Steve_Builds', true], ['LunaMC', 'PixelPaige', true],
    ['NetherNinja', 'CreeperSlayer', true], ['NetherNinja', 'BlazeRunner', true], ['ObsidianOwen', 'NetherNinja', true], ['CreeperSlayer', 'DiamondDave', true], ['BlazeRunner', 'CreeperSlayer', true],
    ['EnderEmma', 'NetherNinja', true], ['TNT_Tommy', 'BlazeRunner', true], ['MossyMax', 'Cobble_Cleo', true], ['MossyMax', 'FarmerFinn', true], ['Axolotl_Ava', 'Cobble_Cleo', false], ['TNT_Tommy', 'Redstone_Rae', true],
  ];
  let friends = 0;
  for (const [a, b, accept] of fr) {
    const sent = await quiet(() => api('POST', '/api/v1/friends/request', { token: tok(a), body: { username: b } }));
    if (sent && accept) {
      const ok = await quiet(() => api('POST', '/api/v1/friends/respond', { token: tok(b), body: { target_uuid: P[a].uuid, accept: true } }));
      if (ok) friends++;
    }
  }
  log(`${friends} friendships, ${fr.filter((f) => !f[2]).length} pending requests`);
  // Direct messages: [from, to, text]
  const dm = [
    ['Steve_Builds', MAIN, 'Alex! Are you coming to build night on Saturday?'], [MAIN, 'Steve_Builds', 'Wouldn\'t miss it. I\'ll bring 6 stacks of stone bricks.'], ['Steve_Builds', MAIN, 'Perfect, the hall roof needs about that much.'],
    [MAIN, 'Steve_Builds', 'Also, did you see the netherite pickaxe on the auction house? 😅'], ['Steve_Builds', MAIN, 'Saw it. I might bid at the last minute.'], [MAIN, 'Steve_Builds', 'Sniper! Fair enough.'], ['Steve_Builds', MAIN, 'See you at 20:00!'],
    ['LunaMC', MAIN, 'Hey, can you sell me 16 diamonds? I want to finish my enchanting setup.'], [MAIN, 'LunaMC', 'Sure - I just listed a stack of 32 on the market, grab 16 off it?'], ['LunaMC', MAIN, 'Deal, thanks!'], [MAIN, 'LunaMC', 'No worries. The Sunday market is going to be huge.'], ['LunaMC', MAIN, 'See you there 🎉'],
    ['Redstone_Rae', MAIN, 'The sorter wiring is done, finally.'], [MAIN, 'Redstone_Rae', 'Legend. Does it handle shulker boxes?'], ['Redstone_Rae', MAIN, 'Nope, those jam it. v3 will.'], [MAIN, 'Redstone_Rae', 'Lol ok, v3 it is. I sent 450$ for the redstone.'], ['Redstone_Rae', MAIN, 'Got it, thanks!'], [MAIN, 'Redstone_Rae', 'Anytime.'],
    ['Cobble_Cleo', MAIN, 'Your cobble order is ready at the usual chest.'], [MAIN, 'Cobble_Cleo', 'Picking it up in 10!'], ['Cobble_Cleo', MAIN, 'Cool.'],
    ['DiamondDave', MAIN, 'Found a huge diamond vein at -59, want to join me tonight?'],
    ['NetherNinja', 'CreeperSlayer', 'Dragon fight at 9? I have the beds.'], ['CreeperSlayer', 'NetherNinja', 'Obviously. Bringing the good bow.'],
    ['PixelPaige', 'Steve_Builds', 'Your castle wall on Creative is insane!'], ['Steve_Builds', 'PixelPaige', 'Thanks! Your mural is better.'],
  ];
  for (const [a, b, content] of dm) await quiet(() => api('POST', `/api/v1/messages/${P[b].uuid}`, { token: tok(a), body: { content } }));
  // Reading marks messages read; mark all of Alex's threads as read except Dave's and Luna's last.
  for (const other of ['Steve_Builds', 'Redstone_Rae', 'Cobble_Cleo']) await quiet(() => api('GET', `/api/v1/messages/${P[other].uuid}?mark_read=true`, { token: tok(MAIN) }));
  log(`${dm.length} direct messages`);
  // Profile posts
  const posts = [
    [MAIN, 'Just hit level 17 and finally found my first ancient debris vein. Y=15 is the way.'], [MAIN, 'Selling a Netherite Pickaxe (Eff V, Unbreaking III) on the auction house. Starting bid 2,500$.'], [MAIN, 'Build night this Saturday at the Iron Pickaxe guild hall. Everyone welcome!'],
    ['Steve_Builds', 'Castle wall finished! Took 14 hours and approximately 9,000 stone bricks.'], ['Steve_Builds', 'Taking commissions again. Medieval, rustic or modern - DM me.'],
    ['LunaMC', 'Sunday market this weekend! Bring your best items and bring a friend.'], ['LunaMC', 'Reminder: be nice in chat. We are all here to have fun. 💚'],
    ['CreeperSlayer', 'Dragon down again. The elytra drops are getting out of hand.'], ['NetherNinja', 'Fortress count: 14. Wither skeleton skulls: 3. Luck: questionable.'],
    ['PixelPaige', 'New mural is up on Creative Build - come take a look and tell me what to add!'], ['Redstone_Rae', 'Auto-sorter v2 is out. Schematic in the guild chest.'], ['TNT_Tommy', 'Do not ask what happened to the village.'],
    ['EnderEmma', 'Found a shulker box with 27 diamonds in an end city. Today is a good day.'], ['Cobble_Cleo', 'Cobblestone, stone, deepslate - bulk discounts at the Sunday market.'],
  ];
  const created = [];
  for (const [n, content] of posts) {
    const r = await quiet(() => api('POST', '/api/v1/profiles/me/posts', { token: tok(n), body: { content } }));
    if (r) created.push({ id: r.id, by: n });
  }
  for (const c of created) {
    const likers = names.filter((n) => n !== c.by && Math.random() < (c.by === MAIN || c.by === 'LunaMC' ? 0.6 : 0.3));
    for (const l of likers) await quiet(() => api('POST', `/api/v1/posts/${c.id}/like`, { token: tok(l) }));
  }
  log(`${created.length} profile posts with likes`);
}

async function seedMarket() {
  step('Market');
  const s1 = S1(), s2 = S2();
  // [seller, item_id, name, amount, price, kind, hours]
  const L1 = [
    ['Steve_Builds', 'DIAMOND_SWORD', 'Diamond Sword (Sharpness V)', 1, 450, 'buy_now'],
    [MAIN, 'NETHERITE_PICKAXE', 'Netherite Pickaxe (Efficiency V)', 1, 2500, 'auction', 71],
    ['NetherNinja', 'ELYTRA', 'Elytra', 1, 3000, 'auction', 5],
    [MAIN, 'DIAMOND', 'Diamond', 32, 1400, 'buy_now'],
    ['LunaMC', 'GOLDEN_APPLE', 'Golden Apple', 8, 640, 'buy_now'],
    ['ObsidianOwen', 'ENCHANTED_GOLDEN_APPLE', 'Enchanted Golden Apple', 1, 1800, 'auction', 2],
    ['FarmerFinn', 'OAK_LOG', 'Oak Log', 64, 90, 'buy_now'],
    [MAIN, 'IRON_INGOT', 'Iron Ingot', 64, 260, 'buy_now'],
    ['Redstone_Rae', 'BEACON', 'Beacon', 1, 2200, 'buy_now'],
    ['CreeperSlayer', 'TOTEM_OF_UNDYING', 'Totem of Undying', 1, 1200, 'auction', 1],
    ['NetherNinja', 'NETHERITE_INGOT', 'Netherite Ingot', 4, 3800, 'buy_now'],
    ['EnderEmma', 'TRIDENT', 'Trident (Loyalty III)', 1, 900, 'auction', 24],
    [MAIN, 'SHULKER_BOX', 'Shulker Box (Blue)', 1, 400, 'auction', 47],
    ['Cobble_Cleo', 'EXPERIENCE_BOTTLE', 'Bottle o\' Enchanting', 64, 800, 'buy_now'],
    ['FarmerFinn', 'WHEAT', 'Wheat', 64, 60, 'buy_now'],
    ['Redstone_Rae', 'REDSTONE', 'Redstone Dust', 64, 180, 'buy_now'],
    ['PixelPaige', 'SEA_LANTERN', 'Sea Lantern', 32, 300, 'buy_now'],
    ['BlazeRunner', 'CROSSBOW', 'Crossbow (Quick Charge III)', 1, 350, 'auction', 11],
    ['EnderEmma', 'ENDER_PEARL', 'Ender Pearl', 16, 320, 'buy_now'],
    ['DiamondDave', 'ENCHANTED_BOOK', 'Enchanted Book (Mending)', 1, 600, 'auction', 6],
    ['NetherNinja', 'DRAGON_HEAD', 'Dragon Head', 1, 2500, 'auction', 95],
    ['Cobble_Cleo', 'COBBLESTONE', 'Cobblestone', 64, 40, 'buy_now'],
    ['MossyMax', 'SPONGE', 'Sponge', 16, 250, 'buy_now'],
    ['TNT_Tommy', 'TNT', 'TNT', 64, 400, 'buy_now'],
    ['ObsidianOwen', 'MUSIC_DISC_PIGSTEP', 'Music Disc (Pigstep)', 1, 700, 'auction', 29],
    ['DiamondDave', 'DIAMOND_BLOCK', 'Block of Diamond', 5, 6200, 'buy_now'],
    ['Steve_Builds', 'STONE_BRICKS', 'Stone Bricks', 64, 70, 'buy_now'],
  ];
  const L2 = [
    ['PixelPaige', 'QUARTZ_BLOCK', 'Block of Quartz', 64, 120, 'buy_now'],
    ['Steve_Builds', 'GLASS', 'Glass', 64, 60, 'buy_now'],
    ['PixelPaige', 'WHITE_BANNER', 'Pixel Mural Banner', 1, 250, 'auction', 20],
    ['LunaMC', 'LANTERN', 'Lantern', 32, 150, 'buy_now'],
  ];
  const list = async (sid, [seller, item_id, item_name, amount, price, kind, hours]) =>
    quiet(() => game(sid, 'economy/market/list', { operation_id: op(), seller_uuid: P[seller].uuid, seller_name: seller, item_id, item_name, amount, price, kind, duration_hours: kind === 'auction' ? hours : undefined, item_data: 'demo' }));
  let n1 = 0;
  for (const l of L1) if (await list(s1, l)) n1++;
  for (const l of L2) await list(s2, l);
  log(`${n1} listings on Survival SMP, ${L2.length} on Creative Build`);

  // Bidding war on the auctions.
  const bidders = names.filter((n) => n !== MAIN && P[n].bal > 2500);
  const view = (n) => api('GET', `/api/v1/market/${s1}`, { token: tok(n) });
  let bids = 0;
  const bid = async (n, listing, extra = 0) => {
    const amount = Math.ceil(listing.min_next_bid * (1 + extra));
    const r = await quiet(() => api('POST', `/api/v1/market/${s1}/bid`, { token: tok(n), body: { listing_id: listing.id, amount } }));
    if (r) bids++;
    return r;
  };
  const listings = (await view(MAIN)).listings.filter((l) => l.kind === 'auction');
  for (const l of listings) {
    const pool = bidders.filter((n) => n !== l.seller_name);
    const rounds = l.seller_name === MAIN && l.item_id === 'NETHERITE_PICKAXE' ? 6 : rint(1, 4);
    for (let i = 0; i < rounds; i++) {
      const fresh = (await view(pick(pool))).listings.find((x) => x.id === l.id); // up to date minimum
      if (!fresh) break;
      const who = pool.filter((n) => n !== fresh.bidder_name && P[n].bal > fresh.min_next_bid * 3);
      if (!who.length) break;
      await bid(pick(who), fresh, rnd(0, 0.15));
    }
  }
  // Alex bids on two of other people's auctions: wins one at the moment, is outbid on the other.
  const mine = async () => (await view(MAIN)).listings;
  const elytra = (await mine()).find((l) => l.item_id === 'ELYTRA');
  if (elytra) {
    await bid(MAIN, elytra, 0.05);
    const after = (await mine()).find((l) => l.id === elytra.id);
    if (after && after.bidder_name !== MAIN) await bid(MAIN, after, 0.02);
  }
  const trident = (await mine()).find((l) => l.item_id === 'TRIDENT');
  if (trident) {
    await bid(MAIN, trident, 0);
    const after = (await mine()).find((l) => l.id === trident.id);
    await bid('LunaMC', after, 0.1);
  }
  log(`${bids} bids placed`);

  // A few purchases so the "sold / bought" history is not empty.
  const buys = [['Steve_Builds', 'IRON_INGOT', MAIN], [MAIN, 'EXPERIENCE_BOTTLE', 'Cobble_Cleo'], ['LunaMC', 'DIAMOND', MAIN], [MAIN, 'REDSTONE', 'Redstone_Rae'], ['Cobble_Cleo', 'WHEAT', 'FarmerFinn']];
  let bought = 0;
  for (const [buyer, item, seller] of buys) {
    const l = (await mine()).find((x) => x.item_id === item && x.seller_name === seller && x.kind === 'buy_now');
    if (l && (await quiet(() => api('POST', `/api/v1/market/${s1}/buy`, { token: tok(buyer), body: { listing_id: l.id } })))) bought++;
  }
  // Re-list what Alex sold so he still has stock for sale.
  await list(s1, [MAIN, 'DIAMOND', 'Diamond', 16, 760, 'buy_now']);
  await list(s1, [MAIN, 'IRON_INGOT', 'Iron Ingot', 64, 255, 'buy_now']);
  log(`${bought} purchases`);
}

async function seedCasino() {
  step('Casino');
  const s1 = S1();
  const t = state.admin;
  const cfg = await api('GET', '/api/admin/casino', { token: t });
  cfg.config.enabled = true;
  await api('PUT', '/api/admin/casino', { token: t, body: cfg.config });
  const lobby = await api('GET', `/api/v1/casino/${s1}`, { token: tok(MAIN) });
  const rowsOpts = Object.keys(lobby.plinko_tables?.high ?? { 12: 1 }).map(Number);
  const play = (n, game, body) => quiet(() => api('POST', `/api/v1/casino/${s1}/${game}`, { token: tok(n), body }));
  let rounds = 0;
  for (const p of PLAYERS) {
    if (p.n === 'Axolotl_Ava') continue;
    await play(p.n, 'daily', {});
    const total = p.n === MAIN ? 34 : Math.max(5, Math.min(14, Math.round(p.bal / 1500)));
    const bets = p.bal > 10000 ? [20, 50, 100, 100, 250, 500] : p.bal > 3000 ? [10, 20, 25, 50, 100] : [10, 10, 20, 25];
    for (let i = 0; i < total; i++) {
      const bet = pick(bets);
      const roll = Math.random();
      let r;
      if (roll < 0.42) r = await play(p.n, 'slots', { bet });
      else if (roll < 0.62) r = await play(p.n, 'wheel', { bet });
      else if (roll < 0.9) r = await play(p.n, 'plinko', { bet, rows: pick(rowsOpts.filter((x) => x >= 8)) || 12, risk: pick(['low', 'medium', 'high']) });
      else {
        const s = await play(p.n, 'mines/start', { bet, mines: pick([2, 3, 4]) });
        r = s;
        if (s) {
          for (let k = 0; k < rint(1, 3); k++) {
            const rv = await play(p.n, 'mines/reveal', { tile: rint(0, 24) });
            if (!rv) break;
          }
          await play(p.n, 'mines/cashout', {});
        }
      }
      if (r) rounds++;
    }
  }
  log(`${rounds} casino rounds + daily spins`);

  const post = (n, path, body) => quiet(() => api('POST', `/api/v1/casino/${s1}/${path}`, { token: tok(n), body }));
  const bounties = [['Steve_Builds', 'CreeperSlayer', 1500, false], ['LunaMC', 'NetherNinja', 800, true], [MAIN, 'BlazeRunner', 600, false], ['Redstone_Rae', 'TNT_Tommy', 250, false], ['ObsidianOwen', 'NetherNinja', 1200, false]];
  let bn = 0;
  for (const [by, target, amount, anonymous] of bounties) if (await post(by, 'bounties', { target, amount, anonymous })) bn++;
  const markets = [
    [MAIN, { subject: 'Steve_Builds', metric: 'player_kills', threshold: 3, window_minutes: 360 }],
    ['Redstone_Rae', { subject: 'NetherNinja', metric: 'mob_kills', threshold: 50, window_minutes: 1440 }],
    ['LunaMC', { subject: 'CreeperSlayer', metric: 'deaths', threshold: 2, window_minutes: 360 }],
    ['Steve_Builds', { subject: 'DiamondDave', metric: 'blocks_broken', threshold: 800, window_minutes: 60 }],
  ];
  let mk = 0;
  for (const [by, body] of markets) {
    const m = await post(by, 'markets', body);
    if (!m) continue;
    mk++;
    const bettors = names.filter((n) => n !== by && n !== body.subject && P[n].bal > 1500);
    for (const b of bettors.slice(0, rint(3, 6))) await post(b, `markets/${m.id}/bet`, { side: Math.random() < 0.5 ? 'yes' : 'no', stake: pick([25, 50, 100, 200]) });
  }
  log(`${bn} bounties, ${mk} bet markets`);
}

async function finish() {
  step('Finishing touches');
  const s1 = S1();
  // Keep the headline player's balance near the designed ~12k after the casino and market swings.
  const bal = async () => (await game(s1, 'economy/balance', { uuid: P.Alex_Miner.uuid, username: MAIN })).balance;
  const b = await bal();
  if (b < 11000 || b > 13500) {
    const delta = round2(P.Alex_Miner.bal - b);
    await game(s1, 'economy/adjust', { uuid: P.Alex_Miner.uuid, username: MAIN, delta, operation_id: op(), description: delta > 0 ? 'Weekly guild dividend' : 'Market district tax' });
  }
  log(`${MAIN} balance: ${await bal()}`);
  // The panel drops game servers whose heartbeat is stale, so one last sync and the tokens for --heartbeat.
  for (const sid of Object.keys(state.servers)) await sync(Number(sid));
  mkdirSync(DATA, { recursive: true });
  writeFileSync(join(DATA, 'demo.json'), JSON.stringify({
    base: BASE, servers: Object.values(state.servers).map(({ id, key, name, token, online }) => ({ id, key, name, token, online })),
    players: PLAYERS.map((p) => ({ name: p.n, uuid: p.uuid })), instances: state.instances, guilds: state.guilds,
  }, null, 2));
}

async function summary() {
  step('Summary');
  const t = state.admin;
  const stats = await api('GET', '/api/admin/stats', { token: t });
  log(`accounts: ${stats.users}, online now: ${stats.live?.online_now}, servers: ${stats.live?.servers?.length}, instances: ${stats.instances}`);
  const alex = tok(MAIN);
  const [g, q, f] = await Promise.all([api('GET', '/api/v1/guilds/my', { token: alex }), api('GET', '/api/v1/quests/my', { token: alex }), api('GET', '/api/v1/friends', { token: alex })]);
  log(`${MAIN}: guild ${g?.tag}, ${f.length} friends/requests, ${q.length} quests`);
}

// ---------------------------------------------------------------------------------------------------------------------
// heartbeat: keeps the demo servers looking online while the panel runs
// ---------------------------------------------------------------------------------------------------------------------

async function heartbeat() {
  const d = JSON.parse(readFileSync(join(DATA, 'demo.json'), 'utf8'));
  const uuid = Object.fromEntries(d.players.map((p) => [p.name, p.uuid]));
  console.log(`heartbeat for ${d.servers.length} servers on ${d.base}`);
  for (;;) {
    for (const s of d.servers) {
      await quiet(() => api('POST', '/api/server/v1/sync', { token: s.token, body: { batch_id: randomUUID(), tps: round2(rnd(19.2, 20)), online: s.online.map((n) => ({ uuid: uuid[n], name: n })), stats: [], events: [] } }));
    }
    await sleep(30_000);
  }
}

// ---------------------------------------------------------------------------------------------------------------------

async function main() {
  if (process.argv.includes('--heartbeat')) return heartbeat();
  console.log(`Seeding demo data into ${BASE}`);
  const login = await api('POST', '/api/v1/auth/login', { body: ADMIN });
  state.admin = login.token;
  await setupBranding();
  await setupInstances();
  await setupUsers();
  await setupServers();
  await seedStats();
  await seedQuests();
  await seedEconomy();
  await seedGuilds();
  await seedSocial();
  await seedMarket();
  await seedCasino();
  await seedLevels(); // last: quest claims, achievements and the like all award XP on the way
  await finish();
  await summary();
  console.log(`\nDone${warnings ? ` with ${warnings} warning(s)` : ''}.`);
}

main().catch((e) => {
  console.error(`\nSEED FAILED: ${e.message}`);
  process.exit(1);
});
