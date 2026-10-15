# Velora launcher

Tauri desktop app with instance selection, community features and client settings.
The native Rust engine lives in ../crates/core; browser UI development can run
without a Panel backend through its built-in synthetic fixture.

## Browser preview

From this directory with Node 24:

```sh
npm ci --no-audit --no-fund
npm run dev
```

Open http://localhost:1420/?mock=main. Supported fixtures: main, setup, login,
update and progress. They are UI demos; native launching requires Tauri.

## Validation and packaging

```sh
npm run check
npm test
npm run check:runes
npm run build
npm run tauri dev
npm run tauri build
```

Native commands require Rust 1.98.1 and the target OS's Tauri toolchain. Preserve
net.scopenet.launcher, credential-store identity and signing continuity. Desktop
installers share the existing download origin; the UI needs no separate server.
See [application development](../docs/APPLICATION.md), [validation](../docs/VALIDATION.md)
and [deployment targets](../docs/DEPLOYMENT_TARGETS.md).

## Preparation ownership

The host admits one preparation at a time. Cancellation targets its owning
instance, retains the busy slot until the worker is joined and finalized, and
publishes cancellation only after that join. Ephemeral operation IDs prevent a
stale completion or cancellation from affecting a newer owner, including another
preparation for the same instance. `launch` and `repair_instance` return the
admitted operation's UUID string; `cancel_launch` requires both `instanceId` and
`operationId`. Missing or malformed tokens are rejected, and a valid nonmatching
token is a no-op. The bundled frontend captures the admission promise before
cancelling, so a delayed reply cannot retarget cancellation. Command names and
persisted identities stay unchanged; older IPC callers must adopt this contract.

Preparation, instance deletion, cache clearing and Panel username changes share
an in-process mutation lease. These operations reject conflicting admission;
cache clearing reports non-missing-filesystem errors instead of false success.
The lease is not a filesystem transaction, interprocess lock, subprocess cleanup
guarantee or proof that independent/background I/O has drained. Background I/O,
child-process ownership and native acceptance remain gates.

Progress and state payloads now carry `instance_id` and `operation_id`; core
progress fields remain at the top level. `game://log` is now an object containing
those identities, `run_id`, and `lines`, not a bare string array. Game idle states
carry their run identity too. Bundled callers migrate together; external event
consumers must adopt the new shapes. The frontend matches preparation events,
bounds early-event buffering to 128 entries, and rejects late events from completed
jobs. Its run inventory remembers exited run IDs for the frontend lifetime.
Recovered foreground runs retain live logs by matching the known run and instance.
Recovery of a preparation token across frontend reload is still not implemented.

## Local regression checks

With Node 24, `npm test` runs direct production-controller tests and client-compiled
Svelte-store tests with synthetic IPC. The VM-module experimental warning is
expected; no new test dependency is installed. Tests do not start Tauri, Java,
the OS credential store, or a real Panel. Source CI runs these checks alongside
check/runes/build in its existing bounded runner; packaging stays manual.

Settings writes use exclusive UUID sibling temporaries, file synchronization and
rename, and refuse to replace corrupt/unreadable existing JSON. In-memory settings
are committed only after persistence succeeds. Existing field defaults and paths
are unchanged. Directory-fsync durability, native sharing/ACL behavior and
path-based concurrency/TOCTOU acceptance remain pending.

## Domain and Panel contracts

The IPC facade reexports 124 existing commands from domain handler modules.
Permanent command-surface tests compare names, signatures and registration order
against the captured pre-extraction contract; compiler checks remain necessary.
The frontend keeps its reactive facade and separates preparation, accounts,
manifest/context and settings-persistence ownership into tested helpers. Failed
or uncertain context mutations reconcile authoritative bootstrap state after
queued mutations settle; stale recovery replies cannot replace newer choices.
Settings save status/conditional rollback does not overwrite newer unsaved edits.

Panel command/auth/instance-manifest reads use a typed bounded client with redacted
structured failures. Ordinary IPC errors remain strings. Audited account error
literals are retained; arbitrary server text/URLs/tokens are not echoed. Success
JSON is capped at 4 MiB and account-error buffers at 16 KiB. Configured base prefixes,
bearer/selected-instance headers and manifest cache fallback remain scoped contracts.
Signed updater/artifact, vendor/Yggdrasil and best-effort telemetry paths retain
their distinct policy; this does not unify every external networking path.

