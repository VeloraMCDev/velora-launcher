# Velora artwork

The project owner supplied the V emblem (`velora1.png`) and Velora wordmark (`velora2.png`). `originals/` preserves their exact bytes and records SHA-256 hashes and dimensions. SVG files are raster wrappers, not vector redraws. The padded icon reserves space for platform masks.

Artwork and trademarks are excluded from the platform MIT license. These assets identify the official Velora distribution; no separate reuse license has been granted.

In the source host, run `node scripts/migration/import-brand.mjs`, then `node scripts/icons/build.mjs` to regenerate web, desktop, mobile and companion icons. Keep the original artwork unchanged.
