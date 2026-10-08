import { Divider, Grid, H1, H2, H3, Row, Select, Stack, Stat, Table, Text, TextInput, useCanvasState } from 'cursor/canvas';

// Companion overview for fabric-26.3-companion-review.md.
// Source supplied in the workspace: the legacy skill's managed Cursor canvas
// directory is not available for this Codex desktop workspace. Runtime preview
// is not verified. All data is inline and no external resources are loaded.
const features = [
  ['Launcher', 'Instances and delivery', 'Install, Java, loaders, imports, hashes, repair', 'Managed companion bundle; compatibility preflight', '1'],
  ['Launcher', 'Authentication and access', 'Private auth, stable UUIDs, groups, revocation', 'Onboarding and server-issued capabilities', '1'],
  ['Launcher', 'Play and network', 'Quick Play, server status, instances and invites', 'Server browser; valid cross-instance handoff', '4'],
  ['Launcher', 'Branding and news', 'Remote branding, artwork, news, landing page', 'Native theme and bounded asset manifest', '4'],
  ['Launcher', 'Settings and diagnostics', 'JVM, keybinds, per-instance settings, crash hints', 'HUD profiles, accessibility, redacted support info', '1'],
  ['Gameplay', 'Progression and stats', 'Global/server XP, ranks, stats, leaderboards', 'Character sheet and milestone track', '2'],
  ['Gameplay', 'Quests and achievements', 'Objective backend, claim route, completion totals in mod', 'Journal, pinned objectives and authenticated claims', '2'],
  ['Gameplay', 'Reward delivery', 'Bundles and queued server delivery', 'Earned/queued/delivered history', '2'],
  ['Gameplay', 'Travel', 'Homes, spawn, back, RTP, warps and approvals', 'Destination browser and authoritative cooldowns', '2'],
  ['Gameplay', 'Teleport requests', 'Request, accept, deny, timeout', 'Request popup and lifecycle events', '2'],
  ['Gameplay', 'Wallet and shop', 'Scoped balances, pay, shop, sell, backend history', 'Catalog, quotes and wallet history', '2'],
  ['Gameplay', 'Market and auctions', 'Listings, bids, cancellation, mailbox; basic mod Buy UI', 'Auction-aware browser; actual orders; exact previews', '2'],
  ['Gameplay', 'Trade', 'Paper shared inventory session', 'Harden overflow/acceptance first; native presentation', '4'],
  ['World', 'Guild membership', 'Directory, roles, requests, invites, feed, artwork', 'Full permissions-aware guild hub', '3'],
  ['World', 'Guild treasury', 'Bank, deposits, withdrawals, guild commerce', 'Scoped ledger and explicit payer selection', '3'],
  ['World', 'Claims and protection', 'Index, listeners, autoclaim, admin areas; local mod borders', 'Territory planner; ownership separate from access', '3'],
  ['World', 'Live map', 'Terrain tiles, pins, positions and visibility layers', 'Authorized full map/minimap and waypoints', '3'],
  ['Content', 'Custom items and assets', 'Vanilla-backed metadata, models and generated packs', 'Stable identity, safe preview DTO, catalog and cosmetics', '4'],
  ['Content', 'NPC shopkeepers', 'AI-disabled shop mobs and interaction fallback', 'Dialogue/role framework with server validation', '5'],
  ['Gameplay', 'Perks and storage', 'Heal, feed, flight, kits, vaults, ender chest', 'Eligibility/kit screens; normal server containers', '2'],
  ['Community', 'Chat and shared items', 'Prefixes, nicknames, formatting, snapshot inspection', 'Channel controls, profile links, native inspection', '4'],
  ['Community', 'Friends and social', 'Friends, DMs, profiles, posts, invites in launcher', 'Player-scoped friends drawer and inbox', '4'],
  ['Community', 'Notifications and connections', 'Inbox, Discord/email, tasks; selected mod toasts', 'Unified drawer; backend connections stay in panel', '4'],
  ['Platform', 'Integrations and administration', 'Permissions, Vault, placeholders, reporting, API', 'Authorized staff overlay and typed extension hooks', '5'],
];
const findings = [
  ['Compatibility', 'Client stays on 1.20.1; release workflow omits companion', 'Port and publish an exact 26.3 client artifact'],
  ['Protocol', 'Client does not check advertised wire version', 'Negotiate protocol and screen capabilities'],
  ['Trade integrity', 'Overflow ignored; acceptance survives another player’s offer edit', 'Preserve overflow and invalidate both acceptances'],
  ['Market', 'Client listing DTO lacks auction state', 'Bid-specific UI, revisions and structured results'],
  ['Feature completeness', '/orders and /transactions do not expose full in-game records', 'Add real player-scoped queries'],
  ['Claims', 'Allowed access is presented as ownership', 'Separate owner, relationship and effective permissions'],
  ['Fallback', 'Disabled/busy client can ignore NPC screen-open request', 'Acknowledge or decline; offer server GUI fallback'],
  ['Throttling', 'Hello bypasses existing request-gap rejection', 'Bound handshake retries and per-domain request rates'],
  ['Custom content', 'Base previews lose metadata; stable custom identity missing', 'Server identity plus safe render data; audit 26.3 pack'],
  ['Map', 'Environment dimensions and world keys differ', 'Explicit world identity and authorized visibility'],
];

