> Historical owner documentation from `VeloraMCDev/authentication` at `5162abd28300300667ea2b029fc63551cb966977`. Current segment instructions are in the monorepo READMEs.

# Launcher admission and Panel activity boundary

Authentication owns launcher IP admission, retention and revocation semantics.
Panel owns launcher event observations and aggregated activity. The extracted
`core::launcher_sessions` adapter operates on a supplied legacy pool/connection;
it does not open a database or imply that the separate service is deployable.

Compatibility is explicit:

- Launch identity comes from the verified bearer account. Raw IP/name/time values
  are retained. Insert, global strict-cutoff pruning and Panel telemetry remain
  separate writes, in that order. A later failure retains the committed launch.
- Admission selects the newest fifty row IDs within the inclusive time window,
  restricted to the requested account or the distinct legacy anonymous bucket.
  IPv4-mapped IPv6 compares as IPv4; invalid addresses never match. Missing/blank
  request IP returns false before storage access; storage errors propagate.
- Rename revocation accepts the caller's open SQLite connection, keeping identity
  changes, session cleanup and the current host audit in the same transaction.
  Audit failure must still roll all three back. Other users/anonymous rows survive.
- Panel observation IDs, source defaults, unknown event payloads and pagination
  remain in their original store. Mixed event history is not copied into this repo.

## Separate-store acceptance still required

The additive owned version-two schema and checkpoint projection preserve session
IDs, nullable user/name fields, timestamps, anonymous history and sequence high-water
marks. Intact version-one stores can be upgraded without rewriting identity records.
Earlier version-one imports omitted this history: an upgrade cannot reconstruct it.
Re-import the original closed legacy checkpoint into a new destination before
cutover. Checkpoints missing the session table are refused. Panel needs an
authenticated admission-recording interface, not a second credential authority.

The transaction adapter is an interim compatibility boundary. It cannot become a
remote callback while keeping its present rollback behavior. Durable identity
audit ownership/projection and retry/error semantics must be designed and tested
before any split-store rename cutover. No no-op audit or online-presence port is
an acceptable standalone implementation. Multi-domain deletion also needs explicit
retryable actions. No production store, migration, deployment or writer cutover
is performed by this extraction.

Synthetic tests use the actual owned schema and cover admission buckets/window/limit,
mapped IPs, global retention, foreign-key refusal/cascade, failed-prune partial commits,
caller rollback, version-one upgrade and exact checkpoint projection/high-water marks.
See [extraction provenance](https://github.com/VeloraMCDev/authentication/blob/5162abd28300300667ea2b029fc63551cb966977/LAUNCHER_SESSION_PROVENANCE.json).
