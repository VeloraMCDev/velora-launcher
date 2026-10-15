# Independent native host extraction review

Date: October 12, 2026. Source-scoped review only; no Cargo/npm builds, native application startup, release or live-store access performed. Read root instructions, MONOREPO.md, docs/BOUNDARIES.md, docs/VALIDATION.md and docs/DEPLOYMENT_TARGETS.md. Only this report and the companion persistence review are reviewer-owned outputs.

## Verdict and checkpoint recommendation

Latest integrated follow-up: independently reran the four command-surface tests together with two schema tests from launcher; all six passed. No IPC signature/name/registration change found after Panel/client integration. Current HTTP bodies are intentionally migrated and no longer subject to original extraction body identity. The complete latest lead native-final.log now records 70 host tests passed, zero failed/ignored, plus 19 shared tests; the earlier 68-pass/one-failure exact-ciphertext assertion was fixed and retained. This supersedes the pending-test wording below for inspected local log evidence, while final build exit/source provenance and native acceptance remain lead-owned. Current ordered extraction-file aggregate is `c2b2028b20b2d0b347dff4dedea1ad2a95a884cf4f9545462c019d9e8efa1af6`; commands.rs is `2efe4b14e14d2e8a3ace29f00c3fd603149a4533e89edcca98d30fc2717c494a`, lib.rs is `f68e2028793921a4917a89accb486d402491a7db19ea825c33515340d25e8de0`. The reviewed client/schema behavior is covered by their separate reports, not silently accepted under extraction equivalence.

Follow-up compiler-gate reconciliation: inspected `/private/tmp/velora-revitalization-50/host-integration.log`, which records a completed host build and **56 passed, zero failed/ignored**, before the latest bytes-writer refinement and active Panel/client integration. The previously missing guild-helper visibility compilation issue is resolved for that prior run; the gate is no longer awaiting its first successful compiler evidence. This does not prove the latest source compiles or passes. Final integrated tests and affected client/helper compatibility review remain conditional. The original extraction hashes/body-equivalence evidence below is historical and does not describe newer HTTP helper changes; this follow-up does not re-run its contract tests or accept client migration behavior.

The bounded command extraction is acceptable at the reviewed source snapshot. No must-fix extraction compatibility defect found. Recommend VEL-006 checkpoints 1, 2 and 4 for the explicitly bounded extraction: domain modules exist, the captured IPC contract is independently checked, and this independent source review is complete. Checkpoint 3 remains conditional on the lead's successful integrated host compiler/regression run against this source or a reconciled successor. Worker-reported npm success is not compiler evidence. No native/release acceptance is implied.

## Independent evidence

- Ran only the lightweight `node --test launcher/tests/command-surface.test.mjs` using the installed Node 24 binary: four tests passed, zero failed/skipped. The full 124 names, sync/async signatures, ordered argument names/types, return types, registration order and `commands::` facade paths match the embedded pre-extraction snapshot. Function and Tauri-generated macro reexports are checked, not merely counts. The deliberate negative signature cases pass.
- Independently verified `/tmp/velora-original-commands.rs` hashes to the embedded dirty-tree provenance SHA-256 `0371ee381c2600f7884c24d58f4020c1ebfd2deb82c5277af41c8360534f5e09`. It is not clean HEAD equivalence. Baseline HEAD is `4f639266a515c3cb05769b055a4772c4dcb89cb8`.
- Recomputed current top-level definition hashes against the retained snapshot, normalizing CRLF/LF and `pub(super)` helper visibility: 142 current definitions versus 144 original. The six changed bodies are exactly `select_account`, `remove_account`, `get_skin_profiles`, `save_skin_profile`, `delete_skin_profile`, `apply_skin_profile`; the two removed definitions are `load_skin_profiles` and `write_skin_profiles`. These are the separately reviewed persistence integration, not unexplained extraction changes. Every other captured definition matches. Legitimate persistence behavior changes mean current whole-body equivalence must not be claimed.
- Sixteen handler domains plus the internal Panel request helper module retain the facade. Helpers `Res`, `err`, `aerr` remain accessible through their parent; Panel helpers have sibling visibility. The lead's specific compiler fix is present: `commands/guilds.rs:107` declares `guild_response_error` as `pub(super)`, and `commands/progression.rs:2` imports it explicitly. This source fix looks correct; only the compiler can close the failed-build gate.
- `lib.rs` differs from HEAD only in module declarations, not registered command paths/order. Scoped tracked whitespace check passed; this is not a full untracked-file formatting or build check.

## Reviewed snapshot

SHA-256 of the ordered `shasum -a 256` output for `commands.rs`, `lib.rs`, lexically ordered `commands/*.rs`, and `launcher/tests/command-surface.test.mjs`: `20f911523c82a121b42e3a98b4308d80a48f404f12ab109730347544f5edd186`.

Key individual SHA-256 values:

| Path (under launcher/src-tauri/src unless noted) | SHA-256 |
| --- | --- |
| commands.rs | 69362f0aafde1b1cdda66772ef763a19c8f036edf8275c7d822a957b3bda4c26 |
| lib.rs | da8c5cc9b937bafdf613453b19f7775875ce2795effd7bfc5253697effab7bb6 |
| commands/accounts.rs | 017ddce5e6872507e327f83955497846ebb705e02a8d30fcd1fe27ccd60133ec |
| commands/profiles.rs | 7f745a56ea09bb328c7658fa764bb0f025ad11dde51d2a6a091e808a5b5297b3 |
| commands/guilds.rs | 365b9dbcdfb1d77812731c592c8f56726c568328896d3c753164671a119600fb |
| commands/progression.rs | c9cd5266a7b0c11878d7961d8529d70539981de33caf9c62a93d869e83676abe |
| launcher/tests/command-surface.test.mjs | 2c7c517a7f6dfc10280f1cfb5863802448bd0d36979e932f943dd686f46feb57 |

## Limits and re-review triggers

The textual detector is tailored to current Rust declarations; it is not a Rust parser, macro expansion check, serialized DTO schema proof or native IPC exercise. It does not freeze command bodies. Shared/client type and Panel request integration still in flight is outside this verdict; changes to reviewed handlers, DTOs, helper imports, error behavior or the snapshot require focused reconciliation/re-review and a new integrated compiler run. Persistence findings are in `reports/account-profile-persistence-review.md` and must not be hidden by successful extraction checks.
