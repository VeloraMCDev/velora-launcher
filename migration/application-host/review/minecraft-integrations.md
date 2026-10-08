Extract vanilla region/NBT sampling, block/biome colors, PNG/zoom/tile caching and generic map uploads into an independent package. WorldMapSync consumes neutral SDK transport; private client settings, claims and gameplay rules are excluded. Keep renderer version, cache paths, vanilla dimensions, wire format, scheduling and HTTP refusal handling.

Preserve the original texture-derived block color table as a packaged resource. A dedicated regression distinguishes its colors from the fallback palette so an omitted resource cannot silently change rendering.

Provide a reviewed local producer through neutral SDK map lifecycle/position/factory ports, with API version 1. API-class-loader discovery works even when the server context loader is isolated. Private gameplay can compile against SDK contracts without importing renderer or loader implementation classes.

Validation: 18 independent tests, Gradle build and CI passed, including generated worlds, captured tile/player/overlay uploads, original palette packaging, an actual restart without replaying persisted tiles and provider/class-loader discovery. The five-file SDK snapshot is pinned and hash-checked; builds need no private source dependency. The source host passed all 136 common Java tests and built the existing Paper plugin with provider registration and the original palette included. Source hashes and Gradle attribution are recorded.

This draft covers map production only. Paper/Fabric/Forge adapters, admission, public companion/private composition and complete loader/runtime/upgrade acceptance remain pending. Repository visibility and live data are unchanged.

Add a customized repository landing page with a component-specific official Velora banner, status/stack/license badges, segment descriptions, development or evidence entry points, ownership boundaries and migration/license guidance. Local links and artwork hashes are verified; presentation makes current readiness explicit.

The checked-in Gradle wrapper now has exact official checksum/artifact provenance and its original embedded Apache-2.0 license. Gradle 9.6.0 distribution SHA-256 is pinned without changing wrapper/gameplay bytes. Owner Java CI at dacfebc passed (37363836144); seven-checkout source preflight also passes. Full loader/application and artifact release gates remain open.
