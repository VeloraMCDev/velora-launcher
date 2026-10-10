package net.scopenet.paper;

import net.scopenet.core.ContentDef;
import net.scopenet.core.ItemSpec;
import net.scopenet.core.Utilities;
import net.scopenet.integration.ChatLayout;
import org.bukkit.Bukkit;
import org.bukkit.GameMode;
import org.bukkit.Location;
import org.bukkit.Material;
import org.bukkit.NamespacedKey;
import org.bukkit.Sound;
import org.bukkit.World;
import org.bukkit.block.Block;
import org.bukkit.block.BlockFace;
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.entity.AbstractHorse;
import org.bukkit.entity.ArmorStand;
import org.bukkit.entity.Display;
import org.bukkit.entity.Entity;
import org.bukkit.entity.EntityType;
import org.bukkit.entity.Interaction;
import org.bukkit.entity.ItemDisplay;
import org.bukkit.entity.LivingEntity;
import org.bukkit.entity.Player;
import org.bukkit.entity.TextDisplay;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.block.Action;
import org.bukkit.event.entity.EntityDamageByEntityEvent;
import org.bukkit.event.entity.EntityDeathEvent;
import org.bukkit.event.inventory.InventoryCloseEvent;
import org.bukkit.event.player.PlayerInteractEntityEvent;
import org.bukkit.event.player.PlayerInteractEvent;
import org.bukkit.event.player.PlayerItemConsumeEvent;
import org.bukkit.event.player.PlayerQuitEvent;
import org.bukkit.inventory.EquipmentSlot;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.InventoryHolder;
import org.bukkit.inventory.ItemStack;
import org.bukkit.inventory.meta.ItemMeta;
import org.bukkit.persistence.PersistentDataContainer;
import org.bukkit.persistence.PersistentDataType;
import org.bukkit.util.Transformation;
import org.joml.AxisAngle4f;
import org.joml.Vector3f;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Random;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.BiFunction;
import java.util.function.Supplier;

/**
 * Runs the Content Studio's placeable content on Paper/Spigot 1.20+: blocks, chests, decorations, NPCs, vehicles, crops and mobs.
 *
 * <p>Everything is built from vanilla pieces so it survives restarts with no database: an {@link ItemDisplay} shows the model (the
 * resource pack re-skins a base item with a model-data number), an {@link Interaction} entity is the hitbox that takes clicks, and
 * the data a prop needs (which content it is, when a crop was planted, what a chest holds) lives in the entities' persistent data.
 * Vehicles and mobs are real mobs, made invisible, with the display riding on them.</p>
 *
 * <p>Players place content by right-clicking a block with its item; sneak + right-click picks a prop up again. Hitting an
 * Interaction entity also breaks it where the server reports that hit.</p>
 */
final class ContentRuntime implements Listener {
    private final ScopenetPlugin plugin;
    private final Supplier<Utilities> utilities;
    private final BiFunction<ItemSpec, Integer, ItemStack> stacks;
    private final Random random = new Random();
    private java.util.function.BiConsumer<Player, ContentDef> dialogue = (player, def) -> {};
    void dialogues(java.util.function.BiConsumer<Player, ContentDef> handler) { dialogue = handler; }

    private final NamespacedKey contentKey, displayKey, extrasKey, plantedKey, inventoryKey, blockKey, seatKey;
    /** Recent punches on a block-like prop: how many, and when the last one landed. */
    private final Map<UUID, long[]> punches = new ConcurrentHashMap<>();

    ContentRuntime(ScopenetPlugin plugin, Supplier<Utilities> utilities, BiFunction<ItemSpec, Integer, ItemStack> stacks) {
        this.plugin = plugin;
        this.utilities = utilities;
        this.stacks = stacks;
        contentKey = new NamespacedKey(plugin, "content");
        displayKey = new NamespacedKey(plugin, "display");
        extrasKey = new NamespacedKey(plugin, "extras");
        plantedKey = new NamespacedKey(plugin, "planted");
        inventoryKey = new NamespacedKey(plugin, "inventory");
        blockKey = new NamespacedKey(plugin, "block");
        seatKey = new NamespacedKey(plugin, "seat");
    }

