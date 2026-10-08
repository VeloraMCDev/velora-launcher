# Minecraft map producer

Neutral world-map renderer and local provider. settings.gradle consumes ../java-platform-client directly. Host gameplay remains separate.

Run from this package directory (Rust workspace commands may also run from root):

```sh
../java-platform-client/gradlew -p . test --no-daemon --max-workers=1
```

Preserve attribution/provenance. See ../../docs/BOUNDARIES.md and the publication audit before distributing source or artifacts.
