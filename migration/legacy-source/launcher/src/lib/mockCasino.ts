// An in-memory casino for the launcher running in a plain browser (dev and screenshots). Mirrors the panel's API and maths.
const seg = (label: string, value: number, weight: number, color: string) => ({ label, value, weight, color });
const sym = (id: string, label: string, weight: number, pay: number, color: string) => ({ id, label, weight, pay, color });

const config = {
  enabled: true, max_payout: 1_000_000, daily_loss_limit: 0, feed_min_win: 500,
  slots: { enabled: true, min_bet: 10, max_bet: 5000, pair_pay: 0.6, symbols: [sym('cherry', 'Cherry', 30, 7, '#ef4444'), sym('lemon', 'Lemon', 26, 10, '#facc15'), sym('bell', 'Bell', 20, 17, '#f59e0b'), sym('clover', 'Clover', 12, 44, '#22c55e'), sym('gem', 'Gem', 8, 87, '#38bdf8'), sym('seven', 'Lucky 7', 4, 218, '#a855f7')] },
  wheel: { enabled: true, min_bet: 10, max_bet: 5000, segments: [seg('0x', 0, 44, '#475569'), seg('0.5x', 0.5, 22, '#64748b'), seg('1x', 1, 14, '#0ea5e9'), seg('1.5x', 1.5, 12, '#22c55e'), seg('2x', 2, 9, '#eab308'), seg('3x', 3, 5, '#f97316'), seg('5x', 5, 2.6, '#ef4444'), seg('10x', 10, 1, '#a855f7'), seg('50x', 50, 0.1, '#ec4899')] },
  daily: { enabled: true, spins_per_day: 1, segments: [seg('$25', 25, 40, '#64748b'), seg('$50', 50, 30, '#0ea5e9'), seg('$100', 100, 18, '#22c55e'), seg('$250', 250, 8, '#eab308'), seg('$500', 500, 3, '#f97316'), seg('$2,500', 2500, 1, '#ec4899')] },
  plinko: { enabled: true, min_bet: 10, max_bet: 5000, min_rows: 8, max_rows: 16, rtp: 0.96, risks: ['low', 'medium', 'high'] },
  mines: { enabled: true, min_bet: 10, max_bet: 5000, size: 5, min_mines: 1, max_mines: 24, house_edge: 0.04, max_multiplier: 5000 },
  blackjack: { enabled: true, min_bet: 10, max_bet: 5000, blackjack_pay: 1.5, dealer_hits_soft_17: false },
  crash: { enabled: true, min_bet: 10, max_bet: 5000, house_edge: 0.04, max_multiplier: 1000 },
  dice: { enabled: true, min_bet: 10, max_bet: 5000, house_edge: 0.03, min_chance: 1, max_chance: 95 },
  coinflip: { enabled: true, min_bet: 10, max_bet: 5000, payout: 1.96 },
  double: { enabled: true, win_chance: 0.49, max_streak: 5, offer_minutes: 5 },
  chaos: { enabled: true, surge_chance: 0.04, curse_chance: 0.06 },
  bounties: { enabled: true }, betting: { enabled: true },
};

const binom = (n: number, k: number) => { k = Math.min(k, n - k); let r = 1; for (let i = 0; i < k; i++) r = (r * (n - i)) / (i + 1); return r; };
const plinkoTable = (rows: number, risk: string, rtp: number) => {
  const [p, floor] = risk === 'low' ? [2, 0.5] : risk === 'high' ? [5, 0] : [3.2, 0.25];
  const odds = Array.from({ length: rows + 1 }, (_, k) => binom(rows, k) / 2 ** rows);
  const shape = Array.from({ length: rows + 1 }, (_, k) => (Math.abs(2 * k - rows) / rows) ** p);
  const w = odds.reduce((a, o, i) => a + o * shape[i], 0);
  const scale = (rtp - floor) / w;
  return shape.map((s) => Math.round((floor + scale * s) * 100) / 100);
};
const minesMult = (cells: number, m: number, safe: number) => { if (!safe) return 1; let x = 0.96; for (let i = 0; i < safe; i++) x *= (cells - i) / (cells - m - i); return Math.max(1, Math.round(Math.min(5000, x) * 100) / 100); };
const pickW = (ws: number[]) => { let r = Math.random() * ws.reduce((a, b) => a + b, 0); for (let i = 0; i < ws.length; i++) if ((r -= ws[i]) < 0) return i; return ws.length - 1; };

