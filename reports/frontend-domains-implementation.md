# Frontend domain boundaries and async ownership

October 12, 2026. Bounded frontend worker delivery; local dirty-tree source,
synthetic data only, no dependency/lockfile changes, commits, native application,
live stores or deployment. The lead owns the fixed 76-checkpoint ledger and the
50% goal. This report does not independently advance that ledger.

## Implemented production boundaries

- `launcher/src/lib/context-domain.ts`: shared serialized host-context mutation
  queue, context/panel/selection generations and manifest refresh ownership.
  A rejected mutation does not poison the queue. Reads wait for queued context
  mutations; old refresh results/errors cannot replace newer account/panel data
  or set offline state. Instance selections made during refresh are retained if
  the instance remains present, otherwise existing selection fallback applies.
- `launcher/src/lib/account-domain.ts`: account inventory upsert/removal and
  addition generations. Store account selection uses latest-request ownership;
  older selection completions cannot reset the newer selection. Successful
  removal still updates inventory when a newer selection is queued, but an old
  removal cannot erase a same-ID account replacement added afterward.
- `launcher/src/lib/settings-domain.ts`: ordered save execution, last accepted
  snapshot and conditional rollback. The store retains the existing 300ms
  debounce and live theme application. It exposes `settingsPersistence` with
  idle/pending/saved/error status and error text, without changing `app` shape.
  Bootstrap starts idle, not a claim of verified disk persistence. Success is
  reported only after save IPC accepts the requested snapshot and the visible
  settings still match it. A newer request keeps pending status. Failure keeps
  an error toast/status and rolls back only fields still equal to the attempted
  snapshot; later unscheduled nested edits are preserved. A failed older save
  never rolls back a newer scheduled edit. Instance selection/context is
  reconciled if its unchanged failed setting is rolled back.
- `launcher/src/lib/store.svelte.ts`: Svelte reactive facade and IPC adapters
  compose those helpers and the unchanged preparation controller. Existing
  exports, command names/payloads, app fields, mock contract, launch/cancel/run
  identities and existing preparation scenarios remain intact. Update replies
  additionally check context generation, not merely panel URL equality.

Panel/account mutations are serialized at the frontend IPC submission boundary.
Connect publishes its manifest and bootstrap-derived URL together only for the
latest panel request. An overlapping newer account selection does not discard a
successful panel URL; the potentially old-account manifest is withheld and a
new owned refresh runs after the context mutation queue settles. The current
revision below adds authoritative reconciliation on failed/uncertain mutations.

## Remaining ownership inventory

| State/domain | Owner and interface | Scope of this delivery |
| --- | --- | --- |
| Preparation, operation events and run console | `preparation-events.ts`, store event/launch adapters | Existing controller and 19 scenarios revalidated, semantics unchanged |
| Accounts/auth inventory and active account | `account-domain.ts`, store account functions | Tested inventory/request ownership; credentials are not stored in these helpers |
| Login form, device-code flow and registration | `components/SignIn.svelte`; `pages/Login.svelte` hosts it | Preexisting component-owned state; returns accepted accounts through `accountAdded`; not rewritten |
| Panel manifest, selected instance, experience context | `context-domain.ts`, store refresh/connect/select functions | Tested generation/selection ownership and serialized context mutations |
| Settings editor and persistence | `pages/Settings.svelte` edits shared settings; `settings-domain.ts` handles save ownership | Tested pending/error/conditional rollback; no editor/page rewrite |
| Profile view/edit/feed | `components/PlayerProfileModal.svelte`; store holds only requested profile UUID and skin invalidation | Preexisting component-local profile state; no transfer of profile data into the central store |
| Friends, conversations, invitations, member search | `pages/Social.svelte` | Preexisting page-local arrays/forms/selection/loading state; store holds only `chatWith` navigation handoff |
| Social sidebar friends/polling/message previews | `components/SocialSidebar.svelte` | Preexisting account/panel effect teardown and preview request generation; inventory inspected, not rewritten or claimed newly tested |
| Guilds, quests, statistics, collections, market, casino, commands | Corresponding `pages/*.svelte` | Existing page ownership retained; no unrelated page rewrite |
| Theme/media/formatting and experience integration | `theme.ts`, `tauri.ts`, shared experience library | Existing adapters retained; store calls them only after owned state application |
| Navigation, modal flags, toasts, update notice | Store facade; pages consume existing fields | Remain presentation shell state, not a new backend/persistence domain |

