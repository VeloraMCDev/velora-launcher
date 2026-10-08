# Kits, custom items and server assets

The Paper plugin and Fabric 1.20.1 feature module support these commands. Other loader/version integrations retain their existing feature coverage.

## Create a kit in game

Fill your inventory's storage slots and hotbar, then run `/kit create <name>`. Operators and players granted `scopenet.admin.kits` may create kits. Kit names use 1–24 letters, numbers, underscores or hyphens; `create` is reserved. Existing kits are never overwritten and empty inventories are rejected.

The command copies the 36 inventory storage slots without consuming items. Armor and offhand slots are excluded. The snapshot preserves the platform's item metadata, including durability, enchantments, containers and plugin data. Captured items work on the same platform: Bukkit snapshots on Paper, NBT snapshots on Fabric 1.20.1. Unsupported formats are reported instead of reconstructed as plain items.

The new kit appears under **Commands & kits** and reaches servers on their next sync. Configure its groups, cooldown and one-time flag there. Items captured in game retain their original metadata; recreate the kit's item entries in game to change that metadata, or replace entries with items designed in the panel.

## Create admin claims from the map

Open a server's **SCOPENET Map** in the admin panel and choose **Create admin claim**. Click the first and opposite corners of the region. The highlighted rectangle snaps to chunk boundaries, including negative coordinates. Enter its name, description and colour, then create it. Changing dimensions starts a new selection.

A selection can contain at most 4096 chunks. Existing guild and admin claims are skipped and the result reports how many chunks were added or skipped. The normal claim sync distributes protection to the game servers. The public map and launcher map do not expose claim-editing controls.

## Custom item rewards

Create an item under **Custom items**, then choose **Custom item** in an achievement, quest or leveling milestone's **Extra rewards**. Pick the item, quantity (1–6400), and optional target server. The reward captures the item's definition when the reward queue is processed, and delivery retains its name, lore, enchantments, attributes and model data. Inventory overflow follows the platform's normal behavior of dropping excess items at the player.

An item referenced by a kit or reward bundle cannot be deleted until those references are removed.

## Textures, models, sounds and fonts

The **Server assets & resource pack** section under **Custom items** accepts PNG, JSON and Ogg files at standard resource-pack paths, such as:

- `assets/scopenet/textures/item/blade.png`
- `assets/scopenet/models/item/blade_3d.json`
- `assets/scopenet/font/default.json`
- `assets/scopenet/sounds/level_up.ogg` and `assets/scopenet/sounds.json`

Upload a texture first, then set a custom item's vanilla base material (for example `minecraft:diamond_sword`), **Model data** (for example `10000`), and **Texture** (`scopenet:item/blade`, without `.png`). SCOPENET generates the flat item model and connects it to the base material. Alternatively upload a JSON model and enter its reference in **Model**. Model data must be unique for each textured item sharing a base material. Uploaded JSON models can reference additional textures uploaded to the pack.

The generated pack contains legacy model predicates and modern 1.21.4+ item definitions. Set **Pack format** to match your Minecraft clients; the default is 15 for 1.20–1.20.1. For complex vanilla bases such as bows, shields, armor and animated items, upload their base model/modern item definition to preserve the relevant rendering and state logic. This system adds content through vanilla item metadata and resource packs; it does not register new modded block or item types or import Oraxen/ItemsAdder configurations.

Enable **Offer pack to players**, optionally **Require acceptance**, and save. Paper and Fabric offer new pack revisions to connected players, and to players who join later, within about a minute. The panel URL configured on the game server must be reachable by players. Revision URLs serve the exact offered ZIP/hash; the last three offered pack revisions are retained. You can also download the pack to inspect it or distribute it through an existing server resource-pack setup.

Uploads are restricted to administrators, up to 2 MiB per file and 256 files / approximately 48 MiB total. Texture dimensions are limited to 4096×4096. A texture or model directly referenced by an item cannot be deleted while that item uses it. Resources referenced inside authored JSON must be maintained together.

