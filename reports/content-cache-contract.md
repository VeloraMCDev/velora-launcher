# VEL-012 / P1-10: isolated content cache prototype contract

## Current recovery contract — October 13, 2026

The October 12 disposable-only contract below is historical. The isolated cache
now supports explicit persistent reopening on **Unix only**; it remains unused
by the downloader and is neither HTTP resume nor production shared caching.

- `create` still requires a new, absolute caller-owned root; `reopen` requires
  that exact existing root with a valid fixed `ownership-v1` format marker.
  Unmarked historical disposable roots are not adopted or silently migrated.
- A retained standard-library file lock excludes cooperating cache processes;
  the stable owner file is never unlinked on close. Root/owner inode substitution
  and unexpected links are rejected. Non-Unix create/reopen return
  `UnsupportedPlatform` before creating/adopting a directory.
- Reopen reconstructs byte/entry bounds from canonical names, exact lengths and
  verified SHA-256/SHA-1 compatibility bytes. No unverified final is served.
  Capacity includes a surviving unlinked scratch, or counts a linked final and
  its one scratch inode once. Empty objects count toward entry limits.
- At most one canonical `staging-N` is accepted. Unlinked partial/verified scratch
  is discarded only after all committed entries validate; a verified final linked
  to scratch is retained and scratch removed. Unknown/ambiguous/corrupt inventory
  fails closed, retaining evidence. No partial prefix is reused or promoted.
- Publication synchronizes payload, links without clobber, synchronizes the root,
  removes scratch and synchronizes again. Reopen tolerates tested process kills
  at these boundaries. A post-link error may leave a valid committed final;
  successful return is not an all-or-nothing transaction receipt.
- Same-user hostile path races, power-loss/filesystem-fault injection, native
  Windows/Linux acceptance, concurrent shared-cache service, eviction/reference
  management and downloader integration remain unaccepted. Existing path helpers
  reopen files; static inode/link checks are not race-free no-follow traversal.
  No installed IDs, default cache paths, dependencies or lockfiles change.

Current implementation/test evidence and precise limitations are in
`content-cache-recovery.md` and its separate review. Earlier claims of no reopen,
lock, fsync or process-kill fixture describe only the historical snapshot below.

## Historical disposable-only contract — October 12, 2026

Date: October 12, 2026. This is an **isolated prototype, not production caching**,
not HTTP resume, and not an approved downloader integration. No completion credit
or recovery/security acceptance is assigned by this implementation.

## Inputs and ownership

- `ContentCache::create` requires an explicit absolute caller-owned root whose
  parent already exists. The root must not exist: exclusive directory creation
  prevents another cache instance from adopting it. No default path, live app
  directory, environment discovery, database, or global singleton is used.
- Ancestors must be real directories, not symlinks. Relative and parent traversal
  paths are rejected. A caller may canonicalize a trusted temporary parent before
  constructing its new child root; the cache itself does not resolve symlinks.
- The caller owns the entire root for this object's lifetime and must exclude
  other filesystem writers. On Unix the cache creates the root with mode 0700
  and scratch files with mode 0600 (subject to umask). These permissions do not
  defend against the same user, administrator, or hostile filesystem races.
- The root is disposable, never reopened or adopted. The caller removes it after
  the object is no longer used. All executable fixtures use synthetic `tempfile`
  directories; no installed data or application stores are touched.
- Namespace IDs are caller-assigned opaque integers for platform, artifact type,
  and trust domain. A caller must map them consistently and must not reuse a trust
  namespace for unrelated sources. No URL, authorization token, account ID, or
  remote response metadata is accepted or persisted by the API.

## Identity and integrity

- A key contains algorithm, normalized hex digest, exact `u64` size and namespace.
  `Key::verified_identity` validates identity syntax only; the caller must obtain
  the expected digest and size from an appropriately trusted manifest. It does
  not certify the caller's provenance or verify bytes until an operation runs.
- SHA-256 is preferred. Existing SHA-1 protocol compatibility is explicitly named
  `Sha1Compatibility`, separated in filenames, and is not upgraded silently.
  SHA-1 matching is compatibility verification, not resistance to deliberate
  cryptographic collisions. A digest is not authentication.
- Missing, malformed or wrong-length digests produce no key. `populate(None, ...)`
  declines without reading the source; `get(None, ...)` returns a miss. There is no
  URL-based key and no unverified cache path.
- Size zero means exactly empty. Short content, extra content and hash mismatch
  reject publication. At most the expected size plus one probe byte is consumed
  from a population source. That extra byte is never written to disk.
- Final names use only numeric IDs, algorithm labels, digest and size. Manifest
  paths cannot become cache paths. Case normalization avoids hex aliases.
- Every warm read, including an existing-key population attempt, copies to a
  fresh scratch file and verifies exact size and digest before returning bytes
  or `Existing`. Corrupted entries are not returned, overwritten, repaired or
  deleted automatically. The caller can discard this disposable root and start
  again; no persistent recovery procedure is provided.

## Bounds, publication and isolation

