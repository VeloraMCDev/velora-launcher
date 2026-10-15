# Correlated launcher operation events

October 12, 2026. Local, uncommitted source work; no application/player stores,
release identities, dependencies or operational schemas were migrated.

## Implemented contract

`launch` and `repair_instance` return the admitted UUID string. `cancel_launch`
requires both instance and operation IDs; malformed IDs fail and valid obsolete
IDs do not cancel a newer job. The manager supplies its UUID to the spawn closure.
Progress and state payloads carry `instance_id` and `operation_id`; running/idle
game state also carries `run_id`. `game://log` now sends an envelope with instance,
operation, run and lines instead of a bare array. Native host, production store
and mock migrate together. External event/IPC consumers must migrate too.

`PreparationEvents` is a production TypeScript domain controller. It matches the
current request, instance and admitted UUID, bounds pre-admission events to 128,
preserves terminal state over progress floods and prevents completed operations
from becoming preparing again. Admission busy checks do not depend on visible UI.
Cancellation captures the request promise before awaiting admission. Rejection
restores the prior owner. Run inventory accepts only current owned running state;
observed exits are tombstoned for the store lifetime. Logs require a known run and
matching instance; bootstrapped foreground runs retain subsequent live logs even
without a preparation token. Other concurrent consoles are not reconstructed.

## Verification and limits

Final Node 24 run: 19 tests passed (9 controller, 10 actual client-compiled store),
Svelte check zero errors/warnings, runes check and production build passed.
The store tests mock the IPC/event boundary but exercise production store code;
they are not native UI, process or reconnection acceptance. Locked native-host
tests also pass; exact suite counts/commands are in `milestone-validation.md`.
Independent source review: `operation-events-review.md`, with earlier manager and
cancellation reviews retained. No remaining blocker within the bounded fix scope.

Active preparation identity recovery across UI reload is not implemented. Run
tombstones are lifetime-local and not bounded/scavenged. Shutdown, background I/O,
process spawn/termination, interprocess admission and supported native OS tests
remain open. A cancel reply does not prove that all independent writers stopped.
No performance improvement or complete Phase 1 acceptance is claimed.
