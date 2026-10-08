# Deployment implementation

[The deployment guide](DEPLOYMENT_GUIDE.md) is the operator's implementation
contract, copied byte for byte from the source workspace's
`llm-instruction/deployment-guide.md` on 2026-10-07. Read it alongside
[current migration progress](../migration/CURRENT_PROGRESS.md) throughout each
development milestone. User corrections take precedence over the frozen guide.

Canonical implementation homes: Infra owns the control API, D1 migrations,
Cloudflare Workflows, contracts and native agent; Panel owns the browser client.
The original game/product Panel control plane is distinct from this new
infrastructure deployment authority. No ninth repository is needed.

Milestone 0 inspected all eight owners and reran their seven existing unchanged
CI pipelines. All passed; Docs had no pre-existing build/CI and was classified
incomplete. Exact revisions, commands, output classes, configuration names,
workflow evidence and gaps are in Infra's `deployment/repository-audit.json` and
`deployment/repository-catalog.json`. The guide checksum is recorded in the audit.

Milestone 1 begins with action pinning, documentation source checks and a harmless
Development probe with exact build identity. This does not complete application
migration or establish the deployment authority. The Development binding-check
Worker and resources are now deployed; see the [Development baseline checkpoint](MILESTONE_2.md).
The [candidate API checkpoint](MILESTONE_3.md) records the tested Development
registration/RBAC API and its live CI proof status.
The [agent enrollment checkpoint](MILESTONE_4.md) records the native agent,
protected enrollment screen and remaining real Hermes verification.
Hermes is operator-confirmed Linux Mint with Docker;
target native systemd with outbound HTTPS, using SSH only for optional bootstrap.

Docs source verification and static output use Node 24 with locked dependencies:

```sh
npm ci --ignore-scripts
npm test
npm run check
npm run build
npx --no-install wrangler deploy --dry-run
```

The build renders sanitized Markdown, local routes and heading anchors into `dist`.
It copies only documentation and named brand assets, adds restrictive static headers,
and stamps every page with the full source SHA. `artifact-manifest.json` records each
file's size/checksum and a deterministic content-manifest checksum. That checksum is
not the checksum of GitHub's uploaded ZIP; the upload action reports that separately.
CI retains the source-named static artifact for 14 days. A repeated clean build at the
same SHA is deterministic. Tests exercise unsafe markup, broken links and stale output.
External links, code examples and every heading anchor are not yet certified.

`wrangler.jsonc` prepares Workers Static Assets for Development and disables default
public/preview endpoints. No account, zone, custom route or API token is committed.
Dry-run validates packaging locally; it does not provision or deploy Cloudflare
resources. The checkpoint records the live exact-artifact deployment, resource
bindings and Access policy. Durable release/candidate registration remains pending. Do not rebuild
these assets when promoting a registered release to another environment.
