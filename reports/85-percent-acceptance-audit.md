# Independent 85% acceptance feasibility audit

October 15, 2026. Documentary/source-summary review only. Owned output:
`reports/85-percent-acceptance-audit.md`. No UI/browser interaction, tests, builds,
native execution, subagents, source edits, ledger edits, signing, releases,
installed-store access, infrastructure changes or purchases were performed.

## Decision for the lead

**Three wholly closable current-Darwin-host gates cannot be identified honestly
from the fixed criteria and available resources. Do not promise 85% from this
acceptance session.** The lead was notified before this report was written.
VEL-015 checkpoint 4 is the strongest local candidate; its complete rendered
review is still work to perform, not an awarded checkpoint. VEL-013 and VEL-018
explicitly retain supported-OS/native requirements. VEL-001 lacks an entire
runtime measurement matrix, not just browser startup timing. VEL-016 lacks
successful native install/repair/launch and acknowledged cancellation E2E.

- Fixed current position: **62/76 = 81.58%**.
- At least 85% requires **65/76 = 85.53%**: three additional complete checkpoints.
- Completing only VEL-015 checkpoint 4 would yield **63/76 = 82.89%**;
  two complete checkpoints would yield **64/76 = 84.21%**, still below target.
- These are conditional arithmetic, not a ledger update. Current verified credit
  remains **62/76**. No partial review, security fix, additional test count,
  proposal or this audit earns a checkpoint.
- The initial **$10 planning allowance** is an allocation, not a claim of actual
  USD spent, remaining credit or purchased resources. Actual billed USD is unknown.

## Authority and evidence

Read the root instructions, `MONOREPO.md`, `docs/BOUNDARIES.md`,
`docs/VALIDATION.md` and `docs/DEPLOYMENT_TARGETS.md`. No deeper `AGENTS.md`
was found under reports. Inspected checkout HEAD:
`4f639266a515c3cb05769b055a4772c4dcb89cb8`, with substantial existing uncommitted
work; HEAD alone does not identify the final source or executable.

The master-plan attachment was found and its relevant sections read at
`/Users/alyssasitler/Downloads/VELORA_REVITALIZATION_MASTER_PLAN.md`:
definition of done, Phase 0 deliverables/Gate 0, Phase 3 startup/Gate 3,
Phase 4 tasks/Gate 4, Phase 5 features, backlog and test/compatibility matrix.
It is not a checked-in file. Earlier reports saying that attachment was unavailable
describe their own contexts; this audit does not repeat that limitation.

The binary checkpoint names below are verbatim from
`reports/completion-checklist.md`. Interpretive acceptance steps come from the
attachment and existing evidence/gate reports, not newly invented replacement
definitions. The ledger fixes four equally weighted checkpoints per VEL item;
for each of these five 3/4 rows only checkpoint 4 remains open.

Principal evidence: `reports/90-percent-gates.md`,
`reports/native-current-host-observations.md`,
`reports/native-acceptance-implementation.md`, `reports/compatibility-matrix.md`,
`reports/performance-baseline.md`, `reports/performance-after.md`,
`reports/frontend-bundle-implementation.md`,
`reports/design-accessibility-implementation.md`,
`reports/visual-state-validation.md`, `reports/rendered-accessibility-followup.md`,
`reports/accessibility-final-review.md`,
`reports/installation-ux-implementation.md`,
`reports/diagnostics-contract.md` and `reports/diagnostics-implementation.md`.
Prior reports and their reported observations were reviewed; their browser/native
runs, screenshots and raw logs were not independently replayed or authenticated.
Historical performance report counts are not newer ledger decisions.

## Fixed gate disposition

| Item and unchanged checkpoint 4 | Current evidence shortfall | Full closure on current resources? |
| --- | --- | --- |
| VEL-015: **Keyboard/contrast/reduced-motion review** | Existing source fixtures, six accepted synthetic rasters and selected rendered fixes do not cover the complete defined review. | **Local candidate**, conditional on the lead completing the full rendered review below. Not already passed. |
| VEL-013: **Runtime startup/no-regression acceptance** | Comparable static JS bytes exist; usable comparable native startup/route acceptance does not. The bundle report explicitly leaves supported-OS/native startup and route regression acceptance open. | **No** for the whole gate; Darwin measurements alone cannot erase that qualification. |
| VEL-001: **Native/runtime performance baseline matrix** | Browser observations, chunk accounting and bounded install/Java fixtures omit native runtime metric/flow cells. | **No demonstrated full path** with the present harness/resources; recording missing cells as pending is honest but not completion. |
| VEL-016: **Native/E2E usability acceptance** | Controlled failures and unconfirmed cancellation do not establish successful install/repair/launch, restart recovery or the native UX matrix. | **No** with the error-only harness and unavailable game/OS/installed-compatibility acceptance. |
| VEL-018: **Native feature acceptance** | Source/helper/VM clipboard/export tests are not supported-native-webview acceptance. | **No**; the implementation gate explicitly requires each supported native desktop webview. A Darwin smoke is useful but insufficient. |

