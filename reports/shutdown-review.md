# Independent bounded shutdown/state review

Date: October 13, 2026. Checkout HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`.
Reviewed the current shared working tree, not a clean HEAD release. Read root
`AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md` and
`docs/DEPLOYMENT_TARGETS.md`. No additional scoped AGENTS file was found for the
reviewed launcher files or this report. This review writes only this new report;
production/test fixes belong to the main integration task.

## Decision

The new tracker and preparation shutdown tests pass, and the explicit permit,
supervisor and asynchronous exit design is substantially sound. **Do not claim
complete launcher-write quiescence yet:** preparation's transitive Tokio filesystem
operations can outlive its cancelled worker. An additional open integration gap is
a nested account commit rejected after an already-admitted rename. The main agent
fixed the initially reported unfenced folder creation during review; its final
guard is included in this snapshot. These are source-review findings, not
reproduced native failures. No new game-killing policy is requested or implied.

## Findings for the main integrator

### S1 — High: preparation abort can release ownership before blocking filesystem work ends

Locations: `launcher/src-tauri/src/commands/launch.rs:29`,
`launcher/src-tauri/src/preparation.rs:51`,
`launcher/src-tauri/src/lib.rs:202`; transitive evidence:
`crates/core/src/loaders/fabric.rs:16`,
`crates/platform-utils/src/http.rs:44` and
`crates/platform-utils/src/http.rs:159`.

The cancellable preparation worker owns its mutation lease; the non-cancellable
supervisor owns the completion permit and joins that worker. However, a
Fabric/Quilt profile fetch reaches `get_json_cached`, whose directory creation and
cache write use `tokio::fs`. Downloads also await `tokio::fs::create_dir_all`.
Cached Tokio 1.53.2 `fs/write.rs` and `fs/mod.rs:312` implement the write using
`spawn_blocking`, with no launcher permit or resource lease inside that closure.
Started blocking work cannot be cancelled by dropping/aborting its async waiter.

Concrete schedule: cache write is dispatched and still running; exit aborts the
preparation; its mutation lease drops; supervisor observes cancelled join,
finalizes the registry, and drops its completion permit; shutdown sees zero
active work and acquires the now-free mutation mutex. It can mark ready and exit
while the cache write still runs. This also means the worker join alone is not
proof of all descendant I/O completion. A running operation can leave a partial
cache write if the process exits. This is ordinary graceful-exit ownership, not
a power-loss claim.

The new blocking-writer test proves the correct design **when a permit is moved
into the actual writer**; it does not establish that the transitive production
Tokio closures do that. Fix via cancellation-safe I/O ownership/draining or
cooperative cancellation that retains the worker lease until outstanding blocking
operations finish. Keep reusable libraries independent of the Tauri host; avoid
injecting application-specific types into platform libraries. Add a synthetic
gated production-path regression proving readiness stays false until the actual
writer completes. No production-path reproduction was run in this review.

### S2 — Resolved during review: open_folder directory creation lacked shutdown admission

Location: `launcher/src-tauri/src/commands/instances.rs:35`.

The initial reviewed version called `std::fs::create_dir_all` without shutdown
admission. The main agent then added `_work = state.shutdown.enter()` at line 36,
before path resolution and directory creation (now line 46). Independently read
the final guard and verified the supplied SHA256. The permit spans synchronous
creation and opener dispatch: new invocation after begin is denied and accepted
creation remains counted. **The shutdown gap is resolved in the final reviewed
source.** IPC signature remains unchanged. No production edit was made by this
reviewer.

The command does not take the mutation mutex, so this narrow patch does not add
exclusion against concurrent instance deletion. That is a separate resource
coordination limitation, not an outstanding shutdown-admission defect. Opening an
external application itself need not be treated as a promise to drain
remote/native activity.

### S3 — Medium: admitted account rename can lose its local commit to nested admission

Locations: `launcher/src-tauri/src/accounts.rs:153` and
`launcher/src-tauri/src/accounts.rs:130`.

`rename_panel` admits a mutation lease before awaiting the remote username change.
After successful response it calls `save_panel_account`, which obtains a fresh
shutdown permit. If exit begins during that request, the outer operation remains
counted but its nested `enter` fails. The server can have renamed the account and
issued new session data while local account/credential persistence is skipped;
the caller gets a shutting-down error rather than completing its accepted local
commit. This is not a self-drain deadlock, but an accepted-operation consistency
failure introduced by independent nested write admission.

Use an internal commit path authorized by the existing operation permit, retaining
fresh admission for standalone login/register commits. Do not globally allow new
writes after closing. Add a synthetic gated rename/commit test covering begin
between remote success and local commit; no real credentials or server are needed.
Standalone untracked login/register responses rejected before local writes are a
fail-closed boundary, not the same defect. Monitor preference persistence similarly
can be refused after closing and is explicitly logged; it does not presently
promise a mandatory final preference capture during deliberate detach.

## Sound paths and qualified observations

- `Shutdown::enter` and `begin` serialize under one mutex: admission either wins
  and retains a permit or loses and is rejected. Late `track_cancellable` aborts
  an already-accepted task. Abort is not equated with completion; permit drop is
  what decrements active ownership. `wait_idle` enables notification before
  inspecting active count, avoiding lost completion; `mark_ready` checks closing
  and zero active under the same mutex.
- Preparation start/shutdown serialize under its manager mutex. An operation
  admitted before begin can win manager registration in the interval before
  manager shutdown, but that shutdown sees and aborts it. If manager shutdown
  wins first, spawn is refused. Registry ownership remains until finished-handle
  finalization. The non-cancellable supervisor joins and finalizes before dropping
  its completion permit. This handles worker-level ownership, subject to S1.
- `MutationLease` includes both owned mutex guard and permit. Admission immediately
  before begin may still complete as accepted work; it cannot silently evade the
  drain. Instance deletion/cache removal move the whole lease into their blocking
  closures, and size scans move their permit into the actual closures. Cancelling
  the async wrapper does not free this ownership prematurely.
- Settings/account/profile write boundaries keep permits through synchronous
  persistence. Texture refresh threads and startup visibility thread capture
  permits, including before first execution. Telemetry background tasks retain
  their permits through the send and have a five-second request timeout; new
  background telemetry is skipped after closing. Neither a timeout guarantee for
  arbitrary filesystem work nor a universal remote-command drain follows.
- ExitRequested first prevents exit, closes admission, cancels preparation, then
  spawns an **untracked** drain coordinator. Waiting for zero permits and then the
  mutation mutex does not self-count; closed admission prevents new legitimate
  lease acquisition between idle and readiness. Ready makes the second exit event
  pass through. No lock is held across worker joining in the event callback.
- Attached-game readers and monitor own permits and register cancellable handles;
  late registration closes the begin race. Aborting the monitor drops the child
  handle and readers, deliberately detaching rather than waiting for Java exit.
  `crates/core/src/launch.rs:266` does not enable kill-on-drop for launched games.
  Explicit `kill_game` remains the existing kill route. Closing piped stdout/stderr
  can affect a game's logging behavior; native survival/logging acceptance remains
  a gate, not an assertion of guaranteed external-process survival.
- The final `is_closing` check before Java spawn is not atomic with spawn. Work
  admitted before begin can cross that synchronous boundary before abort is
  processed. Ownership still drains; this is not a guarantee that no external
  game can start after the begin instant, nor reason to add a new kill policy.
- `close_after` requests exit then returns; updater installation holds its lease,
  requests exit then returns. Cached Tauri 2.12.0 `AppHandle::exit` delegates to
  Wry 2.12.0 `request_exit`, which posts `Message::RequestExit` rather than waiting
  synchronously for drain. Therefore neither caller waits for its own permit.
  Close-after preparation may be cancelled by that event, with its supervisor
  still able to finalize. Tauri's failed-event-post fallback calls process exit;
  the custom drain cannot promise coverage for that fallback.

## Validation actually executed

Exactly the two requested focused Cargo commands were executed, serially, reusing
the existing root target. No additional Cargo/Node test command was run here:

```text
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib shutdown::tests
8 passed; 0 failed; 79 filtered out; test execution 0.01s.

CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib preparation::tests
18 passed; 0 failed; 69 filtered out; test execution 0.00s.
```

The requested 12 new regressions are eight tracker tests plus four appended manager
tests; the preparation filter also executes 14 preceding manager tests. These are
26 passing selected tests, not 26 new tests or a rerun of all 87 host tests. The
lead supplied the 87-host-test pass; this reviewer independently verifies only the
two outputs above. `reports/shutdown-tests.md` describes its earlier author-time
no-Cargo status; this report records the subsequent focused execution without
editing that author's report.

Both focused commands preceded the main agent's one-line `open_folder` guard.
Per the main agent's explicit instruction, they were not duplicated. Final guard
acceptance here is source inspection and hashing; the main agent owns its final
host-suite compilation. All other primary scope hashes were rechecked unchanged
after that patch. Also read `reports/shutdown-implementation.md` for the final
declared implementation limits; its broad host results remain lead-provided.

No live player stores, keychain, native app, external game, paid resource, purchase,
deployment, branch or commit was used. Available disk was about 2.0 GiB when
inspected; existing target reuse avoided a new build tree. Actual USD expenditure
remains unknown. The user's continuation instruction removes the prior model cap
as a blocker, not the native acceptance gates or proof-of-cost requirement.

## Limits and next gates

Resolve S1 before claiming all launcher-owned writes are quiescent; resolve S3
before claiming accepted account commit completion across closing. S2's shutdown
guard is reviewed as fixed, with final compilation owned by main.
Focused tests do not exercise real ExitRequested/window destruction, native
keychain I/O, installer handoff, supported-OS pipe behavior, updater release signing
or native GUI acceptance. There is no force-quit, OS power-loss, external-game
termination, or all-remote-commands guarantee. Ordinary accepted filesystem/thread
work can delay shutdown without a wall-clock deadline. Do not turn an arbitrary
timeout into permission to exit while writers still hold permits.
`detect_java(path)` remains explicitly untracked read-only subprocess work; this
review does not certify its child lifetime or advance native checkpoint 4.

The working tree already contains substantial preceding source changes. Git
diffs against HEAD help identify context, but HEAD is not an exact isolated
pre-shutdown integration snapshot. No assertion is made that every issue above
was newly introduced; S1 is a concrete gap in the new shutdown coverage of existing
transitive writers, and S2 records a direct-writer gap fixed during review.
Hashes identify the reviewed bytes and must
be refreshed/re-reviewed if main changes them.

## SHA256 of reviewed scope

| Repository file | SHA256 |
|---|---|
| launcher/src-tauri/src/shutdown.rs | `30ddfbd784adb2c99064c5fa95423d77c5581b429dc8f8d3c4db6534a50d1d2a` |
| launcher/src-tauri/src/shutdown/tests.rs | `40ddcd85643e4e7582b64d9441e234242cbb1ce60405c35681a10e1a4334a496` |
| launcher/src-tauri/src/preparation.rs | `c69dd1ad8fb45d3758b01b455207538e0076b60469ff800ffa750121ac85e436` |
| launcher/src-tauri/src/preparation/tests.rs | `f96ea3637a1591808c9240b2197b1cedcc6b782f0f75e344a7e7d92942df8330` |
| launcher/src-tauri/src/state.rs | `d8b231aeb22e0d9908fec2d1c5fd6b1a372e588a4f27d9aa462f641d7c3973f2` |
| launcher/src-tauri/src/lib.rs | `bc86334d9ec1c56a60c77e67131784287a0fb8771cab07a5b57244e2d5accb4d` |
| launcher/src-tauri/src/commands/launch.rs | `501136c9b2d3f8cb0c0f40bba8d1a29fda058ae4f0f00fb8757567b27c1448af` |
| launcher/src-tauri/src/commands/instances.rs | `42bb8845b39662fc1d178e559c04aef03f35ff27beae3a3bbd375a24cbc9583a` |
| launcher/src-tauri/src/commands/profiles.rs | `da04de57354e935baef92721afaf6e2a805595dd77cb1f57bae9d8091ba0fbf4` |
| launcher/src-tauri/src/commands/updates.rs | `a93461dbde37727df80cef8ae870db5f5c8975ad5acdab8c9b0e90d734a87957` |
| launcher/src-tauri/src/game.rs | `0d66f5f1f9cef79ee207481aa88c5c1093ea0786a67cdaf34178934099551ffe` |
| launcher/src-tauri/src/accounts.rs | `4117a3cf991c3a1627f4f58677531d9ba879d92647582e6ef7fb74b77d26311b` |
| launcher/src-tauri/src/telemetry.rs | `0c11425e36016cc815f5cfc7a0ed85bf69288907316304a685c2b23306ac8f35` |
| crates/platform-utils/src/http.rs | `406cca7cbffa9005ee41c55b9942afe4aa5d42adccccd26406064a250300e253` |
| crates/core/src/loaders/fabric.rs | `2ce058ed1caef825d9ca3751aa7c14521563e12dcf0d2bdee21fa60d40e8f605` |
| crates/core/src/launch.rs | `9c7678ca7ccaac1e383d59d2ca761089cb6ac5aada94af0158a7fd1cff10427a` |
| launcher/src-tauri/src/updater.rs | `0b082e47d120cdbf8423a9f8551909e24d76a25f18441fd65c89f1c2ba61f132` |

Adjacent official cached source (registry root
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f`):

