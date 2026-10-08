# Rust services and libraries

Auth is in auth-core/http/service/tools; Panel libraries use panel-*; core is the launcher engine; shared/platform-utils are reusable libraries.

## Local checks

```sh
cargo test --locked -j 1 --workspace --exclude scopenet-launcher --no-fail-fast
```

Use the root Cargo workspace. No crate here owns a production deployment merely by being a library.

See the root README for ownership, deployment and publication status.
