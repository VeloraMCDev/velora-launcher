> Historical owner documentation from `VeloraMCDev/panel` at `a5ebb0e81c653a22095893dc71e7e400e5b0e6cc`. Current segment instructions are in the monorepo READMEs.

# Instance metadata boundary

`velora-panel-instances` supplies the original instance/file records and pool-only
metadata operations. It queries `instances` and `instance_files` in the explicitly
supplied platform pool; it opens no database, attaches no foreign store, creates no
production schema and reads no identity/credential or private gameplay tables.

## Caller responsibilities

The host supplies live authenticated identity/groups and enforces visibility and
permissions before publishing manifests or changing instances. Listing metadata
includes disabled/unknown-visibility records, as the original admin inventory did.
The library is not an authorization guard. Experience configuration remains opaque
JSON; private defaults, presets and projections remain with the host.

`get_instance` distinguishes a missing record (`Ok(None)`) from SQL/decode failures.
The compatibility host maps absence to its existing `instance not found` error.
`bump_revision` takes the caller's timestamp and preserves the original missing-row
UPDATE behavior. It does not advance clean epochs or rewrite other metadata.

`unique_slug` takes the caller's reserved-storage directory. Retired directories
and files reserve names even when their database row has gone; the source host
supplies its existing `data_dir/experiences` namespace. Naming retains the original
ASCII normalization, 40-character base, fallback and numeric suffixes. This helper
is not an atomic allocation lock; existing creation constraints/coordination remain
the caller's responsibility.

## Compatibility and validation

Query strings, bound parameters, sort order, nullable/unknown fields, aggregate
casts, missing-file rules and file ordering retain original semantics. Malformed
opaque configuration is returned intact; the host still controls its projection.
No data, storage namespace, route, credential writer or schema is migrated here.

```sh
cargo +1.98.1 test -p velora-panel-instances --locked
cargo +1.98.1 clippy -p velora-panel-instances --all-targets --locked -- -D warnings
```

Synthetic tests exercise metadata/file isolation, exact response fields, disabled
rows, query parameter binding, revision timestamps, retired-name collisions,
database restart and unavailable/incompatible pools. Complete instance
creation/import/installation/publishing and the standalone control-plane service
remain migration work. See [provenance](https://github.com/VeloraMCDev/panel/blob/a5ebb0e81c653a22095893dc71e7e400e5b0e6cc/INSTANCES_PROVENANCE.json).

## Registry and distribution writes

The mutations module owns the original create/update/delete, clean-epoch, icon,
upload/file-delete and transactional pack-replacement SQL. Inputs are already
validated and serialized by the caller. Creation retains empty media strings;
updates clear them. Deletion atomically removes platform server links and the
instance row, then the host retires private storage and removes distribution files.
Pack replacement keeps hand-uploaded files and rolls back all database changes
on any failure. Its supplied clock is evaluated after file writes, at the original
metadata-update boundary. Garbage collection remains after commit in the host.
The upload missing-file DELETE and upsert retain their original separate-statement
behavior; this extraction does not invent atomicity or alter wildcard matching.
