# Final comparable install/Java measurements

Audit dated October 14, 2026; all three measured runs completed on October 13
(local EDT). This is the separate VEL-014 checkpoint-3 measurement deliverable,
not an automatic third credit for the earlier profiling/optimization assignment.
Its independent disposition belongs to `install-final-measurements-review.md`.
The engineering ledger is 61/76 before that disposition; native checkpoint 4
remains open regardless of the measurement result.

## Provenance and raw-data audit

`install-final-measurements.json` retains the complete structured records for
three distinct executions: retained pre-optimization baseline, first optimized
run, and a fresh final repeat on the unchanged independently accepted source.
Each record has its input log SHA256; the artifact also records final source
SHA256. Original logs are under `/tmp/velora-revitalization-final`:
`install-profile-before.log`, `install-profile-after.log` and
`install-profile-final-repeat.log`. The baseline source/hashes remain in that
directory as documented in `install-java-profile.md`.

The final-repeat source hashes are unchanged from checkpoint-1/2 acceptance:

| File | SHA256 |
| --- | --- |
| crates/core/src/install.rs | `2fe9e7928931d341a17b4a9f4144e3eb995821c5ba44ae19619304ea5000fa66` |
| crates/core/src/install/profile_tests.rs | `aa50315333d7be738569b965981095c9896aeec10cced86b4c7908cc680e6770` |
| crates/core/src/assets.rs | `13778453fb2db3c914f1ff38fa2079c7c2bce9ec6dae747f3da863d549f3ce43` |
| crates/core/src/java.rs | `ddb715136bcee9bf3507ebf8e7e0789dd1bad88ab81b23e221c232d99be16b88` |

The extraction/audit checked every one of the **120 full-install flow samples**
and **15 successful harmless JVM outcomes** across these three records. It checks
five samples per family; identical headers/configuration; finite nonnegative
times; monotonic stage offsets within each operation; exact metadata/artifact/
total request counts; 258 retained library checks in every flow; two baseline
resolutions versus one in both optimized runs; and successful native JDK exit.
Telemetry has no remote URLs, local user paths, tokens or player-store contents.
Raw timing arrays are retained, not just manually transcribed medians. The
summary output is `install-final-measurements-summary.log` in the same temporary
directory. No source/configuration change was made for the final repeat.

## Matched workload and boundaries

All records use the same modern Vanilla version 1.21.1 metadata shape, 128
classpath libraries plus one synthetic native archive, one ZIP client over
1 MiB, duplicated asset names sharing one object, logging configuration and a
synthetic unmanaged save. Configured concurrency is four, but the loopback server
is serial. Platform rules and native naming use Darwin arm64. Rust is 1.98.1,
debug test build with test debug info disabled; real local Java is Oracle JDK
21.0.3. Profiles use no proxy/redirect and the same 30-second bounds.

Automatic-runtime flows execute the actual index/verified-manifest/download/
marker path with a fake Java executable that is never run. JDK-override flows
execute actual version detection. Isolated process samples use production launch
command building/spawning of javac-compiled harmless `FixtureMain`, assert its
fixed output and wait/reap it. These are not Minecraft or native-library loads.
All installation artifacts, native exclusions, runtime marker and saved bytes
are checked by the fixture after each operation.

Each sample runs fresh-logical-root cold, intact warm, intact deep verification,
then one same-size library corruption/deep repair. Installation timers include
the actual full await, not fixture setup, javac, corruption or postconditions.
The helper timings are separate and **not additive install substages**. In
particular, isolated native extraction reruns against already extracted files;
texture extraction is a logical first extraction from a synthetic archive.
Stage-event offsets remain in raw records without inventing finer boundaries.

OS page/disk caches, system load and physical cold state are uncontrolled.
Baseline/optimized/repeat runs are sequential, not randomized/interleaved.
Five samples permit descriptive summaries, not population-level confidence or
statistically established gains. Supported-OS, native game/loader, legacy asset
materialization, real-runtime-download redistribution, startup UI/memory and
signed installed upgrades remain outside the workload.

## Complete flow results

Median milliseconds; final-repeat min–max and median absolute deviation (MAD)
show variability, not confidence intervals. All unrounded arrays are in JSON.

