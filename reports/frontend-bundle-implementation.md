# Frontend chunk implementation and comparable measurements

October 13, 2026. Local uncommitted source on main, baseline HEAD
`4f639266a515c3cb05769b055a4772c4dcb89cb8`. This is emitted-byte accounting,
not a native startup, Core Web Vitals, CPU, memory or installation benchmark.

## Implementation

Home, Login and Setup remain eager. Nine secondary pages use the fixed local
dynamic-import registry in `launcher/src/lib/pages.ts`. `PageLoads` coalesces
requests by page key, retains loaded modules and evicts rejected promises. It does
not cache account/instance/page state. App retains its instance/view key and feature
gates. `LazyPage` announces loading, displays a static safe import-failure message
and moves focus to its stable region before a best-effort retry. Module evaluation
failures are not guaranteed recoverable in the same webview.

A mounted synthetic fixture exposed an actual Svelte `state_unsafe_mutation`
failure when the injected loader ran synchronously inside a derived expression.
The loader now executes in a promise callback after capturing the page/loader.
The mounted fixture then observed latest-page-only mounting, failure/retry focus
and no late mount/alert after destruction. These are controlled component fixtures,
not real chunk-fetch failures, pending App logout or native webview acceptance.

## Comparable byte evidence

Both distributions were measured with `node scripts/bundle-metrics.mjs <dist>`
using Node `24.21.0`, zlib `1.3.2.1-motley-8002e91`. Baseline was captured before
this batch's lazy-route changes at `/tmp/velora-revitalization-75/baseline-dist`.
Final is `launcher/dist`, built after the Settings tabindex/keyboard fixes.
Historical Vite-console gzip figures use different accounting; do not mix them
with the script's comparable Node gzip values below.

| JavaScript scope | Baseline raw / gzip bytes | Final raw / gzip bytes |
| --- | ---: | ---: |
| Entry only | 814453 / 222833 | 248842 / 72564 |
| Initial static closure | 814453 / 222833 | 470361 / 125795 |
| All emitted JS | 881396 / 245651 | 914716 / 268970 |
| Initial / all file counts | 1 / 9 | 6 / 24 |

The initial static JS closure decreases **42.25% raw / 43.55% gzip**. All emitted
JS **increases 33320 raw / 23319 gzip bytes**. Splitting defers work; it does not
reduce total product code. The entry alone is not the complete boot payload.
CSS/fonts and dynamic imports triggered by runtime/mock behavior are outside the
static JS closure. No transport compression configuration is certified.

The baseline has no manifest, so the script follows HTML entry/modulepreloads and
static JS imports. The final manifest is available and cross-checked. Metrics tests
cover missing imports, escaped build paths and closure traversal; the script is
not a general JS parser or a guarantee about arbitrary future bundler syntax.

Final entry `assets/index-C0FfodUR.js` SHA-256:
`458daa95b14a20f503ecc504a2f936553b1013cf4085265ff513a3b44b5dae8a`.
Final manifest SHA-256:
`16128bf20d411e9fa1429184d1d555f65fb81550362227fdc5e0f6bfe90c0b45`.
Local baseline/final JSON and build logs remain under
`/tmp/velora-revitalization-75`; these are local evidence, not checked-in artifacts.

## Validation and gates

Final frontend: **98 tests passed**, zero failures; Svelte check has **0 errors,
0 warnings**; runes check and production build pass. This includes four page-load
and three bundle-metrics tests. Node's experimental VM warning is expected for the
production-module fixture linker, not a Svelte compiler warning.

Browser paint timings from hidden/shared previews were unusable and are discarded.
Supported-OS/native startup and route regression acceptance remain open. Do not
award VEL-013 checkpoint 4 or any VEL-014 preparation-performance credit.
