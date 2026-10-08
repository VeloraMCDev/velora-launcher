# Cosmetics

Cosmetics are things a player unlocks and equips: titles, badges, particles, pets, join and leave messages, and **modelled cosmetics** (wings, hats, auras). They are made in the admin panel under **Cosmetics Studio**.

## Giving them out

* **Rewards.** On any quest, achievement or level milestone, open **Extra rewards → Add reward → Cosmetic** and pick one.
* **Automatically.** In the Studio, turn on **Grant automatically** and set a global level or an achievement ID. Players who already qualify get it when you save.
* **By hand.** Open a player in **Progression** and pick a cosmetic under **Collection unlocks**.

Players equip from **Collection** in the launcher or web panel, or in game with `/cosmetic`. One item of each type can be equipped at a time.

## In-game command

| Command | What it does |
| --- | --- |
| `/cosmetic` or `/cosmetic list [type]` | Shows everything you have unlocked, with a star on what you are wearing |
| `/cosmetic equip <name>` | Puts one on (a name, a key, or the start of either); replaces what you wore of that type |
| `/cosmetic unequip <name\|type\|all>` | Takes it off |

Aliases: `/cosmetics`, `/wardrobe`. The permission is `scopenet.command.cosmetic` (everyone by default). Tab completion suggests your own items after the first use. The panel stays the source of truth, so changes show in the launcher too.

## Modelled cosmetics

Create a cosmetic, pick a **texture** (a flat sprite) or a **3D model** from your server assets, and choose where it is worn: head, back, hand or around the body. **Size** and **Height** fine-tune it. Upload textures and Blockbench models under **Custom items → Server assets & resource pack**.

Saving gives the cosmetic its own model number and adds it to the server resource pack. Nothing else needs setting up.

### In game (Paper)

The Paper plugin shows the equipped cosmetic on the player as a display entity riding them, wearing the model from the resource pack. It is plain vanilla, so every client that has the server resource pack sees it, the Fabric companion included. Nothing is saved into the world.

* Equipping with `/cosmetic` shows at once; equipping in the launcher or web panel shows within about 20 seconds.
* The wearer does not see a head cosmetic in their own view.
* Switch the feature off with `cosmetics.enabled: false` in `plugins/SCOPENET/config.yml`.
* Positions are starting values that vary by model: nudge **Size** and **Height** in the Studio after trying it on.

The command also exists on the Fabric 1.20.1 server module, but Fabric servers do not draw modelled cosmetics yet.