    void start() {
        Bukkit.getPluginManager().registerEvents(this, plugin);
        // Crops grow on a slow timer; this also catches up crops in chunks that were unloaded while they grew.
        Bukkit.getScheduler().runTaskTimer(plugin, this::growCrops, 100L, 100L);
    }

    // ---- identifying things -------------------------------------------------------------------------

    private ContentDef defOf(ItemStack stack) {
        if (stack == null || stack.getType() == Material.AIR || !stack.hasItemMeta()) return null;
        ItemMeta meta = stack.getItemMeta();
        if (meta == null || !meta.hasCustomModelData()) return null;
        return utilities.get().contentByItem.get(ContentDef.keyOf(stack.getType().getKey().toString(), meta.getCustomModelData()));
    }

    private ContentDef defOf(Entity entity) {
        String id = entity.getPersistentDataContainer().get(contentKey, PersistentDataType.STRING);
        return id == null ? null : utilities.get().content.get(id);
    }

    private ItemStack itemFor(ContentDef def) { return stacks.apply(def.toItemSpec(), 1); }

    /** A drop that names a custom item, a placeable thing, or a vanilla material. */
    private ItemStack dropStack(String item, int amount) {
        Utilities u = utilities.get();
        ItemSpec custom = u.customItems.get(item);
        if (custom != null) return stacks.apply(custom, amount);
        ContentDef placeable = u.content.get(item);
        if (placeable != null) return stacks.apply(placeable.toItemSpec(), amount);
        Material m = Material.matchMaterial(item);
        return m == null || !m.isItem() || m == Material.AIR ? null : new ItemStack(m, Math.max(1, Math.min(amount, m.getMaxStackSize())));
    }

    private void giveOrDrop(Player player, Location at, ItemStack stack) {
        if (stack == null) return;
        for (ItemStack over : player.getInventory().addItem(stack).values()) at.getWorld().dropItemNaturally(at, over);
    }

    private void dropRolled(ContentDef def, Location at) {
        for (ContentDef.Rolled r : ContentDef.rollDrops(def.drops(), random)) {
            ItemStack s = dropStack(r.item(), r.amount());
            if (s != null) at.getWorld().dropItemNaturally(at, s);
        }
    }

    // ---- placing ------------------------------------------------------------------------------------

    @EventHandler(priority = EventPriority.HIGH)
    public void place(PlayerInteractEvent event) {
        if (event.getAction() != Action.RIGHT_CLICK_BLOCK || event.getHand() != EquipmentSlot.HAND || event.getClickedBlock() == null) return;
        ItemStack held = event.getItem();
        ContentDef def = defOf(held);
        if (def == null) return;
        event.setCancelled(true);
        Player player = event.getPlayer();
        if (!player.hasPermission("scopenet.content.place")) { player.sendMessage("§cYou can't place that."); return; }
        if ((def.is("npc") || def.is("mob")) && !player.hasPermission("scopenet.content.admin")) { player.sendMessage("§cOnly admins can place that."); return; }
        Block clicked = event.getClickedBlock();
        Block target = clicked.isPassable() ? clicked : clicked.getRelative(event.getBlockFace());
        if (!target.isPassable() && !target.getType().isAir()) { player.sendMessage("§cThere's no room for that here."); return; }
        if (def.is("crop") && !plantable(target.getRelative(BlockFace.DOWN).getType())) { player.sendMessage("§cThat grows on farmland, dirt or grass."); return; }
        float yaw = Math.round(player.getLocation().getYaw() / 45f) * 45f + 180f;
        boolean ok = switch (def.kind()) {
            case "mob" -> spawnMob(def, target.getLocation().add(0.5, 0, 0.5), yaw) != null;
            case "vehicle" -> spawnVehicle(def, target.getLocation().add(0.5, 0, 0.5), yaw) != null;
            default -> spawnProp(def, target, yaw) != null;
        };
        if (!ok) { player.sendMessage("§cThat couldn't be placed here."); return; }
        target.getWorld().playSound(target.getLocation(), Sound.BLOCK_STONE_PLACE, 1f, 1f);
        if (player.getGameMode() != GameMode.CREATIVE) held.setAmount(held.getAmount() - 1);
    }