Activity reporting honors the existing persisted `send_stats` preference before
capturing identity and again before sending. Recorders retained by running work
observe a later opt-out without changing which account owns their events.
After the settings save succeeds, opt-out prevents subsequent request admission;
the UI save is debounced/queued and a failed save keeps the old preference. It
does not retract an already-admitted/in-flight request. The default preference and event wire format stay
unchanged. This setting is not a switch for required authentication, manifest,
social or updater requests.

Activity capture now holds a stable settings/account snapshot while reading that
account's credential. A bearer is attached only when its explicit stored Panel
URL matches the validated destination base (including its case-sensitive path).
Host casing, default ports and trailing separators use the request client's
normalization. Missing, invalid or mismatched account origins produce no bearer;
they do not erase or migrate stored sign-ins. Already captured events retain
their original destination and identity. This guarantee is specific to activity
reporting; it does not certify every other authenticated request path.

Configured Panel requests now capture a destination-bound bearer and selected
instance together. When connecting to a prospective Panel, manifest lookup checks
credentials against that requested URL, not the previously configured server.
Missing legacy account origins no longer authorize bearer forwarding, including
instance-manifest fetches. Such accounts may need a fresh sign-in; existing IDs,
encrypted secrets and files are retained, not migrated or erased. Cached manifests
previously marked authenticated under an originless account may no longer be
reusable offline until a valid scoped sign-in and refresh. Installed-upgrade and
recovery acceptance for that compatibility boundary remains a release gate.

## Guarded synthetic native acceptance

This opt-in **debug-only** harness uses the real Tauri host and IPC with a fresh,
private temporary store. It does not certify installed upgrades, native keystore,
game/loader compatibility or release readiness. Never package its Cargo feature.

With Node 24, run `node launcher/scripts/native-fixture.mjs` in one terminal.
It prints a canonical `root` and loopback `serverURL`; leave it running. Run the
ordinary launcher Vite dev server in another terminal. Build the native fixture:

```sh
CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -j1 -p velora-launcher --bin velora-launcher --features native-acceptance
VELORA_NATIVE_ACCEPTANCE_ROOT='<printed root>' target/debug/velora-launcher
```

The feature requires an existing marked UUID-named direct child of the canonical
temporary directory, private regular JSON/key files, and a valid preseeded
fallback key. Missing/invalid input fails **before Tauri setup**, with no fallback
to installed app data. The normal build retains its existing data/keyring identity
and single-instance plugin; only this feature uses a fixture-root file lock instead.
Release builds with the feature are rejected at compile time. The fixture server
returns controlled instance errors, never downloadable game artifacts. Stdin
JSON controls delays/manifest availability; run `--help` for syntax. SIGINT/SIGTERM
close the server and retain only its generated fixture for inspection. Do not point
the harness at a player store or infer production keychain acceptance from it.
`node launcher/scripts/native-fixture.mjs --selftest` validates the fixture server.

Shared Discord DTOs preserve Panel wire omission/null/extension semantics. Host
poll IPC emits only `{account}`, `{pending:true}` or `{linked:true}`, never game
tokens or arbitrary poll extensions. Actual Rust/TypeScript common fixtures and
negative contracts accompany the producer/host/frontend consumer migration.

Account and wardrobe JSON writes now preserve corrupt/nonregular/symlink input.
In-memory accounts/credentials commit after successful persistence. Encrypted
credentials use exclusive UUID siblings rather than the legacy predictable temp.
Account/credential changes now use a prepared/committed interruption journal:
startup reconciles the pair before loading accounts and pending evidence fences
new credential reads/writes. Failed publication attempts rollback; unresolved or
changed evidence is preserved. Process-restart recovery is tested, not certified
power-loss durability. Existing store/keyring identities and account IDs remain
unchanged; exhaustive OS flush faults, native ACL/schema continuity,
interprocess/parent-path races and installed upgrades stay
acceptance gates. The profile mutex is released before network awaits.
