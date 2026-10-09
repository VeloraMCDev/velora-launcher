# Java gameplay package

This maintained extraction preserves shared gameplay commands, guild/shop/economy/
utility/reward/chat rules, private settings, claims, retry batches and operation
receipts. The owner approved implementation and fixtures for source publication
with the complete product; no platform MIT grant applies. Historical source
preservation remains separate from the maintained product source.

Run `gradlew build`
with Java 21 (Java 17 bytecode). Only neutral SDK contracts are required to compile;
there are no renderer, Bukkit, loader or Panel-internal imports. The canonical SDK
is `../packages/java-platform-client`. It is `compileOnly` for the gameplay JAR, so authorized
hosts supply one SDK class identity rather than bundling duplicate contract classes.

Map production is supplied by a reviewed locally installed provider through SDK
ports, with API-version checks and class-loader-aware discovery. Without one,
rendering reports unavailable. No code is loaded from remote manifests. Full loader
extension registration, backend/UI composition and authorized distribution/runtime
acceptance remain pending. No publication plugin is enabled. Gradle attribution is
in `gradle/`; it does not license the private gameplay implementation.
