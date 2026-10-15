# Downloader response and temporary-file hardening

October 12, 2026. Scope: root `velora-platform-utils` downloader and root core
HTTP regression fixtures. The independently versioned pack utility is unchanged.

Fresh responses must be HTTP 200; unsolicited partial/no-content responses are
rejected. Positive declared size is enforced before each chunk is written and
again before publication, even when digest verification succeeds. Zero/omitted
size remains unspecified and optional digest semantics remain unchanged.
Each attempt exclusively creates a unique sibling `.velora-download-<pid>-<counter>.part`
with bounded collision retries rather than sharing/truncating a common part.
Non-cancellation body/hash/size/write/rename errors close the file and remove the
owned temporary. Cleanup failure reports both cleanup and original failure.
Existing destination remains untouched until verified publication.

Controlled local HTTP and synthetic filesystem tests cover response status,
short/truncated/oversized bodies, matching and wrong digest/size, unspecified-size
compatibility, concurrent temporary isolation, rename and cleanup errors.
The oversized-body test does not assume network chunk boundaries or immediate
Tokio buffered-write visibility. Final locked core/utility suites pass; commands
and counts are in `milestone-validation.md`. Independent review is recorded in
`downloader-response-review.md`; its verdict is source/fixture scoped.

Cancellation still leaves an owned partial and remains an explicitly ignored
desired-invariant test. No universal no-digest size ceiling, redirect policy,
resume protocol, cache, retry-progress correction or hostile-directory/native
rename containment is implemented. Unique temporaries prevent ordinary attempt
collisions, not every path race. Native Windows/filesystem acceptance remains open.
