# Harmless Development deployment probe

This stateless service is the first typed `AGENT_COMPOSE` target for deployment
guide milestone 5. It manages no game data, database, credentials or host commands.
GET `/health`, `/health/live` and `/health/ready` return baked service/version/
commit/build identity. The caller must compare the expected candidate SHA; HTTP
200 alone is insufficient. Other paths/methods fail. Shutdown drains connections.

Build/test locally with Node 24 from Infra root:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run test:deployment
node scripts/check-workflows.mjs
```

Docker validation in `deployment-baseline.yml` builds a disposable test image,
then checks the real container without root, writable filesystem or network.
The pinned official Node 24 Alpine index digest was verified from Docker Hub on
2026-10-07. No mutable base/tag is authoritative for release deployment.

`release-probe.yml` is manual and main-only. It tests, builds once, checks the actual
image, pushes those same bytes to GHCR, resolves the image digest, validates candidate
metadata and checksums the JSON handoff. Its token is job-scoped `packages: write`;
it has no Cloudflare or node credentials. Transient handoff retention is 14 days.
The immutable OCI locator, not the convenience tag, is the deployable identity.

After successful publication, a separate opt-in OIDC job can register that exact
image as a Development candidate. It receives only contents-read and OIDC token
permission. See [registration activation requirements](../../control-plane/REGISTRATION.md#main-branch-handoff-2026-10-08).
Registration does not activate a container or grant an agent additional capability.

The first release passed in [run 37729741128](https://github.com/VeloraMCDev/infra/actions/runs/37729741128)
at source `ecf17d815897f9036f1480857858c4bd1c3e561a`, publishing digest
`sha256:fd1f2f71f254fb540c57c6d0b7bca038fc2e533ac2f0bf770951f610f1c89189`.
Candidate JSON alone is not authenticated control-plane registration. The live
probe service/CI policy still requires activation configuration. No service has
been activated on Hermes. No Production target is allowed by `service.json`.
Milestone 5 and subsequent recovery work still must prove signed execution,
expected-SHA health, failure rollback and separately approved data restoration.

Source/image metadata belongs to Infra. Future environment topology must remain
separate from the service contract. Development/Beta/Production promote previously
verified artifacts rather than rebuild them. This probe initially targets Linux
amd64; agent/node capability checks must reject incompatible architecture.
