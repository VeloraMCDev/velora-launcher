# Native agent — Milestone 4 EXIT PASSED

Initial target: Linux/systemd, Development, outbound HTTPS and only
`REPORT_HEARTBEAT`. There is no network listener, generic shell, Docker operation,
job execution, self-update or broad account token. The agent keeps polling with
bounded backoff during outages and never stops workloads. Terminal admission/
signature/receipt failures stop the daemon with exit 78; systemd does not restart
that exit. Only network failures, rate limits and server failures are retried.

The daemon retains that heartbeat-only scope. The separate host-local
`probe-execute --policy PATH --job PATH` building block verifies a signed,
immutable, Development-only probe job and uses fixed Compose/health/rollback
operations. It is not wired into the live heartbeat service or a remote API.
See [the execution policy](../deployment/probe/EXECUTION.md) for the local privilege
boundary and pending durable Workflow/job-transport gates.

The Rust CLI supports `--version` and `doctor`, `identity-init`, `enroll`,
`heartbeat`, `daemon`, each with `--config PATH`. Unknown commands/fields fail.
Configuration requires HTTPS without embedded credentials/query/fragment,
Development scope, an absolute identity path and 30–60 second polling.
Networking refuses unstamped local builds; CI stamps the full source revision.

Identity initialization creates a new 0600 raw Ed25519 seed atomically and never
overwrites an existing identity. Native reads reject symlinks, public permissions
and malformed lengths. Private bytes stay on the agent host. Enrollment reads a
short-lived token with hidden terminal input, never from CLI arguments/config;
token buffers are zeroized and errors omit credentials/response bodies.

Request protocol v1 signs exact method, path, empty query, timestamp, nonce and
body SHA-256. Only enroll/heartbeat paths are allowed. Key/agent IDs derive from
the public-key checksum. A shared synthetic vector proves Node/Worker/Rust
signature compatibility. The origin enforces ±120 seconds, body limits, active
key/capability policy and atomic nonce replay protection. Enrollment uses hash-only
10-minute one-use tokens and atomically binds one public key; revoke denies later
signed requests. Initial limits are 10 identities, 20 outstanding tokens and
2,000 active nonces per identity. Expired nonce cleanup is scoped to that identity.

The systemd unit uses a dedicated unprivileged account with no Docker group or
capabilities. `install.sh install BUNDLE MANIFEST_SHA256 HTTPS_ORIGIN` verifies
every artifact file before first installation, creates an isolated service account
and local identity, and refuses existing destinations/accounts without replacing
anything. It does not start the service. Run the installed script's `enroll` command
in your sudo terminal; it accepts a hidden one-use code, verifies enrollment and
heartbeat, then enables the service. No package installation is needed on the host.
The reference Linux workflow tests repeat-install refusal and private-key
permissions on a disposable runner. Review its working directories and canonical
source policy before enabling it in the monorepo. Public configuration defaults
the agent API off; registration and deployment require explicit scoped policies.
Enrollment/heartbeat checks do not prove job execution or workload deployment.

```sh
cargo test --workspace --locked
cargo clippy --workspace --locked --all-targets -- -D warnings
npm test
```

The reference Linux job additionally tests private-file permissions/symlink/overwrite
rejection and produces the checksum-manifested source-named bootstrap binary.
It is temporary 14-day artifact custody, not a durable signed updater release.
