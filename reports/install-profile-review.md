# Independent install/Java profile and optimization review

## Acceptance planning — October 13, 2026

Review is pending stable source and benchmark evidence from main. No checkpoint
is accepted by this planning section. This reviewer owns only this new report;
no production/test code, Cargo command, browser, Java subprocess, branch, commit,
purchase or deployment was used in planning.

Read repository instructions and the boundaries/validation/deployment documents.
The fixed definitions are in `reports/completion-checklist.md:32`:

1. **Comparable install/Java profile.**
2. **Evidence-led path optimization.**
3. **Comparable final measurements.**
4. **Native launch/loader no-regression acceptance.**

`reports/90-percent-gates.md` confirms these remain open, and the preceding
`performance-baseline.md`, `preparation-path-profile.md`,
`preparation-profile-review.md`, `java-runtime-profile.md` and
`java-runtime-integrity-review.md` explicitly do not accept the earlier sync-only
and tiny automatic-runtime fixtures as the complete install/Java profile.
Those narrower measurements cannot simply be relabeled to satisfy checkpoint 1.
The ledger is the explicit checkpoint authority available in this checkout; its
wording and denominator must not be relaxed for the percentage target.

The current fixed total is **59/76 = 77.63%**. Two independently accepted
checkpoints would yield **61/76 = 80.26%**, not native/release readiness. This
report does not award either checkpoint before inspecting actual evidence.

## Up-front criteria and gaps for main

### Checkpoint 1: full comparable install and Java baseline

The proposed full `install` body with loopback metadata/client/libraries/assets/
native/logging sources and a real local JDK 21 probe/subprocess can address the
previous missing scope. Acceptance depends on the following evidence, not merely
the existence of the new fixture:

- Execute the actual production installation body, including metadata resolution,
  initial downloads, final library presence guard, native extraction and any
  claimed legacy-asset materialization. Test-only origin injection must not bypass
  those operations or change public production origins/default behavior. Declare
  the selected version, loader, platform rules and component coverage; a complete
  Vanilla body is not a profile of every loader or the whole launcher preparation.
- Use identical fixed payloads, library/object counts, native archive contents,
  logging metadata, concurrency, client/server policy, toolchain/build profile,
  stage order and timing boundaries for baseline and final comparison. Record raw
  repeated samples, operation/request counters and output invariants, not one Cargo
  harness duration or an aggregate assembled from unrelated earlier profiles.
- Cold means a fresh logical installation root; warm means the defined reused
  installation; deep means intact content reverified; repair must state exactly
  which same-size content is corrupted/deleted and prove restoration. Do not call
  fresh directories physically cold OS caches. Fixture setup, corruption and
  assertions should have explicitly stated inclusion/exclusion from timed work.
- Profile actual `detect_major` against the available JDK, and a bounded harmless
  synthetic JVM subprocess with verified exit/output, not version-string parsing
  or a shell script presented as JVM performance. State JDK version/architecture,
  whether compilation is outside timing, and whether subprocess timing is separate
  from installation rather than silently adding it to install timings.
- Include the automatic `ensure_runtime` branch through its real body, with
  verified manifest/artifact publication and cold/warm/deep-corruption repair
  behavior as applicable. An override-only install does not exercise automatic
  runtime installation. If automatic-runtime payloads are synthetic/non-JVM bytes,
  label them as ensure-path coverage and keep real JDK detection/execution evidence
  distinct. Do not claim a downloaded real runtime was executed when it was not.
- Verify digest/size expectations, resolved classpath, native extracted bytes,
  logging argument, artifact/request counts and zero unexpected/external routes.
  Preserve unmanaged sentinels and cleanup fixture roots/subprocesses. Record
  source/workload hashes and provenance so final measurements remain comparable.

Neither an all-loader native launch nor a release-OS matrix is required by the
literal checkpoint-1 definition; they remain checkpoint 4. Conversely, the real
JDK subprocess does not manufacture that checkpoint-4 acceptance. Synthetic
fixture coverage must remain explicitly bounded, with no production-network
throughput or real-game startup claim.

### Checkpoint 2: evidence-led Vanilla path optimization

Source inspection finds the proposed target in `crates/core/src/install.rs`:
the initial Vanilla stage resolves libraries and discovers/downloads missing
files; after the loader match, even Vanilla resolves the same libraries and
performs another discovery pass. This is a plausible target, not yet evidence
that it is a measured material bottleneck or a verified optimization.

- Capture baseline stage/call/file-check evidence before changing the behavior.
  Loopback request counts alone will not demonstrate removing redundant local
  discovery: an intact first pass already leaves zero second-pass downloads.
  Separate library resolution/discovery overhead from JDK/process, native
  extraction and network noise. Record the evidence that motivates the change.
