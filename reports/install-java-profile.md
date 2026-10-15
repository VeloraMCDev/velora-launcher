# Complete install / Java profile and bounded optimization

## Separate final measurement acceptance — October 14, 2026

The October 13 checkpoint-1/2 milestone below remains historical evidence.
The distinct `install-final-measurements.md` / `.json` deliverable now has separate
independent acceptance in `install-final-measurements-review.md` for checkpoint 3.
All three records, 120 flow samples, 15 JVM outcomes, 48 summaries and 88 published
timing values reconcile. Final-repeat regressions are explicit; no global speedup
or wall-clock no-regression claim. Current total: **62/76 (81.58%)**; checkpoint 4
remains open. No production change or native-game/OS/release credit from this audit.

October 13, 2026. Base HEAD `4f639266a515c3cb05769b055a4772c4dcb89cb8`;
uncommitted source, no release or deployment. This is new full-install evidence,
not relabeling the earlier separate sync/Java fixtures. Checkpoint acceptance is
owned by the independent `install-profile-review.md` report and fixed ledger.

## Workload and execution

The private source seam executes the actual `install` body with disposable
loopback metadata origins. Public `install` still uses Mojang's fixed version
manifest, automatic Java index and asset origin. No external configuration,
public signature, persisted path, protocol identity, dependency or lockfile changes.
The Java and asset source helpers are crate-private, not public host APIs.

Every logical install contains 128 synthetic classpath libraries, one native
archive with an excluded META-INF entry, a valid ZIP client over 1 MiB, a digest-
checked asset index with two names for one object, one logging file and an unmanaged
synthetic save. Each library has a stored 16-KiB payload. Automatic Java includes
the real ensure/index/manifest/artifact/marker path with a 4-KiB fake executable;
that executable is **never run**. Native archives contain synthetic bytes, not
loadable native binaries. All installed bytes and save contents are checked.

The separate native family uses the already-installed local Oracle JDK 21.0.3
override. The actual detection function executes Java's `-version`; javac compiles
a harmless `FixtureMain`, and production `launch::build` / `launch::spawn` run it.
The process prints one fixed synthetic marker, exits successfully and is waited/
reaped. It neither launches Minecraft nor loads the fake native library. The long
classpath also exercises the existing launch argument-file path on this host.
No real credentials, keychain, launcher data, server or player store is accessed.

Both families have five independent logical roots per run. Each sample measures
cold, intact warm, intact deep verification and same-size one-library corruption
followed by deep repair. Timers include the actual full install await but exclude
fixture construction, javac and post-operation assertions. Stage event offsets
identify metadata / automatic Java / game work; they are not invented additive
substage durations. Separate helper timers cover JSON parsing, library resolution,
deep hashing, native ZIP extraction, texture extraction, Java detection, launch
command building and spawn-to-successful-exit.

The server handles connections serially and closes each response. Concurrency 4
is the client setting, not four-way server throughput. Requests disable proxies
and redirects. Each install/network/server/native-process await has a 30-second
bound. JVM timeout explicitly kills and waits; compiler kill-on-drop is enabled.
Roots and loopback servers are closed after samples. OS caches, system load and
physical disk state are uncontrolled. This is Darwin arm64, Rust 1.98.1,
unoptimized test profile with test debug info disabled; not real-game throughput.

## Baseline and optimization

The complete baseline ran before the optimization. Its exact four source files
are retained in `/tmp/velora-revitalization-final/install-profile-baseline-source`
with SHA256 in `install-profile-baseline-hashes.txt`. The corresponding profile
is `install-profile-before.log`; the identical workload rerun is
`install-profile-after.log` in the same parent directory.

Test-only thread-local counters show **two library resolutions and 258 library
file checks per flow** before the change. These counters wrap actual calls and
run on current-thread Tokio fixtures; they do not infer filesystem work from
network requests. Each resolution parses/builds the same 129 resolved entries.

The bounded optimization retains the initial resolved vanilla library vector
when no child loader profile exists. With a child, it still merges and resolves
the final version as before. Every profiled vanilla flow now has **one resolution,
still 258 file checks**. Both download-discovery passes, deep verification,
native extraction, final classpath-presence guard and classpath ordering remain.
No integrity check or online Java-manifest verification was removed for speed.
On child-loader paths, the initial vector now remains alive through loader work
until final resolution discards it. This can retain some library metadata longer;
no loader-memory improvement is claimed or measured.