    private static boolean plantable(Material m) {
        return m == Material.FARMLAND || m == Material.DIRT || m == Material.GRASS_BLOCK || m == Material.PODZOL || m == Material.COARSE_DIRT || m == Material.ROOTED_DIRT;
    }

    private ItemStack displayStack(ContentDef.Look look) {
        ItemSpec spec = new ItemSpec(look.item(), 1, "", List.of(), Map.of(), false, false, false, look.customModelData(), List.of());
        return stacks.apply(spec, 1);
    }

    private ItemDisplay spawnDisplay(Location at, ContentDef.Look look, float yaw, float height, int light, ContentDef def) {
        World w = at.getWorld();
        Location loc = at.clone();
        loc.setYaw(yaw);
        loc.setPitch(0);
        ItemDisplay d = w.spawn(loc, ItemDisplay.class);
        ItemStack stack = displayStack(look);
        if (stack != null) d.setItemStack(stack);
        d.setItemDisplayTransform(ItemDisplay.ItemDisplayTransform.NONE);
        float s = (float) Math.max(0.05, look.scale());
        // Models are authored standing on y=0; the display is centred on its origin, so lift it by half its height.
        d.setTransformation(new Transformation(new Vector3f(0f, 0.5f * s + height, 0f), new AxisAngle4f(), new Vector3f(s, s, s), new AxisAngle4f()));
        d.setViewRange(2.0f);
        d.setPersistent(true);
        if (light > 0) d.setBrightness(new Display.Brightness(Math.min(15, light), Math.min(15, light)));
        d.getPersistentDataContainer().set(contentKey, PersistentDataType.STRING, def.id());
        return d;
    }

    /** Blocks, chests, decorations, NPCs and crops: a display plus a clickable hitbox (and a barrier block where it must be solid). */
    private Interaction spawnProp(ContentDef def, Block block, float yaw) {
        World w = block.getWorld();
        Location base = block.getLocation().add(0.5, 0, 0.5);
        boolean solid = def.is("block") || def.is("chest") || (def.is("decoration") && def.solid());
        if (solid && !block.getType().isAir() && block.getType() != Material.WATER && !block.isPassable()) return null;
        ItemDisplay display = spawnDisplay(base, def.look(), yaw, 0f, def.is("block") ? def.light() : 0, def);
        Interaction hit = w.spawn(base, Interaction.class);
        boolean blockLike = def.is("block") || def.is("chest");
        hit.setInteractionWidth(blockLike ? 1.0f : (float) def.hitWidth());
        hit.setInteractionHeight(blockLike ? 1.0f : (float) def.hitHeight());
        hit.setResponsive(true);
        hit.setPersistent(true);
        PersistentDataContainer pdc = hit.getPersistentDataContainer();
        pdc.set(contentKey, PersistentDataType.STRING, def.id());
        pdc.set(displayKey, PersistentDataType.STRING, display.getUniqueId().toString());
        List<String> extras = new ArrayList<>();
        if (solid) {
            block.setType(Material.BARRIER);
            pdc.set(blockKey, PersistentDataType.STRING, w.getName() + "," + block.getX() + "," + block.getY() + "," + block.getZ());
        }
        if (def.is("npc") && def.nameVisible()) {
            TextDisplay label = w.spawn(base.clone().add(0, def.hitHeight() + 0.25, 0), TextDisplay.class);
            label.setText(ChatLayout.colorize(def.name().isBlank() ? def.title() : def.name()));
            label.setBillboard(Display.Billboard.CENTER);
            label.setPersistent(true);
            label.getPersistentDataContainer().set(contentKey, PersistentDataType.STRING, def.id());
            extras.add(label.getUniqueId().toString());
        }
        if (def.is("crop")) {
            pdc.set(plantedKey, PersistentDataType.LONG, System.currentTimeMillis());
            setStage(display, def, 0);
        }
        pdc.set(extrasKey, PersistentDataType.STRING, String.join(",", extras));
        return hit;
    }

