# Account and wardrobe persistence

October 12, 2026. Local source changes only; existing `accounts.json`,
`skin_profiles.json`, `secrets.bin`, encryption/keyring identities and account IDs
remain unchanged. Tests use UUID-named synthetic directories and test-only key
files; no native credential prompt, player store or operational migration ran.

## Implemented behavior

Account JSON uses the existing typed atomic writer: corruption/nonregular/symlink
destinations fail closed and are not overwritten. Account changes clone, persist,
then commit memory under AppState's write lock. Offline addition and selection
share this path. Same panel/kind/UUID upsert continues retaining the original ID.
Missing game-session auth fails before mutating account or credential state.

Panel login/rename/removal serialize candidate accounts and credentials under
their existing in-process locks. Credential changes are encrypted/persisted before
account publication; account publication failure restores the previous encrypted
contents semantically and leaves memory uncommitted. Rollback failure reports both
errors and does not pretend that two-store atomicity was achieved. The account
write is never a blind fallback from corrupt JSON.

Encrypted writes now use exclusively created UUID sibling temporaries, file-sync,
close-before-rename, regular-file/Unix identity checks, and validation of existing
ciphertext with the same key. The predictable symlink-following `secrets.bin.tmp`
write identified by independent review is removed; legacy temporary paths are not
followed or scavenged. Failed set/remove no longer mutates in-memory credentials.
This shares the JSON writer's verified-byte publication primitive without changing
the encryption format or key identity.

Wardrobe load fails explicitly on corrupt/nonregular input instead of silently
returning an empty list. Updates use typed atomic replacement and a dedicated
in-process profile mutex. Remote skin retrieval completes before locking; applying
a profile copies its snapshot and releases the guard before network awaits.
Missing profile files still allow first save and legacy defaults remain compatible.

## Evidence and limits

Permanent synthetic tests cover legacy account/profile round trips, stable IDs,
selection/removal, corrupt files, missing game auth, failed credential/account
publication, semantic credential rollback, failed rollback diagnostics/retained
recovery contents, memory/disk consistency, legacy temp symlink preservation and
encrypted destination symlink rejection. Exact final execution is recorded in
`fifty-percent-validation.md`; independent review is separate.

This is not a durable cross-file transaction. A crash between credential and
account publication can leave mismatched stores; no startup journal/recovery is
implemented. Rollback failure requires explicit inspection. Directory durability,
hostile parent/cleanup TOCTOU, cross-process lost updates, future unknown-field
retention, ACL/permissions continuity and native OS/keystore/installer upgrade
acceptance remain gates. Symlinked destination files are intentionally refused.
No claim covers every possible installed data schema or remote username rollback.
