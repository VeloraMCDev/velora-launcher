# Launcher shutdown admission and draining

October 13, 2026. Uncommitted source based on
`4f639266a515c3cb05769b055a4772c4dcb89cb8`. The user explicitly superseded
the model spending cap with “Ignore the cap, just go.” Actual metered USD remains
unknown; no infrastructure/service purchase, signing, release or deployment is
authorized or performed by this change.

## Implemented lifecycle

`Shutdown` owns a mutex-protected closing/readiness state and a count of owned
work permits. Admission increments under the same lock that closes admission.
Existing work retains its permit until its actual owner drops it. Async drain
waiters register before checking the count to avoid a missed final notification.
Readiness requires closed admission and zero counted work.

The Tauri exit-request callback now prevents normal exit while draining, starts
one shutdown coordinator, closes preparation admission, aborts an active
preparation, waits for owned work and fences the mutation mutex. It then marks
readiness and requests exit with the original requested code. Repeated normal
exit requests do not spawn another coordinator. Forced termination, crashes and
Tauri's special restart path are not certified by this normal-exit flow.

Preparation workers retain their mutation lease/work ownership through task
completion or processed cancellation. The completion supervisor has a separate
permit, so final manager/event handling is included in draining. Preparation
shutdown permanently rejects replacement work, including work racing behind
shutdown's preparation lock; the existing owner slot remains until finalization.

## Ownership integration

- Texture extraction and the initial four-second hidden-window fallback thread
  retain owned permits; extraction is allowed to finish, not forcefully stopped.
  The fallback does not reveal the window after shutdown has started. Early quit
  can wait for its remaining delay.
- Background telemetry retains a permit through its existing five-second request
  timeout. New background reports are rejected after admission closes.
- Game stdout/stderr readers and the attached monitor own cancellable permits.
  Shutdown aborts those launcher tasks and waits for their permit destruction.
  This deliberately does not introduce a new kill-games-on-launcher-exit policy:
  external games retain the existing survive-launcher-exit behavior, including
  `after_launch == "close"`. This is not certification of descendant process-tree
  reaping or native pipe behavior.
- Disk-size queries move permits into their blocking closures. Cache/instance
  deletion moves both mutation/resource and work ownership into the blocking
  closure, so cancelling its awaiting IPC future does not falsely release the
  actual filesystem worker or exclusion lease. Missing directories stay a no-op.
- Settings/account updates, account credential-plus-file commits, refreshed
  credentials, manifest caching and local profile writes have admission boundaries.
  Updater installation owns a mutation lease through its exit request. Existing
  formats, paths, credential identities and commit/rollback routines remain.

The open-folder command's directory creation also has an admission permit.
The transitive platform utility JSON cache reads/writes and downloader parent
creation use inline filesystem calls rather than detached Tokio blocking jobs.
Cancellation cannot release the owning preparation permit in the middle of an
inline filesystem call. This can block an async runtime worker on a slow disk;
no performance improvement or hostile-path/power-loss guarantee is claimed.

Admitted account rename commits a successful remote response using its existing
lease, without attempting fresh admission after closing. Standalone account
commits still reject closing. Two actual loopback/store regressions prove this
boundary and exact-byte preservation on a rejected standalone commit.

Installer downloading also uses inline filesystem operations and an owned partial
file guard. Exclusive creation preserves preexisting partials; cancellation and
errors close the handle before best-effort partial removal. A verified installer
is file-synced and closed before rename. Native installer execution, signature
identities and handoff policy are unchanged. Cleanup failures remain warnings,
not proof of successful removal; parent durability and hostile filesystem races
are not certified. Slow installer writes can block a runtime worker.

Not every remote IPC request is tracked or cancelled: responses that later attempt
the covered local writes encounter closed admission. Java detection subprocesses
and any unenumerated helper are not claimed covered by this initial source
snapshot; review must inspect gaps.
No force-exit timeout is used to pretend unfinished writes are quiescent. Slow or
stuck admitted native work can delay exit indefinitely and needs native acceptance.

## Validation and gates

Lead non-test host library check passes. Stable launcher library suite:
**92 passed, zero failed**, including eight new tracker, four new preparation,
two account shutdown and three installer ownership tests. Core: **132 passed,
three explicit timing profiles ignored**; utility: **13 passed**; shared:
**19 passed**. Seven existing online tests remain ignored. The latest locked
host/core/utility/shared sweep has **256 passes**, not a complete workspace or
native acceptance run. Four IPC-surface tests pass: registered names/order, sync/async signatures,
arguments and results retain the captured contract. Logs are under
`/tmp/velora-revitalization-final/shutdown-*`. Test worker wrote only its bounded
test files and `shutdown-tests.md`; no worker test claim was adopted without lead
execution. Separate final review and source hashes belong to `shutdown-review.md`.

Fixtures use temporary synthetic files/tasks, not the actual application window,
real keychain, installed player data, Java/game accounts or a signed updater.
These checks do not close VEL-002 checkpoint 4, complete native shutdown or the
supported-OS/keystore/installer/game matrix. The fixed ledger remains **59/76
(77.63%)**, ten checkpoints short of 90%. Source implementation is progress,
but scoped synthetic tests do not manufacture native acceptance.

Fresh final origin fetch reports zero commits on either side of
`HEAD...origin/main`. Boundary/identity checks, 16 workflow policies, publication
inventory, local Markdown links, scoped rustfmt and whitespace checks pass.
Tracked Gitleaks: six previously reviewed matches, zero unreviewed; separate
nonignored-additions scan: 157 files, zero findings. No files were staged to scan.
The final formatting-only host rerun passes 92. Disk now has about 1.9 GiB free
(APFS 100%); existing build artifacts were reused and unrelated files preserved.
