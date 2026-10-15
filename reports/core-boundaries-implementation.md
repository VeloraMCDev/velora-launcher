# Core dependency and loader fixture boundary

October 12, 2026. Root core already delegates neutral HTTP/digest operations to
`velora-platform-utils`. Source inventory found no direct `sha1`, `sha2` or `hex`
uses in core; their three unused dependency edges are removed from core Cargo.toml
and the corresponding package dependency list in root Cargo.lock. No versions,
package sources, nested/segment lockfiles, licenses or runtime protocols change.
Forge/library ZIP processing and authlib base64 remain direct, intentional uses.

The reusable platform/gameplay boundary is revalidated, not newly invented. No
platform library imports the complete application or gameplay. Loader installation
remains owned by root core's Fabric/Quilt and Forge/NeoForge modules; a universal
adapter or broader loader-support promise is not added.

Four permanent synthetic compatibility tests exercise actual VersionJson parsing
and production merge: child main class/ID with inherited runtime/assets/download,
version-less library precedence, distinct native classifier maps, legacy arguments,
modern quick-play and malformed required fields. They name protocol identities
but are not downloaded vendor fixtures or proof of running those loaders. Existing
Maven, rules, Forge processor-argument and version tests remain part of core tests.

The locked core test/build validates removed direct dependencies and unchanged
shared delegation. Exact final counts are in `fifty-percent-validation.md`;
independent review is `core-boundaries-review.md`. Native install/processor/runtime/
server-join/OS compatibility and recorded real-version provenance stay open.