| Cached source | SHA256 |
|---|---|
| tauri-2.12.0/src/app.rs | `ef71e5c2da6696f9bef817f5f32d34c7710b678ca7ec8a00e539527bca5ad1d4` |
| tauri-runtime-wry-2.12.0/src/lib.rs | `c909d268dc5340c19f3980a768b6956f3b0a5efdcca4243b1e6c3904773de0c6` |
| tokio-1.53.2/src/fs/write.rs | `84749a1b339b0840bef0a38a5320ec0c144760a730c5906146c981545c053763` |
| tokio-1.53.2/src/fs/mod.rs | `3b2060c5ecf43e1c28bd1c712fbd93c09eead6f0e91a2b78e6a93dd7b92243f9` |

## Follow-up acceptance — stable S1/S3 fixes, October 13, 2026

This section supersedes the earlier **open** S1/S3 status for the exact paths
reviewed below. The historical findings and initial hashes remain above for
provenance. Changed production and tests were read directly, and the supplied
logs were inspected. No Cargo command was run for this follow-up: main retains
Cargo and owns the stable final host/core sweep. Only this report was edited.

### S1 accepted for the identified preparation cache/download paths

`crates/platform-utils/src/http.rs:37` now delegates public cached JSON fetching
to private `get_json_cached_with`, with synchronous `write_cached_json` as the
production writer. Directory creation and cache writes run inline using `std::fs`,
and offline cache reads are also synchronous. Download destination directory
creation in `try_download` is now inline `std::fs::create_dir_all` rather than a
Tokio filesystem future. No `tokio::fs` or `spawn_blocking` remains in this file.

