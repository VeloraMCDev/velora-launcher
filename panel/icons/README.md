# Panel icon packs

Builds the locked Iconify packs into compressed `dist` assets served by the Panel.
This is a build input, not an independent hosted service or domain.

From this directory:

```sh
npm ci --no-audit --no-fund
node build.mjs
```

The Panel resolves its icon directory through existing compatible configuration.
Keep individual icon-pack license/attribution requirements; the root licensing
note does not grant blanket rights to artwork.
