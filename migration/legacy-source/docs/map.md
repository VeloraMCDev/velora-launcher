# The SCOPENET Map

Every server draws its own world map. There is nothing to install, host or configure besides the plugin or mod you already run: the game server renders its world to small tiles in the background, sends only what changed to the panel, and the launcher and admin panel show them in a fast native map.

What's on it:

- **Terrain** drawn in the style of [Pl3xMap](https://github.com/granny/Pl3xMap) (MIT licensed): block colours from the real textures (modded blocks get a matching colour), grass, leaves and water tinted by biome and blended across biome borders, relief shading from how the ground steps up or down, and see-through water that darkens with depth so the sea floor shows in the shallows. The Nether is drawn below its ceiling, so you see caves and lava lakes, not bedrock. Worlds from Minecraft 1.18 through 26.x are read.
- **Guild land** as coloured outlines with the guild's icon and name, holes and all.
- **Spawn, warps, guild homes, markets and shop points** as pins you can click.
- **Players online**, with their head and facing direction. Click one to open the player panel (profile, teleport request, message in game, invite to your guild).
- **Dimensions**: Overworld, Nether and End tabs appear as soon as the server has drawn them.

## Turn it on

It is on by default for every server. To change it:

1. In the admin panel open **Servers → your server → Settings** and use the **SCOPENET Map** switch.
2. In the plugin/mod config: `map.enabled` (Paper `config.yml`) or `map.enabled=true` (Fabric `scopenet.properties`). The old `livemap.enabled` key is still read.

The first tiles should appear after the server starts; the time depends on region size and storage speed. The server draws the regions closest to players first, then works through the rest quietly in the background at low priority.

## In the launcher

Players with a linked server get a **Live Map** button on the Home screen. Pan by dragging, zoom with the wheel or the + and − buttons, double click to zoom in. Use the toggles to hide claims, places or players. Open **Online** to choose individual players and whether their skin heads appear, or to jump to a player; use the crosshair to find yourself and the star to go to spawn. The **Guilds → Territory** claim painter uses the same tiles.

## In the admin panel

**Servers → your server** shows tile counts, the game server’s current rendering stage, region files found, queued and uploaded tiles, recent errors, and the live map itself. **Redraw world** deletes the stored tiles; the server notices and draws everything again (useful after a world reset or a renderer update).

## On your public landing page

Add a **Live map** section in the landing page builder to show the map to visitors, with only the layers you allow. See [discord-and-email.md](discord-and-email.md#live-map-on-the-landing-page).

## How it works

- The game server reads its region files (`region/*.mca`), works out the top block of every column, and writes 256 × 256 pixel tiles at one pixel per block. Zoomed-out levels are built from them, averaging the colours below them.
- Flat tiles are palette PNGs (a few kilobytes) and richer ones are true-colour PNGs (tens of kilobytes). Only tiles that changed are uploaded, and a region is redrawn at most every 90 seconds while players are building.
- The panel stores tiles under its data folder (`map/<server id>/…`) and serves them to signed-in players only, with a short-lived key on every address and ETag caching.
- Player positions go up about every two seconds; claims and pins whenever they change.
- A local cache lives next to the plugin (`plugins/SCOPENET/map`) or mod (`config/scopenet/map`) so a restart doesn't redraw anything.

Disk use is moderate: a world of 500 explored regions is roughly 100–300 MB on the panel. Servers using region files compressed with LZ4 (an opt-in setting in recent Minecraft versions) are not drawn; leave `region-file-compression` on its default.

## Troubleshooting

- **"Drawing the world…" never ends.** Run `/map` on Paper or `/scopenet map` on Fabric to see the server-side stage. Check **Servers → your server** for the same progress and the last error. If no region files are found, check the world folder; if uploads fail, check the panel URL and token. The server log says `SCOPENET Map is on` when it starts drawing.
- **A flat grey or missing area.** The chunk was never generated, or is saved in an older world format (before Minecraft 1.18).
- **Colours look off for a modded block.** Unknown blocks get a colour from their name; vanilla blocks use their texture colour.

- **The map looks different after an update.** When the map style changes, the plugin or mod notices and draws the world again once; the old tiles are replaced as the new ones arrive.