    private void setStage(ItemDisplay display, ContentDef def, int stage) {
        ItemStack s = displayStack(def.stageLook(stage));
        if (s != null) display.setItemStack(s);
        float sc = (float) Math.max(0.05, def.stageLook(stage).scale());
        display.setTransformation(new Transformation(new Vector3f(0f, 0.5f * sc, 0f), new AxisAngle4f(), new Vector3f(sc, sc, sc), new AxisAngle4f()));
    }

    private LivingEntity spawnMob(ContentDef def, Location at, float yaw) {
        EntityType type = entityType(def.entity());
        if (type == null) return null;
        Location loc = at.clone();
        loc.setYaw(yaw);
        Entity spawned = at.getWorld().spawnEntity(loc, type);
        if (!(spawned instanceof LivingEntity mob)) { spawned.remove(); return null; }
        setAttribute(mob, "MAX_HEALTH", def.health());
        mob.setHealth(Math.min(def.health(), maxHealth(mob)));
        if (def.damage() > 0) setAttribute(mob, "ATTACK_DAMAGE", def.damage());
        if (def.armor() > 0) setAttribute(mob, "ARMOR", def.armor());
        if (def.speed() > 0) setAttribute(mob, "MOVEMENT_SPEED", def.speed());
        String shown = def.name().isBlank() ? def.title() : def.name();
        mob.setCustomName(ChatLayout.colorize(shown));
        mob.setCustomNameVisible(def.nameVisible());
        mob.setRemoveWhenFarAway(false);
        mob.setPersistent(true);
        mob.getPersistentDataContainer().set(contentKey, PersistentDataType.STRING, def.id());
        if (def.look().customModelData() >= 0) wearModel(mob, def);
        return mob;
    }

    /** Hide the vanilla body and ride a display of the model on it. */
    private void wearModel(LivingEntity mob, ContentDef def) {
        mob.setInvisible(true);
        ItemDisplay d = spawnDisplay(mob.getLocation(), def.look(), mob.getLocation().getYaw(), 0f, 0, def);
        // A passenger sits at the vehicle's seat height; lower the display back to the mob's feet.
        float s = (float) Math.max(0.05, def.look().scale());
        d.setTransformation(new Transformation(new Vector3f(0f, 0.5f * s - (float) (mob.getHeight() * 0.75), 0f), new AxisAngle4f(), new Vector3f(s, s, s), new AxisAngle4f()));
        mob.addPassenger(d);
        mob.getPersistentDataContainer().set(displayKey, PersistentDataType.STRING, d.getUniqueId().toString());
    }

    private LivingEntity spawnVehicle(ContentDef def, Location at, float yaw) {
        EntityType type = switch (def.mount()) {
            case "donkey" -> entityType("DONKEY");
            case "mule" -> entityType("MULE");
            default -> entityType("HORSE");
        };
        if (type == null) return null;
        Location loc = at.clone();
        loc.setYaw(yaw);
        Entity spawned = at.getWorld().spawnEntity(loc, type);
        if (!(spawned instanceof AbstractHorse horse)) { spawned.remove(); return null; }
        horse.setTamed(true);
        horse.setAdult();
        horse.getInventory().setSaddle(new ItemStack(Material.SADDLE));
        horse.setSilent(true);
        horse.setInvulnerable(true);
        horse.setRemoveWhenFarAway(false);
        horse.setPersistent(true);
        setAttribute(horse, "MOVEMENT_SPEED", def.speed());
        horse.getPersistentDataContainer().set(contentKey, PersistentDataType.STRING, def.id());
        if (def.look().customModelData() >= 0) wearModel(horse, def);
        else horse.setInvisible(false);
        return horse;
    }