This inventory and the extracted tested account/context/settings helpers provide
bounded source evidence for VEL-008 checkpoint 3. Existing profile/social ownership
is explicitly preexisting, not a new implementation claim. **Checkpoint 3 is
withheld pending independent re-review of the stale-context fix below.** The lead
must decide credit after that review. Checkpoint 4 remains open:
render/navigation/reconnection and native acceptance were not performed.

## Evidence

The new `launcher/tests/store-domain-state.test.mjs` compiles the actual production
Svelte store with the client runtime and imports the actual production domain
implementations into its VM dependency modules. Only transport, timers, theme and
experience integration are fixtures; the domains are not silent stubs. Its 18
scenarios cover overlapping refresh success/failure, selections during refresh,
serial panel connection, account queue failure/recovery, latest account selection,
removal/re-add races, connect/account overlap, offline cache retention, settings
debounce, ordered saves, current/stale failure, nested later edits, last-success
rollback and selection reconciliation. Existing store-preparation tests received
only real-domain import/link entries; their scenarios/assertions are unchanged.

Final commands use the repository's Node 24 executable:

```sh
cd launcher
npm test
npm run check
npm run check:runes
npm run build
```

Logs: `/tmp/velora-frontend-domains-full-suite.log`,
`/tmp/velora-frontend-domains-check.log`, `/tmp/velora-frontend-domains-runes.log`,
`/tmp/velora-frontend-domains-build.log`. Scoped `git diff --check` is also run.
Initial delivery run: **41 tests passed, zero failures/skips** (18 new domain-store
scenarios, 19 unchanged preparation/event scenarios, four concurrent worker
command-surface scenarios). Svelte check: **zero errors/warnings**. Rune check
and production build passed. Main build chunk: **813.48 kB raw / 224.33 kB gzip**;
the existing >500 kB chunk warning remains, so this is not a bundle-performance
improvement claim. Scoped whitespace check passed.

An intermediate whole-suite run observed an unrelated command-surface registration
assertion while the native command extraction worker was editing that surface.
No native code/test assertion was changed by this worker. A later rerun passed;
whole-suite counts include the concurrent native worker's tests, not exclusively
frontend evidence. Node prints its expected experimental VM Modules warning.

## Limits and gates

- Tests validate store state and domain ownership, not DOM rendering, focus,
  accessibility, native event transport, real reconnects, keyring or installer
  upgrades. No native/durable-crash acceptance is inferred.
- `settingsPersistence` is available to consumers but this delivery does not
  add a rendered persistence badge. Debounced settings are not flushed on window
  shutdown. Save success means host IPC acceptance, not independently inspected
  disk/crash durability. Rollback baseline starts with bootstrap settings, which
  may be defaults when the host could not load its settings file.
- Settings edits must still call `saveSettings`. Equality rollback is top-level:
  when later edits change a nested object, that entire object is preserved rather
  than selectively restoring its untouched children.
- Serialization covers store connect/select/remove submissions and settings saves
  within their own save queue, not all commands submitted directly by pages or
  other processes. Login/device-code IPC itself remains owned by SignIn. Native
  account/login concurrency and context-versus-full-settings-write reconciliation
  require host/native review. Request ownership does not undo a host mutation
  already accepted or cancel network requests.
- Failed/latest and superseded-partial context mutations now have source-level
  reconciliation coverage below. Actual reconnect rendering and native transport
  acceptance remain gates. Reconciliation cannot guarantee current host context
  if bootstrap IPC itself is unavailable; the visible active identity/manifest
  are cleared with an offline/error indication instead of presenting stale data.
- Social page/profile async flows are not claimed race-free. For example,
  `Social.svelte` conversation/search replies lack the sidebar's explicit request
  generation, and profile loads need separate lifecycle/ownership review. Their
  component ownership is inventoried honestly rather than rewritten outside scope.

## Changed paths