const ME = { uuid: 'mock-user-1', name: 'Alex', admin: false };
const people = [{ uuid: 'p-steve', name: 'Steve', online: true }, { uuid: 'p-mia', name: 'Mia', online: true }, { uuid: 'p-notch', name: 'Notch', online: false }, { uuid: 'p-herobrine', name: 'Herobrine', online: false }];
let balance = 1840.5;
let freeLeft = 1;
let mines: any = null;
let crash: any = null;
let bj: any = null;
let offer: any = null;
let nextId = 10;
const rounds: any[] = [];
const bounties: any[] = [
  { uuid: 'p-steve', name: 'Steve', total: 4500, count: 3, by: 'Notch, Mia', me: false },
  { uuid: 'p-herobrine', name: 'Herobrine', total: 1200, count: 1, by: null, me: false },
  { uuid: 'p-mia', name: 'Mia', total: 360, count: 1, by: 'Steve', me: false },
];
const myBounties: any[] = [];
const markets: any[] = [
  { id: 1, subject_uuid: 'p-steve', subject: 'Steve', creator: 'Mia', metric: 'player_kills', metric_label: 'player kills', threshold: 5, progress: 3, locks_at: iso(40 * 60e3), ends_at: iso(45 * 60e3), status: 'open', outcome: null, yes_pool: 420, no_pool: 880, bettors: 7, odds_yes: 3.0, odds_no: 1.4, mine: [], mine_creator: false },
  { id: 2, subject_uuid: 'p-notch', subject: 'Notch', creator: 'Alex', metric: 'deaths', metric_label: 'deaths', threshold: 2, progress: 0, locks_at: iso(5 * 3600e3), ends_at: iso(5.1 * 3600e3), status: 'open', outcome: null, yes_pool: 150, no_pool: 150, bettors: 2, odds_yes: 1.9, odds_no: 1.9, mine: [{ side: 'no', stake: 50, payout: null }], mine_creator: true },
];
const done: any[] = [
  { id: 0, subject_uuid: 'p-mia', subject: 'Mia', creator: 'Steve', metric: 'mob_kills', metric_label: 'mob kills', threshold: 100, progress: 131, locks_at: iso(-7200e3), ends_at: iso(-3600e3), status: 'settled', outcome: 'yes', yes_pool: 600, no_pool: 400, bettors: 5, odds_yes: 1.6, odds_no: 2.4, mine: [{ side: 'yes', stake: 100, payout: 158 }], mine_creator: false },
];
function iso(ms: number) { return new Date(Date.now() + ms).toISOString(); }
const err = (m: string) => { throw m; };