    // ---- removing -----------------------------------------------------------------------------------

    private void removeProp(Interaction anchor, ContentDef def, Player by, boolean giveItem) {
        PersistentDataContainer pdc = anchor.getPersistentDataContainer();
        Location at = anchor.getLocation();
        removeUuid(pdc.get(displayKey, PersistentDataType.STRING));
        String extras = pdc.get(extrasKey, PersistentDataType.STRING);
        if (extras != null && !extras.isBlank()) for (String id : extras.split(",")) removeUuid(id);
        String where = pdc.get(blockKey, PersistentDataType.STRING);
        if (where != null) {
            String[] p = where.split(",");
            World w = Bukkit.getWorld(p[0]);
            if (w != null && p.length == 4) {
                try {
                    Block b = w.getBlockAt(Integer.parseInt(p[1]), Integer.parseInt(p[2]), Integer.parseInt(p[3]));
                    if (b.getType() == Material.BARRIER) b.setType(Material.AIR);
                } catch (NumberFormatException ignored) { /* corrupted marker, nothing to clear */ }
            }
        }
        if (def.is("chest")) {
            for (ItemStack s : loadInventory(pdc.get(inventoryKey, PersistentDataType.STRING), def.rows() * 9)) if (s != null) at.getWorld().dropItemNaturally(at, s);
        }
        anchor.remove();
        punches.remove(anchor.getUniqueId());
        if (def.is("block")) {
            if (def.drops().isEmpty()) { if (giveItem) giveOrDrop(by, at, itemFor(def)); } else dropRolled(def, at);
        } else if (def.is("crop")) {
            if (giveItem) giveOrDrop(by, at, itemFor(def));
        } else if (giveItem) {
            giveOrDrop(by, at, itemFor(def));
        }
        at.getWorld().playSound(at, Sound.BLOCK_STONE_BREAK, 1f, 1f);
    }

    private void removeUuid(String id) {
        if (id == null || id.isBlank()) return;
        try {
            Entity e = Bukkit.getEntity(UUID.fromString(id));
            if (e != null) e.remove();
        } catch (IllegalArgumentException ignored) { /* not a uuid */ }
    }

    private void removeVehicleOrMob(LivingEntity mob) {
        removeUuid(mob.getPersistentDataContainer().get(displayKey, PersistentDataType.STRING));
        for (Entity passenger : new ArrayList<>(mob.getPassengers())) passenger.remove();
        mob.remove();
    }

    // ---- clicks -------------------------------------------------------------------------------------

    @EventHandler(priority = EventPriority.HIGH)
    public void click(PlayerInteractEntityEvent event) {
        if (event.getHand() != EquipmentSlot.HAND) return;
        Entity clicked = event.getRightClicked();
        ContentDef def = defOf(clicked);
        if (def == null) return;
        Player player = event.getPlayer();
        if (clicked instanceof LivingEntity living) {
            if (player.isSneaking() && def.is("vehicle") && player.hasPermission("scopenet.content.break")) {
                event.setCancelled(true);
                removeVehicleOrMob(living);
                giveOrDrop(player, living.getLocation(), itemFor(def));
            }
            return; // riding a vehicle or poking a mob is vanilla behaviour
        }
        if (!(clicked instanceof Interaction anchor)) return;
        event.setCancelled(true);
        if (player.isSneaking() && !def.is("chest")) {
            if (def.is("npc") && !player.hasPermission("scopenet.content.admin")) return;
            if (player.hasPermission("scopenet.content.break")) removeProp(anchor, def, player, player.getGameMode() != GameMode.CREATIVE);
            return;
        }
        switch (def.kind()) {
            case "chest" -> openChest(player, anchor, def);
            case "decoration" -> { if (def.seat()) sit(player, anchor); }
            case "npc" -> talk(player, def);
            case "crop" -> harvest(player, anchor, def);
            default -> { /* blocks have no right-click behaviour of their own */ }
        }
    }

