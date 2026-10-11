# Interface artwork sources

`vault-icons.json` and `vault-interface.json` contain URL metadata collected from
[ccVaults / ccLeaf](https://ccvaults.com/) on 2026-10-11. They contain no artwork,
authentication keys or session tokens. The applications request the original PNG
assets directly from ccVaults. Its availability and asset paths are external
dependencies; unavailable images fall back to the existing glyph or disappear
when decorative.

Minecraft artwork and ccLeaf artwork retain their respective owners' rights and
the source site's terms. They are not relicensed under Velora's MIT license.
The Admin Panel links to the source and retains Iconify pack license attribution.

The Operations dashboard retains an independent URL catalogue adapter in
`panel/operations/web/src/lib/icons.svelte.ts` so its existing isolated Docker
build context and lockfile remain usable. Keep its URL metadata and semantic
defaults consistent with these catalogues when updating sources.

Administrators can override `nav:<route>`, `ui:<glyph>`, `item:<asset-id>` and
`sprite:<vanilla-path>` through platform branding or instance branding. Guide
articles, interface overrides and logo dimensions use the existing settings and
experience JSON stores; no database-path, protocol or installed-client identity
changes are required.

The launcher and player apps retain the `commands` route for compatibility. The
Player Guide is reachable independently of the command capability; admin command
tools continue to respect that capability. Customize the global guide and icons
under **Design & website → Launcher design** or use instance branding under
**Experience design**. Resize the instance wordmark under **Instance → General**.