There is no await between entering the cache writer and completing its directory
creation/write. Aborting the outer preparation cannot interrupt an executing
future poll in the middle of those synchronous calls; the worker retains its
mutation lease until the poll returns and the task completes or processes abort.
The supervisor still joins before relinquishing completion ownership. This closes
the specific detached blocking-closure escape reported as S1, without putting a
Tauri-specific tracker in the reusable platform library. Download writes/cleanup
in this helper already use synchronous ownership; the changed mkdir no longer
dispatches an unowned blocking operation.

The three new `http/cache_cancellation_tests.rs` tests cover:

- A two-worker runtime and a gated injected synchronous cache writer. Abort is
  requested while the writer is blocked; the task remains unfinished and its
  synthetic drop-owner remains live until release. The real `write_cached_json`
  then writes the exact expected bytes before owner destruction. Either normal
  task completion or processed cancellation is valid after the synchronous poll.
- Public online caching and exact offline fallback after the loopback server ends.
- Public online success when a cache parent is a file, preserving the sentinel
  and the existing best-effort caching semantics.

The abort fixture exercises the private production control-flow seam and real
writer, not an AppState permit or full Tauri shutdown orchestration. It establishes
the relevant synchronous-owner ordering, not a native acceptance checkpoint.
The two-worker choice permits the controller to release a blocked worker; this
test should not be interpreted as proof of single-worker responsiveness.

