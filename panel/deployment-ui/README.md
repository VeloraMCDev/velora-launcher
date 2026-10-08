# Development deployment screen — Milestone 4 PARTIAL

Independent, dependency-free operator frontend for the Infra control API.
It shows registered repositories, immutable candidates, agent health and audit
pages; it can issue a one-use enrollment code and revoke an agent connection.
An optional, manually loaded deployment history shows business status and bounded
executor events from Infra's Development probe API. Opening or refreshing a
timeline makes one read request; there are no timers or automatic status polls.
History is not fetched during the initial page load, preserving existing enrollment
use while the new backend and deployment.read permission are rolled out.
Server-side Access and environment-scoped RBAC remain authoritative. This does
not replace the migrating Svelte product UI or start deployment jobs.

Requests use relative, same-origin API paths, no browser credentials or token
storage. Enrollment codes appear only in the current page, are cleared on hide,
navigation, expiry or session loss, and are never logged or added to URLs.
Permission loss clears displayed records and invalidates concurrent reads.
Refresh is manual, keeping idle traffic at zero. Records paginate in pages of 20.

```sh
node --test deployment-ui/client.test.mjs
node --check deployment-ui/app.mjs
GITHUB_SHA=<full-source-sha> node deployment-ui/build.mjs
```

CI packages checksum-manifested static assets with the exact Panel source SHA.
Hosting requires the operator host's protected Worker to serve the verified
artifact and same-origin API. No live hosting or Hermes enrollment is claimed
by this source checkpoint. Deployment, promotion, backups, approvals and the
full Milestone 7 screen set remain deferred.

Deploy this history feature only with the API from Infra PR #4 and explicit
Development deployment.read authorization. A Workflow completing is distinct from
a service succeeding: BLOCKED and ROLLBACK_FAILED remain visible as unresolved
deployment states. Browser rendering uses text nodes, and session loss clears
deployment history alongside the existing records. No signing key, host command,
Cloudflare token or background connection is introduced.
