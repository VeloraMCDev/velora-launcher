# Minecraft integrations

Gradle common/api/paper builds plus opt-in Fabric, Forge and client companion builds. The neutral map producer lives in ../packages/java-map-producer.

## Local checks

```sh
./gradlew build
./gradlew -Ploader=fabric -PmcVersion=1.20.1 build
```

Run from this directory. Build each loader/version with its supported Gradle/JDK toolchain; do not combine incompatible Forge/Loom generations. Minecraft protocol and net.scopenet identifiers are compatibility contracts.

See the root README for ownership, deployment and publication status.
