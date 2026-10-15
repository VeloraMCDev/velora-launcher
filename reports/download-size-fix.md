# Declared download size validation

## Scope and status

On October 12, 2026, this test-only change enabled the existing size-mismatch
contract and added focused loopback HTTP coverage in the test module of
`crates/core/src/http.rs`. This report is the only other path written by this
agent. Production implementation in `crates/platform-utils/src/http.rs` is
reserved for main and was unchanged for the pre-fix run.

## Coverage

- Existing matching-checksum size-mismatch contract is no longer ignored.
- Complete eight-byte HTTP bodies with matching content SHA-1 and declared sizes
  of nine and seven bytes must reject publication and preserve the old destination.
  HTTP Content-Length matches the actual body, isolating manifest size validation
  from transport truncation.
- Explicit declared size zero accepts an eight-byte body with matching SHA-1,
  replaces the old destination, and leaves no publication `.part` artifact.
- Missing declared size (`None`) has the same successful-publication behavior.
- Both shorter and longer complete bodies reject incorrect nonzero declared size
  without a checksum as well. Mismatch helper tests assert a size-mismatch error.
- Every size-mismatch scenario inspects the temporary directory and asserts that
  only the preserved destination remains, with no `.part` or other artifact.
- Matching nonzero declared size and SHA-1 publish successfully and leave no
  publication `.part` artifact.
- Existing checksum, truncated-response, interruption, and ignored cancellation
  cleanup tests remain unchanged. No sync source was edited.

## Pre-fix validation

Command:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib http::regression_tests
```

Actual result: exit 101; **5 passed, 3 failed, 1 ignored**, 36 filtered out.
Compilation completed in 3.20 seconds; tests completed in 5.62 seconds.

Failures were the existing `size_mismatch_must_not_publish_even_with_matching_checksum`
contract and the new shorter/longer-body mismatch tests. All three failed the old
destination preservation assertion: the downloader published the received body
despite the incorrect nonzero declared size and correct content SHA-1.
Both successful-publication compatibility tests passed, as did the existing
checksum mismatch, truncated-response, and interruption/retry tests.
The cancellation cleanup contract remained ignored.

Red-test confirmation was sent to main after this run.

## Post-fix validation

Main supplied the production patch in `crates/platform-utils/src/http.rs`:
after flush/drop, nonzero declared size differing from bytes written rejects
publication before SHA-1 verification and rename. Partial-file removal failure
returns contextual error. This agent inspected but did not edit that source.

After adding missing-size compatibility, mismatch-without-checksum coverage,
and mismatch directory inspection, the same targeted command exited **0**:
**10 passed, 0 failed, 1 ignored**, 36 filtered out. Compilation completed in
4.68 seconds; tests completed in 11.24 seconds. Both mismatch directions with
and without SHA-1 preserve the destination and leave only that destination in
the directory. `None`, `Some(0)`, and matching nonzero size publication pass.
Existing checksum mismatch, truncated-response, and interruption/retry coverage
passes; cancellation cleanup remains ignored and unchanged.

Main owns broader platform-utils, core, and launcher validation; those suites
were not duplicated here. Cleanup-failure fault injection and release/production
acceptance are not claimed by these targeted tests.

## Formatting and final handoff

Ran `rustfmt --edition 2021 --config-path "$PWD/rustfmt.toml" --check` on
temporary extracted test-module source. The initial check identified new
directory-inspection helper wrapping plus two pre-existing differences in
`local_client` and `interrupt_download`. Only the new helper wrapping was patched.
The full extracted module still exits 1 solely for those two pre-existing
differences, which were left untouched under the scoped formatting instruction.
An extraction of all new helpers and tests passes the same rustfmt check (exit 0).
Scoped `git diff --check` passes. Temporary extraction files were removed.
Tests were not rerun for the whitespace-only helper adjustment.

Main reports broader suites passing: platform-utils 6 passed; core 43 passed,
4 ignored; launcher 22 passed, 3 ignored. These are main-reported results, not
additional executions by this agent. Reviewer callsite semantics are reported
acceptable; final exact-diff review remains pending. This agent's write scope
remains the core HTTP test module and this report only.

No dependencies, lockfiles, branches, commits, PRs, deployments, or live stores
were changed or used.

## Lead final integration

The independent reviewer subsequently approved the current production guard,
README and regression contract with no source-review blocker; see
`download-size-review.md`. The worker's pending-review wording above describes
its earlier handoff, not final approval status. The only later test-source
adjustment was rustfmt whitespace in the directory-inspection helper. Main
reran core tests after that adjustment and repeated source/secret/link/diff
checks; exact final results are in `revitalization-status.md`.
