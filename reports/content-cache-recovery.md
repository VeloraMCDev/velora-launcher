# Isolated persistent cache recovery follow-up

October 13, 2026. Working-tree source on base
`4f639266a515c3cb05769b055a4772c4dcb89cb8`, Darwin arm64, Rust 1.98.1. This
extends only the isolated cache prototype, with no live downloader call site,
default cache path, HTTP range resume, new dependency or lockfile change.

## Ownership and handoff

The worker partially delivered `content_cache.rs`, `content_cache/recovery.rs`,
the tests module and new `tests/recovery_tests.rs`, then reported an expired-auth
failure without a final report. Lead inspected the delivered source rather than
adopting worker claims. A separate reviewer ran the suite: 25 passed, two failed,
one ignored. Both failures were fixture mistakes, not rejected safety checks:
appending zero to canonical size 3 creates valid size 30; a wrongly sized hostile
hardlink fails length checking before link-count checking.

Lead corrected those fixtures, added canonical-size/mismatched-file and SHA-1
corrupt-reopen preservation regressions, and gated Unix tests with a separate
non-Unix fail-before-creation fixture. Current narrow run passes **29 tests,
zero failures, one explicit timing benchmark ignored**:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib content_cache::
```

The reviewer is separately revalidating this final snapshot in
`content-cache-recovery-review.md`. No blanket security/platform acceptance or
checklist credit follows merely from lead tests or the initial blocked review.

## Implemented behavior

Unix `create` retains new-root exclusivity; `reopen` requires the explicit
caller-owned existing root and a valid fixed format marker. Stable advisory owner
locking is retained until handle drop; root/owner inode substitution and
unexpected link counts are rejected. Non-Unix usage is explicitly unsupported
rather than silently weakening these checks. Old unmarked disposable roots are
not adopted. Reopen scans bounded canonical identities, exact sizes and actual
digest bytes, reconstructing limits including zero-length entries and scratch
headroom. SHA-1 remains separately named compatibility verification.

Process-kill fixtures cover held ownership, partial scratch, verified scratch,
linked-unsynchronized, linked-synchronized and published boundaries. Parent
reopen confirms exclusivity release, preserves a separate verified final,
discards unlinked scratch without resuming it, retains linked verified finals,
and leaves no scratch. Malformed/ambiguous/over-limit inventory, corruption and
static path/link substitution reject reopening before scratch removal.

Publication flushes file bytes and synchronizes directories around hardlink
publication and scratch deletion. This is tested process-interruption recovery,
not power-loss proof. An error after hardlink creation can leave a valid final;
there is no transactional success receipt. Caller limits are payload budgets,
not physical filesystem quotas. SHA-256 helpers may read a bounded object into
memory. Same-user hostile races remain outside exclusive-writer assumptions.

## Remaining gates

No HTTP resume, prefix validator, cross-instance production cache, eviction,
reference management or downloader integration. No native Windows/Linux,
permission/disk-full/fsync fault matrix, power-loss, hostile-race/openat guarantees
or installed-store migration acceptance. Reconcile actual billing before a larger
implementation batch. The original disposable-only reports are retained under
historical headings; no checkpoint definitions or denominator were changed.