- Reuse the first resolved Vanilla library list only after the first download batch
  succeeds. Skip redundant second discovery only for Vanilla; merged loader
  profiles still need their effective-library resolution and downloads. Preserve
  classpath rules/order, initial deep verification, download deduplication,
  native extraction and the final missing-classpath guard.
- Add ordinary, non-ignored regressions for cold/warm/deep/repair correctness and
  unchanged returned artifacts. Exercise missing/no-download library errors and
  loader-specific behavior through relevant existing/new tests; do not infer loader
  safety just because a Vanilla benchmark passes. Preserve public signatures,
  persisted paths, lifecycle/cancellation ownership and dependency boundaries.
- Report eliminated work quantitatively and any observed before/after variation
  honestly. A small/noisy timing change is not a broad speedup. At minimum,
  demonstrate the intended redundant work is gone without losing checks and the
  relevant source regressions pass. A speculative refactor alone earns no credit.

Comparable final measurements are checkpoint 3's distinct deliverable. Final
measurement evidence may corroborate checkpoint 2, but must not be automatically
counted twice or awarded checkpoint 3 simply to pass 80%. Native loader acceptance
remains checkpoint 4, regardless of the earlier loader contract tests.

## Planned final review boundary

After main signals stable: inspect changed production, origin seams, fixture
workload/assertions, actual baseline/final records and validation logs; verify
hashes and document acceptance or blockers per checkpoint. Main retains Cargo.
This reviewer will not run Cargo/browser/Java without a subsequent instruction.
Only this report will be appended/updated; no production optimization is authored
by this review. Billing remains unknown and the superseded model cap does not
relax acceptance. Disk-constrained existing-target reuse and no paid/native/release
actions remain the operating bounds.

## Baseline inspection and narrowed design — October 13, 2026

Main supplied the completed five-sample baseline at
`/tmp/velora-revitalization-final/install-profile-before.log`. Read its profile
record and passing exact-profile test summary without running Cargo. Also
inspected the in-progress full-pipeline fixture and install origin seam; this is
not final acceptance of evolving production/test source.

The record has five automatic-runtime samples and five local-JDK-override samples,
each with cold/warm/deep/repair flows. Every recorded flow reports **two library
resolutions and 258 library-discovery file checks**. The fixture has 128 classpath
libraries plus one native-library entry: 129 resolved entries per pass. Its
separate resolution helper samples are approximately **0.101–0.109 ms**. That is
measured evidence of duplicated local resolution work, not evidence that library
resolution dominates total install/JVM time.

Baseline medians computed from the supplied raw samples:

| Flow | Automatic synthetic ensure ms | Local JDK override ms |
|---|---:|---:|
| Cold logical install | 65.579 | 89.513 |
| Warm | 2.063 | 24.775 |
| Deep intact verification | 8.169 | 31.458 |
| Same-size library repair | 9.204 | 31.943 |

These columns are different Java branches, not a before/after optimization
comparison. The record labels debug/no-debug-info, concurrency four, serial
loopback transport, uncontrolled OS caches and synthetic automatic executable
never executed. No global speedup or game/native-library loading follows.
The inspected fixture repairs `library-0.jar`; its named repair flow is not a
claim of repairing every automatic-runtime or asset corruption case.

### Narrower checkpoint-2 target is suitable, pending stable validation

Main now proposes retaining the second `library_downloads`/file-check pass and
final presence guard, but reusing the first resolved library vector only when
the loader child is `None`. This supersedes the earlier proposed removal of
second discovery in this report's planning section. **Skipping file checks is
not an acceptance requirement.** The literal checkpoint remains evidence-led
path optimization; a smaller safe elimination of demonstrated duplicate work
can satisfy it if implemented and verified.

The narrowed design is preferable for preserving the second pass's ability to
rediscover missing/changed files. It should retain **258 discovery checks**, all
first-pass deep checks and the final guard, while reducing Vanilla resolution
calls **2 to 1**. Merged loader versions must still resolve their effective list.
Do not describe this as 258 checks removed, skipped download verification or
concurrent-filesystem safety certification. The second pass retains its existing
non-deep semantics and cannot prevent arbitrary mutations after its last check.

The 129 entries establish vector/entry construction avoided, not an instrumented
exact count of 129 allocator calls. Multiple strings/vectors can allocate per
entry; use “one resolution of 129 entries eliminated” unless actual allocator
counts are collected. Roughly 0.1 ms isolated resolution cost does not predict a
significant whole-install improvement amid JVM and loopback timing variation.