- Caller limits bound each object, total unique payload bytes including one
  in-flight scratch copy, and committed entry count. Checked arithmetic prevents
  overflow. This is a payload budget, **not a physical filesystem quota**: block
  allocation, directory entries, metadata and filesystem overhead are excluded.
- Entry count includes zero-length objects. Admission declines at capacity; no
  eviction, reference tracking or automatic deletion of committed entries occurs.
  Retrieval requires headroom equal to the object size for its verified scratch
  copy; a full cache can therefore return `Capacity`, not a hit. The committed
  source is counted once in `state.bytes`; the reservation adds only the distinct
  new scratch payload. Two committed 3-byte objects need a 9-byte budget to retrieve
  one of them; an 8-byte budget correctly rejects that operation.
- A single object's mutex serializes requests for that root, including verification
  and cancellation checks. At most one scratch payload is active. Scratch names
  use monotonic per-object sequence numbers and `create_new`, never a shared `.part`.
  Separate cache roots cannot see each other's entries; namespace separation also
  applies inside one object. Cross-instance deduplication and interprocess sharing
  from P1-10 are deliberately not implemented.
- Streaming uses a 64 KiB buffer. Existing SHA-1 helpers stream; the existing
  SHA-256 helper reads the bounded scratch object into memory. Retrieval returns
  a bounded `Vec<u8>`. Memory is O(max object size), not constant-memory retrieval;
  caller limits must fit available memory. There is no new dependency or lockfile.
- Publication uses a same-root hard link from fully verified scratch to the final
  name, then unlinks scratch. It never renames over or truncates a final name.
  A collision fails rather than replacing bytes. Both names briefly refer to the
  same inode, not two payload allocations. Hard-link support is required.
- "Immutable" means cache operations never rewrite a published entry. It is not
  an OS immutable flag: outside modification is possible and subsequent integrity
  checks reject it. Scratch files and entries are never installed into a game tree.
- Root inventory is checked before each operation. Unknown entries, missing entries,
  nonregular objects, static symlink substitutions and size changes fail closed.
  Unix hard-link aliases are also rejected. Corruption of one entry can block the
  root; availability is intentionally secondary to integrity in this prototype.

## Interruption, errors and remaining gates

- Cancellation is a caller-provided synchronous predicate, checked before work,
  between copy chunks, before the EOF probe and after digest verification. Retrieval
  also checks before returning bytes. Cancellation observed before publication
  returns `Cancelled` and removes that request's scratch file.
- Reader failure, digest/size rejection and in-process unwind also trigger scratch
  RAII cleanup. Explicit success cleanup and `Drop` first take and close the file
  handle, then unlink its path, avoiding reliance on Windows open-file deletion.
  Successful cleanup is idempotent: subsequent `Drop` does not unlink a replacement
  at the old scratch name. Previously published entries remain untouched. Unwind
  poisons the mutex and subsequent requests fail closed. Native Windows execution
  is still required; source ordering and local regressions are not native acceptance.
- Cleanup during error/unwind is best effort. Cleanup failure can leave scratch;
  the next inventory check rejects the unexpected entry. If publication's unlink
  fails, `Io` can mean a verified final entry already exists. This is not a fully
  transactional success/error receipt. No disk-full/unlink-denial injection has
  established cleanup behavior under every failure mode.
- Cancellation cannot interrupt a blocked synchronous reader, a whole-object
  SHA-256 helper call, or a filesystem operation. Predicates must not reenter this
  object. There is no asynchronous task-abort or process-kill cleanup guarantee.
- Errors expose only fixed enum categories; underlying reader/filesystem error
  messages, paths and URLs are discarded. No tracing or metadata sidecar is emitted.
  Content bytes themselves are caller-owned and are not classified for secrets.
- No HTTP ranges, validators, partial-file reuse, resume or network transport are
  implemented. After an interruption a new attempt reads and verifies the complete
  fixture from byte zero. No `fsync`, durable journal, startup reconciliation,
  crash recovery, multi-process locking or hostile path-race resistance is claimed.
- Existing helpers reopen scratch paths; metadata checks are not race-free
  `openat`/no-follow handle traversal. Windows ACL/reparse/hard-link behavior,
  Linux/native filesystem behavior, permission denial, disk full, power loss and
  independent integrity/security review remain gates. The local Unix checks are
  not universal security acceptance.
- Downloader integration is deferred until native install/upgrade and durable
  recovery gates are accepted. Source tests and a local timing fixture do not
  award VEL-012 checklist points, production readiness, or release approval.

## Plan alignment

Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md`,
`docs/DEPLOYMENT_TARGETS.md`, and the local `VELORA_REVITALIZATION_MASTER_PLAN.md`
attachment, specifically P1-10. No nested `AGENTS.md` exists under the edited core
or report directories. The plan's shared cache, eviction, reference management,
resume, real cold/warm installs and multi-instance disk comparison remain deferred;
this bounded, caller-isolated prototype exercises the digest/integrity foundation
without coupling reusable libraries to a complete application host.
