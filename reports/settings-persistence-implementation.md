# Settings persistence implementation

Final integration adds `Settings::update`: clone the proposed settings, save, then
replace memory only on success under AppState's write guard. All five mutating IPC
handlers and game-exit preference capture use it. The memory/disk consistency
regression brings inspected settings/persistence coverage to 15 synthetic tests.
The full locked host suite passes (45 tests); exact final evidence is in
`milestone-validation.md`. Independent review closes the host-memory divergence
finding while retaining native, filesystem, schema and frontend rollback gates.

Date: October 12, 2026.

## Scope and compatibility

Only `launcher/src-tauri/src/settings.rs`, new
`launcher/src-tauri/src/persistence.rs`, and this report are changed by this task.
Existing unrelated checkout changes are left alone. Required root monorepo,
boundary, validation, and deployment-target documentation was read.

Every settings field, serde name/default, companion/widget default, instance
override default, and caller-selected destination path remains unchanged.
`Settings::load(&Path) -> Self` and `Settings::save(&self, &Path) -> anyhow::Result<()>`
retain their signatures. Rustfmt also reformatted existing settings definitions.
No dependencies, schema migrations, backups, key rotation, accounts, secrets,
credential-store access, HTTP, sync, deployment, or application hosts are changed.

## Implemented behavior

- Missing settings load defaults and can be saved normally. Existing JSON must
  deserialize as `Settings` before a save can create a temporary file, and is
  validated again immediately before replacement. Legacy partial objects retain
  serde defaults and remain writable. Corrupt, wrong-typed, unreadable, symlink,
  and nonregular existing destinations cause save errors rather than replacement.
- Failed loads retain the default-returning API, emit a static warning without
  contents, tokens, paths, or parser/I/O error text, and never alter the source.
  A subsequent save still validates the existing source independently.
- Pretty JSON is serialized in memory, then written to a distinct sibling named
  `.settings.json.<UUID-v4>.tmp` for the usual destination. Other destination
  filenames remain supported, including non-UTF-8 filenames.
- `OpenOptions::create_new(true)` provides exclusive creation. A collision fails
  immediately without retrying, truncating, or cleaning up somebody else's file.
  Existing temporary symlinks and directories are rejected by exclusive creation.
- The newly opened temporary must be regular. The implementation uses `write_all`,
  `sync_all`, closes the handle before rename, and rechecks the temporary's type.
  Unix also rechecks its device/inode identity before replacement.
- After successful exclusive creation, failures trigger best-effort temporary
  removal. The original error is returned unchanged; cleanup failure emits only
  a static warning and does not mask the original error. Rename is the last
  fallible operation on the success path. No backup file is overwritten.
- On Unix, newly created temporaries use mode `0600` subject to the process umask;
  replacement inherits the temporary's permissions rather than copying the old
  destination's permissions. On other platforms native inherited ACLs apply.

## Validation performed

Main was notified that the module is ready and must wire `mod persistence;` in
`launcher/src-tauri/src/lib.rs`. That file remained unwired at validation time;
this task did not edit it or run a knowingly failing full launcher compilation.

An isolated Rust test harness declares both real source modules using `#[path]`
and links existing cached project dependency artifacts, including the actual
`velora_launcher_core` crate. It does not copy or mirror persistence logic and
does not instantiate AppState or open a keychain. The only generated harness
artifact is `target/settings-persistence-tests`; no harness source file or
manifest is added. Executed on macOS (Darwin), non-root UID 501, Rust 1.98.1.

Reproduction from the repository root, with compatible cached rlibs present:

```sh
args=()
for dependency in anyhow serde serde_json uuid tracing velora_launcher_core; do
  artifact=$(ls -t target/debug/deps/lib${dependency}-*.rlib | head -1)
  args+=(--extern "$dependency=$artifact")
done
printf '#[path = "%s/launcher/src-tauri/src/persistence.rs"] mod persistence;\n#[path = "%s/launcher/src-tauri/src/settings.rs"] mod settings;\n' "$PWD" "$PWD" |
  rustc --edition 2021 --test --crate-name settings_persistence_tests - \
    -L dependency=target/debug/deps "${args[@]}" -o target/settings-persistence-tests
target/settings-persistence-tests --nocapture
rustfmt --check --edition 2021 launcher/src-tauri/src/settings.rs launcher/src-tauri/src/persistence.rs
git diff --check -- launcher/src-tauri/src/settings.rs launcher/src-tauri/src/persistence.rs reports/settings-persistence-implementation.md
```

Results: **14 tests passed, zero failed**. Scoped rustfmt and whitespace checks
passed. All filesystem fixtures use synthetic data in UUID-unique temporary
directories and remove their fixtures on completion. Coverage includes:

- Legacy partial settings, nested defaults, missing widget scale, empty settings
  objects, and missing-file defaults.
- Successful creation and replacement through the real Settings API.
- Malformed, wrong-typed, null, and empty settings surviving default load/save.
- Unreadable Unix settings preserving bytes; this branch executed under the
  non-root test user. The test conditionally skips permission-denial assertions
  for environments where privileges bypass mode bits.
- Injected partial-write and rename failures preserving the original bytes,
  removing owned temporaries, and retaining the original I/O error kinds.
- Injected cleanup failure preserving the original error kind and message.
- Exclusive collision, unsafe temporary locations, existing temporary
  directories/symlinks, destination symlinks/directories, UUID-v4 sibling naming,
  private Unix permissions, and corruption introduced between validations.

## Remaining gates and limitations

- Lead must wire the module and run integrated launcher Cargo tests. The isolated
  harness validates the actual modules but not Tauri host composition. No native
  Windows/Linux acceptance, packaging, or release test is claimed.
- Atomic replacement and `sync_all` guarantees depend on the operating system,
  filesystem, storage hardware, and native sharing/permission behavior. Rename
  failures return errors; there is deliberately no delete-first fallback.
- The file is synced, but the containing directory is not fsynced. Persistence
  across sudden power loss, rename durability, and full crash recovery are not
  guaranteed. Crashes can leave UUID temporaries; no startup scavenger is added.
- Checks and operations remain path-based and have TOCTOU windows. Parent paths
  are not pinned or checked for symlinks, metadata/read is not a no-follow handle
  operation, and final temporary/destination checks can race with rename. This
  is not protection against an adversary controlling the containing directory.
  Cleanup is also path-based and can race with external temporary replacement.
- There is no cross-process lock or compare-and-swap. A concurrent valid save
  can be overwritten, and concurrent deletion can permit a missing-file save.
  Unix temporary identity checks narrow but do not eliminate these races;
  equivalent native identity checking is not implemented elsewhere.
- Replacement does not preserve old inode ownership, ACLs, extended attributes,
  or mode bits. Parent directory creation and live-store recovery remain outside
  scope. Fault tests inject write/rename/cleanup errors rather than simulating
  native disk-full, fsync failure, power loss, or filesystem-specific failures.

No commit, branch, worktree, PR, release, deployment, or paid infrastructure was
created. No quota block occurred during this task.
