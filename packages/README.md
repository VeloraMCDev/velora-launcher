# Reusable and gameplay packages

platform-* and rust-platform-contracts are SDK packages; java-* provide Java APIs, clients and map production; panel-ui is reusable login UI; private-* contain gameplay/storage. The directory names preserve existing build compatibility; the complete product source is approved for publication.

## Local checks

```sh
cargo test --locked --manifest-path packages/rust-platform-contracts/Cargo.toml
# Java standalone packages: use the checked-in wrapper
packages/java-platform-client/gradlew -p packages/java-platform-client test
packages/java-platform-client/gradlew -p packages/java-legacy-api test
packages/java-platform-client/gradlew -p packages/java-map-producer test
```

SDK packages must not depend on gameplay implementations. Each Node package keeps its own lockfile and check/test scripts. Libraries are deployed through their consuming service; they do not each need a domain.

See the root README for ownership, deployment and publication status.
