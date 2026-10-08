// In-memory buy orders and contracts for the launcher running in a plain browser (dev and screenshots). Mirrors the panel's API.
const iso = (ms: number) => new Date(Date.now() + ms).toISOString();
let next = 20;
let balance = 1840.5;
const err = (m: string) => { throw m; };

const orders: any[] = [
  { id: 11, buyer_name: 'Mia', item_id: 'DIAMOND', item_name: 'Diamond', amount: 16, total: 640, status: 'open', claimer_name: null, claim_until: null, created_at: iso(-3600e3), expires_at: iso(6 * 86400e3), mine: false, claimed_by_me: false },
  { id: 12, buyer_name: 'Steve', item_id: 'COBBLESTONE', item_name: 'Cobblestone', amount: 512, total: 380, status: 'claimed', claimer_name: 'Notch', claim_until: iso(40 * 60e3), created_at: iso(-7200e3), expires_at: iso(5 * 86400e3), mine: false, claimed_by_me: false },
  { id: 13, buyer_name: 'Alex', item_id: 'IRON_INGOT', item_name: 'Iron Ingot', amount: 128, total: 900, status: 'open', claimer_name: null, claim_until: null, created_at: iso(-1800e3), expires_at: iso(6 * 86400e3), mine: true, claimed_by_me: false },
];
const history: any[] = [{ id: 9, buyer_name: 'Alex', item_id: 'OAK_LOG', item_name: 'Oak Log', amount: 256, total: 300, status: 'filled', resolved: 'filled', filler_name: 'Mia', claimer_name: null, claim_until: null, created_at: iso(-86400e3), expires_at: iso(0), mine: true, claimed_by_me: false }];
const each = (o: any) => Math.round((o.total / o.amount) * 100) / 100;

let contracts: any[] = [
  { id: 1, kind: 'kill', target: 'SKELETON', title: 'Kill 50 Skeletons', required: 50, progress: 31, reward: 410, bonus: false, status: 'active', expires_at: iso(14 * 3600e3), item_name: null },
  { id: 2, kind: 'gather', target: 'COBBLESTONE', title: 'Submit 256x Cobblestone', required: 256, progress: 64, reward: 218, bonus: false, status: 'active', expires_at: iso(20 * 3600e3), item_name: 'Cobblestone' },
  { id: 3, kind: 'kill', target: 'ENDERMAN', title: 'Kill 15 Endermen', required: 15, progress: 0, reward: 905, bonus: true, status: 'active', expires_at: iso(9 * 3600e3), item_name: null },
  { id: 4, kind: 'gather', target: 'IRON_INGOT', title: 'Submit 64x Iron Ingots', required: 64, progress: 0, reward: 520, bonus: false, status: 'active', expires_at: iso(22 * 3600e3), item_name: 'Iron Ingot' },
];
let rerolls = 3;

const ordersBoard = () => ({
  enabled: true, balance, orders: orders.map((o) => ({ ...o, each: each(o) })), history: history.map((o) => ({ ...o, each: each(o) })),
  my_open: orders.filter((o) => o.mine).length, my_claims: orders.filter((o) => o.claimed_by_me).length,
  rules: { max_open: 5, min_total: 1, max_total: 1_000_000, max_amount: 2304, expire_hours: 168, claim_minutes: 60, max_claims: 3, fee_percent: 0 },
});
const contractsBoard = () => ({
  enabled: true, balance, contracts, recent: [{ id: 0, kind: 'kill', target: 'ZOMBIE', title: 'Kill 40 Zombies', required: 40, progress: 40, reward: 290, bonus: false, status: 'completed', expires_at: iso(0), item_name: null }],
  stats: { done_today: 1, daily_limit: 12, limit_reached: false, rerolls_left: rerolls, completed: 14, earned: 5120, board_size: 4, expire_hours: 24 },
});

export function boardGet(path: string): any {
  if (path === '/orders') return ordersBoard();
  if (path === '/contracts') return contractsBoard();
  return err('unknown board request');
}

export function boardPost(path: string, body: any): any {
  if (path === '/orders') {
    if (body.total > balance) err("You can't afford that.");
    balance = Math.round((balance - body.total) * 100) / 100;
    const id = next++;
    orders.unshift({ id, buyer_name: 'Alex', item_id: String(body.item_id).toUpperCase(), item_name: String(body.item_id).toLowerCase().split('_').map((w: string) => w[0]?.toUpperCase() + w.slice(1)).join(' '), amount: body.amount, total: body.total, status: 'open', claimer_name: null, claim_until: null, created_at: iso(0), expires_at: iso(7 * 86400e3), mine: true, claimed_by_me: false });
    return { ok: true, id, balance };
  }
  const m = path.match(/^\/orders\/(\d+)\/(cancel|claim|release)$/);
  if (m) {
    const i = orders.findIndex((o) => o.id === Number(m[1]));
    if (i < 0) err('There is no such buy order.');
    const o = orders[i];
    if (m[2] === 'cancel') { orders.splice(i, 1); balance += o.total; return { ok: true, balance }; }
    if (m[2] === 'claim') { o.status = 'claimed'; o.claimer_name = 'Alex'; o.claimed_by_me = true; o.claim_until = iso(60 * 60e3); return { ok: true }; }
    o.status = 'open'; o.claimer_name = null; o.claimed_by_me = false; o.claim_until = null; return { ok: true };
  }
  const a = path.match(/^\/contracts\/(\d+)\/abandon$/);
  if (a) {
    if (rerolls <= 0) err("You've swapped as many contracts as you can today.");
    rerolls--;
    contracts = contracts.filter((c) => c.id !== Number(a[1]));
    contracts.push({ id: next++, kind: 'kill', target: 'ZOMBIE', title: 'Kill 40 Zombies', required: 40, progress: 0, reward: 300, bonus: false, status: 'active', expires_at: iso(24 * 3600e3), item_name: null });
    return contractsBoard();
  }
  return err('unknown board request');
}
