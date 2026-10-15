# Performance baseline

## Follow-up sync profile — October 13, 2026

`preparation-path-profile.md` records five controlled synthetic samples through
actual sync: cold logical install, same-revision warm, deep verification and
one-file repair, with separate metadata/fingerprint helper timings. No production
optimization, Java/process timing or before/after speedup is claimed. This is a
new bounded discovery baseline, not closure of VEL-014's complete profile gate.

## Comparable pre-lazy-route baseline — October 13, 2026

Captured batch baseline and final metrics are in `frontend-bundle-implementation.md`.
Use its matched Node 24/zlib static closure measurements for chunk comparisons.
The older uncontrolled browser timings below are historical observations only;
hidden/shared preview timings are not evidence of a boot improvement.

## Historical initial observations

Baseline source: `4f639266a515c3cb05769b055a4772c4dcb89cb8`. macOS arm64, local Vite production preview, browser viewport 1280×800, synthetic `?mock=main`. No native game/profile/store access.

## Recorded observations

| Measurement | Value | Scope |
| --- | ---: | --- |
| Main JS | 809.49 kB / 223.09 kB gzip | Vite build output, decimal units |
| Main fixture JS | 29.39 kB / 9.99 kB gzip | Separate synthetic mock chunk |
| Casino fixture JS | 15.45 kB / 5.88 kB gzip | Separate mock chunk, not loaded by main observation |
| Build time | 2.22 s | One initial run, Node 25.6.1; not comparable benchmark |
| First production navigation DOMContentLoaded / load | 134.6 / 140.8 ms | One local observation; cold OS/browser cache not controlled |
| First production navigation FCP | 176 ms | Synthetic browser fixture only |
| Repeat navigation DOMContentLoaded / load | 26.9 / 31 ms | Reload, cached assets |
| Repeat navigation FCP | 80 ms | Synthetic browser fixture only |
| Main JS first / repeat transferSize | 221,423 / 300 bytes | Resource Timing, preview transport/conditional cache |
| Browser used JS heap | 32,992,597 bytes | One browser snapshot; not native RAM or steady-state usage |

## Reproduction and limitations

In `launcher`: use Node 24, `npm ci --no-audit --no-fund`, `npm run build`, `npm run preview -- --host 127.0.0.1 --port 1421`. Navigate to `http://localhost:1421/?mock=main`, collect Navigation/Paint/Resource Timing, then reload and repeat at the same viewport. Build output gzip estimates and browser transfer sizes are different measurements.

These are observations, not a statistical cold/warm startup benchmark. Mock avatar lookup to reserved `.example` fails DNS and must not be mistaken for a production Panel outage. No speedup or regression is claimed. The main chunk warning predates changes. Source-size inventories do not attribute bundle bytes to particular features.

Pending: controlled multi-run cold/warm native startup, idle/active CPU/RAM, cold/warm install and verification, download throughput/retry bytes, cache size, time to launch, loader preparation, and matched before/after measurements. Use synthetic stores, controlled origins and disposable installations.
