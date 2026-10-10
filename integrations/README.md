# Minecraft integrations

Gradle common build plus the Fabric 1.20.1 server and client builds. Paper, Forge and modern-version sources are in [../archive](../archive/README.md). The neutral map producer lives in ../packages/java-map-producer.

Velora SMP's active target is **Velora Core**, with separate Fabric 1.20.1 server and client jars. See [its build guide](velora-core/README.md) and [implemented behavior and remaining work](../docs/VELORA_CORE.md).

The [Calagopus extension](calagopus/README.md) connects Calagopus servers to the Velora Panel and reports installed mod versions. Download jars from the Admin Panel and upload them manually.

## Local checks

```sh
./gradlew build
sh toolchains/fabric-1.20.1/gradlew -p . -Ploader=fabric -PmcVersion=1.20.1 :fabric:build :common:test
sh toolchains/fabric-1.20.1/gradlew -p . -Ploader=fabric-client -PmcVersion=1.20.1 :fabric-client:build
```

Run from this directory. Build the server and client in separate invocations.

See the root README for ownership, deployment and publication status.