### VEL-015 — exact remaining rendered review

The ledger's accepted-evidence note expressly leaves **all-theme/full rendered
keyboard/contrast/motion acceptance** open. The attachment requires coherent UX
across Home, Instances, Launch/Repair, Account/Login, Settings, Social and error
states; loading, empty, offline, error, long-text and small-window visual checks;
960×600 minimum, system scaling, keyboard flow and reduced motion. Do not replace
that with one attractive default-theme screen or the already accepted rasters.

The lead owns the local browser/design-fixture inspection. Concrete review steps:

1. Record the exact served source/artifact, fixture entry, synthetic data, viewport,
   scale, theme and motion configuration. Verify the fixture mounts the production
   components/state it purports to review. A detached component gallery cannot
   establish application navigation and overlay behavior by itself.
2. Traverse the named flows using Tab/Shift+Tab, Enter/Space, Escape, Settings
   Arrow/Home/End and palette navigation. Observe visible, unobscured focus,
   logical order, named controls, single active tab, overlay background isolation,
   forward/reverse containment, nested/empty-dialog handling and opener restoration
   or safe handling of a removed opener. Check that launch shortcuts inside
   editable content/palette/modals do not activate the background launch action.
3. Review rendered foreground/background pairs and focus/non-text indicators,
   not token hex values alone: normal, hover, focus, selected, disabled, loading,
   success/warning/error, progress and safe-detail states. Include the supported
   predefined accents, default/custom theme paths and applicable branded surfaces,
   including pale accents, gradients and translucent/image-backed surfaces.
   Measure actual composed colors and record the applicable accessibility
   thresholds/results. The existing 4.5:1 opaque action-helper test certifies
   neither every control nor full rendered contrast.
4. Exercise both the in-app Reduce motion switch and the system/browser media
   preference, independently and together, on the final rendered UI. Inspect
   computed behavior for elements/pseudo-elements, scrolling and any non-CSS
   animation/media used by the reviewed flows. Do not infer system-preference
   acceptance from the prior run where that preference was false.
5. Repeat relevant focus/layout checks at 960×600 and supported UI scaling,
   including 130%, plus a normal laptop layout. Reach lower Settings categories,
   long content and error actions without clipping, hidden focus or horizontal
   overflow. Retain fresh DOM-matched synthetic rasters and measurements.
6. Produce a case-by-case review disposition with failures repaired/rechecked or
   explicitly left open. The prior nine-swatches sample, Add account focus/Escape
   and in-app motion sample are starting evidence, not substitutes for this matrix.

There is no finite guarantee for arbitrary administrator-injected CSS. Name the
actual supported/custom configurations reviewed; do not silently exclude that
surface while claiming all-theme acceptance. If its required scope cannot be
established or covered, leave the checkpoint open and escalate the ambiguity.
The ledger does not explicitly make this design-review checkpoint a signed
three-OS release or full WCAG certification gate. Conversely, do not use a browser
review to certify native screen readers, OS integration or installed releases;
those remain separately identified native/release gates. A browser-only closure
is defensible only for the complete defined rendered review, not a narrowed subset.

### VEL-013 — runtime startup and route non-regression

The attachment calls for first meaningful paint, responsiveness, WebView memory
and navigation cost on realistic hardware, responsive/progressive boot, useful
offline state and no blank-screen/stale-render regressions. Its numeric startup
and bundle goals are aspirations, not substitute acceptance thresholds.

1. Establish matched baseline and final production-frontend/native-host artifacts
   with hashes, toolchain, host/webview version and identical synthetic workload.
   The retained pre-lazy-route distribution is a candidate input whose availability
   and suitability must be checked, not an already validated native baseline.
2. Capture repeated controlled cold/warm startup-to-usable-shell/paint observations,
   responsiveness, memory and route costs with the same methodology. Preserve raw
   samples and explain cache conditions. Hidden/shared preview timings are explicitly
   discarded; the 42.25% raw static-closure reduction is not startup evidence.
3. Exercise eager Home/Login/Setup and all nine lazy routes, first/repeat navigation,
   rapid switching, import failure/retry, teardown/logout, offline boot and cached
   content refresh. Confirm current-route-only rendering, useful failure states,
   focus continuity and no blank screen or stale account/instance content.
