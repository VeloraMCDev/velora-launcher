# Independent account transaction I/O and startup review

October 14, 2026 (America/New_York).

## Verdict

**Accept this bounded source/process-restart slice: no confirmed blocking defect found in the reviewed transaction paths.** Independently repeated focused tests pass: **23 transaction module entries and 9 encrypted restart integration entries**, zero failures. Each total includes one child dispatcher. This is not broader persistence/security or native acceptance.

The startup root guard is an intentional production change: `Secrets::try_open` rejects a non-directory or direct symlink root before `load_or_create_key`. The named I/O hooks are test-only failures; their non-test implementations return `Ok(())`. They do not introduce a production environment-variable fault switch or change the transaction's publication/error policy. Accounts and credentials still publish sequentially, with the existing journal as the reconciliation authority.

Ledger remains **62/76**. No percentage, native credential-store, supported-OS, installed-upgrade, power-loss or whole-checkpoint credit is awarded. The user overrides the review budget; actual billed USD is unknown and is not inferred from runtime or test counts.

## Ownership and method

Read root `AGENTS.md`, `MONOREPO.md`, `docs/BOUNDARIES.md`, `docs/VALIDATION.md` and `docs/DEPLOYMENT_TARGETS.md`. Checked for nested launcher/report instructions; none were found. Reviewed the three requested transaction/test files and existing `account-transaction-review.md` / `account-transaction-implementation.md`, then inspected the adjacent secrets, account integration, startup and atomic persistence helper for ordering and error semantics.

Source review began while the lead owned Cargo. Focused execution began only after the explicit Cargo-idle notice, ran serially using the existing target, and finished before this reviewer's Cargo-idle notice. The lead may subsequently edit game manifest cache/state for a separate gate; those changes are outside this transaction review. The state hash below identifies the startup context inspected, not approval of later independent edits.

Only `reports/account-transaction-io-review.md` is authored by this reviewer. No source/test/ledger write, branch, commit, worktree, purchase, dependency change, deployment or real player/operator store access was performed. Tests necessarily use build output and disposable synthetic temporary stores; no native keychain access occurs under this Rust test configuration.

## Production findings

### Startup validation and identities

- `Secrets::try_open` calls `validate_root` before loading or creating the key, then recovers the journal before constructing the secret map. `AppState::try_new` initializes secrets before loading accounts. A recovery error propagates instead of admitting a mixed saved pair.
- `validate_root` uses `symlink_metadata(...).file_type().is_dir()`, rejecting a direct symlink even if it points at a directory. The new Unix integration regression calls both state startup and secrets startup through a symlink root, verifies failure, no key/ciphertext creation and an unchanged synthetic sentinel. It proves rejection before fallback-key creation, not native keychain behavior.
- `AppState::try_new` still calls `create_dir_all` first. This is not a descriptor-relative, race-free or ancestor-symlink defense; intermediate symlink components and hostile directory replacement remain outside acceptance. Likewise this slice does not harden every preexisting key/store read against leaf symlinks or hard links.
- Fixed stores remain `accounts.json`, `secrets.bin` and fallback `secrets.key`; journal remains `.account-transaction.json`, version 1. Credential service/account remain `net.scopenet.launcher` / `secrets-key`. The existing nonce-plus-ChaCha20-Poly1305 ciphertext format and account upsert identity remain intact. `accounts.rs` and inspected `state.rs` match their prior-review hashes.
- Tests compile the temporary fallback-key branch, not the native keyring branch. The integration fixtures retain the exact key bytes and existing account ID through rollback and committed replay. Account metadata in the journal is base64, not encrypted; credential images are existing ciphertext. Old ciphertext retention is not secure erasure, and the journal itself is not authenticated as a whole.

### I/O hooks and returned results

`io_fault` examines environment variables only under `cfg(test)`, requires an exact supplied root path match, and selects exact comma-delimited point names. Child fixtures pass their temporary roots and invoke the real commit/recover/account methods. The hook returns a synthetic `std::io::Error::other` before the named operation; non-test code simply returns success. `sync_directory` still performs the existing Unix directory sync after that hook and remains a no-op on non-Unix systems. Exact-root matching scopes fixture interference; it is not a production authorization mechanism.