    @EventHandler(priority = EventPriority.HIGH)
    public void hit(EntityDamageByEntityEvent event) {
        Entity victim = event.getEntity();
        ContentDef def = defOf(victim);
        if (def == null) return;
        if (victim instanceof Interaction anchor) {
            event.setCancelled(true);
            if (!(event.getDamager() instanceof Player player)) return;
            if (def.is("npc") && !player.hasPermission("scopenet.content.admin")) return;
            if (!player.hasPermission("scopenet.content.break")) return;
            long now = System.currentTimeMillis();
            long[] state = punches.compute(anchor.getUniqueId(), (k, v) -> v == null || now - v[1] > 2000 ? new long[] {1, now} : new long[] {v[0] + 1, now});
            anchor.getWorld().playSound(anchor.getLocation(), Sound.BLOCK_STONE_HIT, 0.8f, 1f);
            if (!def.is("block") || state[0] >= def.punchesToBreak()) removeProp(anchor, def, player, player.getGameMode() != GameMode.CREATIVE);
        } else if (def.is("vehicle")) {
            event.setCancelled(true); // vehicles cannot be hurt; sneak + right-click picks them up
        }
    }

    @EventHandler
    public void death(EntityDeathEvent event) {
        ContentDef def = defOf(event.getEntity());
        if (def == null || !def.is("mob")) return;
        removeUuid(event.getEntity().getPersistentDataContainer().get(displayKey, PersistentDataType.STRING));
        for (Entity passenger : new ArrayList<>(event.getEntity().getPassengers())) passenger.remove();
        if (!def.drops().isEmpty()) {
            event.getDrops().clear();
            for (ContentDef.Rolled r : ContentDef.rollDrops(def.drops(), random)) {
                ItemStack s = dropStack(r.item(), r.amount());
                if (s != null) event.getDrops().add(s);
            }
        }
    }

    // ---- kinds --------------------------------------------------------------------------------------

    private void talk(Player player, ContentDef def) {
        for (String line : def.messages()) player.sendMessage(ChatLayout.colorize(line.replace("{player}", player.getName())));
        if (def.commands().isEmpty() && !def.messages().isEmpty()) dialogue.accept(player, def);
        for (String command : def.commands()) {
            String c = command.startsWith("/") ? command.substring(1) : command;
            Bukkit.dispatchCommand(Bukkit.getConsoleSender(), c.replace("{player}", player.getName()));
        }
    }

    private void sit(Player player, Interaction anchor) {
        if (player.getVehicle() != null) return;
        Location at = anchor.getLocation().add(0, Math.max(0.1, anchor.getInteractionHeight() * 0.35) - 0.9, 0);
        ArmorStand seat = anchor.getWorld().spawn(at, ArmorStand.class);
        seat.setVisible(false);
        seat.setGravity(false);
        seat.setMarker(true);
        seat.setSmall(true);
        seat.setPersistent(false);
        seat.getPersistentDataContainer().set(seatKey, PersistentDataType.BYTE, (byte) 1);
        seat.addPassenger(player);
    }

    @EventHandler
    public void dismount(org.spigotmc.event.entity.EntityDismountEvent event) {
        Entity seat = event.getDismounted();
        if (seat instanceof ArmorStand && seat.getPersistentDataContainer().has(seatKey, PersistentDataType.BYTE)) {
            Bukkit.getScheduler().runTask(plugin, seat::remove);
        }
    }

