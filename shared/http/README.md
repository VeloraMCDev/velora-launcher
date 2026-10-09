# Browser HTTP transport

Neutral TypeScript client for platform APIs. Authentication/session transport and
wire contracts belong here; gameplay UI and policy remain outside this package.
The maintained source is consumed locally by the applications, without fetching
another repository. Snapshot/provenance metadata retains its original attribution.

From this directory with Node 24:

```sh
npm ci --no-audit --no-fund
npm test
```

This is a library, so it needs no independent host or domain. Keep wire compatibility
and instance scoping when updating it; validate affected application consumers.
