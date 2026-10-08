# ADR 0001: launcher admission and platform observations

Date: 2026-10-05. Status: implemented library/storage boundary; service cutover deferred.
Applies to migration decisions D3/D8/D9 and the tested compatibility host.

## Context

Legacy launcher records support both server admission and platform activity.
Rename currently commits identity/game-session changes, launcher revocation and
activity audit in one SQLite transaction. Launch recording instead commits the
session, prunes old rows, then records telemetry in separate statements. Replacing
either arrangement with an unexamined remote callback would change failure behavior.

## Decision implemented

Authentication owns admission verification, session retention/revocation and the
owned `launcher_sessions` schema/projection. Panel owns launch/activity observations
and their report projections. Verified identity and caller time/proxy policy are
supplied through compatibility adapters. Anonymous legacy records remain a distinct
admission bucket; absence of an account does not fall back to another account's IP.

Version-one owned schema SQL remains unchanged. Additive version two creates session
rows and indices after validating intact stores. Import preserves IDs, nullable
fields, IP bytes, timestamps and sequence high-water marks. Original mixed activity
history stays in the protected platform checkpoint. It is not public source content.

An earlier authority-only version-one import omitted session history. Upgrading its
schema cannot recover those records: cutover requires re-importing the original
closed legacy checkpoint into a new owned destination. Missing session tables are
refused. Immutable asset validation checks existing supported stores without DDL.

The host preserves launch partial commits and transactional rename audit rollback.
Panel's neutral library receives supplied pools and metadata; credential guards,
private gameplay and migrations remain outside that public library. Trusted IP
resolution preserves the existing socket-peer/chain policy and its error precedence.

## Evidence

Authentication session schema/import: 77 independent tests and both CI runs at
`0a9f0fb` (37314936986, 37314928213). Network boundary: 80 independent tests and both
CI runs at `b6e92bb` (37315959487, 37315953947). Source runs 37313872649, 37315322254
and 37316545961 passed all seven jobs, covering session/transaction, owned-schema
and trusted-proxy adoption. Panel reporting: three independent tests, strict clippy
and both CI runs at `fca92d9` (37317824265, 37317820349); host adoption passed 84 units
and twelve identity/server/progression integrations. See the [current progress report](https://github.com/VeloraMCDev/docs/blob/scopedd/velora-migration/migration/CURRENT_PROGRESS.md) for
the latest full source checkpoint and extraction provenance.

## Deferred service boundary

This does not settle or implement durable audit delivery/projection, authenticated
session-recording RPC, presence freshness, separate-store report/deletion coordination
or gateway/cutover/rollback acceptance. The current transaction callback remains
local until an owned durable event/projection design preserves identity rollback,
historical observation IDs, retry/idempotence and unavailable-store behavior.

No cross-service shared SQLite transaction, no-op audit/presence adapter, duplicated
credential authority or cached admission grant is authorized by this boundary.
No service deployment, production import, publication or visibility change occurred.