**Tradeoff:** synchronous directory creation, cache read/write, and existing
download filesystem work occupy an async runtime worker. Slow filesystem work
can delay task cancellation, request timers, event delivery and shutdown, and
starve a single-worker executor until the call returns. This ownership fix favors
correct drain accounting over responsiveness; it supplies no filesystem deadline.
Cache writes remain best-effort and are not newly atomic or power-loss durable.
Any later offload must transfer actual owner/lease lifetime into the blocking
work or explicitly join it, rather than restoring the original escape.

### S3 accepted for admitted rename and fresh standalone commits

`launcher/src-tauri/src/accounts.rs:130` keeps fresh admission in
`save_panel_account`, then calls private `commit_panel_account`. The rename path
retains its original `_lease` through HTTP response parsing and directly calls
that commit helper at line 171. The helper preserves the prior account identity,
candidate-file commit and credential rollback routine; it does not reopen global
admission. Its only production callers are the admission wrapper and admitted
rename. No unrelated public bypass or IPC signature change was added.

The new `accounts/shutdown_tests.rs` contains one gated loopback rename regression
and one standalone rejection regression (not two loopback tests). The loopback
fixture verifies `PUT /api/v1/account/username`, sends success headers, holds the
response body, then begins shutdown. Readiness remains false and fresh mutation
admission is denied. After body release, rename succeeds, keeps the original
account ID, updates in-memory and saved account data and the panel token, drains,
and allows readiness. The standalone case closes admission before a subsequent
save and verifies exact saved accounts/credential bytes and active identity remain
unchanged. These tests use temporary synthetic state; `Secrets`' test cfg selects
its temporary key-file path rather than native OS keychain access.

The helper's authorization is currently a private-call-site invariant rather than
a permit parameter encoded in its type. Future callers must not invoke it without
already-held ownership. Network failure, malformed successful response or local
persistence failure after a remote rename remain possible; this change resolves
the nested-admission refusal, not general distributed account recovery.

### Inspected validation, remaining scope and revised decision

The utility log shows **13 unit tests passed, zero failed**, including the three
cache tests, plus zero doc tests. The rename log shows **2 passed, zero failed,
87 filtered out**, matching the two new account tests. These are main-executed log
results, not independently rerun commands or the final full host/core sweep.

**No remaining blocker found in the reviewed S1/S3 fixes.** S2's prior admission
guard remains unchanged and accepted. This is bounded source acceptance only.
No native/full checkpoint or revised completion percentage is awarded.

Not all helper/subprocess lifetimes are covered: `detect_java(path)` remains
untracked read-only subprocess work; arbitrary remote IPC commands remain outside
the drain. Updater downloading still uses Tokio filesystem operations under its
non-cancellable-by-shutdown command lease. Ordinary graceful begin does not abort
that command, so the former preparation abort schedule is not directly reproduced
there; independent command-future cancellation/descendant I/O has not been proved
safe by these fixes/tests. Texture threads, explicit blocking size/delete workers,
installer handoff, loader processor lifetimes and attached-game pipe behavior
retain their separate documented scope/native gates. No whole-dependency audit,
process-tree termination, force-quit, power-loss or stuck-filesystem guarantee is
claimed. Main's final stable sweep and supported-OS/native acceptance remain gates.

### Follow-up SHA256

These hashes supersede the earlier `http.rs` and `accounts.rs` rows for this
accepted follow-up snapshot. Other rechecked orchestration files (`shutdown.rs`,
`preparation.rs`, `state.rs`, `lib.rs`, `commands/launch.rs`,
`commands/instances.rs`, `updater.rs`) match their earlier recorded hashes.

