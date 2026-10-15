# Account/credential restart transaction

October 14, 2026. This implements the missing two-store interruption protocol in
the actual login/registration/rename/logout persistence path. It is not native
keystore, installed-upgrade, every-OS or power-loss acceptance. The fixed ledger
remains **62/76 (81.58%)** pending the existing complete checkpoint definitions.

## Implemented protocol

`account_transaction.rs` owns exactly two existing destinations: `accounts.json`
and `secrets.bin`. It never accepts journal-supplied file paths. A new private
`.account-transaction.json` journal contains version-1 prepared/committed phase
and exact before/after images. Account metadata is base64-encoded, not encrypted;
credential images are already encrypted with the existing ChaCha20-Poly1305 key
and nonce format before entering the journal. No bearer, password or game token
is intentionally serialized as plaintext journal data.

Before mutation, both destination kinds/content and both candidate images are
validated. Production secret validation decrypts using the existing key; account
validation uses the existing `AccountsFile` schema. Per-image limits are 8 MiB,
the journal limit 48 MiB; reads are bounded. An exclusive initial journal file
prevents replacing a preexisting journal. On Unix it is created with mode 0600.
The prepared record is written and synced before any store publication.

Publication retains the existing exclusive-sibling atomic-write helper. It
publishes credentials, then accounts, then an atomic committed journal marker.
Store writes and directory entries are synchronized on Unix. Once committed,
journal cleanup is best effort: retained committed evidence blocks further
account changes and credential use until successful recovery. Non-Unix directory
synchronization is not implemented or certified by these Darwin tests.

Recovery validates the journal/version, both before/after image pairs and
**both current stores before modifying either one**. Each current store must
equal a recorded before or after image, including recorded absence. Prepared
transactions restore the prior pair; committed transactions restore the new
pair. Interrupted recovery can resume because each destination can independently
be at either known image. Changed, corrupt, missing-unexpectedly or nonregular
evidence fails closed and remains available for repair rather than guessed at.

`Secrets::set_with_accounts` / `remove_with_accounts` integrate this protocol into
the production account mutations. The old arbitrary closure persistence helper
remains test-only for the earlier in-process fault fixture; it is no longer the
production account commit path. In-memory account/secret publication follows a
successful transaction result.

## Startup and live safety

Native setup now calls `AppState::try_new`, which initializes/reconciles secrets
**before loading accounts**. Malformed/unsupported/changed/locked journal recovery
returns an initialization error instead of starting with a mixed pair. A pending
journal also prevents creating a replacement credential key when the original
key is unavailable. The OS credential service name, fallback key file, encryption
format, persisted account IDs and existing database/application paths are unchanged.
The immediate data-directory type is validated before any credential-key lookup
or creation. A static symlink root therefore cannot create a fallback key in an
unrelated target; this does not certify hostile ancestor/descriptor races.

Account-only mutations and standalone token refresh writes reject a pending
journal. Credential reads return no token while that journal remains, so a live
process cannot obtain new stale in-memory credentials after indeterminate
recovery. Tokens already captured by an in-flight request are not revoked by this
prospective read fence. Skin profiles are independent global wardrobe data; no profile file is
rewritten by this account transaction. Real encrypted restart fixtures retain a
synthetic `skin_profiles.json` sentinel unchanged.

## Executed validation

Lead executed Mill's original **15 module tests**, including child-process exits at
prepared, credentials-published, accounts-published and committed boundaries.
Those module tests use clearly synthetic encrypted markers with a test validator;
they are not keystore acceptance. They additionally cover first-save absence,
interrupted recovery, changed evidence, corrupt/unknown journals, rejected
candidate/recovery validation, fixed filenames and Unix symlink destinations/
direct parents. Child dispatchers are included in the test totals, not extra
business assertions.

Lead's separate **nine real encrypted integration tests** exercise production
account saving/removal and `AppState::try_new` through bounded child processes.
They cover replacement, first sign-in, logout, repeated rollback, retained IDs/
original keys, blocked startup with lost keys/corrupt journals, and live pending
write/read exclusion, pre-key symlink-root rejection, and returned I/O errors in
the actual save/reopen path. Unit-test credential storage uses temporary generated
fallback keys; it never accesses the native OS keychain. Every child has a
15-second deadline with kill/reap cleanup. Fault hooks are compiled only into
tests and require the exact synthetic root plus selected fault point.

Eight additional named I/O-boundary tests bring the module total to **23**. They
cover journal write/file sync/prepared-directory sync, store publication/marker
failure, rollback and recovery failure, image-directory sync, committed-directory
sync, cleanup removal and cleanup-directory sync. Child-isolated exact-root hooks
return synthetic errors before the selected operation. They do not simulate OS
short writes, a real full disk, failed rename inside the atomic helper, or sudden
power loss. The two new real-encryption integration loops assert returned result,
in-memory account state, prospective live read/write fencing, exact key retention,
and old/new pair recovery after these injected errors.

The transaction/I/O full host library sweep passes **134 tests**, zero failures. The
non-test host check and unchanged 124-command IPC-surface contract are rerun
separately. Logs remain under `/tmp/velora-revitalization-20261014/` with prefix
`account-transaction-`; no new full-workspace/frontend/native acceptance is inferred.

```sh
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib account_transaction::
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib accounts::restart_tests::
CARGO_PROFILE_TEST_DEBUG=0 cargo test --locked -j1 -p velora-launcher --lib
CARGO_PROFILE_DEV_DEBUG=0 cargo check --locked -j1 -p velora-launcher --lib
```

## Remaining gates and limits

The separate `account-transaction-review.md` and follow-up
`account-transaction-io-review.md` own independent review, not this
implementation report. No checkpoint is awarded simply for these extra tests.
Native credential-store/installed-upgrade and supported-OS acceptance remain open.

Ordinary process exits are not sudden power loss. APFS full-device flush,
Windows directory durability, exhaustive OS write/fsync/rename/cleanup failures and
hostile descriptor/ancestor-swap attacks are unaccepted. A partially written
initial journal fails closed rather than being silently discarded. After a
committed-marker rename, a directory-sync failure retains committed evidence,
warns and returns success; credential access/further account writes then remain
blocked until recovery. That result is not a certified durable success across
power loss, and no no-regression/UX claim is made for this rare I/O path.
Journal cleanup synchronization failure can likewise leave committed evidence
after a machine crash; recovery is designed to retain the committed pair.

Single-instance/account/secret locks bound normal in-process writers; this is not
a protocol for unrelated concurrent programs editing the same stores. Encrypted
images do not recover a lost key or reverse changes already accepted by the
remote account server. Startup currently fails on unrecoverable journal evidence;
native startup error presentation/manual repair remains an acceptance concern.
No real store, dependency, keyring/signing identity, operational SQL, paid
infrastructure, branch, commit, release or deployment is changed in this slice.
