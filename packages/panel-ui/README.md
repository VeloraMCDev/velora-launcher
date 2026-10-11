# Reusable Panel login UI

Uses the canonical ../../shared/http platform client. Sign-in and recovery behavior remain covered by imported tests. Hosts may pass an optional `iconRenderer` Svelte component receiving `name`, `fallback` and display props to customize submit and busy icons; default Lucide icons remain available.

Run from this package directory (Rust workspace commands may also run from root):

```sh
npm ci --no-audit --no-fund
npm run check
npm test
```

Preserve attribution/provenance. See ../../docs/BOUNDARIES.md and the publication audit before distributing source or artifacts.
