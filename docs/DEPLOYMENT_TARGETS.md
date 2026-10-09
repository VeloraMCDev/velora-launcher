# Deployment targets and minimum-cost approach

Reuse existing Cloudflare resources and host capacity. Do not provision paid plans,
duplicate environment resources or new VPS machines during source consolidation.
Source CI is enabled. Packaging and deployment require explicit manual promotion;
these assignments describe the code, not live deployment proof.

| Component | Current implementation / intended home | Domain requirement |
|---|---|---|
| Deployment authority | `infra/control-plane`: Worker, D1, R2, Workflows | Protected operator/API hostname; preserve Access and machine ingress separation |
| Deployment Panel | `panel/deployment-ui`: protected static assets | Share the operator hostname; no additional server |
| Documentation | `docs`: generated static bundle | Optional docs hostname after content review; no dedicated VPS |
| Complete Panel backend | `panel/server`: native Rust/Axum, SQLite, filesystem | HTTPS API origin; native host/VPS until a reviewed Workers/storage port exists |
| Panel web apps | `panel/web`: static frontend | Share Panel origin or use selected Cloudflare static origins; review CORS/cookies/paths before splitting |
| Standalone Authentication | `crates/auth-service`: native Rust, owned SQLite/keys | Reachable auth origin, possibly gateway paths on a shared hostname |
| Minecraft servers | Long-running Java workloads | Reachable host/IP and port; friendly DNS/SRV optional |
| Native deployment agent | `infra/agent`: outbound HTTPS service | No inbound public listener or domain |
| Launcher/integrations | Desktop installers and Java artifacts | Share existing Panel/R2 download origin |
| SDK/private libraries | Build inputs | No independent hosting/domain |

The complete Rust Panel/Authentication is not yet a Workers/D1 implementation.
Do not substitute D1 for owned stores without migration and continuity tests.
Cloudflare already runs the separate deployment authority.

Development uses an existing operator-managed host. Beta and Production hosts
and final domains remain unresolved. Keep live addresses, resource IDs, keys and enrollment details
in ignored operator configuration.

Before promotion: validate exact images on Docker, register the canonical source
and immutable workflow policy, verify service/schema/backup contracts, prove
agent job transport, test failed rollback and restore, and confirm host/domain
assignments. Successful Development probe evidence does not complete these gates.