    @EventHandler
    public void quit(PlayerQuitEvent event) {
        Entity vehicle = event.getPlayer().getVehicle();
        if (vehicle instanceof ArmorStand && vehicle.getPersistentDataContainer().has(seatKey, PersistentDataType.BYTE)) vehicle.remove();
    }

    // chests

    private static final class ChestHolder implements InventoryHolder {
        final UUID anchor;
        private Inventory inventory;
        ChestHolder(UUID anchor) { this.anchor = anchor; }
        @Override public Inventory getInventory() { return inventory; }
    }

    private void openChest(Player player, Interaction anchor, ContentDef def) {
        ChestHolder holder = new ChestHolder(anchor.getUniqueId());
        String title = ChatLayout.colorize(def.chestTitle().isBlank() ? (def.name().isBlank() ? def.title() : def.name()) : def.chestTitle());
        Inventory inv = Bukkit.createInventory(holder, def.rows() * 9, title);
        holder.inventory = inv;
        ItemStack[] saved = loadInventory(anchor.getPersistentDataContainer().get(inventoryKey, PersistentDataType.STRING), def.rows() * 9);
        inv.setContents(saved);
        player.openInventory(inv);
        player.playSound(player.getLocation(), Sound.BLOCK_CHEST_OPEN, 0.6f, 1f);
    }

    @EventHandler
    public void closeChest(InventoryCloseEvent event) {
        if (!(event.getInventory().getHolder() instanceof ChestHolder holder)) return;
        Entity anchor = Bukkit.getEntity(holder.anchor);
        if (anchor != null) anchor.getPersistentDataContainer().set(inventoryKey, PersistentDataType.STRING, saveInventory(event.getInventory().getContents()));
        if (event.getPlayer() instanceof Player p) p.playSound(p.getLocation(), Sound.BLOCK_CHEST_CLOSE, 0.6f, 1f);
    }

    private static String saveInventory(ItemStack[] contents) {
        YamlConfiguration yaml = new YamlConfiguration();
        yaml.set("items", java.util.Arrays.asList(contents));
        return yaml.saveToString();
    }

    private static ItemStack[] loadInventory(String data, int size) {
        ItemStack[] out = new ItemStack[size];
        if (data == null || data.isBlank()) return out;
        try {
            YamlConfiguration yaml = new YamlConfiguration();
            yaml.loadFromString(data);
            List<?> list = yaml.getList("items");
            if (list != null) for (int i = 0; i < Math.min(size, list.size()); i++) if (list.get(i) instanceof ItemStack s) out[i] = s;
        } catch (Exception ignored) { /* an unreadable save is treated as empty */ }
        return out;
    }

    // crops

    private void harvest(Player player, Interaction anchor, ContentDef def) {
        long planted = anchor.getPersistentDataContainer().getOrDefault(plantedKey, PersistentDataType.LONG, System.currentTimeMillis());
        long now = System.currentTimeMillis();
        int count = def.stages().size();
        int stage = ContentDef.cropStage(planted, now, def.growthSeconds(), count);
        if (stage < def.lastStage()) {
            long wait = ContentDef.millisToNextStage(planted, now, def.growthSeconds(), count);
            player.sendMessage("§7Still growing — " + Math.max(1, wait / 1000) + "s until its next stage.");
            return;
        }
        dropRolled(def, anchor.getLocation());
        anchor.getWorld().playSound(anchor.getLocation(), Sound.BLOCK_CROP_BREAK, 1f, 1f);
        if (def.replant()) {
            anchor.getPersistentDataContainer().set(plantedKey, PersistentDataType.LONG, now);
            Entity d = displayOf(anchor);
            if (d instanceof ItemDisplay display) setStage(display, def, 0);
        } else {
            removeProp(anchor, def, player, false);
        }
    }