Stable acceptance still needs exact source inspection showing the first vector is
reused only for unmerged Vanilla, ordinary regressions for invariant preservation,
final counters of one resolution/258 checks across the defined flows and the
relevant loader regressions. Baseline duplication counters plus helper timing
provide evidence for this narrower target; final measurements should honestly
report noise/regressions rather than force a speedup narrative. No checkpoint is
awarded in this interim inspection. Main retains Cargo, and this reviewer changed
only this report.

## Stable candidate source/evidence review — October 13, 2026

Main signaled the candidate source stable while retaining Cargo for its final
sweep. This reviewer inspected `install.rs`, `assets.rs`, `java.rs`, the new
`install/profile_tests.rs`, `reports/install-java-profile.md`, both explicit
profile logs, ordinary final fixture output and the retained baseline sources.
No reviewer Cargo/JVM/browser execution occurred at this stage.

**No source or evidence blocker identified for VEL-014 checkpoints 1 and 2 in
this bounded candidate.** Their definitions are unchanged. The inspected evidence
supports acceptance; final disposition/repeatability remains to be recorded after
main releases Cargo and supplies the exact local JDK invocation. Neither a
percentage change nor native/checkpoint-4 credit is authorized by this interim
statement alone.

### Exact optimization and preserved behavior

Compared all four retained baseline files against current source and independently
verified the baseline manifest's hashes. `assets.rs` and `java.rs` are byte-identical
between profile baseline and candidate; they contain the reviewed crate-private
asset-origin and runtime-index seams. Public install passes `None` sources, using
the unchanged Mojang version manifest, Java runtime index and asset origin.
No external origin override is added to the public API or persisted settings.

The baseline-to-final production change is narrowly contained in `install.rs`:
store the first `resolve_libraries` result, pass it through to `resolved_version`,
and return it directly for `child == None`. For `Some(child)`, merge Vanilla/child
and resolve the effective library list exactly as before. The environment is the
same captured value and resolution depends on metadata/rules rather than changing
filesystem state, so the unmerged vector is reusable.

The first deep/non-deep discovery and download batch, second non-deep discovery/
download batch, final classpath-existence guard, extraction and returned library
ordering remain intact. Failure of first downloads still returns before reuse.
The new child-profile regression compares complete resolved values to the original
merge/resolution composition and asserts child-first override/base ordering. It
does not claim running the Fabric/Quilt/Forge/NeoForge installers natively.

Ordinary actual-pipeline fault fixtures remove a real fixture library at the final
Game progress event, verify a second fetch restores it, then independently remove
it at both Game/Loader completion and verify the final presence guard errors while
preserving the save. They use the existing reporter callback, not a replacement
installer body. This gives concrete evidence the intentionally retained second
pass and guard still execute. Existing loader-related source regressions are
visible passing in the lead's final-sweep log; online loader tests remain ignored.

Tradeoff: keeping the initial vector alive across download/loader processing can
retain its memory longer on child-loader paths than the previous temporary vector.
It is discarded after resolving the child. No loader-memory improvement is claimed
or measured. Test counters are cfg(test), thread-local and reset for current-thread
fixtures; their 258 count covers library discovery calls to `file_ok`, not every
filesystem check or digest operation in the installation.

### Profile completeness, matching and measurements

The fixture traverses the real complete modern Vanilla installer with valid stored
ZIP libraries/client/native archive, version/asset metadata, logging, automatic
runtime index/verified manifest/artifact/marker publication or a real JDK override.
It verifies exact client/library/native/logging/object/runtime bytes, native
exclusions, classpath cardinality, runtime marker and unmanaged save preservation.
Its two asset names intentionally share one object. The artifact workload is
fixed at 128 classpath libraries plus one native entry and a client over 1 MiB.

Local javac compiles one harmless class outside timing, and production
`launch::build`/`launch::spawn` execute it after actual Java-major detection. The
subprocess must exit successfully and print exactly `VELORA_SYNTHETIC_JVM_OK`.
The synthetic automatic-runtime executable, fake native bytes and fake texture
bytes are not presented as a real downloaded/executed runtime, loaded native
library, real rendered texture or Minecraft/game process. Modern asset behavior
is exercised; legacy-asset copying is not configured in this workload.

Parsed both raw records and checked **all 40 family/flow samples per record**:
before every flow has two resolutions, after every flow has one, and all retain
258 discovery checks. Corresponding network metadata/artifact/total counts match
in all samples. All ten before/after JVM samples record successful exit. Record
headers match: five samples per family, concurrency four, serial loopback,
debug-test/no-debug-info, uncontrolled OS cache and Java major 21.

