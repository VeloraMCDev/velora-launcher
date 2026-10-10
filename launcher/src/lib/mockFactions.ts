// Synthetic faction market for browser previews; lives only in memory.
const view = { bank_cents: 400000, outposts: 0, outpost_limit: 3, claims_per_tier: 8, members_per_tier: 2,
  tracks: {
    claims: { tiers: 0, max: 10, next_price_cents: 50000 as number | null },
    members: { tiers: 0, max: 6, next_price_cents: 75000 as number | null },
    outposts: { tiers: 0, max: 3, next_price_cents: 100000 as number | null },
    vault: { tiers: 0, max: 3, next_price_cents: 150000 as number | null },
  } };
export function upgrades(body?: { track: keyof typeof view.tracks; expected_tiers: number; expected_price_cents: number }) {
  if (!body) return structuredClone(view);
  const terms = view.tracks[body.track];
  if (!terms || terms.tiers !== body.expected_tiers || terms.next_price_cents == null) throw 'Upgrade changed; reload';
  if (terms.next_price_cents !== body.expected_price_cents) throw 'Price changed; review again';
  if (terms.next_price_cents > view.bank_cents) throw 'Insufficient faction funds';
  view.bank_cents -= terms.next_price_cents; terms.tiers++;
  if (terms.tiers >= terms.max) terms.next_price_cents = null;
  return { ok: true };
}