The initially considered removal of the second file-check pass was rejected:
external changes can happen after initial downloads. A new actual-pipeline fault
fixture removes a library at the final game-download progress event and proves
the second pass refetches it. Another removes it again after the second pass and
proves the final presence guard rejects installation while preserving the save.
A child-profile regression compares merge/resolution/order against the existing
implementation. The ordinary full-pipeline fixture checks cold/warm/deep/repair,
native exclusions, exact artifacts, stable runtime marker and save preservation.

## Matched observations

Five-sample medians in milliseconds; fixed logical workload and runtime settings.
Runs are sequential before/after, not interleaved or statistically controlled.

| Flow | Before | After | Before min–max | After min–max |
| --- | ---: | ---: | --- | --- |
| Automatic cold | 65.579 | 54.992 | 60.676–115.547 | 50.231–64.081 |
| Automatic warm | 2.063 | 2.056 | 1.741–2.310 | 1.771–2.170 |
| Automatic deep | 8.169 | 8.339 | 7.857–8.965 | 8.070–8.921 |
| Automatic repair | 9.204 | 9.280 | 8.855–9.472 | 9.179–9.659 |
| JDK override cold | 89.513 | 79.123 | 81.789–110.828 | 70.595–83.605 |
| JDK override warm | 24.775 | 25.537 | 22.584–25.882 | 24.704–32.697 |
| JDK override deep | 31.458 | 31.119 | 30.997–31.713 | 30.873–32.243 |
| JDK override repair | 31.943 | 32.202 | 31.760–32.806 | 31.959–32.724 |

Cold medians are lower but the change's deterministic saving is only one redundant
resolution per vanilla install. Other medians are mixed: **no global faster-install
claim or statistically established wall-clock improvement** is made. The before
helper resolution median is 0.104 ms, after 0.108 ms; the optimization avoids one
such invocation and its temporary vector, not all library checks. It does not
address Java detection latency, cold disk throughput, sync journaling overhead,
metadata caching/expiry or real-modpack performance.

| Isolated helper / process | Before median ms | After median ms |
| --- | ---: | ---: |
| Version JSON parsing | 0.204 | 0.201 |
| Library resolution | 0.104 | 0.108 |
| Deep library hashing | 5.096 | 4.976 |
| Native extraction | 0.039 | 0.042 |
| Texture extraction | 0.465 | 0.439 |
| Java detection | 23.157 | 23.106 |
| Launch command building | 0.461 | 0.455 |
| JVM spawn through successful exit | 42.036 | 42.451 |

Request counts stay identical before/after: automatic cold 5 metadata / 133
artifacts; automatic warm/deep 3 / 0 and repair 3 / 1. JDK override cold 3 / 132;
warm/deep 1 / 0 and repair 1 / 1. Duplicate asset names produce one object download.
Helper times are measured separately and cannot be summed into full-install time.

## Reproduction and acceptance limits

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib install::profile_tests
VELORA_PROFILE_JAVA=/path/to/jdk-21/bin/java CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher-core --lib install::profile_tests::controlled_complete_install_java_profile -- --ignored --exact --nocapture
```

Ordinary fixture suite: **four passes, one explicit profile ignored**. Both
explicit five-sample runs pass independently. Initial fixture expectations for
the runtime marker and child-first library ordering were corrected to match
existing production behavior; neither failure was concealed as a passing run
or fixed by changing those production contracts.

Final locked scoped Rust regression sweep passes **260 tests**: launcher 92,
core 136, utility 13 and shared 19. Four explicit core profiles (including this
one) and seven existing online tests stay ignored by default. Non-test host check,
four IPC contracts, boundary/identity checks, 16 workflow policies, publication
inventory, tracked secret scan, local Markdown links, scoped rustfmt and whitespace
checks pass. Tracked Gitleaks has six already reviewed matches and zero unreviewed
findings; nonignored additions are scanned separately before the milestone handoff.
The separate nonignored-additions scan passes **160 files with zero findings**.
Logs are `/tmp/velora-revitalization-final/install-final-*`.
Independent checkpoint disposition is separate, not inferred from this test count.
The final independent review accepts **VEL-014 checkpoints 1–2** after source,
baseline matching and its own four-test / five-sample reruns. The unchanged
completion ledger therefore reaches **61/76 (80.26%)**, meeting the user's 80%
milestone. Checkpoints 3–4 are not awarded by this scoped review.
This is complete comparable evidence for this bounded install/Java workload,
not Tauri/native game, every loader, slow-disk/OS matrix, power-loss recovery,
real runtime-download redistribution, signed upgrade or release acceptance.
No paid capacity, dependency download, commit, branch, PR or deployment occurred.
