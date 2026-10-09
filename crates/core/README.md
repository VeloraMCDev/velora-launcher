# velora-launcher-core

Maintained Rust launcher engine segment in the root Cargo workspace.

```sh
cargo test --locked -j 1 -p velora-launcher-core
```

Run from the repository root. Reusable libraries must not depend on private gameplay or the complete Panel host. See ../../docs/BOUNDARIES.md and docs/components for component contracts and source attribution.