| File or inspected evidence | SHA256 |
|---|---|
| crates/platform-utils/src/http.rs | `455d58ef2702c5d58973dfe71a3ff8d773be77579a6c038ea373eb3ecd7d8f90` |
| crates/platform-utils/src/http/cache_cancellation_tests.rs | `f03ff6539798be21308d1f5d3a085f3c779609c2174cc40254f5e3df4b19aaa2` |
| launcher/src-tauri/src/accounts.rs | `f16de53cf10d24d55077dd7a61f9de19c5e03421d07a8e25074bd13652062089` |
| launcher/src-tauri/src/accounts/shutdown_tests.rs | `ad0c7e3f27408b121793af19f07b288d681297dd20d848a38f274a11d31e36a5` |
| /tmp/velora-revitalization-final/shutdown-utility-tests.log | `203a3dec2302825e388ad934c84896f31fc4beadae712ea36699d1736074cbac` |
| /tmp/velora-revitalization-final/shutdown-rename-tests.log | `4f0f6b02f2cb3a1921659f570c7390b3e0314346fdba4e4a0962d5f8653815c2` |

## Updater follow-up acceptance — October 13, 2026

Reviewed stable `launcher/src-tauri/src/updater.rs` and its new
`updater/shutdown_tests.rs`, the unchanged `commands/updates.rs` admission path,
and the supplied updater log. This section supersedes the preceding snapshot's
open updater writer-lifetime limitation for these exact operations. No Cargo
command, installer execution, native application or release action was run by
this reviewer. Main retains the final stable host/core sweep.

### Ownership and publication accepted

`InstallerPart` at `updater.rs:167` owns an exclusive-created part path and an
optional standard file. `create_new(true)` refuses an existing part without
constructing a cleanup owner, so failure cannot delete that preexisting file.
Unix creation mode is 0600. The UUID destination convention and `.exe.part`
derivation are retained; no path/signing/credential identity changes are implied.

Production create, chunk writes, `sync_all`, rename and error/cancellation cleanup
now run inline through `std::fs`; remaining Tokio filesystem references in this
file are inside the preceding test module only. `publish` syncs the file, takes
and closes its handle before rename, and marks published only after rename
succeeds. Failed sync or rename drops the still-unpublished owner and attempts
cleanup. Drop closes any remaining handle before removing its part path.

An abort can be processed at a network-stream await, but cannot interrupt a
synchronous file operation midway through its executing future poll. Cancelling
the nested download future drops `InstallerPart` before its containing task's
work permit or `install_update` mutation lease is relinquished. Cleanup is itself
synchronous, so no descendant blocking writer/remover is dispatched after permit
release. If abort arrives during the final synchronous verification/publication
poll, publication can finish before cancellation is observed; this is accepted
completion ordering, not a guarantee that cancellation always prevents publication.
Ordinary begin still does not abort the admitted updater command automatically.

### Test evidence inspected

The supplied `shutdown-updater-tests.log` shows **8 passed, zero failed, 84
filtered out**, including the three new tests and five preceding updater tests.
These are inspected main-executed results, not independently rerun Cargo tests.

- A real loopback response sends only five bytes of its advertised body and waits.
  The downloader wrapper owns a real `Shutdown` permit. The fixture observes the
  five-byte part, begins shutdown, verifies readiness is false, explicitly aborts
  and joins the task, then verifies both part and destination are absent before
  waiting idle and marking ready. This is the actual helper path without execution,
  not the full Tauri IPC cancellation mechanism or a mutation-lease fixture.
- Exclusive part creation fails while preserving exact preexisting part bytes.
- Publication to an existing directory fails, removes the owned part and preserves
  that destination directory's sentinel. This is not proof that publication to an
  arbitrary existing regular file preserves it.

### Limits and revised decision

**No blocker found in this bounded updater ownership fix.** The synchronous
filesystem/runtime-worker responsiveness tradeoff recorded for S1 also applies
here, especially to `sync_all` and destructor cleanup: network timeout cannot
interrupt a stuck standard filesystem call, and shutdown may wait indefinitely.
Cleanup failure is warned and can leave a part file; it does not certify guaranteed
artifact removal. Successful publication followed by failed external installer
spawn can leave the completed installer, as before.

`sync_all` on the file does not prove parent-directory rename durability or
power-loss recovery. Exclusive creation establishes initial ownership, not
hostile-parent or subsequent path-replacement safety for pathname-based cleanup
and rename. The complete temp-directory/symlink/race/security matrix is explicitly
uncertified. No installer signature/release acceptance, external-process lifetime,
supported-OS close/rename behavior, native keychain or GUI acceptance is inferred.
Untracked Java detection, arbitrary remote commands and other previously stated
helper/native limitations remain. No full/native checkpoint credit is granted.