The approach follows the vanilla material, item metadata and generated resource-pack model described by [Oraxen](https://docs.oraxen.com/configuration/items) and the custom-content/resource-pack structure documented by [ItemsAdder](https://wiki.itemsadder.com/adding-content/items/item-properties/resource/).

## Command discovery

`/scopenet help` includes utility commands, custom items, admin claims, in-game kit creation and the newer guild/market commands. Tab completion is registered for Paper commands and aliases, with context-aware choices for homes, warps, players, kits, vaults, custom items and subcommands. Fabric completes individual arguments, including nested guild and market subcommands. Free-text names, messages and arbitrary numbers remain user input.

## Importing Oraxen and ItemsAdder packs

Under **Custom items → Server assets & resource pack**, choose **Import an Oraxen or ItemsAdder pack** and pick a zip of your pack folder.

* **Oraxen** — zip `plugins/Oraxen/pack` and `plugins/Oraxen/items` (or a built pack with `assets/`). Textures and models in `pack/` land in the `minecraft` namespace, as in Oraxen.
* **ItemsAdder** — zip `plugins/ItemsAdder/contents` (or one pack folder inside it). Both `resourcepack/<namespace>/…` and `resourcepack/assets/<namespace>/…` layouts work, and each item's `resource.textures` / `resource.model_path` are resolved in the pack's namespace.

The importer shows a preview first. On import, textures, models, sounds, fonts and language files are added to the server assets, and every item with a texture or model becomes a custom item (same id as in the plugin). Items whose id already exists are left alone. If the plugin didn't pin a Custom Model Data number, the importer assigns one starting at 100000, unique per base item. Names have MiniMessage tags (`<gradient>`, `<#ff0000>`) removed, keeping `&` colour codes.

Plugin-specific behaviour (Oraxen mechanics, ItemsAdder furniture, custom blocks, recipes and font icons that run through the plugin) is not imported — only how the items look and what they're called.

### The import wizard and the item creator

Dropping a pack zip shows a preview card for every item: a slot with its real texture, the tooltip as players will see it, its model data number and base item. Rename items, change the base item, or untick the ones you don't want, then choose:

* **Create the items now** — they exist the moment the import finishes (`/customitem give <player> <id>`, kits, rewards, shops). The finish screen has a copy-command button and *Open in creator*.
* **Import the models only** — textures and models go into the server assets and you build the items yourself.

Either way every item is editable afterwards. In the Custom Item Creator, **Appearance** shows what the item looks like and lets you **Upload PNG** (or drop one on the box), **Choose texture** or **Choose model** from everything in the server assets; the model data number is assigned for you.

### Why an imported model can show black and magenta

That is Minecraft's "missing texture". There are two causes, both handled:

* **Textures in their own folders.** Minecraft only loads `textures/item` and `textures/block` on its own. Packs that keep textures elsewhere (`textures/default/…`, `textures/medieval/…`) ship an `atlases/*.json` telling the game to load those folders. The importer now keeps those files, and the generated pack adds an atlas entry for every extra texture folder, so models load their textures wherever they are.
* **Textures missing from the zip.** The import preview marks any item whose model names textures that aren't in the zip with a ⚠ so you can fix the zip before importing (for Oraxen include the whole `pack/` folder; for ItemsAdder the whole `contents/<pack>/resourcepack/`).

After a change, rejoin the server (or use `/reload` of the pack) so clients download the rebuilt pack.

**Check models.** Under *Server assets*, **Check models** lists every model that names a texture or parent model the pack doesn't contain. When a reference is only missing its namespace (Minecraft reads a bare `default/x` as `minecraft:default/x`), the pack repairs it for you when it is built; anything else needs the file added. The importer also reads ItemsAdder's `contents/<pack>/textures` and `models` folders and `.mcmeta` animation files, and registers every texture a model uses outside `item/` and `block/` with the game's texture atlases.
