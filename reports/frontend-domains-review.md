# Independent bounded frontend-domain review

October 12, 2026. Source review only; no production/test edits, duplicate builds,
services, live stores, commits or deployments. Native host extraction and account
persistence are outside this review and assigned to another reviewer.

## Verdict

**Prior blocking stale-context finding is closed by source fixes and targeted
actual-store regressions.** VEL-008 checkpoint 3 now has bounded source-domain
ownership, fixture and independent review evidence. Checkpoint 4 remains open:
no rendering/navigation/reconnection/native acceptance was performed here.

## Closed finding: superseded success followed by latest failure

- The prior `switchAccount` implementation: starting both account
  requests increments the revision before either queued mutation completes. From
  active account A, enqueue selection B, then selection A. If B succeeds, its
  completion is suppressed at line 245. If A subsequently rejects, the function
  propagates rejection without restoring or reading authoritative context. The
  host is now B while `app.active` and the cached manifest still describe A.
  Prior sequential completion behavior would have published successful B before
  A failed. This is an introduced stale-state regression, not merely delayed UI
  refresh. The existing test covers first failure followed by later success, not
  this opposite outcome.
- The prior `connect` implementation: two queued connects had the
  same failure pattern. The first accepted panel change and bootstrap result are
  discarded at line 175 because a newer request exists. If the newer connect
  fails before changing the host, the visible panel URL/manifest remain from
  before the first connect, while the host uses the first new panel. A single
  connect whose `set_panel_url` succeeds but whose bootstrap fails also leaves
  host/UI disagreement. The implementation report acknowledges the superseded
  panel case as a remaining gate; acknowledgement does not resolve it.
- Re-review: `ContextDomain.mutate` marks failed mutation outcomes uncertain,
  invokes reconciliation only for the current context generation and preserves
  the original mutation rejection. `reconcileContext` serializes authoritative
  bootstrap behind queued mutations, checks generation before submission and
  after completion, replaces account inventory/active account/panel URL, discards
  cached context content and requests a fresh owned manifest. Recovery failure
  hides active account/manifest/update and clears experience context while
  reporting offline/error, rather than presenting stale content as verified.
  Successful subsequent account mutations reconcile when uncertainty remains;
  current successful connect can use its authoritative bootstrap. These changes
  close the specific accepted-older/failed-newer account and panel outcomes and
  the accepted-panel/failed-bootstrap outcome.
- Eleven additional actual-store scenarios cover those exact failures, stale
  recovery bootstrap/manifest replies, recovery failure, uncertain removal,
  subsequent successful mutations and preservation of instance/local settings
  edits. Generation checks prevent recovery from overwriting newer requests.
  This is source/local fixture acceptance, not native concurrency certification.

## Accepted source behavior

- `context-domain.ts` uses a rejection-tolerant serialized queue; manifest
  refresh ownership checks context generation, refresh request, panel and account
  before applying either content or offline failure. A selection made while a
  refresh is pending is retained when still present in the returned manifest.
- `account-domain.ts` preserves inventory upsert/removal semantics. Addition
  generations prevent a delayed removal reply from erasing a same-ID replacement
  in the frontend. This does not certify native remove/login concurrency.
- `settings-domain.ts` serializes full snapshot saves, records the last accepted
  snapshot, and invokes success only for the latest scheduled revision. Current
  failure conditionally rolls back top-level fields still equal to the failed
  candidate; later nested edits preserve the entire changed object. Older failure
  cannot roll back a newer scheduled edit. Store status checks snapshot equality
  before advertising saved; bootstrap is idle, not verified disk persistence.
  Selection rollback reconciles experience context when applicable. No scoped
  blocking settings rollback defect was found in the inspected code.
- Existing store exports, app fields, preparation ownership and transport names
  are retained. The actual-store VM fixtures compile the production Svelte store
  and import real domain implementations, not copied state-machine substitutes.
  Transport, timers, theme and experience adapters are synthetic.
- Profile data remains component-local in `PlayerProfileModal.svelte`; social
  page arrays/forms remain in `Social.svelte`, with navigation handoffs in the
  store. `SocialSidebar.svelte` has preexisting effect teardown and preview
  request generations. This supports the ownership inventory, not new profile
  or social concurrency coverage. Existing profile/social async limitations
  remain and are honestly described in the implementation report.

## Evidence and limits

The inspected worker log
`/tmp/velora-frontend-domains-reconciliation-full-suite.log` records
52 passing tests, no failures/skips; the reconciliation check log records zero Svelte errors
and warnings. These are worker-produced results, not reviewer-run tests or the
lead's final whole-tree validation. The original eighteen actual-store
scenarios cover successful overlap, queue recovery, inventory replacement,
debounce and rollback; eleven added scenarios cover the previously blocking
outcomes above. The earlier
19 preparation/event scenarios remain evidence for the previous milestone.

Settings and context mutations use different queues; full settings/context
write reconciliation, direct page IPC, login IPC, multiprocess saves, shutdown
flush, native keystore, crash durability and actual reconnects are not accepted
here. No platform/gameplay dependency, protocol identity or application lifecycle
change is introduced by the three frontend helpers. That scoped observation is
not an attestation of all concurrent dirty-tree changes.

## Reviewed SHA-256 snapshots

| Path | SHA-256 |
| --- | --- |
| `launcher/src/lib/account-domain.ts` | `1980eac6afcce3d9389daf662e89f2c7443abd1b321140fb03f3a507d6789a71` |
| `launcher/src/lib/context-domain.ts` | `355e6e26966eaaadea7d11a16cb6ee855806ebe34ba37740fef57053e185218b` |
| `launcher/src/lib/settings-domain.ts` | `96a5e08db9c1a0a581c39ad55be8f33aa7e76638cac34f03c0978421f5db2764` |
| `launcher/src/lib/store.svelte.ts` | `208b40d15c9ae1b2011adf5edf422ce80d4d4764db4e9c7d60bce216acf58258` |
| `launcher/tests/store-domain-state.test.mjs` | `f82f59bf0915148563aa4718f5effa491f5906c2a92bba725ea944ec11ac04de` |
| `launcher/tests/store-preparation.test.mjs` | `c9fc0d20ec53aaaeda6f0838878fb07ad48647164785077c506605781264b061` |

Any subsequent fix requires refreshed scoped review/evidence; these hashes are
source snapshots, not release identities.
