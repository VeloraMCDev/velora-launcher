# Independent authority and accessibility review

October 14, 2026 (America/New_York). Bounded review of the new instance-manifest authority behavior, shared filled-action contrast, selected accent swatches and Settings tab scrolling. This is not a review of all accumulated checkout changes.

## Verdict

**Accept the bounded source implementation; no blocking defect found within this slice.** The authority comparison, flattened-format compatibility, typed failure policy and contrast direction are implemented rather than merely asserted in reports. Independent focused checks pass: **15 Rust tests and 13 frontend tests**. These are overlapping subsets of the lead's suites, not additional product test counts.

No full-cache, all-theme, WCAG, native, OS, game, keystore, installed-upgrade or release acceptance is awarded. The fixed ledger remains **62/76 (81.58%)**. VEL-012 checkpoint 4 and the complete rendered/native accessibility gates remain open. Nothing in this report changes their definitions or the denominator.

## Review method and boundaries

Read the root instructions, MONOREPO, boundaries, validation and deployment targets. No more-specific launcher/reports AGENTS file applies. Inspected the specified production files and both frontend tests, including all manifest-cache regressions. Inspected adjacent `persistence.rs`, `panel_client.rs`, account/secret lock ordering, state Panel selection and the existing contrast helper to check the actual dependencies of the new claims. Compared the two implementation reports with source; inspected retained test results rather than treating their prose as acceptance.

Only this new report is written. No production/test/ledger changes, Cargo workspace build, browser session, real credential/player-store access, native launch, game, purchase, deployment, branch or commit occurred during this review.

## Manifest authority findings

- **Request-bound identity is coherent.** `instance_manifest` captures a Panel string, then reads the active account and that account's secret while holding the account read lock. The cloned token and derived authority are used by the same request. Account writes inspected take the account lock before the secret lock, consistent with this order. A panel-kind account with a different explicit Panel URL does not donate its bearer to the selected Panel; missing legacy account Panel URLs retain the previous compatibility policy. The Panel settings lock is not held across the operation: this is an immutable request snapshot, not a globally frozen application state or a transaction covering the subsequent game-auth/install pipeline.
- **Origin/base-path comparison agrees with the actual client.** Both normalize the parsed HTTP(S) URL to a trailing-slash base, reject userinfo/query/fragment and retain tenant path case. Parsed URL normalization handles host/default ports. `PanelClient::request` joins below the base's `api/v1/` prefix and checks origin/path containment. The cache fingerprint includes Panel base, account ID/kind/UUID and credential presence; both fetched and recovered instance IDs are checked. It contains no bearer or game token.
- **Fallback is narrower than a generic request failure.** Only typed transport failures with absent or successful HTTP status can invoke recovery. 401/403/404, other HTTP failures, decode/config errors, untyped errors and instance-identity mismatches cannot fall back. A truncated successful response may still use the authorized cache under the retained transport policy. Successful online responses do not read old cache data to replace their result.
- **Known denials revoke matching local provenance when persistence succeeds.** Actual production-caller loopback tests exercise 401, 403 and 404 followed by refused-network fallback. Invalidation clears only a matching authority and preserves the legacy-readable payload. Other authority scopes cannot invalidate it through this helper. A later successful online fetch can publish fresh provenance.
- **Compatibility is deliberate, not implicit trust of legacy data.** New version-1 fields sit alongside the unchanged flattened manifest payload and destination. A regression deserializes newly published bytes using `InstanceManifest`. Valid old flattened files can be atomically upgraded by an online response, but cannot be used offline before that refresh. Corrupt files are not silently overwritten, even when an online response succeeds; that response can still be returned without caching it. This is a security restriction on legacy offline availability, not transparent legacy offline equivalence.
- **Publication avoids truncation and casual symlink following.** The existing persistence helper validates the destination, creates an exclusive unique sibling (0600 on Unix), writes/syncs/closes it, checks temporary-file identity on Unix, validates again and renames. Failed writes clean up the temporary. Cache code checks the immediate parent before/after creation. Destination and immediate-parent symlink tests preserve unrelated targets. Read and invalidation use the same regular-file validation rather than following a deliberate destination symlink.

## Nonblocking authority gaps / remaining security gates

1. **Failed invalidation is not a durable revocation fence.** It logs a generic warning and the current denied request fails, but old matching provenance may remain usable on a later transport failure, including in the same process. The limitation is not confined to restart. There is no in-memory deny tombstone, signed lease or offline revocation discovery. Acceptable for this disclosed bounded change; it blocks any stronger claim that every observed denial permanently prevents fallback despite write failure. A targeted invalidation-write-failure/retry regression is still useful.
2. **Credential presence is not a credential generation.** Replacing a token while keeping the same account identity and a present credential leaves the cache fingerprint unchanged. A request is consistent with its captured account/token, but concurrent account/settings changes do not cancel it, and no epoch/ordered-response protocol is tested for concurrent publication versus denial. Do not equate this fingerprint with current server authorization or claim an atomic snapshot of every application identity store.
3. **Local/hostile filesystem protection is incomplete.** An attacker able to edit cache/identity stores can forge provenance. Immediate-parent checks do not prevent ancestor symlinks, active directory swaps, destination read-check races or hard-link/local-file manipulation. The writer syncs the file, not its parent directory; atomic visibility is not certified power-loss durability. No hostile filesystem matrix, Windows rename/symlink matrix or crash recovery was exercised here.
4. **Synchronous filesystem work runs on the async caller.** This keeps each read/write/rename owned inline rather than detaching Tokio filesystem work, but may stall a runtime worker on a slow filesystem. The remote response is size-bounded by the existing client; local cache reads have no new size limit. No latency/stress or adversarial oversized-local-cache guarantee follows.
5. This slice does not audit all HTTP redirect behavior, transport trust, downstream file URLs, remote commands, content-cache cleanup/TTL or the full install/game-auth pipeline. The authority names the configured Panel base, not a signed attestation of the final network responder. Existing transitive policies are not newly certified by these tests.

