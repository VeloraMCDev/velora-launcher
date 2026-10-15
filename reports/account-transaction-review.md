# Independent account restart-transaction review

October 14, 2026 (America/New_York).

## Verdict

**Accept the bounded process-interruption/restart implementation; no confirmed blocking defect found in the reviewed scope.** Recovery determines one desired pair from the journal phase, authenticates encrypted images with the existing key, and validates both current destinations before its first publication. Startup recovery precedes account loading. Pending evidence fences subsequent account/credential mutations and new credential reads.

**Broader security/persistence acceptance remains open.** This is not an atomic two-file snapshot for arbitrary external readers, a hostile-filesystem or multiprocess transaction protocol, or certified power-loss durability. Native keychain/GUI, Windows, supported-OS, installed upgrade and whole-checkpoint gates receive no credit. Ledger remains **62/76**; the requested percentage does not alter criteria.

## Scope and method

Reviewed `account_transaction.rs` and its tests; `Secrets::try_open`, transaction/encryption/get changes; production account set/remove integration; `AppState::try_new` and account write guard; Tauri setup's fallible initialization; and `accounts/restart_tests.rs`. Inspected adjacent existing atomic persistence behavior and credential/account call sites to check ownership and lock order. Root AGENTS and previously read monorepo/boundaries/validation/deployment instructions apply; no nested launcher/reports AGENTS applies.

This review independently analyzes source and the retained execution logs, then independently repeats the two focused Rust modules after the explicit Cargo-idle notice. No browser/native application, real keychain/player store, network service, purchase or deployment was used. Only this new report is written; source/tests/ledger are untouched. No branch or commit is created.

## Protocol findings

### Fixed identities and evidence validation

- Journal version 1 contains `phase`, `accounts` and `secrets` before/after images; fixed code supplies `accounts.json` and `secrets.bin`. There is no journal-controlled destination/path field. `deny_unknown_fields` rejects attempted path injection and other unknown journal/image fields. The actual uncommitted phase name is **`prepared`**, not `pending`; “pending” describes the unresolved-file condition.
- Decode/read limits bound individual images to 8 MiB and journal input to 48 MiB, including a bounded read after checking file metadata. Accounts are parsed as the existing `AccountsFile`; every present before image and each after image of the secret store must decrypt/authenticate and parse with the existing key before recovery writes anything. This is structural validation, not new semantic validation of every account ID, uniqueness or token policy.
- Both current files must match their corresponding known before or after image, including `None` only when that before image was absent. A missing formerly present file or external byte change rejects recovery without partially repairing the other file. Checks compare exact serialized evidence, not just similar parsed account identities.
- Journal secret images are base64 of the existing nonce-plus-authenticated-ciphertext format. They are not plaintext tokens. Account images are ordinary account metadata encoded as base64, **not encryption of account identities**. The journal also retains encrypted old credentials until cleanup; this is not secure erasure. Keyring service/account, filenames, persisted account IDs and upsert identity are unchanged.

### Commit and restart ordering

1. Candidate accounts and encrypted credentials, plus existing destination images, are validated before a journal is created.
2. A unique-to-the-active-transaction journal is acquired using fixed-path `create_new`, written as `prepared`, synced and followed by directory sync on Unix. Existing journal evidence is not overwritten to start a new transaction.
3. Secrets publish first, then accounts; each image publication checks expected current bytes, uses the existing exclusive sibling/atomic rename helper, and syncs the directory on Unix.
4. Only after both images publish does an atomic journal replacement mark `committed`. A successful final directory sync precedes ordinary cleanup. Publication failure invokes recovery and still returns the original failure (or failure with recovery context), rather than updating in-memory candidates as if rollback succeeded.
5. On restart, `prepared` selects the before pair, while `committed` selects the after pair. Recovery rechecks each expected current image before publication, syncs each step and leaves the journal until both are complete. A second interruption can therefore be retried from either known mixed pair. First-save rollback removes files whose before image was absent.

The pair is published sequentially, not simultaneously. In the inspected production integrations the account write lock and secret mutex remain held through the transaction; new in-process account/secret snapshots cannot observe those intermediate publications through those locked stores. Locks use account-then-secret ordering, consistent with the inspected readers. This is not an interprocess filesystem lock.

### Startup and live fencing

`Secrets::try_open` obtains the existing key and performs journal recovery before `AppState::try_new` loads accounts. Journal parse/version/decryption/evidence failures propagate through fallible Tauri setup rather than loading the mixed account file as normal state. When a journal exists, missing/denied/invalid key acquisition does not regenerate a replacement key. No recovery is attempted with an unauthenticated encrypted image.

