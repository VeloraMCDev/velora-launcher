# Documentation

Application guides, imported migration and deployment records, component history and security/publication policy. SITE.md preserves the former documentation repository introduction.

## Local checks

```sh
npm ci --no-audit --no-fund
npm test
npm run check
npm run build
```

Run from this directory. The generated site is static; do not deploy private migration, operator or security documents publicly without reviewing the selected content. Root CI is disabled to conserve Actions minutes.

See the root README for ownership, deployment and publication status.
