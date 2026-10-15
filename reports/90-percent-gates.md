# Remaining route to 90%

## 85% request and design follow-up — October 15, 2026

`85-percent-acceptance-audit.md` maps exact remaining criteria and resource gaps.
85% requires three full gates, not more partial fixes. New contrast/motion code
and repeated browser evidence are in `design-gate-followup.md`; manual composite,
key-flow/system-motion and native/OS checks remain. Ledger still **62/76**.

## Destination-binding follow-up — October 15, 2026

The source-confirmed prospective-Panel credential-binding defect and remaining
generic token-helper callers now use destination-bound lookup/current snapshots.
See `panel-context-implementation.md`: eleven new regressions, final host 161 /
feature-host 168 (overlapping), frontend 102, non-test check pass. Originless
legacy accounts intentionally fail closed without stored-ID/token migration;
installed-upgrade/offline-cache compatibility remains an explicit gate. This
security fix does not replace complete native/release acceptance. Ledger 62/76.

## Activity snapshot source follow-up — October 15, 2026

Activity capture now prevents the reviewed mixed destination/token/username
snapshot and requires a validated matching explicit account origin for its
bearer. Seven direct source/loopback regressions pass; final host suites are
150 default / 157 feature-enabled, overlapping. See
`telemetry-snapshot-implementation.md`. This does not migrate every authenticated
caller, revoke already captured tokens, or close a native/release checkpoint.
The fixed ledger remains 62/76; seven complete checkpoints are still needed.

## Current-host evidence follow-up — October 15, 2026

The reviewed guarded harness enabled real macOS/Tauri IPC observations, now
documented separately in `native-current-host-observations.md`. Controlled
failure/retry, explicitly unconfirmed cancellation, narrow modal focus and
busy-close observations do not close full native/OS checkpoints; exit status
remains unknown. The run exposed optional activity reporting ignoring the
persisted preference. The production fix and Settings switch have direct host,
loopback and rendered-state tests in `telemetry-consent-implementation.md`.
This privacy fix is source-verified, not a patched-binary GUI replay. The ledger
is still 62/76, requiring seven more complete checkpoints for 90.79%.

## Latest bounded source follow-up — October 14, 2026

Transaction returned-I/O/pre-key-root tests and a process-local manifest denial
fence are implemented and independently reviewed. Final host tests: 138; check:
pass. See `account-transaction-io-review.md` and `manifest-denial-review.md`.
These close narrower source gaps, not complete native/OS/power-loss/offline-lease
gates. The unchanged denominator still needs seven additional full checkpoints
to reach 69/76 (90.79%); current verified total remains 62/76 (81.58%).

## Current authority update

On October 13, 2026 the user explicitly overrode the model spending cap:
“Ignore the cap, just go.” Budget-cap stop language below is historical; actual
USD remains unknown and no infrastructure purchase/deployment is authorized.
Shutdown admission/draining implementation is now in `shutdown-implementation.md`.
Its synthetic source checks do not yet close the native shutdown checkpoint.

October 14, 2026. The unchanged ledger in `completion-checklist.md` has
**62 verified checkpoints out of 76 (81.58%)**. The user's 80% milestone was met
on October 13 through VEL-014 checkpoints 1–2. The separate final measurement
audit now accepts checkpoint 3 in `install-final-measurements-review.md`.
Reaching at least 90% still requires **69/76 (90.79%)**, so seven additional
checkpoints must be verified. There are 14 open checkpoints; no weighting or
definition is relaxed to meet the target.

| Item | Open checkpoints | Acceptance still required |
| --- | ---: | --- |
| VEL-001 | 1 | Native/runtime performance baseline matrix |
| VEL-002 | 1 | Native shutdown, background I/O and process quiescence |
| VEL-004 | 1 | Complete durable restart and supported-OS sync matrix |
| VEL-005 | 1 | Native keystore and installed upgrade acceptance |
| VEL-010 | 1 | Independent loader/native compatibility acceptance |
| VEL-012 | 1 | Complete cache recovery, integrity and security acceptance |
| VEL-013 | 1 | Runtime startup and no-regression acceptance |
| VEL-014 | 1 | Native launch/loader no-regression acceptance |
| VEL-015 | 1 | Complete defined keyboard, contrast and reduced-motion review |
| VEL-016 | 1 | Native/E2E install, repair, retry and cancellation usability |
| VEL-018 | 1 | Native feature acceptance |
| VEL-019 | 3 | Final independent security/data/compatibility review, release-OS/native upgrade matrix and rollback/restore readiness |

## Implementation still needed

Complete account/credential restart-fault and native durability acceptance, full native shutdown
and helper/process acceptance, cache failure/security coverage and production
integration, durable offline-authority policy, and cold/repair overhead
remain implementation work. Current-host sync process fixtures do not replace
these features or their final acceptance. New fault hooks close specific test
requests, not another whole ledger checkpoint.

The production account/credential pair now has an interruption journal and
startup recovery before accounts load; real encrypted and lower-level process
fixtures pass. See `account-transaction-implementation.md`. Independent review,
all I/O/power-loss/native keystore/installed-upgrade and supported-OS acceptance
remain distinct; this source implementation does not close the native checkpoint.

Manifest-cache origin/account binding, compatible-shape atomic publication and
known-denial invalidation now have production-caller synthetic tests; see
`manifest-cache-authority.md`. Durable revocation on write failure, an explicit
offline authorization lease and complete cache integration/security acceptance
are not established. The fixed ledger remains 62/76.

Shutdown source admission/draining, transitive inline cache I/O, accepted rename
commit completion and owned installer partial cleanup are now implemented and
covered by synthetic regressions. See `shutdown-implementation.md` and its
separate review; they do not close the native shutdown checkpoint.

## Practical gates

Native/supported-OS, approved synthetic installed-data/keystore/game fixtures
and restore targets need an explicitly bounded acceptance run. This checkout's
Darwin source/tests do not certify Windows/Linux or signed installed releases.
No real player/operator store, production credentials, signing, release or
deployment is authorized by the percentage target alone.

Disk measured about 2.1 GiB free during the shutdown follow-up (APFS 99%). Large
new target builds should not be launched blindly. Actual provider spending
remains unavailable. The user superseded the original model-spending ceiling;
earlier ceiling/threshold language is historical, not a current continuation
blocker. No paid infrastructure or deployment is authorized. These are not claims that
everything remaining is externally blocked or that source work is complete.