function state() {
  const tables: any = {};
  for (const r of config.plinko.risks) { tables[r] = {}; for (let n = config.plinko.min_rows; n <= config.plinko.max_rows; n++) tables[r][n] = plinkoTable(n, r, config.plinko.rtp); }
  return {
    enabled: true, server: { id: 1, name: 'Survival SMP' }, me: ME, balance, config, plinko_tables: tables,
    rtp: { slots: 0.953, slots_hit: 0.55, wheel: 0.948, plinko: 0.96, mines: 0.96, blackjack: 0.99, crash: 0.96, dice: 0.97, coinflip: 0.98, double: 0.98, chaos: 1 },
    crash: crashView(), blackjack: bjView(), double: offerView(),
    free: { per_day: 1, used: 1 - freeLeft, left: freeLeft, resets_at: iso(6.5 * 3600e3) }, mines: mines && minesView(mines),
    lost_today: 0, bounty_on_me: 990,
    feed: [{ name: 'Mia', game: 'plinko', bet: 200, payout: 4400, at: iso(-9 * 60e3) }, { name: 'Steve', game: 'slots', bet: 100, payout: 8700, at: iso(-52 * 60e3) }, { name: 'Notch', game: 'wheel', bet: 500, payout: 5000, at: iso(-3 * 3600e3) }],
  };
}
function minesView(g: any) {
  const cells = g.size * g.size, safe = g.revealed.length;
  const v: any = { id: 1, bet: g.bet, size: g.size, mines: g.mines, revealed: g.revealed, status: g.status, multiplier: minesMult(cells, g.mines, safe), next_multiplier: minesMult(cells, g.mines, safe + 1) };
  v.cashout = Math.round(g.bet * v.multiplier * 100) / 100;
  if (g.status !== 'active') v.layout = g.layout;
  return v;
}
function play(game: string, bet: number, mult: number, result: any) {
  if (bet > balance) err('You can\'t afford that.');
  // Chaos: a win can be boosted by a lucky surge or halved by a curse.
  if (mult > 0) {
    const r = Math.random();
    if (r < 0.04) { const x = [1.5, 2, 3][pickW([60, 30, 10])]; result = { ...result, twist: { kind: 'surge', x, table: mult } }; mult = Math.round(mult * x * 100) / 100; }
    else if (r < 0.1) { result = { ...result, twist: { kind: 'curse', x: 0.5, table: mult } }; mult = Math.round(mult * 50) / 100; }
  }
  const payout = Math.round(bet * mult * 100) / 100;
  balance = Math.round((balance - bet + payout) * 100) / 100;
  rounds.unshift({ game, bet, payout, at: iso(0) });
  return { game, bet, multiplier: mult, payout, profit: Math.round((payout - bet) * 100) / 100, balance, result, double: payout > bet ? makeOffer(payout, 0) : null };
}
function makeOffer(stake: number, streak: number) {
  offer = streak >= config.double.max_streak ? null : { id: nextId++, stake, streak };
  return offerView();
}
function offerView() {
  return offer && { id: offer.id, stake: offer.stake, streak: offer.streak, max_streak: config.double.max_streak, win_chance: config.double.win_chance, payout: Math.round(offer.stake * 200) / 100, expires_at: iso(5 * 60e3) };
}
const CRASH_RATE = 0.12;
function crashView(final = false, payout = 0) {
  if (!crash) return null;
  const elapsed = Date.now() - crash.started;
  const v: any = { id: 1, bet: crash.bet, status: crash.status, auto: crash.auto, elapsed_ms: elapsed, multiplier: Math.min(1000, Math.exp((CRASH_RATE * elapsed) / 1000)), rate: CRASH_RATE };
  if (crash.status !== 'active' || final) { v.crash_point = crash.point; v.payout = payout; }
  return v;
}
// Settle a Crash round that has crashed or reached its auto cash-out.
function crashTick() {
  if (!crash || crash.status !== 'active') return null;
  const m = Math.exp((CRASH_RATE * (Date.now() - crash.started)) / 1000);
  if (crash.auto && crash.auto <= m && crash.auto < crash.point) return crashEnd(Math.round(crash.bet * crash.auto * 100) / 100);
  if (m >= crash.point) return crashEnd(0);
  return null;
}
function crashEnd(payout: number) {
  crash.status = payout > 0 ? 'cashed' : 'busted';
  balance = Math.round((balance + payout) * 100) / 100;
  rounds.unshift({ game: 'crash', bet: crash.bet, payout, at: iso(0) });
  return { game: crashView(true, payout), balance, payout, double: payout > crash.bet ? makeOffer(payout, 0) : null };
}
const card = () => Math.floor(Math.random() * 52);
const bjTotal = (h: number[]) => { let t = 0, aces = 0; for (const c of h) { const r = (c % 13) + 1; t += r === 1 ? 11 : Math.min(r, 10); if (r === 1) aces++; } while (t > 21 && aces) { t -= 10; aces--; } return { t, soft: aces > 0 }; };
const bjNat = (h: number[]) => h.length === 2 && bjTotal(h).t === 21;
function bjView(payout = 0) {
  if (!bj) return null;
  const done = bj.status === 'done', shown = done ? bj.dealer : bj.dealer.slice(0, 1), p = bjTotal(bj.player);
  const v: any = { id: 1, bet: bj.bet, status: bj.status, player: bj.player, player_total: p.t, soft: p.soft, dealer: shown, dealer_total: bjTotal(shown).t, hidden: done ? 0 : 1, doubled: bj.doubled, can_double: !done && bj.player.length === 2 && !bj.doubled };
  if (done) {
    const d = bjTotal(bj.dealer).t;
    v.payout = payout;
    v.outcome = p.t > 21 ? 'bust' : bjNat(bj.player) && !bj.doubled && payout > bj.bet ? 'blackjack' : payout > bj.bet ? 'win' : payout === bj.bet ? 'push' : 'lose';
    void d;
  }
  return v;
}
function bjFinish() {
  const p = bjTotal(bj.player).t;
  if (p <= 21 && !bjNat(bj.player) && !bjNat(bj.dealer)) while (bjTotal(bj.dealer).t < 17) bj.dealer.push(card());
  const d = bjTotal(bj.dealer).t;
  const mult = p > 21 ? 0 : bjNat(bj.player) && !bj.doubled ? (bjNat(bj.dealer) ? 1 : 2.5) : bjNat(bj.dealer) || (d <= 21 && d > p) ? 0 : d > 21 || p > d ? 2 : 1;
  const payout = Math.round(bj.bet * mult * 100) / 100;
  bj.status = 'done';
  balance = Math.round((balance + payout) * 100) / 100;
  rounds.unshift({ game: 'blackjack', bet: bj.bet, payout, at: iso(0) });
  return { game: bjView(payout), balance, payout, double: payout > bj.bet ? makeOffer(payout, 0) : null };
}