- `launcher/src/lib/store.svelte.ts`
- `launcher/src/lib/context-domain.ts`
- `launcher/src/lib/account-domain.ts`
- `launcher/src/lib/settings-domain.ts`
- `launcher/tests/store-domain-state.test.mjs`
- `launcher/tests/store-preparation.test.mjs` (dependency import/link only)
- `reports/frontend-domains-implementation.md`

## Independent review refinement: stale host/UI context

Feynman's `frontend-domains-review.md` identified a blocking introduced regression:
an older account/panel mutation could succeed but have its UI reply suppressed,
then the latest mutation could fail, leaving the host on the older accepted
context and the UI on its original context. A panel mutation followed by a failed
bootstrap reply had the same problem. That finding was accepted; it is not
dismissed as merely an untested native gate.

Current implementation:

- `ContextDomain.mutate` retains the original mutation error, marks host context
  uncertain after every failure (including superseded failures), and requests
  reconciliation when the failing mutation still owns the context generation.
- Reconciliation enqueues an authoritative bootstrap read behind completed
  mutations in the same rejection-tolerant queue. Both before submission and
  after the read it checks context generation. A response for a generation
  superseded by a new account/panel request is never published.
- An owned bootstrap reconciles account inventory, active account and panel URL
  together. It deliberately does **not** trust bootstrap's cached manifest as
  account-authoritative: native bootstrap returns a cache, while account selection
  may not have refreshed that cache. The visible manifest/update and old experience
  context are cleared and a fresh, generation-owned manifest request follows.
- Older/in-flight manifest requests are invalidated when recovery starts and
  again when the authoritative bootstrap is applied. Normal manifest ownership
  guards protect the subsequent recovery refresh from newer context requests.
- A failed recovery read clears the unverified active identity and manifest,
  marks offline, reports recovery failure separately, and still rejects the
  initiating call with its original mutation error. Later mutations can retry
  reconciliation because uncertainty is retained until an owned read succeeds.
- A superseded failure can represent a real host-side effect. The newest
  successful account mutation/addition therefore reconciles first when context
  remains uncertain, rather than assuming its success resolved the panel URL.
  Successful owned connect already contains an authoritative bootstrap read;
  when uncertainty remains it also applies that read's accounts/active identity,
  rather than clearing uncertainty while retaining an unverified old account.
- Account-add refresh waits for context mutations to settle. A stale connect does
  not launch a competing duplicate refresh; the latest account operation owns
  that refresh or reconciliation. Existing successful-overlap assertions remain.
- Reconciliation does not replace editable settings or touch preparation/run
  state. Local settings edits and instance selection during recovery are retained.

New actual-production-store regressions cover older-account-success/latest-failure,
older-panel-success/latest-failure, accepted-panel/failed-bootstrap, uncertain
removal, unavailable recovery preserving the original error, delayed recovery
bootstrap versus a later account request, delayed recovery bootstrap versus a
later panel request, recovery manifest versus later connect, superseded partial
connect failure followed by successful account selection, and settings/instance
edits during recovery, and a successful latest connect following an uncertain
account mutation. The existing queue-recovery scenario additionally verifies
the new authoritative bootstrap before its successful manifest result; no prior
assertion was relaxed. Existing preparation test scenarios/assertions are untouched.

Refinement write scope: `launcher/src/lib/context-domain.ts`,
`launcher/src/lib/store.svelte.ts`, `launcher/tests/store-domain-state.test.mjs`,
and this report only. No native source/builds, dependency changes, live stores,
commits or deployment. The independent review file remains reviewer-owned.

Refinement commands are the same four npm commands above. Logs are
`/tmp/velora-frontend-domains-reconciliation-full-suite.log`,
`/tmp/velora-frontend-domains-reconciliation-check.log`,
`/tmp/velora-frontend-domains-reconciliation-runes.log`, and
`/tmp/velora-frontend-domains-reconciliation-build.log`.

Final refinement results: **52 tests passed, zero failures/skips** (29 actual-store
domain scenarios, 19 unchanged preparation/event scenarios, four command-surface
scenarios). **Svelte check: zero errors/warnings; rune check and production build
pass.** Main chunk: **814.45 kB raw / 224.60 kB gzip**; the existing large-chunk
warning remains. Scoped `git diff --check` passed. No native builds were run.
These are worker-run results; independent re-review has not yet accepted this
refinement, and **VEL-008 checkpoint 3 remains withheld**.
