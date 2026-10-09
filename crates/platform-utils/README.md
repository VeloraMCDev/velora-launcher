# velora-platform-utils

Maintained Rust platform segment in the root Cargo workspace.

```sh
cargo test --locked -j 1 -p velora-platform-utils
```

Run from the repository root. Reusable libraries must not depend on private gameplay or the complete Panel host. See ../../docs/BOUNDARIES.md and docs/components for component contracts and source attribution.