4. Review regressions against the matched baseline; resolve or explicitly leave
   failing acceptance open. Repeat the supported native OS/webview cells retained
   by `frontend-bundle-implementation.md`; record exact versions and artifacts.

Darwin can contribute its own cell. It cannot supply Windows/Linux native startup
or routing results. Real game play, keychain restore and signed installers are not
inherently prerequisites for this frontend gate; do not add them as excuses.
But the present debug/Vite observations do not establish production cold/warm
startup or the missing supported-OS cells. This reviewer starts no build.

### VEL-001 — complete native/runtime baseline matrix

Phase 0 explicitly lists cold UI startup, warm startup, bundle chunks, idle/active
CPU/RAM, cold/warm install preparation, verification, download throughput, cache
size and time to launch. `performance-baseline.md` additionally identifies retry
bytes and loader preparation as pending. A documented list of unavailable metrics
does not satisfy the measurement checkpoint.

1. Freeze machine/build/workload context and collect the native cold/warm startup
   observations above; retain actual values and raw evidence, not speed claims.
2. Measure native idle and representative active CPU/RAM with defined intervals;
   distinguish browser JS heap from whole native/WebView/process memory.
3. Use disposable synthetic instances and controlled artifacts to measure complete
   cold/warm preparation, deep verification and repair, download throughput/retry
   bytes and resulting cache/disk sizes. Report logical versus physical cache state.
4. Record native time to launch and applicable Java/loader preparation/process
   outcomes with exact runtime/Minecraft/loader versions. A harmless JVM probe or
   loopback sync profile is not a successful representative game launch.
5. Populate the runtime/compatibility matrix with provenance, commands, outcomes
   and limitations; classify failures as preexisting/new. Supported-platform cells
   must not be implied by module names or by Darwin source-test results.

The existing native fixture serves a minimal manifest and controlled instance
errors, **never install artifacts**. It cannot produce successful install/game,
throughput or launch-time baseline cells. The missing game/artifact workload and
other native hosts require a separately bounded acceptance arrangement; no real
player store is needed or authorized. Even a current-host-only interpretation
would still lack the complete native metric set, so cannot be awarded now.

### VEL-016 — actual native/E2E installation usability

Existing source acceptance leaves native Windows/macOS/Linux admission,
cancellation, repair, offline/error/disk/permission scenarios, cross-instance and
multi-window E2E, keyboard/screen-reader, rendered/scaling/motion review, restart
behavior and installed-data compatibility open. Preserve those explicit gates.

1. In authorized disposable native fixtures, complete install → verify → corrupt
   one managed file → repair → launch. Preserve synthetic custom mods/config/saves;
   demonstrate accurate phase-local progress, safe details and correct typed retry.
2. Exercise offline, disconnect/timeout, denied permissions, disk-full and failed
   preparation; verify useful recoverable UI without exposing raw sensitive inputs.
3. Cancel admitted work and retry it. Capture correlated native acknowledgement
   and resulting backend/file/process state; test late completion/error races.
   “Ended after cancellation request” is intentionally **unconfirmed**, not a pass.
4. Switch instances and use multiple windows during busy/failure/retry flows;
   verify admission exclusion, owner isolation, no stale UI and no duplicate work.
   Restart/reload during relevant work and verify truthful recovery/progress/retry
   state. Current presentation memory does not survive leaving Home/restart.
5. Complete native keyboard/assistive-technology, contrast/scaling/motion checks,
   existing-format synthetic installed-data compatibility and supported-OS cells.
   Use reviewed disposable old-format fixtures rather than opening real stores.

Prior 503/focus/busy-close observations satisfy only narrow cases. The initial
Vite-absent launch failed readiness; delayed completion does not prove cancel;
later process absence does not prove graceful shutdown. The error-only harness,
unavailable successful game flow and missing Windows/Linux/installed-compatibility
acceptance prevent whole closure. UI fixture review remains useful preparation.

### VEL-018 — diagnostics native feature acceptance

Use the approved bounded diagnostics contract, not the attachment's broader
optional feature wish list. Do not expand it into health monitoring or claim
Java/mod/system information that the implemented allowlist does not provide.
The implementation's explicit open gate requires **each supported native desktop
webview** and the following remaining observations:

1. Inject synthetic crash and preparation snapshots with known canary secrets,
   paths, addresses and oversized/unrecognized text. Check accessible preview,
   categories, qualified advice, output bounds and zero original sensitive strings.
2. Confirm no sharing on render and no upload/network operation from diagnostics.
   Copy successfully via the real native clipboard, then exercise permission denial;
   verify exact preview/copied-text equality and truthful fixed failure feedback.
3. Export to a disposable destination: verify the local filename
   `velora-support-diagnostics.txt` and exact content; cancel the native download
   path and observe cleanup/object-URL revocation without claiming completion.
