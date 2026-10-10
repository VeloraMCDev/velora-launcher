// Synthetic, in-memory preview data. Never reads Minecraft inventories or player stores.
let balance = 5000;
type Stack = { slot: number; item: string; count: number; name: string; enchanted?: boolean };
const vaults = [
  { key: 'player:synthetic', owner: 'player', number: 1, label: 'Vault 1', rows: 7, unlocked: true, price_cents: null as number | null, revision: 1, in_game: false, contents: [
    { slot: 0, item: 'minecraft:diamond', count: 24, name: 'Diamond' },
    { slot: 10, item: 'minecraft:oak_log', count: 64, name: 'Oak Log' },
    { slot: 35, item: 'minecraft:enchanted_book', count: 1, name: 'Enchanted Book', enchanted: true },
  ] },
  { key: 'player:synthetic', owner: 'player', number: 2, label: 'Vault 2', rows: 7, unlocked: false, price_cents: 250000 as number | null, revision: 0, in_game: false, contents: [] as Stack[] },
];
const receipts = new Map<string, { number: number; balance: number }>();
export function vaultGet() { return structuredClone({ vaults, incoming: [{ id: 1, item_name: 'Emerald', amount: 8 }], balance }); }
export function vaultPost(action: string, body: any) {
  if (action === 'buy') {
    const previous = receipts.get(body.operation_id);
    if (previous) { if (previous.number !== body.number) throw 'Purchase identity reused'; return previous; }
    const vault = vaults.find(v => v.number === body.number && v.owner === 'player');
    if (!vault || vault.unlocked) throw 'Vault cannot be bought';
    if (body.expected_price_cents !== vault.price_cents) throw 'Vault price changed';
    if (balance * 100 < vault.price_cents!) throw 'Insufficient funds';
    balance -= vault.price_cents! / 100; vault.unlocked = true; vault.price_cents = null;
    if (vault.number < 54 && !vaults.some(v => !v.unlocked)) vaults.push({ key: 'player:synthetic', owner: 'player', number: vault.number + 1, label: `Vault ${vault.number + 1}`, rows: 7, unlocked: false, price_cents: 250000, revision: 0, in_game: false, contents: [] });
    const result = { number: vault.number, balance }; receipts.set(body.operation_id, result); return result;
  }
  if (action !== 'move') throw 'Unknown vault action';
  const find = (p: any) => vaults.find(v => v.key === p.key && v.number === p.number);
  const from = find(body.from), to = find(body.to);
  if (!from?.unlocked || !to?.unlocked || from.in_game || to.in_game) throw 'Vault cannot be edited';
  if (from.revision !== body.from.revision || to.revision !== body.to.revision) throw 'Vault changed; refresh it';
  const moving = from.contents.find(s => s.slot === body.from.slot), other = to.contents.find(s => s.slot === body.to.slot);
  if (!moving) throw 'Empty slot';
  from.contents = from.contents.filter(s => s !== moving); to.contents = to.contents.filter(s => s !== other);
  if (other) { other.slot = body.from.slot; from.contents.push(other); }
  moving.slot = body.to.slot; to.contents.push(moving);
  from.revision++; if (from !== to) to.revision++;
  return { ok: true };
}