| Boundary | Returned result / retained state | Interpretation |
| --- | --- | --- |
| Initial journal open/write | Error; no store publication. An opened, failed write can leave empty/partial journal evidence. | New fixture retains an empty journal and verifies recovery refusal without writes. Fail-closed manual repair can be necessary; automatic rollback is not promised for malformed evidence. |
| Journal file sync or prepared directory sync | Error; before pair and prepared journal remain. | Recovery can restore/confirm the before pair and clean evidence once the failure is removed. These operations occur before the publication-error recovery closure. |
| Secrets/accounts publication or committed-marker write before marker replacement | Error, even if rollback succeeds. | Recovery selects the prepared before pair. If rollback also fails, error gains rollback context and journal evidence remains for restart. In-memory candidates are not published on error. |
| Image directory sync after an image replacement | Error; a known image may already have been installed. | Immediate recovery is attempted. The persistent named hook can also fail recovery's image sync; the retained prepared journal makes retry possible, rather than proving that every returned error already restored disk state. |
| Committed directory sync after successful marker replacement | `Ok(())`, published after pair, committed journal retained. | Warning explicitly disclaims power-loss durability. Success means published in this running filesystem, not guaranteed stable storage after power loss. |
| Cleanup removal failure after committed publication | `Ok(())`, after pair, committed journal retained. | Cleanup warning; new credential reads and saved mutations remain fenced until successful restart recovery. |
| Cleanup directory sync after successful removal | `Ok(())`, after pair, journal absent in the live filesystem. | Warning only; no live pending-journal fence remains. This is not evidence that the removal survives machine failure. |

The marker is the decisive distinction, not merely whether both store replacements ran. `atomic_write_bytes` validates current evidence, creates a private exclusive sibling, writes and syncs that sibling, revalidates, and renames; it has no subsequent fallible operation after a successful rename. Thus the inspected marker helper does not return a normal post-rename sync error that would spuriously send a committed transaction down the prepared rollback path. Directory sync of that marker is deliberately separate and handled as published success. This conclusion is bounded to the current helper and ordinary filesystem/process semantics, not ambiguous device failure or hostile concurrent mutation.

### Recovery and read fencing

- Recovery rejects unknown fields/version, oversized/malformed images and nonregular journal/destination entries. Individual images are limited to 8 MiB and the journal to 48 MiB. Every account image parses as `AccountsFile`; every present secret image authenticates/decrypts with the existing key in production/integration fixtures.
- Both current destinations must match their own recorded before or after bytes, including recorded absence, before either recovery write starts. Recovery then revalidates each destination at publication. Changed evidence is preserved, not guessed at. `prepared` selects before; `committed` selects after. Mixed known pairs and repeated/interrupted replay remain recoverable.
- Account save/remove hold the account write lock while the secret mutex guards the transaction; both in-memory candidates publish only after transaction `Ok`. Account-only mutation uses `ensure_clean`. Standalone secret writes check the same fence before persistence; `Secrets::get` returns an empty/default secret while evidence is pending or the root check fails.
- The new real-encryption I/O fixtures verify that returned errors preserve the old in-memory account, post-marker success admits the new account, retained evidence blocks `get`/`set`/select/offline-add without store writes, and restart produces the phase-selected pair with unchanged ID/key bytes.
- Fencing prevents new credential retrieval, not revocation of token copies already returned to callers or reversal of remote server-side account changes. It is not an atomic pair snapshot for unrelated readers or a transaction protocol for multiple independent writers. Startup failure presentation/manual repair remains a native acceptance concern.

## Tests and evidence