Only this report was appended. The following updater hash supersedes its earlier
scope row; prior S1/S3 acceptance remains bounded to its recorded snapshots.

| File or inspected evidence | SHA256 |
|---|---|
| launcher/src-tauri/src/updater.rs | `e2edc76f63b77d57a067567269ae0740b2a88552dd017f688d34e2e44cc80054` |
| launcher/src-tauri/src/updater/shutdown_tests.rs | `7e3224ce2500c4685aad737c9572c2c29ec3d0b3686172a5c3f99aca130e1188` |
| /tmp/velora-revitalization-final/shutdown-updater-tests.log | `515d7269dc4bf9a771983a5ae59452264e11c137cac13ae92b8ef8b76dfb1522` |

## Final handoff and formatted test hashes — October 13, 2026

**Final updater disposition: accepted for the bounded source ownership fix; no
remaining blocker identified in that reviewed change.** Its production and new
test hashes were rechecked unchanged from the updater follow-up. All synchronous
I/O, best-effort cleanup, hostile-path/race, native execution and release limitations
in that acceptance remain in force. No native/full-checkpoint credit is granted.

Main reports its stable full suite passed: launcher host **92**, core **132**,
platform utilities **13**, shared **19**: **256 passed** in total. Three core timing
tests and seven existing online tests remain ignored. Main also reports non-test
host check passed, source checks and four IPC tests passed, all 157 nonignored
additions scanned with zero Gitleaks findings, and tracked scanning with six
reviewed/zero unreviewed findings. These final-sweep/scan claims are attributed to
main's supplied status, not fresh reviewer executions or independently inspected
logs in this final hash refresh.

After that sweep, main reports applying rustfmt only to `shutdown/tests.rs` and
`preparation/tests.rs`, with no semantic change, and starting a launcher-host
92-test rerun. This reviewer refreshed those files' hashes directly; no pre-format
byte copy was retained for an independent formatting-only diff. The post-format
host rerun was still in progress in the supplied status and is not claimed passed
here. Main owns its completion/result. No additional Cargo command was run by
this reviewer, and only this report was appended.

These rows supersede the initial test-file hashes; the earlier focused results
refer to their pre-format snapshots.

| Formatted file | SHA256 |
|---|---|
| launcher/src-tauri/src/shutdown/tests.rs | `8ff0cadeb2a377c448bb6cc9b1e88ac88f89e17abd577fdf65115c3fe30b854c` |
| launcher/src-tauri/src/preparation/tests.rs | `822c3046ab1edfda66004fe2a9fab018849646d54f6397ec6748c94731a8da92` |

## Completion confirmation — October 13, 2026

Inspected `/tmp/velora-revitalization-final/shutdown-final-host-format-tests.log`:
the post-format launcher host rerun completed with **92 passed, zero failed,
zero ignored**, test execution 0.20s. This supersedes the pending-rerun status
above. Its SHA256 is
`a08dca6c9ab5f830b47772890a1e3652e7ff412fefe56440c4218bd504e839f1`.
Both formatted test hashes were rechecked and match the final handoff rows;
updater production SHA256 remains
`e2edc76f63b77d57a067567269ae0740b2a88552dd017f688d34e2e44cc80054`.
No additional reviewer Cargo command was run.

The bounded review is complete: S1/S3 fixes and the updater ownership follow-up
are accepted for their enumerated source paths; S2 is resolved. No remaining
blocker was identified within these fixes. Earlier limits remain explicit:
source/synthetic acceptance is not native/full-checkpoint, installer execution,
release, security-matrix, power-loss or universal helper-lifetime acceptance.

Main reports coding complete and latest disk availability about 1.9 GiB with
APFS at 100%. Existing-target reuse was retained; this review made no purchase,
deployment, branch or commit. The superseded model cap is not treated as a stop
condition, and actual metered USD remains unknown. Only this report was changed
by the reviewer.
