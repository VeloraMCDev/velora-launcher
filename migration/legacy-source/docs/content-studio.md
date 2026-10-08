# Content Studio

The Content Studio (admin panel → **Content Studio**) turns packs from other custom-content plugins into things your players can use in game. Items, blocks, chests, decorations, NPCs, vehicles, crops, food and mobs all go through one setup wizard.

## Importing (the main way)

Open **Library → Import** and follow the wizard:

1. **What** — pick what you are bringing in (an item, NPC, block, chest, decoration, vehicle, crop, food or mob).
2. **Where from** — pick the framework: ItemsAdder, Nexo, Oraxen, CraftEngine, MythicMobs or ModelEngine.
3. **Files** — drop the pack as a `.zip` (or the folder zipped). Everything is read in your browser session and sent to the panel; nothing is installed on the server yet.
4. **Review** — each thing found is listed with a preview. Select the ones you want, rename them, change the kind or base item, and spin the model in 3D before you commit. Problems (missing textures, unsupported features) appear as warnings next to the entry.
5. **Done** — the imported content is in your library, in the resource pack, and syncs to every server within about 30 seconds.

| Framework | What is read |
| --- | --- |
| ItemsAdder | `contents/<namespace>/configs`, models and textures |
| Nexo / Oraxen | `items/*.yml` with `Pack`, `Mechanics` and `Components`, plus the pack assets |
| CraftEngine | `items`, `blocks` and `furniture` configs |
| MythicMobs | mob `.yml` files (type, display name, health, damage, armor, speed, drops) |
| ModelEngine | `.bbmodel` Blockbench models, converted to a static model |

Not supported, and reported as warnings instead of failing the import: animations, MythicMobs skills and AI, scripted behaviours and custom GUIs.

## Advanced mode

**Advanced editor** lets you build or tweak anything by hand: base item, model and texture picker, markdown-style name and lore, enchantment and attribute pickers with level sliders, and per-kind options (block hardness and light, chest rows, seats, NPC messages and commands, crop stages and growth time, mob stats and drops).

Name and lore use light markdown: `**bold**`, `*italic*`, `__underline__`, `~~strike~~`, `{gold}`, `{#ff8800}` and `{/}` to reset.

## In game (Paper)

* Players need `scopenet.content.place` to place things (default: everyone) and `scopenet.content.break` to pick them up (default: everyone). `scopenet.content.admin` is needed to place or remove NPCs and mobs (default: ops).
* Right-click a block with the item to place it. **Sneak + right-click** picks a prop up. Blocks take a number of punches based on their hardness.
* Chests keep their contents; decorations can be sat on; NPCs run their messages and console commands when clicked; crops grow through their stages and drop loot when ripe; vehicles are saddled horses, donkeys or mules wearing the model; mobs are real mobs wearing the model, with the stats and drops you set.
* Give yourself anything with `/customitem give <player> <id>`.

Fabric and Forge do not run placeable content yet; items and food still work through the resource pack.

## Limitations

Models are static: a fixed pose, no animation. Props are built from item-display and interaction entities, so very large builds cost entities. Vehicles ride on horse, donkey or mule bodies. The Paper runtime is compiled by the **Checks** workflow (Actions → Checks → Run workflow) once it is on the default branch.

## Keeping the world's textures intact

Packs from other plugins are built to be the only pack on a server, so they often carry replacements for vanilla blocks, block states, colour maps and shaders, plus atlas definitions that stitch whole folders into the block atlas. On a client that also loads other packs or mods, any of that can leave the block atlas and the baked block models out of step (grass solid green, one face of a stair untextured, one texture everywhere). When the server's pack is built, SCOPENET leaves out:

* anything under `assets/minecraft/` that redraws vanilla terrain: `blockstates`, `models/block`, `textures/block`, `textures/colormap`, `textures/entity`, `textures/environment`, particles, shaders and equipment,
* atlas definitions other than `blocks` and `items`, and atlas sources that would pull in a whole folder (or everything),
* PNG files that cannot be read, are damaged, or are larger than 4096 pixels.

Custom items, blocks, NPCs and everything under your own namespaces are untouched. **Resource pack → Check models** lists exactly what was left out. If you really do want to retexture vanilla blocks, turn on **Keep vanilla block overrides** (damaged and oversized images are still removed). Players are also offered the pack a few seconds after they join rather than the instant they appear, so the reload does not race the client's first chunk build.