export function casinoGet(path: string): any {
  if (path === '') return state();
  if (path === '/crash/status') {
    if (!crash || crash.status !== 'active') return { game: null };
    return crashTick() ?? { game: crashView() };
  }
  if (path === '/history') return { rounds: rounds.slice(0, 40), wagered: rounds.reduce((a, r) => a + r.bet, 0), won: rounds.reduce((a, r) => a + r.payout, 0), rounds_played: rounds.length };
  if (path === '/bounties') return { enabled: true, board: bounties, mine: myBounties, recent: [{ target: 'Herobrine', killer: 'Mia', total: 900, at: iso(-25 * 60e3) }],
    rules: { min: 100, max: 1_000_000, tax_percent: 10, expire_days: 14, max_active: 5, allow_anonymous: true, allow_cancel: true } };
  if (path === '/markets') return { enabled: true, open: markets, done, can_create: true, my_open: 1,
    rules: { metrics: [{ id: 'player_kills', label: 'player kills' }, { id: 'deaths', label: 'deaths' }, { id: 'mob_kills', label: 'mob kills' }, { id: 'blocks_broken', label: 'blocks broken' }], windows: [30, 60, 360, 1440], lock_minutes: 5, min_stake: 10, max_stake: 100000, rake_percent: 5, max_open: 3, max_threshold: 10000, creators: 'players' } };
  if (path.startsWith('/players')) { const q = decodeURIComponent(path.split('q=')[1] ?? '').toLowerCase(); return { players: people.filter((p) => p.name.toLowerCase().includes(q)) }; }
  err('unknown casino request');
}