The baseline profile fixture differs only by the added optional no-op observer
hook and post-timer counter assertions used by the new fault regressions. Ordinary
profile flows pass no observer; setup/assertions remain outside elapsed install
measurement. The no-op option check adds tiny reporter-path overhead, so these
are matched fixed-workload runs, not literally byte-identical instrumentation or
statistically controlled/interleaved trials. Sources embed fresh loopback ports,
and roots/JAR bytecode differ between automatic and real-JDK families by design.
Compare the same family before/after, never one family against the other for a
speedup. Artifact/classpath/source and request invariants are the control evidence.

Final medians in the lead report match inspected records; variation is mixed:
automatic cold/warm/deep/repair approximately 54.992/2.056/8.339/9.280 ms versus
65.579/2.063/8.169/9.204 ms before; JDK override approximately
79.123/25.537/31.119/32.202 ms versus 89.513/24.775/31.458/31.943 ms. Deep/repair or
warm medians sometimes worsen. Resolution helper medians are approximately
0.104 ms before and 0.108 ms after; the deterministic optimization is removing
one such invocation, not making that helper faster or proving a global install
speedup. Separate helper times and stage-event offsets are not additive subtimings.

The fixture's 30-second awaits and subprocess timeout cleanup are test bounds,
not production filesystem deadlines. Synchronous I/O may block a runtime poll;
timeout cannot interrupt that call. Existing `detect_major` does not newly prove
child kill/reap on cancellation; the successful real JDK run is what is measured.
The successful tiny fixed-output JVM path does not certify general pipe/descendant
handling, cancellation, every loader, OS/runtime acceptance or release signing.

### Validation inspected, not rerun here

`install-profile-fixture-final.log`: four ordinary tests passed, zero failed,
one timing profile explicitly ignored. `install-profile-before.log` and
`install-profile-after.log`: exact ignored five-sample profile each passed with
zero failures. The ordinary automatic-runtime fixture and the explicit real-JDK
work constitute different outcomes, not duplicate checkpoint credit.

Checkpoint 1 is supported by comparable full-install/Java baseline coverage;
checkpoint 2 is supported by measured duplicate work, its actual elimination and
the preserved-check regressions. Checkpoint 3's final-measurement deliverable is
not separately awarded by this checkpoint-1/2 review, even though the final record
corroborates optimization. Native launch/loader acceptance (checkpoint 4) stays
open. Independent focused/profile repeatability will be appended once main
releases Cargo; no broader suite is requested from this reviewer.

### Stable candidate and evidence SHA256

| File | SHA256 |
|---|---|
| crates/core/src/install.rs | `2fe9e7928931d341a17b4a9f4144e3eb995821c5ba44ae19619304ea5000fa66` |
| crates/core/src/assets.rs | `13778453fb2db3c914f1ff38fa2079c7c2bce9ec6dae747f3da863d549f3ce43` |
| crates/core/src/java.rs | `ddb715136bcee9bf3507ebf8e7e0789dd1bad88ab81b23e221c232d99be16b88` |
| crates/core/src/install/profile_tests.rs | `aa50315333d7be738569b965981095c9896aeec10cced86b4c7908cc680e6770` |
| reports/install-java-profile.md | `3b83ffaacb64f7f5eccfcebc11b77f6bf94624b5af4a754d4c73a59b221ec165` |
| install-profile-before.log | `26b44d8fcbc3847c04b4efd947e36f94fd83d7533823a56fb938b4c2dea5ebb4` |
| install-profile-after.log | `3542f65351d9ca882d0b4f53455b1cee894c19ffbe8645eeb05546b0efa264cf` |
| install-profile-fixture-final.log | `1169f1e800fc6d668d169077ef6306e4df6dfd3934aa9645af184756e22ad07e` |
| baseline-source/install.rs | `5e5b3c22355837853e18c327405ed9dbb5f8cf776cf7d75e1c27e4b694a536a0` |
| baseline-source/assets.rs | `13778453fb2db3c914f1ff38fa2079c7c2bce9ec6dae747f3da863d549f3ce43` |
| baseline-source/java.rs | `ddb715136bcee9bf3507ebf8e7e0789dd1bad88ab81b23e221c232d99be16b88` |
| baseline-source/profile_tests.rs | `3eb2792d9543a9f6b34dd4e007f58089045fb329f83cac41231c3d7cab0b19ed` |

Log and baseline-source rows refer to
`/tmp/velora-revitalization-final/install-profile-...`; the retained baseline
directory is `install-profile-baseline-source`. These are local evidence artifacts,
not committed reproducible releases. Only this report was edited by the reviewer.

## Final independent verdict and repeatability — October 13, 2026