Independent commands, after the lead's idle notice:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib account_transaction::tests::
# 23 passed; 0 failed; 0 ignored; 111 filtered out; 0.83s test execution
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib accounts::restart_tests::
# 9 passed; 0 failed; 0 ignored; 125 filtered out; 0.96s test execution
```

Both commands exited successfully in one serial shell session and used `target/debug/deps/velora_launcher_lib-0fa20be1932b3e4c`. Independent output is retained in the review tool transcript, not relabeled as a lead log. No independent full-host or non-test check is claimed. Scoped tracked-file `git diff --check` for inspected secrets/accounts/state passed; it does not check untracked additions.

The eight added module test entries cover initial journal write; prepared synchronization; publication/marker rollback; publication plus rollback failure; image synchronization; recovery write failures; committed sync/cleanup removal; and cleanup synchronization. Several entries iterate named points; they do not constitute exhaustive syscall fault injection. Together with the two new real-encryption I/O tests and prior symlink-root regression, the current source contains 23 module and 9 integration entries, including dispatchers.

Synthetic hooks are not actual disk-full, partial-write, short-write, file-sync, rename or removal failures inside every helper. The journal-write hook runs before any bytes are written, not midway through serialization. Recovery-hook coverage exercises prepared rollback; it is not an exhaustive committed roll-forward I/O matrix. Exact-root mismatch itself has no dedicated regression here. The existing process exits use bounded child deadlines; `exit(77)` is process interruption, not power loss. These limits do not invalidate the bounded returned-result assertions, but prevent broader durability claims.

Inspected lead logs under `/tmp/velora-revitalization-20261014/`:

- `account-transaction-io-tests.log`: 23 pass, zero failures, 109 filtered out in that earlier test registry.
- `account-transaction-io-integration.log`: 9 pass, zero failures, 125 filtered out.
- `account-transaction-io-host.log`: 134 pass, zero failures.
- `account-transaction-io-check.log`: non-test launcher check completed successfully.

The independent transaction run has 111 filtered entries, rather than the earlier lead transaction log's 109; the final registry totals 134. Focused counts match. Overlapping runs must not be added to manufacture an acceptance total. Earlier review/implementation reports describe earlier 15/6 and 123-test snapshots; their hashes below preserve that provenance, not current test counts.

## Exact snapshot hashes

Repository HEAD: `4f639266a515c3cb05769b055a4772c4dcb89cb8`. This is a dirty working-tree review; HEAD alone does not identify the uncommitted implementation. SHA-256 source hashes were captured before focused execution and rechecked after it; all nine entries below were unchanged at that recheck. Whole-file hashes identify inspected context, not acceptance of every historical change or later separate work.

| File | SHA-256 |
| --- | --- |
| `launcher/src-tauri/src/account_transaction.rs` | `6ce2615686b324390fdc473c3d4f513546dc01b77a89a372cb08da626ef65a9b` |
| `launcher/src-tauri/src/account_transaction/tests.rs` | `dca44c3334733913be9c979a408e622ab3842f905671761bd44041a6aac3f70f` |
| `launcher/src-tauri/src/accounts/restart_tests.rs` | `990051ace11d9b1cad8ab829d329a9fdc277c6e6af8a2d8223bc60b4a5abd012` |
| `launcher/src-tauri/src/secrets.rs` | `ec4ee48a31175d48f91b76daaf8f9be690f00b2e62941a0e8ef93cb4aa3d527e` |
| `launcher/src-tauri/src/accounts.rs` | `66472f3ebf23add9a75628f5c00d4297f0eae4aeaadda4640b87fcad6050907d` |
| `launcher/src-tauri/src/state.rs` | `a7e5fef3e69c193a69934bac2da431b01d0e2f0f3584536824cacbbf8fce7a75` |
| `launcher/src-tauri/src/persistence.rs` | `2ab379a9089283bef8548b8101b3d518af3b3d4a649a9dee23f8ab71e7aee89e` |
| `reports/account-transaction-review.md` | `691740223470895f5f653640c79afd567da8fc4aa981acf4b97fe62170b66f6a` |
| `reports/account-transaction-implementation.md` | `c00297e75f1e7453caed8004ddcfd28edd1bf9191e5a91d11f89650075f92ed2` |

| Lead log | SHA-256 |
| --- | --- |
| `account-transaction-io-tests.log` | `553005f0a73ac3ceb9bc423d0770d179c9306212f907ccf169dee6bedf55418b` |
| `account-transaction-io-integration.log` | `4048e73889c71c3d79f76568fa40f96dc91c96e28fd4abe662dd77d158a41759` |
| `account-transaction-io-host.log` | `f55ec3704d8fd1ff6b5cd2edd4fd1ae39484716b10a0ca2d688826d8a7eb4493` |
| `account-transaction-io-check.log` | `d9b9742237f0b5df3568445b61241e8e2e439e41f1d7638d0b327fe3f882531e` |

## Remaining gates

Native keychain denial/access and recovery, supported operating systems, installed-store upgrades, Windows replacement/directory behavior, device flush/power loss, exhaustive partial-I/O faults, malicious ancestor/descriptor races and unrelated-writer isolation remain unaccepted. Test-only wrappers and a successful non-test compilation are not evidence of those gates. Source review and synthetic execution do not authorize deployment or alter any release/signing identity.
