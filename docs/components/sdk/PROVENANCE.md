> Historical owner documentation from `VeloraMCDev/sdk` at `de3b8fd7539c3b9d20fbc8a3b58317f61a321faa`. Current segment instructions are in the monorepo READMEs.

# Platform contract and utility extraction

Source: `https://github.com/scopeddlol/SCOPENET-MC`, revision `57daa92cb6cb4027982d06bfa1c692ab9edb88ff`.
Original work by the SCOPENET-MC contributors; attribution is retained here.
MIT was selected by the project owner for public Velora code on 2026-10-04.

| Source | SHA-256 | Extracted surface |
| --- | --- | --- |
| `crates/shared/src/lib.rs` | `1a0fbe6e526455c98e9605e2998f7abc6338f7a110bdfaa683f121e0942bd4e9` | Complete wire declarations, including gameplay/profile DTOs; no gameplay implementation |
| `crates/shared/src/experience.rs` | `72da2d5022fb19d9606e4d0a4f7c07e609d63c26526ad2e6d6233ba24fdc0780` | Generic experience presentation schema; no SMP feature preset |
| `crates/shared/src/updates.rs` | `574346a6c21b3b70d79c5f3622937d396c6d20c26dd4fd685c31cf5d6a74b040` | Release metadata and semantic version comparison |

Wire field names, API_VERSION and Minecraft offline UUID semantics remain unchanged.
New platform branding defaults say Velora. Explicit operator branding is retained.
Missing experience configuration defaults to a generic experience with no features,
instead of silently enabling SMP systems. This intentionally changes only the new
SDK default; existing applications still use the legacy crate and are not switched.
Their adoption must preserve legacy instance configuration explicitly and be tested.
Brand colors remain the legacy palette pending approved asset/token intake.
The original emblem and wordmark supplied by the project owner are preserved in
`branding/originals/`, with hashes and dimensions in its manifest. SVGs embed the
original PNGs without redrawing them. MIT does not grant artwork/trademark rights.

Platform utilities extract the reusable portions of core HTTP/path/authlib helpers and metadata; the panel no longer needs the install engine. `PROVENANCE.json` records source and output hashes.

This is the Rust surface, not the complete canonical SDK or generated clients.
No packages have been published. Cross-language code generation, headers/IPC,
extension trust/composition and remaining mixed contracts require later tasks.