After main explicitly released Cargo, this reviewer executed exactly these two
authorized commands, serially against the existing target:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib install::profile_tests
VELORA_PROFILE_JAVA=/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home/bin/java CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib install::profile_tests::controlled_complete_install_java_profile -- --ignored --exact --nocapture
```

- Focused ordinary suite: **4 passed, zero failed, 1 explicit profile ignored**,
  135 filtered out, test execution 0.16s.
- Explicit independent profile: **1 passed, zero failed**, 139 filtered out,
  test execution 2.27s. It executes five independent automatic and five real-JDK
  samples, each with all four install flows, plus five harmless JVM runs. In every
  flow the fixture asserts one resolution and 258 discovery checks and exact
  artifact/save invariants before accepting the sample. The profile emitted
  `COMPLETE_INSTALL_JAVA_PROFILE` schema 1. No new exact timing threshold or global
  speedup assertion was introduced. Reviewer output is in the tool transcript;
  no separate reviewer log/source file was written.

The real Java compilation/detection/subprocess execution here is the explicitly
authorized harmless fixture, not a game/native-loader test. No live player store,
credential, native GUI, installer, release or deployment was exercised.

### Precise checkpoint disposition

| VEL-014 checkpoint | Verdict | Reason |
|---|---|---|
| 1 — Comparable install/Java profile | **Accepted** | New full modern Vanilla install body, separate automatic runtime and actual JDK/detect/build/spawn coverage, fixed five-sample workload, retained pre-optimization source/records, matched controls/invariants and independent successful repeat. This is not relabeled prior sync-only evidence. |
| 2 — Evidence-led path optimization | **Accepted** | Baseline proves duplicate resolution; stable code eliminates one resolution of 129 entries while retaining both discovery passes, deep checks, loader merging and final presence guard. Actual-pipeline repair/presence fault regressions and child-order regression pass independently; final counters/network invariants match the intended change. |
| 3 — Comparable final measurements | **Not awarded by this review** | Final records corroborate the checkpoint-2 change, but this assignment evaluates/credits only checkpoints 1 and 2. No third credit is inferred from their reuse here. |
| 4 — Native launch/loader no-regression acceptance | **Open; not accepted** | Harmless JDK execution and synthetic extracted native bytes are not Minecraft/native-loader or supported-OS/release acceptance. |

**No remaining blocker identified for the accepted checkpoint-1/2 scope.** This
changes VEL-014's accepted outcomes from 0/4 to **2/4**, without changing its
definitions, weighting or the 76-checkpoint denominator. If the other 59 accepted
checkpoints remain as recorded in the fixed ledger, the supported total is
**61/76 = 80.26%**. This report supplies the independent verdict; main owns ledger
updates. The target did not substitute for evidence, and no release-readiness or
global faster-install claim is made.

### Final reconciliation and retained limits

Rechecked all four production/test source hashes after the independent runs:
they match the stable candidate table. Main updated its profile report's final
sweep section during review; inspected that update. Its current report SHA256
supersedes the earlier report row:
`9122c259c132d4629868cae3e0c37660e90a25ae0320b3f777bd3dc09d33291f`.

Also inspected main's completed `install-final-rust.log`: host 92/core 136/utils
13/shared 19 gives **260 passed**, with four core explicit profiles and seven
existing online integration tests ignored by default. The independently executed
profile above is not additional production test coverage to inflate that total.
Inspected non-test host check completion. Other source/fmt/IPC/link/secret outcomes
are attributed to main's reported checks, not additional reviewer executions.

| Final inspected evidence | SHA256 |
|---|---|
| /tmp/velora-revitalization-final/install-final-rust.log | `ff17d6b92a1f46b6614574602cd0683d941c2446a4589538a1f6200b1010cd26` |
| /tmp/velora-revitalization-final/install-final-host-check.log | `0083ffb8c22727bc6d7364b7d0db0d81cde3ba291404c4521576727d518b560d` |

Observed disk remains about **1.8 GiB available, APFS 100%**. No alternate target
or new dependency configuration was created. Actual billing remains unknown and
the superseded model cap is not a blocking condition. No paid infrastructure,
branch or commit was introduced. Only `reports/install-profile-review.md` was
authored/edited by this reviewer; generated target/temp fixture activity belongs
to the two authorized test runs.

The source review and independent repeatability work are complete. All previously
stated synthetic-workload, mixed wall-clock results, long-lived vector-memory,
synchronous-I/O, physical-cache, timeout/child-cancellation, legacy-assets,
every-loader, OS/native/game, hostile-path/security and release limitations remain
explicit. No remaining item is silently certified to reach the percentage target.
