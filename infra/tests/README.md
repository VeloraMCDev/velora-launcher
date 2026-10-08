# Component acceptance and public source preflight

`game_auth_gateway.py` and `container_game_auth_gateway.py` run real game
Authentication/Gateway components against new synthetic stores. See the pinned
[development recipe](../recipes/game-auth-gateway/README.md). These tests never
use an operator installation or existing account data.

`public_source_preflight.mjs` accepts prepared public repository checkout paths:

```sh
node --test tests/public_source_preflight.test.mjs
node tests/public_source_preflight.mjs ../sdk ../authentication ../panel ../launcher ../minecraft-integrations ../docs . --output .runtime-checks/public-source-preflight.json
```

It reads tracked HEAD manifests/workflows, refuses mandatory private Experiences
dependencies and paths outside each checkout, validates tracked snapshot hashes,
and flags database/binary artifacts unless exact bytes have documented public
upstream/license provenance. It scans each reachable local Git blob once for
high-confidence GitHub token and PEM private-key material patterns. Candidate
credential contents are never emitted; credential-bearing remote URLs are redacted.
Experiences itself is refused as an audit input. Synthetic tests cover history
retention after deletion, drift, private dependencies, path escape, remote redaction
and reviewed artifact byte changes.

This is a repeatable preflight, not full release certification. It does not inspect
unfetched refs, remote releases/LFS, generated bundles/source maps, full third-party
licenses, all credential forms or production data semantics. Registry availability,
anonymous whole-product builds and full migration acceptance remain separate gates.
The CI job tests the checker and audits Infra's own reachable local Git history with
read-only repository permissions. It fetches no private implementation.