| Flow | Baseline median | First optimized median | Final repeat median | Final min–max | Final MAD |
| --- | ---: | ---: | ---: | --- | ---: |
| Automatic cold | 65.579 | 54.992 | 68.431 | 66.581–85.596 | 1.850 |
| Automatic warm | 2.063 | 2.056 | 2.004 | 1.918–2.178 | 0.086 |
| Automatic deep | 8.169 | 8.339 | 8.236 | 7.886–8.629 | 0.284 |
| Automatic repair | 9.204 | 9.280 | 9.078 | 8.895–9.226 | 0.070 |
| JDK override cold | 89.513 | 79.123 | 87.153 | 81.882–91.027 | 3.790 |
| JDK override warm | 24.775 | 25.537 | 25.533 | 24.595–31.413 | 0.247 |
| JDK override deep | 31.458 | 31.119 | 31.746 | 28.098–32.069 | 0.323 |
| JDK override repair | 31.943 | 32.202 | 32.597 | 30.188–33.867 | 0.648 |

The earlier lower cold medians do **not** repeat consistently: automatic cold
is now higher than baseline; JDK cold remains somewhat lower. Final automatic
deep and JDK warm/deep/repair medians are higher than baseline. Automatic warm/
repair are lower. These mixed observations neither establish a general speedup
nor certify wall-clock no-regression. No result is omitted to improve the story.

The repeat confirms the deterministic optimization outcome: every Vanilla flow
avoids one resolution of 129 entries while retaining all 258 checks, both
discovery passes and identical requests. That bounded work reduction must not be
equated with the uncontrolled cold timing differences. Child-loader vector
retention/memory tradeoffs remain as recorded in the earlier source review.

## Helper and process results

All values in milliseconds. Metadata parse/resolution/hash/extraction helpers
are independently timed after install flows; they are not startup/game benchmarks.

| Helper / process | Baseline median | First optimized median | Final repeat median | Final min–max |
| --- | ---: | ---: | ---: | --- |
| Version JSON parsing | 0.204 | 0.201 | 0.211 | 0.202–0.258 |
| Library resolution | 0.104 | 0.108 | 0.109 | 0.102–0.166 |
| Deep library hashing | 5.096 | 4.976 | 4.897 | 4.846–5.326 |
| Native extraction | 0.039 | 0.042 | 0.042 | 0.039–0.048 |
| Texture extraction | 0.465 | 0.439 | 0.472 | 0.421–0.565 |
| Java detection | 23.157 | 23.106 | 23.115 | 22.381–23.744 |
| Launch command building | 0.461 | 0.455 | 0.453 | 0.434–0.677 |
| JVM spawn through successful exit | 42.036 | 42.451 | 42.822 | 41.925–55.067 |

No Java-spawn improvement is observed or claimed. The retained resolution work
cost is around 0.1 ms in this tiny workload, far below native detection/process
latency. Remaining optimization choices need new measured evidence; this report
does not justify skipping integrity checks or adopting speculative caches.

## Invariant results and next gates

All three records agree on requests:

| Family / flow | Metadata | Artifacts | Total |
| --- | ---: | ---: | ---: |
| Automatic cold | 5 | 133 | 138 |
| Automatic warm / deep | 3 | 0 | 3 |
| Automatic repair | 3 | 1 | 4 |
| JDK override cold | 3 | 132 | 135 |
| JDK override warm / deep | 1 | 0 | 1 |
| JDK override repair | 1 | 1 | 2 |

The final explicit profile passes, source hashes remain unchanged, and prior
stable scoped regression remains **260 Rust passes**, four default-ignored
profiles/seven online cases. The repeat is not added to that unit-test count.
No new frontend/native-game/whole-workspace sweep is claimed.

There is no production change to roll back in this measurement slice. Remove
only the added measurement artifacts to revert reporting; the prior source
optimization remains separately reviewed. No live store, signing, installer,
paid capacity, dependency upgrade, commit, branch, release or deployment occurred.
Checkpoint 3 acceptance requires the separate independent audit; checkpoint 4
and all native/data/security/upgrade/release gates remain explicit.
