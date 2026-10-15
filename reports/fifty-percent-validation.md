# 50% milestone validation

October 12, 2026. Lead-observed local results after integration, review fixes and
scoped formatting. Local main HEAD remains
`4f639266a515c3cb05769b055a4772c4dcb89cb8` with uncommitted changes.
A fresh `git fetch origin` and `git rev-list --left-right --count
HEAD...origin/main` return zero divergent commits. No remote writes occurred.

## Final executed checks

Node 24.21.0, Rust/Cargo 1.98.1, Gitleaks 8.30.1. Fixtures use synthetic local
HTTP/filesystem/IPC data, not live player/operator stores. No native app/game,
Java integration, packaging, signing, release, deployment or paid service ran.

| Command/scope | Final result |
| --- | --- |
| `CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j 1 -p velora-launcher -p velora-launcher-core -p velora-platform-utils -p velora-shared` | Exit 0: 70 host + 65 core + 10 utility + 19 shared = 164 passed; zero failed/desired-gap ignored; seven separate preexisting online tests ignored; doc tests pass |
| Launcher `npm test` | Exit 0: 54 passed, zero failed/skipped; expected experimental VM Modules warning |
| Launcher `npm run check` | Exit 0: Svelte zero errors/warnings |
| Launcher `npm run check:runes` | Exit 0 |
| Launcher `npm run build` | Exit 0: main JS 814.45 kB / 224.60 kB gzip; preexisting large-chunk warning |
| `CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked -j 1 -p velora-panel` | Exit 0; preexisting dead-code warnings for op and clip |
| Scoped `rustfmt --check --edition 2021 --config skip_children=true` | Exit 0 on command facade/modules, Panel client, profiles, persistence, account persistence tests, loader compatibility tests and shared contracts; not whole-workspace formatting |
| Boundary, identity, workflow, publication and whitespace checks | Exit 0; identities unchanged; publication inventory PASS/REVIEWED, not content certification |
| Tracked-source Gitleaks | Exit 0: 1535 tracked files, 6 exact reviewed matches, zero unreviewed findings |
| Copied nonignored untracked-source Gitleaks | Exit 0: 87 files, zero findings; no staging needed |
| Local Markdown links | Exit 0: 155 Markdown files, 101 local links, zero failures; external URLs/anchors not certified |

Raw supporting logs are local under `/tmp/velora-revitalization-50/`:
`native-postformat.log`, `frontend-postformat.log`, `panel-postformat.log`,
`format-final.log` and `source-postformat.log`. Temporary logs are not committed
runtime inputs. Earlier pre-format executions/review hashes are historical
snapshots; the post-format runs validate the actual final code tree.

## Failures resolved without weakening contracts

The first integrated host run exposed a missing cross-module helper import.
Later, an actual Discord poll/account-publication regression failed because
credential rollback re-encrypted identical plaintext with a new nonce. Restoring
the original ciphertext bytes (or original absence) fixed the production behavior;
the strong exact-byte assertion remains. Rollback failure retains both errors.
Frontend independent review identified older-success/newest-failure divergence;
queue-settled authoritative bootstrap and manifest reconciliation now close it.
Client/schema reviews required static redacted failures and bounded decoding.

After rustfmt, the IPC detector rejected harmless multiline signatures and
single-name exports. The parser now accepts both Rust export forms and normalizes
signature whitespace/trailing commas on both observed and frozen snapshot values.
It still asserts all 124 command names, argument/result types, async modes,
registration order and macro exports, with positive formatting and deliberate
negative mutation assertions. Frozen contract/provenance values were not changed.

## Checklist reconciliation and acceptance boundary

Previous total: 20/76. Added credit: downloads +1, persistence +1, host extraction
+4, Panel client +4, frontend domains +1, bounded schemas +4, core boundaries +3.
Final total: **38/76 = 50%**. Denominator and checkpoint definitions are unchanged.
This is equally weighted engineering deliverable credit, not effort, release
readiness or budget compliance. See `completion-checklist.md` for per-item evidence.

Independent review pairs cover extraction, account/profile persistence, Panel
client, shared schemas, frontend domains, core boundaries and download policy.
Their recommendations are bounded source acceptance; reviewers did not duplicate
the final full suite. Final rustfmt edits are formatting-only; test-parser changes
affect the detector, not production IPC. Reviewer snapshot hashes do not attest
to the final formatted tree. Original extraction body-equivalence evidence predates
intentional persistence/client changes and is not whole-tree equivalence today.

Remaining gates: durable interrupted-sync/cross-store journaling/restart recovery,
directory fsync/power loss, shutdown/background/child-process quiescence,
cross-process/handle-bound containment, active preparation identity after reload,
future schema/ACL/keystore continuity, installed upgrades/restore and supported-OS/
native loader acceptance. Remaining stable Value endpoint families are not fully
modeled. Existing all-error instance cache fallback is not auth-revocation safety.
Custom HTTP clients own intermediate redirect policy. Synchronous download writes
need slow-storage/runtime profiling; no native or warm-sync speedup is claimed.

No commits, branches, worktrees, PRs or deployments were created. Actual billed USD
and remaining balance remain unknown. The milestone does not close Phase 1 or
authorize a release.
