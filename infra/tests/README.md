# Infrastructure tests

Run npm test from infra/ for synthetic Worker/D1, candidate registration, source
policy, artifact and workflow-contract checks. Typecheck the Worker with
`npx --no-install tsc -p control-plane/tsconfig.json`.

game_auth_gateway.py exercises native components
against new synthetic stores; see [the recipe](../recipes/game-auth-gateway/README.md).
Native agent tests use Infra's Cargo workspace. Full Panel image checks live under .github/scripts at the repository root.

The monorepo's maintained source gates are scripts/check-publication.mjs,
scripts/check-secrets.mjs and scripts/check-boundaries.mjs at the repository root.
The superseded per-repository extraction checker has been removed. Tests never
certify live deployment, operator data conversion or production restore behavior.