Without a journal, existing locked-credential behavior is retained: startup can expose accounts with unavailable secrets rather than globally refusing all application initialization. This is not contradictory to fail-closed **journal recovery**, and must not be described as universal startup refusal whenever any credential file is damaged.

`Secrets::get` holds its mutex and returns an empty secret if the journal/root check fails. `flush` checks the same fence for secret-only refresh writes; paired set/remove calls encounter it in `commit`. `AppState::update_accounts` checks it while holding the account write lock, covering selection/offline-account changes. Production sign-in/replacement/removal use the new paired APIs. The old generic callback transaction is `cfg(test)` only. The session-refresh path still performs an atomic secret-only write; it does not require an account-pair journal because it does not change the account record.

## Scrutinized success-with-retained-journal semantics

If the **directory sync after committed-journal replacement** fails, both new destinations and the committed journal have already been published in the running process. The function intentionally logs a warning and returns `Ok`, without deleting that journal. Paired account/secret callers then update their in-memory candidates. Subsequent `get` calls return no credentials, and further account/credential writes reject the unresolved journal. A normal restart authenticates the images and selects the new pair from the visible committed phase.

This is coherent, conservative source behavior, not a rollback or a claim of durable success. The API result means the operation published its new state; **it does not mean credentials remain usable before restart or survive power loss**. A user may see sign-in/removal succeed and then need restart. The warning is in tracing, not a structured “committed but restart required” result to the UI. A richer status would improve operational clarity but is not a confirmed blocker for the disclosed bounded semantics.

Cleanup also has two distinct outcomes: failure before unlink leaves committed journal evidence and fences future operations; failure of directory sync **after successful unlink** only warns and does not leave a live fence. If an old committed journal reappears after a crash, replay selects the same after pair. None of this establishes physical-media durability.

## Remaining gaps and targeted tests

- **Final-sync and cleanup error branches lack direct injected regression coverage.** Current tests exercise process exits after successful phase boundaries, interrupted replay and unresolved-journal guards, not a real/fault-injected failed final directory sync returning `Ok`. Add a focused test of returned success, visible committed journal, candidate memory, suppressed reads/rejected writes, and successful restart. Also distinguish cleanup-unlink failure from cleanup-directory-sync failure. Source inspection supports these branches; the passing tests do not execute them.
- **Other I/O failures are not a complete matrix.** Partial journal creation/write, ENOSPC, temporary publication/sync failure, failed rollback, malformed base64/phase/image size boundaries and wrong-key-but-present recovery merit targeted checks. A failed initial journal write may leave malformed evidence and intentionally require fail-closed manual recovery; no automatic repair of arbitrary damaged evidence is promised.
- **No hostile or competing-writer guarantee.** Immediate-root/destination checks reject static symlinks, but reads/checks/renames are path-based and not descriptor-relative. Ancestor swaps, TOCTOU, hard links and another process modifying the journal or stores during replay remain outside acceptance. Checking the journal again at cleanup does not undo recovery writes made before a competing journal replacement is detected. The single-instance application policy is not certification of arbitrary shared-directory writers.
- **Concrete startup ordering gap outside pending-journal recovery:** `Secrets::try_open` calls `load_or_create_key` before `recover` checks the root's file type. On an empty store reached through a static symlink root, key acquisition can therefore create a fallback key in the target (or create an absent native keyring entry) before startup rejects the root. This does not bypass the existing-journal missing-key guard or overwrite an existing encrypted pair, so it is not a blocker for the bounded transaction verdict; do not claim all startup side effects are preceded by root validation. A real `try_new` symlink-root/no-key fixture and earlier root validation would strengthen this boundary. The module-level symlink-root test does not exercise key acquisition.
- **No global credential revocation.** The fence prevents new reads through `get`; it cannot recall tokens/account objects already captured by ongoing remote requests or game-auth operations. Existing in-flight snapshots are not covered by a new cancellation/credential-epoch protocol.
- **Durability/platform limits remain substantive.** Unix `sync_all` calls are present, but no macOS F_FULLFSYNC, power-cut experiment or Windows directory-sync/rename matrix was performed. On non-Unix targets the directory-sync helper is a no-op. Tests terminate at named successful boundaries; they do not simulate hardware write-cache loss or arbitrary syscall interruption. Fallback key-file creation itself is not newly journaled/directory-synced.
- **Runtime-worker cost and user recovery remain open.** Encryption/serialization/full-image reads and synchronous filesystem work run inline while locks are held. This avoids detached writer ownership but can block runtime workers on slow storage. No stress/latency bound or native user-facing damaged-journal recovery workflow is accepted.

