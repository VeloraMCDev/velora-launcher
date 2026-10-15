# Activity reporting preference enforcement

October 15, 2026. Source follow-up discovered through the bounded synthetic native
run, not another complete master-plan checkpoint. Ledger remains **62/76 (81.58%)**.

## Defect and implementation

The real native fixture persisted `send_stats: false`, but the host still emitted
`activity report failed` warnings. Production `telemetry::capture` previously
ignored the existing setting. Its activity requests include the captured username,
event kind and instance ID, so treating the preference as a cosmetic setting was
incorrect. The loopback fixture's request log alone cannot identify every
`/unmatched` route and does not prove external delivery or exhaustively monitor
all network activity.

`AppState.settings` now shares the existing settings lock through `Arc`; JSON,
defaults, filenames, persistence transaction and command shapes are unchanged.
`Recorder` retains that same settings handle alongside its original account and
Panel snapshot. Disabled statistics prevent capture before reading credentials
or accounts. Retained recorders also check the live preference before admitting
background work and again before building/sending a request. A later account
switch still cannot reattribute an earlier event.

Locks are released before awaiting network I/O. This is a pre-send admission
policy, not cancellation of requests already admitted/in flight. A concurrent
opt-out after the send check may race with the already admitted request; no
retroactive deletion or absolute post-toggle delivery guarantee is claimed.
The preference applies to optional activity reporting, not required auth,
manifest, social or updater networking. Its existing default remains true.

## Regression scope

Five direct host tests use private disposable stores and loopback only:

- Disabled statistics reject capture and skip the public send/background paths.
- An already captured recorder observes a persisted opt-out and makes no request.
- A restarted state loads the persisted false preference and skips reporting.
- A failed settings write preserves the prior enabled preference rather than
  falsely treating a rejected save as consent revocation.
- Enabled reporting preserves the POST route and exact JSON fields, omits the
  bearer header for an offline account, and retains its original username after
  the active account switches to a different synthetic account.

The enabled-response fixture has bounded accept/read deadlines; privacy cases
check their nonblocking loopback listener for an absence of requests. Tests do
not open the real credential store. Initial focused run: **3 passed**, strengthened
to **5 passed**. Final default host: **143 passed**; feature-enabled host:
**150 passed** (overlapping suites, not 293 distinct outcomes); default non-test
check: pass.

The Launcher Settings category now exposes the existing preference through the
shared named/associated-help switch and normal settings-save path. Disclosure
names the username/instance information, distinguishes required server/gameplay
networking, and does not promise immediate toggle-time revocation. Settings saves
are debounced/queued: enforcement follows successful native persistence, and
already-admitted or in-flight requests may still send. Failed saves preserve the
saved preference. The first disclosure omitted those timing qualifications and
received an independent HOLD; it was corrected, not waived. SSR fixtures cover both
checked states and the help association; source binding uses `saveSettings`.
This is not a new native interaction/persistence acceptance run.

Node 24.21.0: focused accessibility **10 passed**; complete frontend **102 passed**;
Svelte check: zero errors/warnings; runes/build: pass. Boundary/identity,
local-link, whitespace and scoped telemetry rustfmt checks pass. Tracked secret
scan: 1,535 files, six reviewed matches, zero unreviewed. The separate nonignored
final untracked snapshot, including the review: 182 files, zero findings.
Independent review is in `telemetry-consent-review.md`; source tests are not a
repeat of the GUI run with the patched binary.

No dependency/lockfile, installed identity, credential key/service, signing,
deployment or release changes. Source remains uncommitted. Actual billed USD
is unavailable. Temporary logs retain the existing October 14 directory name;
the new follow-up itself occurred October 15.
