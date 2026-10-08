# Panel backend

Axum compatibility host backed by owned SQLite files. Runtime configuration and database ownership are described in ../../docs/APPLICATION.md.

## Local checks

```sh
cargo test --locked -j 1 -p scopenet-panel
```

Runs as a native/container service; it is not currently a Workers/D1 application. Never start a second credential writer against an existing live store.

See the root README for ownership, deployment and publication status.