export default function ScopenetFabricReview() {
  const [query, setQuery] = useCanvasState('scopenet-review-query', '');
  const [phase, setPhase] = useCanvasState('scopenet-review-phase', 'all');
  const filtered = features.filter(row => (phase === 'all' || row[4] === phase) && row.join(' ').toLowerCase().includes(query.toLowerCase()));
  return <Stack gap={20}>
    <H1>SCOPENET Companion · Fabric 26.3</H1>
    <Text>Source review • October 1, 2026 • Proposed work, not runtime verification</Text>
    <Text>Keep Paper authoritative. Make the companion the in-game interface to your existing backend and launcher systems.</Text>
    <Grid columns={3} gap={16}>
      <Stat value="24" label="Feature groups reviewed" />
      <Stat value="1.20.1" label="Existing companion target" />
      <Stat value="5" label="Delivery phases" />
    </Grid>
    <Divider />
    <H2>Architecture</H2>
    <Grid columns={2} gap={20}>
      <Stack gap={8}><H3>Panel → Paper → Companion</H3><Text>Durable records → authenticated actions and live inventory → native screens and rendering.</Text><Text>Small gameplay payloads; scoped asset downloads for maps and artwork. Never expose the server token.</Text></Stack>
      <Stack gap={8}><H3>Launcher → Managed instance</H3><Text>Install, version, repair and authenticate. Add companion preflight and preserve player preferences.</Text><Text>Fabric client connects to Paper; registered Fabric-only gameplay content needs compatible server support.</Text></Stack>
    </Grid>
    <Divider />
    <H2>Feature explorer</H2>
    <Row gap={12} wrap>
      <TextInput value={query} onChange={setQuery} placeholder="Search features or proposals" />
      <Select value={phase} onChange={setPhase} options={[
        {value:'all', label:'All phases'}, {value:'1', label:'1 · Platform proof'},
        {value:'2', label:'2 · Daily play loop'}, {value:'3', label:'3 · Guild and world'},
        {value:'4', label:'4 · Community and identity'}, {value:'5', label:'5 · Content presentation'},
      ]} />
    </Row>
    <Text>{filtered.length} feature groups shown. Detailed dependencies and evidence are in fabric-26.3-companion-review.md.</Text>
    <Table headers={['Area','Feature','Existing foundation','Companion proposal','Phase']} rows={filtered} striped />
    <Divider />
    <H2>Resolve before expanding the UI</H2>
    <Table headers={['Area','Observed gap','Next step']} rows={findings} />
    <H2>First playable milestone</H2>
    <Text>One native hub with quest journal, auction/mailbox browser, travel screen and guild summary, backed by a negotiated Paper bridge.</Text>
    <Text>Validate on dedicated Paper 26.3 with two Fabric clients and a vanilla client: duplicate clicks, full inventories, reconnect, panel outage, permissions, disabled-client fallback and resource-pack acceptance.</Text>
  </Stack>;
}