export function casinoPost(path: string, body: any): any {
  if (path === '/slots') {
    const ws = config.slots.symbols.map((s) => s.weight);
    const reels = [pickW(ws), pickW(ws), pickW(ws)];
    const triple = reels[0] === reels[1] && reels[1] === reels[2];
    const pair = !triple && (reels[0] === reels[1] || reels[1] === reels[2] || reels[0] === reels[2]);
    return play('slots', body.bet, triple ? config.slots.symbols[reels[0]].pay : pair ? 0.6 : 0, { reels, pair });
  }
  if (path === '/wheel') { const i = pickW(config.wheel.segments.map((s) => s.weight)); return play('wheel', body.bet, config.wheel.segments[i].value, { segment: i }); }
  if (path === '/plinko') {
    const t = plinkoTable(body.rows, body.risk, 0.96);
    const p = Array.from({ length: body.rows }, () => (Math.random() < 0.5 ? 0 : 1));
    const slot = p.reduce((a: number, b: number) => a + b, 0);
    return play('plinko', body.bet, t[slot], { rows: body.rows, risk: body.risk, path: p, slot });
  }
  if (path === '/dice') {
    const chance = Math.round(body.chance * 100) / 100, over = body.mode === 'over';
    if (!(chance >= 1 && chance <= 95)) err('Pick a win chance between 1% and 95%.');
    const roll = Math.floor(Math.random() * 10000) / 100, win = over ? roll >= 100 - chance : roll < chance;
    return play('dice', body.bet, win ? Math.floor((100 * 0.97 / chance) * 10000) / 10000 : 0, { roll, chance, over, win, target: over ? 100 - chance : chance });
  }
  if (path === '/coinflip') {
    const landed = Math.random() < 0.5 ? 'heads' : 'tails';
    return play('coinflip', body.bet, landed === body.side ? 1.96 : 0, { call: body.side, landed, win: landed === body.side });
  }
  if (path === '/double') {
    if (!offer || offer.id !== body.id) err('That offer has expired. Win another round to get a new one.');
    const { stake, streak } = offer; offer = null;
    if (stake > balance) err('You can\'t afford that.');
    const won = Math.random() < config.double.win_chance, payout = won ? Math.round(stake * 200) / 100 : 0;
    balance = Math.round((balance - stake + payout) * 100) / 100;
    rounds.unshift({ game: 'double', bet: stake, payout, at: iso(0) });
    return { won, stake, payout, profit: payout - stake, streak: streak + 1, balance, double: won ? makeOffer(payout, streak + 1) : null };
  }
  if (path === '/crash/start') {
    const g = crashTick(); void g;
    if (crash?.status === 'active') err('Finish your current Crash round first.');
    if (body.bet > balance) err('You can\'t afford that.');
    balance = Math.round((balance - body.bet) * 100) / 100;
    crash = { bet: body.bet, auto: body.auto ?? null, started: Date.now(), status: 'active', point: Math.min(1000, Math.max(1, Math.floor((0.96 / (1 - Math.random())) * 100) / 100)) };
    return { game: crashView(), balance };
  }
  if (path === '/crash/cashout') {
    if (crash?.status !== 'active') err('You have no Crash round running.');
    const settled = crashTick(); if (settled) return settled;
    return crashEnd(Math.round(crash.bet * Math.min(1000, Math.exp((CRASH_RATE * (Date.now() - crash.started)) / 1000)) * 100) / 100);
  }
  if (path === '/blackjack/start') {
    if (bj?.status === 'active') err('Finish your current Blackjack hand first.');
    if (body.bet > balance) err('You can\'t afford that.');
    balance = Math.round((balance - body.bet) * 100) / 100;
    bj = { bet: body.bet, player: [card(), card()], dealer: [card(), card()], doubled: false, status: 'active' };
    return bjNat(bj.player) || bjNat(bj.dealer) ? bjFinish() : { game: bjView(), balance, payout: 0, double: null };
  }
  if (path.startsWith('/blackjack/')) {
    if (bj?.status !== 'active') err('You have no Blackjack hand running.');
    const mv = path.split('/')[2];
    if (mv === 'hit') { bj.player.push(card()); if (bjTotal(bj.player).t >= 21) return bjFinish(); }
    else if (mv === 'stand') return bjFinish();
    else if (mv === 'double') {
      if (bj.player.length !== 2) err('You can only double on your first two cards.');
      balance = Math.round((balance - bj.bet) * 100) / 100; bj.bet *= 2; bj.doubled = true; bj.player.push(card()); return bjFinish();
    }
    return { game: bjView(), balance, payout: 0, double: null };
  }
  if (path === '/daily') {
    if (freeLeft <= 0) err('You\'ve used today\'s free spins. They reset at midnight UTC.');
    freeLeft--;
    const i = pickW(config.daily.segments.map((s) => s.weight));
    const payout = config.daily.segments[i].value;
    balance += payout;
    return { segment: i, payout, balance, left: freeLeft, resets_at: iso(6.5 * 3600e3) };
  }
  if (path === '/mines/start') {
    if (mines?.status === 'active') err('Finish your current Mines game first.');
    if (body.bet > balance) err('You can\'t afford that.');
    balance -= body.bet;
    const cells = 25, layout: number[] = [];
    while (layout.length < body.mines) { const t = Math.floor(Math.random() * cells); if (!layout.includes(t)) layout.push(t); }
    mines = { bet: body.bet, size: 5, mines: body.mines, layout, revealed: [], status: 'active' };
    return { game: minesView(mines), balance };
  }
  if (path === '/mines/reveal') {
    if (mines?.status !== 'active') err('You have no Mines game running.');
    if (mines.layout.includes(body.tile)) { mines.status = 'lost'; rounds.unshift({ game: 'mines', bet: mines.bet, payout: 0, at: iso(0) }); return { game: { ...minesView(mines), hit: body.tile }, balance, payout: 0 }; }
    mines.revealed.push(body.tile);
    if (mines.revealed.length === 25 - mines.mines) return cash();
    return { game: minesView(mines), balance };
  }
  if (path === '/mines/cashout') { if (mines?.status !== 'active') err('You have no Mines game running.'); if (!mines.revealed.length) err('Turn over at least one tile before cashing out.'); return cash(); }
  if (path === '/bounties') {
    const p = people.find((x) => x.name.toLowerCase() === String(body.target).toLowerCase()) ?? err('That player hasn\'t played on this server.');
    if (body.amount > balance) err('You can\'t afford that.');
    balance -= body.amount;
    const reward = Math.round(body.amount * 0.9 * 100) / 100;
    myBounties.unshift({ id: nextId++, target: p.name, paid: body.amount, reward, at: iso(0), expires_at: iso(14 * 86400e3) });
    const row = bounties.find((b) => b.uuid === p.uuid);
    if (row) { row.total += reward; row.count++; } else bounties.push({ uuid: p.uuid, name: p.name, total: reward, count: 1, by: body.anonymous ? null : 'Alex', me: false });
    bounties.sort((a, b) => b.total - a.total);
    return { ok: true, balance, reward };
  }
  const cancelB = path.match(/^\/bounties\/(\d+)\/cancel$/);
  if (cancelB) { const i = myBounties.findIndex((b) => b.id === Number(cancelB[1])); const [b] = myBounties.splice(i, 1); balance += b.reward; return { ok: true, refunded: b.reward, balance }; }
  if (path === '/markets') {
    const p = people.find((x) => x.name.toLowerCase() === String(body.subject).toLowerCase()) ?? err('That player hasn\'t played on this server.');
    markets.unshift({ id: nextId++, subject_uuid: p.uuid, subject: p.name, creator: 'Alex', metric: body.metric, metric_label: String(body.metric).replace('_', ' '), threshold: body.threshold, progress: 0, locks_at: iso((body.window_minutes - 5) * 60e3), ends_at: iso(body.window_minutes * 60e3), status: 'open', outcome: null, yes_pool: 0, no_pool: 0, bettors: 0, odds_yes: 0, odds_no: 0, mine: [], mine_creator: true });
    return { ok: true, id: nextId };
  }
  const bet = path.match(/^\/markets\/(\d+)\/bet$/);
  if (bet) {
    const m = markets.find((x) => x.id === Number(bet[1])) ?? err('That bet doesn\'t exist.');
    if (body.stake > balance) err('You can\'t afford that.');
    if (m.mine[0] && m.mine[0].side !== body.side) err('You\'ve already bet the other way on this.');
    balance -= body.stake; m[body.side + '_pool'] += body.stake; m.bettors++;
    m.mine = [{ side: body.side, stake: (m.mine[0]?.stake ?? 0) + body.stake, payout: null }];
    const t = m.yes_pool + m.no_pool;
    m.odds_yes = m.yes_pool ? Math.round((t / m.yes_pool) * 95) / 100 : 0; m.odds_no = m.no_pool ? Math.round((t / m.no_pool) * 95) / 100 : 0;
    return { ok: true, balance };
  }
  const cancelM = path.match(/^\/markets\/(\d+)\/cancel$/);
  if (cancelM) { const i = markets.findIndex((x) => x.id === Number(cancelM[1])); markets.splice(i, 1); return { ok: true }; }
  err('unknown casino request');
}
function cash() {
  const v = minesView(mines);
  balance = Math.round((balance + v.cashout) * 100) / 100;
  mines.status = 'cashed';
  rounds.unshift({ game: 'mines', bet: mines.bet, payout: v.cashout, at: iso(0) });
  return { game: minesView(mines), balance, payout: v.cashout };
}