    private Entity displayOf(Interaction anchor) {
        String id = anchor.getPersistentDataContainer().get(displayKey, PersistentDataType.STRING);
        if (id == null) return null;
        try { return Bukkit.getEntity(UUID.fromString(id)); } catch (IllegalArgumentException e) { return null; }
    }

    private void growCrops() {
        long now = System.currentTimeMillis();
        for (World world : Bukkit.getWorlds()) {
            for (Interaction anchor : world.getEntitiesByClass(Interaction.class)) {
                PersistentDataContainer pdc = anchor.getPersistentDataContainer();
                if (!pdc.has(plantedKey, PersistentDataType.LONG)) continue;
                ContentDef def = defOf(anchor);
                if (def == null || !def.is("crop") || def.stages().size() < 2) continue;
                int stage = ContentDef.cropStage(pdc.get(plantedKey, PersistentDataType.LONG), now, def.growthSeconds(), def.stages().size());
                Entity d = displayOf(anchor);
                if (!(d instanceof ItemDisplay display)) continue;
                ItemStack current = display.getItemStack();
                ItemStack wanted = displayStack(def.stageLook(stage));
                if (wanted != null && (current == null || !current.isSimilar(wanted))) setStage(display, def, stage);
            }
        }
    }

    // ---- custom food --------------------------------------------------------------------------------

    @EventHandler(priority = EventPriority.HIGH, ignoreCancelled = true)
    public void eat(PlayerItemConsumeEvent event) {
        ItemStack item = event.getItem();
        if (item == null || !item.hasItemMeta() || !item.getItemMeta().hasCustomModelData()) return;
        ContentDef.Food food = utilities.get().foods.get(ContentDef.keyOf(item.getType().getKey().toString(), item.getItemMeta().getCustomModelData()));
        if (food == null) return;
        Player player = event.getPlayer();
        event.setCancelled(true);
        ItemStack main = player.getInventory().getItemInMainHand();
        if (main != null && main.isSimilar(item)) main.setAmount(main.getAmount() - 1);
        else {
            ItemStack off = player.getInventory().getItemInOffHand();
            if (off != null && off.isSimilar(item)) off.setAmount(off.getAmount() - 1);
        }
        int level = Math.min(20, player.getFoodLevel() + food.nutrition());
        player.setFoodLevel(level);
        player.setSaturation((float) Math.min(level, player.getSaturation() + food.saturation()));
        player.playSound(player.getLocation(), Sound.ENTITY_PLAYER_BURP, 0.7f, 1f);
    }

    // ---- reflection helpers (these enums became registries in newer Minecraft versions) ---------------

    private static EntityType entityType(String name) {
        String n = name.toUpperCase(Locale.ROOT);
        try { return (EntityType) EntityType.class.getField(n).get(null); }
        catch (ReflectiveOperationException | ClassCastException e) {
            try { return EntityType.valueOf(n); } catch (IllegalArgumentException | NoClassDefFoundError ex) { return null; }
        }
    }

    private static org.bukkit.attribute.Attribute attribute(String base) {
        for (String candidate : new String[] {"GENERIC_" + base, base}) {
            try { return (org.bukkit.attribute.Attribute) org.bukkit.attribute.Attribute.class.getField(candidate).get(null); }
            catch (ReflectiveOperationException ignored) { /* try the other spelling */ }
        }
        return null;
    }

    private static void setAttribute(LivingEntity entity, String base, double value) {
        org.bukkit.attribute.Attribute a = attribute(base);
        org.bukkit.attribute.AttributeInstance inst = a == null ? null : entity.getAttribute(a);
        if (inst != null) inst.setBaseValue(value);
    }

    private static double maxHealth(LivingEntity entity) {
        org.bukkit.attribute.Attribute a = attribute("MAX_HEALTH");
        org.bukkit.attribute.AttributeInstance inst = a == null ? null : entity.getAttribute(a);
        return inst == null ? 20 : inst.getValue();
    }
}