4. Revalidate user-triggered original Crash report/Logs file/folder actions using
   synthetic paths and Try again IPC compatibility. Preserve the warning that
   original files can contain private information; a retry request is not a launch.
5. Repeat on supported Windows/macOS/Linux webviews and retain versioned results.

A real game crash, production credentials, installed-store migration and signing
are **not required to synthesize this feature's inputs**; those are separate gates.
The concrete unavailable resource here is the other supported native webviews,
plus unperformed native clipboard/download/file-action acceptance, not a generic
“needs real game” claim. macOS-only testing cannot close this explicit whole gate.

## Evidence freshness and sequencing

### Other source-heavy open checkpoints are not quick substitutes

The lead subsequently reports origin/main divergence **0/0** and a live T3
synthetic domain fixture. Those are lead-supplied facts, not checks replayed by
this reviewer. They improve local rendered-review feasibility; they do not remove
the dirty-source/artifact correspondence requirement or any native OS gate.

- **VEL-012 checkpoint 4: Recovery/integrity/security acceptance.** This is not
  merely a native game test, but it is also not a second browser-review point.
  `content-cache-contract.md` still identifies same-user path races,
  power-loss/filesystem faults, Windows/Linux acceptance and broader cache
  lifecycle/integration limitations. `90-percent-gates.md` explicitly retains
  cache failure/security coverage, production integration and durable offline
  authority work. Complete the applicable implementation/contract acceptance and
  independently validate failure preservation, authenticated authority/revocation,
  containment and platform behavior before requesting this whole checkpoint.
  A Unix prototype passing existing tests is already credited in checkpoints 1–3.
  This is substantial implementation/acceptance work outside this review-only
  session, not an available whole gate to self-award now. Do not silently turn a
  prototype contract into a promise of HTTP resume or production caching.
- **VEL-019 checkpoint 2: Final independent security/data/compatibility review.**
  This is a genuine review-heavy checkpoint, not intrinsically a requirement to
  sign a release. It could in principle be pursued as a separately scoped final
  review; unavailable native release runs remain separate checkpoints. However,
  the supplied reports explicitly leave whole-change security/data/compatibility
  acceptance unresolved. The complete source snapshot must stabilize, findings
  must be reconciled across auth/destination binding, persistence/recovery, cache
  authority, operations/shutdown, installer/updater and frontend compatibility,
  and material findings must be resolved and independently reviewed. A review
  reporting those required outcomes still unaccepted is not a pass. This narrow
  five-gate documentary audit does not perform that final sweep, and no evidence
  establishes it as a presently closable second or third checkpoint.
- **VEL-002/004/005/010/014 and VEL-019 checkpoints 3–4** retain explicit native,
  process/durability/OS, keystore/upgrade, loader/game or restore/readiness outcomes
  in the fixed ledger and `90-percent-gates.md`. More local source tests or a live
  browser fixture do not complete them. The resources would be reviewed disposable
  installed-format/keystore fixtures, appropriate OS/game environments and bounded
  restore targets, not real player stores or an unsolicited paid runner.

Thus **VEL-015 is the only realistic immediate non-native closure established by
this audit**. This is a feasibility conclusion for the present acceptance-only
resources and scope, not a claim that all future source implementation or final
review work is impossible. No alternate three-checkpoint plan is substantiated.

The prior native run exposed activity-report work despite `send_stats=false`.
The subsequent consent, activity-snapshot and destination-binding fixes have
bounded source evidence, not a patched-binary GUI replay. Do not reuse the old
binary's hash or observations as acceptance of current source. The observation
report also does not certify zero installed-store/keychain/OS side effects.
Before any future native action, the lead must retain the reviewed isolation
conditions and verify the actual current artifact and network/data boundaries.
This audit authorizes no rerun or additional native build.

Recommended order:

1. Finish the lead's **complete VEL-015 rendered acceptance** and independent
   disposition. Fix findings within the lead's ownership; this reviewer edits none.
2. Do not chase unrelated security patches for percentage credit. Identify exact
   missing host/artifact/fixture resources for two additional complete gates before
   promising 65/76. Startup and diagnostics need other supported native webviews;
   baseline/installation additionally need complete runtime/install/game workloads.
3. If those resources remain unavailable, report the accepted total honestly and
   request a bounded acceptance-resource decision. No partial-credit splitting,
   denominator change, new milestone substitution, paid infrastructure, signing,
   release or access to real installed stores is authorized by the target.

Review-only conclusion: **85% is not presently supported by an executable
three-whole-gate closure plan.** The lead can make meaningful design-review
progress now; actual completion and any ledger award belong to subsequent
acceptance, not this feasibility audit.
