# Retained build and branding assets

The four tracked Gradle wrapper JARs share SHA-256
`2db75c40782f5e8ba1fc278a5574bab070adccb2d21ca5a6e5ed840888448046`.
This is an official wrapper checksum listed for Gradle 8.10 through 8.12.1 in
the [Gradle checksum reference](https://gradle.org/release-checksums/).
The wrappers launch the pinned Gradle 9.6.0 distribution. Gradle documents that
[an older wrapper can run a newer distribution](https://docs.gradle.org/current/userguide/wrapper_plugin.html).
No JAR replacement or toolchain version change was needed.

The maintained integrations, Java gameplay and standalone Java SDK wrapper
properties now pin the official Gradle 9.6.0 binary distribution checksum:
`bbaeb2fef8710818cf0e261201dab964c572f92b942812df0c3620d62a529a01`.
It was verified against the [official checksum endpoint](https://services.gradle.org/distributions/gradle-9.6.0-bin.zip.sha256).
The historical archive retains its original properties. Wrapper JARs contain
the upstream LICENSE; maintained Java/SDK/integration builds also retain Gradle
LICENSE/NOTICE files. These notices do not license product gameplay.

All 30 tracked PNG files were checked for text and EXIF chunks (`tEXt`, `iTXt`,
`zTXt`, `eXIf`); none were present. The retained raster assets were visually
reviewed as Velora or legacy logos, banners and application icons. The source
publication includes these product assets under their existing terms. It grants
no additional trademark rights and makes no claim to grant third-party rights.

Obsolete executable/JAR releases and player screenshots were removed from the
current tree. Their old Git/GitHub custody remains a separate publication gate.
