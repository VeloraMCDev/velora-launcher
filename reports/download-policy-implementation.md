# Bounded download policy and cancellation cleanup

October 12, 2026. Root utility/core only; the independent pack utility snapshot
remains unchanged. No persisted download identity or `Download` field shape changes.

Downloads with positive size retain that exact size ceiling. Zero/omitted sizes
retain unspecified-length semantics but now have an 8 GiB per-attempt ceiling,
whether or not a digest is provided. `download_one_with_limit` permits a caller
to impose a smaller explicit ceiling without changing the ordinary API. Declared
size above that ceiling is rejected before networking. Advertised unknown-size
bodies are checked before temporary creation and actual chunk bytes are always
bounded before writing, including chunked/no-digest bodies.

The shared production client follows at most 10 redirects, permits HTTP(S)/CDN
hops, rejects embedded URL credentials/non-HTTP schemes, and refuses HTTP downgrade
after any HTTPS hop. Initial/final artifact URLs are also checked by download
helpers. Deliberately caller-supplied clients retain their own redirect chain
policy; final checking cannot establish safety of hidden intermediate hops.
Local/private HTTP is still permitted for configured Panel/self-hosted artifacts.
This is not a general SSRF/private-network prohibition.

Each partial now owns a standard file handle and exclusively created path through
a drop guard. Chunk writes/flush/publication/cleanup have no detached filesystem
worker: cancellation can occur at network awaits, then closes the handle and
removes the owned partial before the aborted task join completes. Explicit errors
retain cleanup context; aborted-future cleanup failures can only be logged.
Publication is synchronous after verification and cannot be interrupted halfway
by a future cancellation await.

Correctness tradeoff: local chunk writes/flush/rename/cleanup are synchronous.
They may delay an async runtime worker on slow/storage faults; no latency or
throughput equivalence is claimed. Moving them to independent background tasks
without owned shutdown/join would reintroduce the cancellation race. Performance
profiling and a bounded acknowledged writer design remain follow-ups if needed.

Tests cover ordinary abort cleanup, unowned sibling preservation, independent
attempts, no-digest advertised/chunked ceilings, pre-request rejection, redirect
hop/loop behavior and pure downgrade/credential/scheme policy decisions. The last
ignored core cancellation contract is enabled. Exact final execution and source
review are in `fifty-percent-validation.md` and the independent policy review.
Native Windows/storage faults, hostile path replacement, concurrent destination
writers, crashes/power-loss and broader process/background quiescence remain open.
