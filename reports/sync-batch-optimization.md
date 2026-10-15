# Batched replacement-intent optimization

October 13, 2026. Uncommitted source based on
`4f639266a515c3cb05769b055a4772c4dcb89cb8`. No commit, branch, PR, release,
deployment, installed-store operation or additional infrastructure.

## Implementation and durability scope

The measured cold-sync cost motivated batching replacement intents, not removing
durability operations. `Publication::plan_moves` validates distinct, nonoverlapping
case-folded source/destination paths, absent destinations and source fingerprints
before recording moves. Missing parents retain individually journaled creation
intents and directory flushes. The entire independent replacement list is then
persisted once in the existing bounded version-1 journal before any replacement
rename. Each rename still checks its source/destination evidence and flushes both
parents. Original-file backups, quarantine and resource-pack moves remain
single-move batches. State is still published last; cleanup authorization and
fingerprint/subset checks are unchanged.

Existing rollback understands each pending move: an unchanged occupied source
with an absent destination is skipped; an absent source with a matching
destination is reversed. Pending records are removed in reverse order before
earlier original-file moves. Both occupied, changed or missing evidence fails
preserved. No sealed-plan format, applied-prefix inference, write-behind journal
or relaxed directory durability was introduced. Additional source fingerprint
rechecks are deliberate and have a cost.

## Regression evidence

The child-process fixture now exits after batch planning with zero applied moves,
after one or all three moves, after one manual reverse move, and after final state
publication. Restart restores prior bytes/state, removes planned empty parents
and preserves synthetic saves for precommit cases; the committed case retains
all three verified files and matching successor ownership. Changed pending staged
bytes and foreign occupied destinations retain backup/evidence for review.
Duplicate/case-aliased/ancestor destinations and duplicate sources are rejected
before journal moves or parent creation. Existing non-batch cases remain active.

Lead focused run: **39 passed, zero failed, one ignored**, before the subsequent
test-only successor-ownership and distinct-source assertion refinements.
Independent final source review and stable-suite results belong to
`sync-batch-review.md`; earlier `sync-recovery-review.md` accepts the predecessor,
not this changed snapshot.

Final stable source: lead locked host/core/utility/shared sweep **75 + 131 + 10 +
19 = 235** passes, separate Panel library **72** passes, combined **307 Rust**,
zero failures. Three core explicit timing profiles and seven preexisting online
tests remain ignored by default. Launcher **98 Node** tests, Svelte check with
zero errors/warnings, runes and production build pass. Independent review ran
the pre-refinement sync suite (39/0/1 ignored) and final recovery suite
(20/0/0); these reruns are not added to lead totals. Review identifies nonblocking
coverage gaps for actual rollback-method interruption, execution-boundary
mutation and batch persist failure; they remain documented, not claimed tested.

Boundary, identity, all 16 source workflow checks, publication inventory,
tracked Gitleaks (six reviewed/zero unreviewed matches), separate nonignored
addition scan, local Markdown links, scoped rustfmt and whitespace checks pass.
These checks are not a blanket security audit. Fresh origin fetch reports zero
divergent commits. No fresh native Panel binary/restore was run for this
sync-only slice; earlier restore evidence remains attributable to that source.

## Comparable descriptive measurements

Same explicit fixture: 64 files × 16,384 bytes (1 MiB), loopback HTTP,
concurrency four, five samples, debug test build, temporary logical instances,
uncontrolled OS cache/background load. Timing logs are outside the repository in
`/tmp/velora-revitalization-final/`.

| Median milliseconds | Final pre-batch epoch-fixed source | Batched replacement intents |
| --- | ---: | ---: |
| Cold logical instance | 1206.136 | 694.028 |
| Verified unchanged warm | 9.738 | 12.082 |
| Deep unchanged verification | 9.523 | 12.430 |
| One-file repair | 99.949 | 107.871 |

`sync-final-profile.log` is the pre-batch record; `sync-batch-profile.log` is the
new run, one passing explicit timing fixture. Cold samples in microseconds:
735814, 677425, 699087, 694028, 653805. Warm: 12082, 12495, 10892, 11102,
12941. Deep: 12568, 12551, 12430, 11314, 11999. Repair: 105523, 105931,
107871, 108616, 108070. Ordinary test runs and timing reruns are not additive
acceptance counts.

Cold median is about 42% below the directly preceding synchronized source.
That is a descriptive synthetic comparison, not a significance, physical-cold,
native-startup or overall speedup claim. Historical pre-journal cold/repair
medians were 229.703/20.579 ms: substantial synchronization overhead still
remains. Warm/deep/repair medians are higher in this uncontrolled new run; do not
hide those figures or interpret one small run as a proven regression threshold.

## Remaining gates

No supported-OS/power-loss, native Java/game/loader, keystore, installed-data,
cross-store recovery, final blanket security or release acceptance. Existing
cooperative-lock/owner-controlled-journal assumptions remain. No new engineering
checkpoint is inferred from this optimization alone; the fixed ledger stays
59/76 (77.63%). At least ten additional checkpoints are needed to reach 90%.
Actual billed USD is unavailable; the original budget limits remain unchanged.