## Accessibility findings

- `applyTheme` derives opaque filled-action background and black/white foreground from the actual selected accent. Unsupported accent formats use the helper's safe fallback. The highlight is white for black text and black for white text, so the new 12% mix moves the background away from the chosen foreground rather than lightening every dark button under white text. Every channel moves in the contrast-safe direction for the explicitly sRGB mix; the normal stop already meets 4.5:1.
- Both global primary declarations now consume those variables. The later gradient declaration wins for the normal background (and also supersedes the earlier equal-specificity solid hover declaration); it retains the safe endpoints. Hover still changes the shadow. This is not an unverified claim that hover always becomes a separate solid highlight. The sample test checks sRGB highlight mixtures, not every browser's computed gradient pixels/interpolation/compositing.
- Selected predefined swatches use inline foreground color from the real helper, overriding the component's older white default; the check icon inherits it. `aria-label` and `aria-pressed` remain present. The SSR fixture imports the actual helper, not a replacement with a guaranteed answer.
- Settings' bounded-height flex layout now permits the category column to scroll (`min-height: 0; overflow-y: auto`) and prevents ordinary tab targets shrinking. Existing arrow/Home/End navigation focuses and activates the target; focus uses the browser's normal scrolling behavior, without `preventScroll`. Compact layout remains separately wrapped/scrollable. The new regression is a source contract for these CSS rules, not a runtime scroll-distance assertion.

The lead's retained rendered report records the previously clipped About target at 130%/960-by-600 becoming visible after End, pale swatch/Sign in colors and modal/motion observations. These are lead-executed observations, **not an independent browser rerun in this review**. The source and independent SSR/contracts support the specific fixes, not the whole rendered matrix. Arbitrary admin CSS and page-specific overrides can defeat shared tokens; disabled opacity, every error/modal state, focus-ring contrast, compact/native keyboard behavior, system motion preference and screen-reader traversal remain unaccepted. No all-theme or full accessibility checkpoint is inferred.

## Independent validation

Executed on the unchanged reviewed source, reusing the existing target:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib game::manifest_cache::
# 15 passed; 0 failed; 87 filtered out

# From launcher/
node --experimental-vm-modules --test tests/action-contrast.test.mjs tests/accessibility.test.mjs
# 13 passed; 0 failed; 0 skipped
```

The frontend repeat used the installed **Node v25.6.1**, not the Node 24 specified by `docs/VALIDATION.md`; it is supplementary focused evidence, not a Node-24 compatibility claim. The expected experimental VM Modules warning appeared. No dependency/toolchain was installed to obtain the repeat. Rust reused compiled artifacts and ran disposable synthetic fixtures; no native keychain was opened.

Inspected the lead's retained whole-host result (102 pass) and frontend result (101 pass). The separately reported non-test check/typecheck/runes/build are not independently rerun here. No broad build was necessary for this report-only review.

## Reviewed SHA-256 snapshot

Whole-file hashes identify reviewed bytes; review of `game.rs`, CSS and Settings remains limited to the specified new behavior and its immediate context.

| File | SHA-256 |
| --- | --- |
| `launcher/src-tauri/src/game.rs` | `cd7a144bf1972e6b5278dab340f1205a1de6a8fdaddb76904166e9acdb28cd4c` |
| `launcher/src-tauri/src/game/manifest_cache.rs` | `ae2f6890c6ec5b6c89cc47cedc0793b657a10361220542babbd6a3bde573dcd0` |
| `launcher/src/lib/theme.ts` | `75efe071a2d4a6740dfa82bfd6bb0169440f392c9ed02d004ad180b508d2e644` |
| `launcher/src/app.css` | `81fb32e8439e7b71c668da251cf6f74eb9713f027ed010057a2d014172206e0a` |
| `launcher/src/pages/Settings.svelte` | `915d695afe7ab5db3fc234dd3277691b5f9277975b91294a728d013d11894457` |
| `launcher/tests/action-contrast.test.mjs` | `1bbc8d288fe31ddb2219a4e9ad75c45d101316dce8654829d0b50dd314e1cd3f` |
| `launcher/tests/accessibility.test.mjs` | `b42dcdb4bd9627ad08810dad25dd9efa72ced3b737b25cb667d97d84359d739f` |
| Adjacent `launcher/src-tauri/src/persistence.rs` | `2ab379a9089283bef8548b8101b3d518af3b3d4a649a9dee23f8ab71e7aee89e` |
| Adjacent `launcher/src-tauri/src/panel_client.rs` | `0297efe0135d042cca1badf4786c8aeea6392b85135382e662e9c16209be8b8f` |

Retained lead artifacts under `/tmp/velora-revitalization-20261014/`:

| Artifact | SHA-256 |
| --- | --- |
| `manifest-authority-integration.log` | `b72f599697d27b425e49df085fd6b8918838c58b2dbf2a6e01821ce036d2e1e3` |
| `manifest-authority-final-host.log` | `9f5a5c57a695ad95fa8f2eb114df133482000c2da1aa3bb83a9f71feb2fcea09` |
| `accessibility-followup-tests.log` | `d4e7a5ec2d13fc90f1a5175191f306f4d63f0b09028d1ac0b3f695762ea83e81` |

Independent command results are retained in this review session's tool output; the artifact hashes above identify lead logs, not newly manufactured independent logs.
