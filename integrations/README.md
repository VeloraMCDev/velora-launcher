# Minecraft integrations

Gradle common/api/paper builds plus opt-in Fabric, Forge and client companion builds. The neutral map producer lives in ../packages/java-map-producer.

Velora SMP's active target is **Velora Core**, with separate Fabric 1.20.1 server and client jars. See [its build guide](velora-core/README.md) and [implemented behavior and remaining work](../docs/VELORA_CORE.md).

## Local checks

```sh
./gradlew build
sh toolchains/fabric-1.20.1/gradlew -p . -Ploader=fabric -PmcVersion=1.20.1 :fabric:build :common:test
sh toolchains/fabric-1.20.1/gradlew -p . -Ploader=fabric-client -PmcVersion=1.20.1 :fabric-client:build
```

Run from this directory. Build each loader/version with its supported Gradle/JDK toolchain; do not combine incompatible Forge/Loom generations. Minecraft protocol and net.scopenet identifiers are compatibility contracts.

See the root README for ownership, deployment and publication status.