These are nonblocking gaps for the narrowly stated process-interruption implementation, but prevent broader durability/security/native checkpoint closure.

## Test evidence inspected

Logs reside under `/tmp/velora-revitalization-20261014/`:

- `account-transaction-module-tests.log`: **15 pass**, including a subprocess helper; synthetic secret-image validation is deliberately injected here, not real encryption. Tests cover phase-boundary exits, repeat/interrupted replay, changed/missing evidence preservation, corrupt/unknown/path-injecting journals, first save and static Unix symlinks.
- `account-transaction-encrypted-tests.log`: earlier focused **5 pass**, including the subprocess helper. This earlier log does **not** include the subsequently added live-journal guard regression. Real encrypted fixtures call production account save/remove and fallible state startup, retain the same fallback key, and use only temporary synthetic stores.
- `account-transaction-final-host-tests.log`: **123 pass**, zero failures, includes all **six** current `accounts::restart_tests` entries (five substantive tests plus helper), including `unresolved_journal_blocks_live_account_and_credential_changes`. This is the retained main-agent run, not an independent reviewer rerun.
- `account-transaction-final-host-check.log`: non-test launcher library check finishes successfully. This verifies production `cfg` compilation, not native keyring behavior or native startup.

The subprocess tests check exit code 77 at actual production fault hooks, use temporary roots and deadlines, and kill/reap timed-out children. Their execution is more meaningful than manually constructing only a finished journal; it is still a bounded process-exit model. Independent review also ran scoped `git diff --check` with no findings. Test counts overlap and must not be added to create a new total.

After the lead's explicit idle notice, independently executed these commands serially on the final hashed source using the existing target:

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib account_transaction::tests::
# 15 passed; 0 failed; 108 filtered out
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib accounts::restart_tests::
# 6 passed; 0 failed; 117 filtered out
```

Both include their child dispatchers. Results are retained in the review session's tool output; the artifact hashes below identify the lead's logs, not newly manufactured independent logs. Refreshed all seven source hashes after the repeats; they match the snapshot below, including the final journal plaintext-token exclusion assertion and live pending regression. Read the final implementation report as well: its explicit durability limits agree with the reviewed source, subject to the in-flight-token and startup-ordering qualifications above.

## SHA-256 reviewed source snapshot

| File | SHA-256 |
| --- | --- |
| `launcher/src-tauri/src/account_transaction.rs` | `314776e95b4891abb32d7d40e02cc68e22c60e12c4d218e434dedfe6650952f4` |
| `launcher/src-tauri/src/account_transaction/tests.rs` | `66575b4c79de684df5de5f95f6b9fd8b5462cf6416c69507c3fff5db2998da4d` |
| `launcher/src-tauri/src/secrets.rs` | `9d4ab0bb4835e47da9ba4eaf077b9b074c18e9fc2700a7c1e6c35790060acbfb` |
| `launcher/src-tauri/src/accounts.rs` | `66472f3ebf23add9a75628f5c00d4297f0eae4aeaadda4640b87fcad6050907d` |
| `launcher/src-tauri/src/state.rs` | `a7e5fef3e69c193a69934bac2da431b01d0e2f0f3584536824cacbbf8fce7a75` |
| `launcher/src-tauri/src/lib.rs` | `98d8223fe0b88a774c3bec35b423264c3e110ac41d59d7fd41fdb858ba6e276c` |
| `launcher/src-tauri/src/accounts/restart_tests.rs` | `354e3aa067cd4b1ee1b85275e4b7e6da5774d3c1dd8cfbd0ebba57fb858bd6df` |

Whole-file hashes identify the snapshot, not review/acceptance of every historical change in those files.

| Retained artifact | SHA-256 |
| --- | --- |
| `account-transaction-module-tests.log` | `3fb3666a470846fbfe4e6995ccf1051066b42b146f051cd21916e7e2f39b9384` |
| `account-transaction-encrypted-tests.log` | `92a1b11e311fe95d1e4a62a7c70b5e45798e8d4771dab307fe352ba3095103eb` |
| `account-transaction-final-host-tests.log` | `53389fe2a975460ef690ecbded53b333843b4db2d89fce5f27fd9f6bc53d98af` |
| `account-transaction-final-host-check.log` | `d45ab10e2bf4946adc0183fca5884e3abb16e9bf809fad148396a81403dee9d0` |
