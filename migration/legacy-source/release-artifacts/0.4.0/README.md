# Local 0.4.0 build artifacts

These files were built from PR #3 source with GitHub Actions disabled.

- `SCOPENET Launcher_0.4.0_x64-setup.exe`: Windows x64 NSIS installer,
  cross-compiled on macOS with Tauri and cargo-xwin. It is unsigned and asks
  the player for the panel URL on first launch.
- `scopenet-fabric-{1.20.1,1.21.1,26.3}-0.4.0.jar`: server-side Fabric mods.
- `scopenet-forge-{1.20.1,1.21.1,26.3}-0.4.0.jar`: server-side Forge mods.
- `scopenet-paper-0.4.0.jar`: Paper plugin, built against Spigot API 1.20.1.

The repository has a Forge server mod but no separate NeoForge server mod, so
these artifacts do not include a NeoForge JAR.

The build commands and integration tests passed locally. Every JAR passed ZIP
integrity checks and contains its loader descriptor. `SHA256SUMS` records the
artifact hashes; run `shasum -a 256 -c SHA256SUMS` from this folder to verify.

Minecraft client/server runtime testing and installation of the Windows
installer on Windows still need to be done before a public release.
