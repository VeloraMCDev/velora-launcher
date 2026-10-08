package net.scopenet.paper;

import net.scopenet.core.*;
import net.scopenet.integration.ChatLayout;
import net.scopenet.integration.Integration;
import org.bukkit.Bukkit;
import org.bukkit.Material;
import org.bukkit.NamespacedKey;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.command.PluginCommand;
import org.bukkit.command.TabCompleter;
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.enchantments.Enchantment;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.entity.EntityDamageEvent;
import org.bukkit.event.inventory.InventoryCloseEvent;
import org.bukkit.event.player.PlayerQuitEvent;
import org.bukkit.GameMode;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.InventoryHolder;
import org.bukkit.inventory.ItemFlag;
import org.bukkit.inventory.ItemStack;
import org.bukkit.inventory.meta.ItemMeta;

import java.io.File;
import java.io.IOException;
import java.nio.file.Path;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Brings the shared utility commands (/heal /feed /fly /vault /echest /kit /customitem /adminclaim) and the claim banner
 * to Paper: a {@link Platform} over the Bukkit API, vault storage, and command binding. The behaviour itself lives in
 * {@link UtilityHub}, which the Fabric mod uses too.
 */
final class UtilityBridge implements Listener {
    private final ScopenetPlugin plugin;
    private final Integration integration;
    private final UtilityHub hub;
    private final Env hubEnv;
    private final List<CoreCommand> commands;
    private final ResourcePackPoller packs;
    private final File vaultDir;
    /** Players who just stopped flying: fall damage is waived until they land (or a few seconds pass). */
    private final Map<UUID, Long> softLanding = new ConcurrentHashMap<>();

    UtilityBridge(ScopenetPlugin plugin, Integration integration) {
        this.plugin = plugin;
        this.integration = integration;
        this.vaultDir = new File(plugin.getDataFolder(), "vaults");
        Env env = new Env(new PaperHubPlatform(), new PanelAdapter(integration), Features.all(), plugin.getDataFolder().toPath(),
                System::currentTimeMillis, new Random(), plugin.getLogger());
        this.hubEnv = env;
        this.packs = new ResourcePackPoller(env, () -> integration.settings().panel().toString());
        this.hub = new UtilityHub(env, integration::utilities, new UtilityStore(plugin.getDataFolder().toPath().resolve("utility-state.json")));
        this.commands = hub.commands();
    }

    /** Told when a player changes what they wear with /cosmetic. */
    void cosmeticsChanged(java.util.function.Consumer<Player> listener) {
        hub.cosmeticsChanged = p -> {
            Player online = Bukkit.getPlayer(p.uuid());
            if (online != null) listener.accept(online);
        };
    }

    void start() {
        plugin.getServer().getPluginManager().registerEvents(this, plugin);
        for (CoreCommand c : commands) {
            PluginCommand cmd = plugin.getCommand(c.name());
            if (cmd == null) { plugin.getLogger().warning("Could not bind command '/" + c.name() + "' - not found in plugin.yml"); continue; }
            Bridge bridge = new Bridge(c);
            cmd.setExecutor(bridge);
            cmd.setTabCompleter(bridge);
        }
        Bukkit.getScheduler().runTaskTimer(plugin, hub::tick, 40L, 20L);
        Bukkit.getScheduler().runTaskTimer(plugin, packs::tick, 40L, 20L);
    }

    /** The shared guild-management commands (/guild list, kick, promote, post…) for Paper's /guild. */
    void guildManage(Player player, String[] args) {
        new GuildManage(hubEnv).run(new PaperHubPlayer(player), args[0], java.util.Arrays.copyOfRange(args, 1, args.length));
    }

    void stop() {
        for (Player p : Bukkit.getOnlinePlayers()) {
            if (p.getOpenInventory().getTopInventory().getHolder() instanceof VaultHolder h) save(h, p.getOpenInventory().getTopInventory());
        }
    }

    boolean giveSpec(UUID id, ItemSpec spec) { return new PaperHubPlatform().giveSpec(id, spec); }

    /** A styled stack for a designed item, or {@code null} when its material does not exist on this server. */
    ItemStack stack(ItemSpec spec, int amount) {
        Material m = Material.matchMaterial(spec.item());
        if (m == null && spec.item().startsWith("minecraft:")) m = Material.matchMaterial(spec.item().substring(10));
        if (m == null || !m.isItem() || m == Material.AIR) return null;
        ItemStack stack = new ItemStack(m, Math.max(1, Math.min(amount, m.getMaxStackSize())));
        if (!spec.isPlain()) style(stack, spec);
        return stack;
    }

    // ---- Bukkit command <-> CoreCommand ---------------------------------------------------------------

    private final class Bridge implements CommandExecutor, TabCompleter {
        private final CoreCommand core;
        Bridge(CoreCommand core) { this.core = core; }

        @Override public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
            if (!(sender instanceof Player player)) { sender.sendMessage("This command is for players."); return true; }
            String node = core.permission();
            if (node != null && !node.isBlank() && !player.hasPermission(node)) {
                player.sendMessage("§cYou don't have permission to do that.");
                return true;
            }
            core.run(new PaperHubPlayer(player), args);
            return true;
        }

        @Override public List<String> onTabComplete(CommandSender sender, Command command, String alias, String[] args) {
            if (!(sender instanceof Player player)) return List.of();
            if (!core.permission().isBlank() && !player.hasPermission(core.permission())) return List.of();
            String last = args.length == 0 ? "" : args[args.length - 1].toLowerCase(Locale.ROOT);
            List<String> out = new ArrayList<>();
            for (String s : core.complete(new PaperHubPlayer(player), args)) if (s.toLowerCase(Locale.ROOT).startsWith(last)) out.add(s);
            return out;
        }
    }

    // ---- events ---------------------------------------------------------------------------------------

    /** A pack the client could not download or apply is offered again (a couple of times); declining it is the player's choice and is left alone. */
    @EventHandler public void packStatus(org.bukkit.event.player.PlayerResourcePackStatusEvent event) {
        String status = event.getStatus().name();
        if (status.equals("FAILED_DOWNLOAD") || status.equals("INVALID_URL") || status.equals("FAILED_RELOAD") || status.equals("DISCARDED")) {
            if (!packs.failed(event.getPlayer().getUniqueId())) plugin.getLogger().warning(event.getPlayer().getName() + " could not load the resource pack (" + status + "); not retrying again this session.");
        }
    }

    @EventHandler public void quit(PlayerQuitEvent event) {
        hub.left(event.getPlayer().getUniqueId());
        packs.left(event.getPlayer().getUniqueId());
        softLanding.remove(event.getPlayer().getUniqueId());
    }

    @EventHandler(priority = EventPriority.HIGH, ignoreCancelled = true)
    public void fall(EntityDamageEvent event) {
        if (event.getCause() != EntityDamageEvent.DamageCause.FALL || !(event.getEntity() instanceof Player p)) return;
        Long until = softLanding.get(p.getUniqueId());
        if (until == null) return;
        if (System.currentTimeMillis() < until) event.setCancelled(true);
        softLanding.remove(p.getUniqueId());
    }

    @EventHandler public void close(InventoryCloseEvent event) {
        Inventory top = event.getInventory();
        if (top.getHolder() instanceof VaultHolder h) save(h, top);
    }

    // ---- vault storage --------------------------------------------------------------------------------

    private record VaultHolder(UUID owner, int number) implements InventoryHolder {
        @Override public Inventory getInventory() { return null; }
    }

    private File vaultFile(UUID owner) { return new File(vaultDir, owner + ".yml"); }

    private void save(VaultHolder h, Inventory inv) {
        try {
            vaultDir.mkdirs();
            File file = vaultFile(h.owner());
            YamlConfiguration yaml = YamlConfiguration.loadConfiguration(file);
            yaml.set("v" + h.number(), null);
            for (int slot = 0; slot < inv.getSize(); slot++) {
                ItemStack item = inv.getItem(slot);
                if (item != null && item.getType() != Material.AIR) yaml.set("v" + h.number() + "." + slot, item);
            }
            yaml.save(file);
        } catch (IOException e) {
            plugin.getLogger().severe("Could not save vault " + h.number() + " of " + h.owner() + ": " + e.getMessage());
        }
    }

    // ---- the platform the hub talks to ------------------------------------------------------------------

    /** The one id every SCOPENET pack revision is sent under, so a new revision replaces the old one. */
    private static final UUID PACK_ID = UUID.nameUUIDFromBytes("scopenet:resource-pack".getBytes(java.nio.charset.StandardCharsets.UTF_8));
    private static java.lang.reflect.Method PACK_OFFER;

    private final class PaperHubPlatform implements Platform {
        @Override public boolean offerResourcePack(UUID id, String url, String sha1, boolean required) {
            Player p = Bukkit.getPlayer(id); if (p == null) return false;
            byte[] hash = java.util.HexFormat.of().parseHex(sha1);
            // Minecraft 1.20.3+ identifies server packs by id and stacks packs with different ids. The id Paper derives from the URL changes
            // with every revision, so a player who stayed connected (or moved between servers behind a proxy) ended up with the old and
            // the new pack loaded together, which is how textures broke for some players. One fixed id makes each revision replace the last.
            try {
                if (PACK_OFFER == null) PACK_OFFER = Player.class.getMethod("setResourcePack", UUID.class, String.class, byte[].class, String.class, boolean.class);
                PACK_OFFER.invoke(p, PACK_ID, url, hash, "SCOPENET custom assets", required);
            } catch (ReflectiveOperationException | RuntimeException e) {
                p.setResourcePack(url, hash, "SCOPENET custom assets", required); // older servers know only one pack at a time
            }
            return true;
        }
        @Override public Optional<CorePlayer> player(UUID uuid) { return Optional.ofNullable(Bukkit.getPlayer(uuid)).map(PaperHubPlayer::new); }
        @Override public Optional<CorePlayer> playerByName(String name) { return Optional.ofNullable(Bukkit.getPlayerExact(name)).map(PaperHubPlayer::new); }
        @Override public Collection<? extends CorePlayer> online() {
            List<CorePlayer> out = new ArrayList<>();
            for (Player p : Bukkit.getOnlinePlayers()) out.add(new PaperHubPlayer(p));
            return out;
        }
        @Override public Optional<Pos> safeSurface(String world, int x, int z) { return Optional.empty(); }
        @Override public Pos worldSpawn(String world) {
            org.bukkit.Location l = Bukkit.getWorlds().get(0).getSpawnLocation();
            return new Pos(ScopenetPlugin.dimension(Bukkit.getWorlds().get(0)), l.getX(), l.getY(), l.getZ(), 0, 0);
        }
        @Override public void runMain(Runnable task) { if (Bukkit.isPrimaryThread()) task.run(); else Bukkit.getScheduler().runTask(plugin, task); }
        @Override public void runAsync(Runnable task) { Bukkit.getScheduler().runTaskAsynchronously(plugin, task); }
        @Override public boolean console(String command) { return Bukkit.dispatchCommand(Bukkit.getConsoleSender(), command); }

        @Override public boolean giveItem(UUID id, String item, int amount) { return giveSpec(id, ItemSpec.plain(item, amount)); }

        @Override public com.google.gson.JsonArray inventorySnapshot(UUID id) {
            var out = new com.google.gson.JsonArray();
            Player player = Bukkit.getPlayer(id);
            if (player == null) return out;
            for (ItemStack stack : player.getInventory().getStorageContents()) {
                if (stack == null || stack.getType().isAir()) continue;
                YamlConfiguration yaml = new YamlConfiguration();
                yaml.set("item", stack.clone());
                var item = new com.google.gson.JsonObject();
                item.addProperty("item", stack.getType().getKey().toString());
                item.addProperty("amount", stack.getAmount());
                var data = new com.google.gson.JsonObject();
                data.addProperty("format", "bukkit");
                data.addProperty("value", yaml.saveToString());
                item.add("data", data);
                out.add(item);
            }
            return out;
        }

        @Override public boolean giveSpec(UUID id, ItemSpec spec) {
            Player p = Bukkit.getPlayer(id);
            if (p == null) return false;
            if (!spec.dataFormat().isEmpty()) {
                if (!spec.dataFormat().equals("bukkit")) return false;
                try {
                    YamlConfiguration yaml = new YamlConfiguration();
                    yaml.loadFromString(spec.dataValue());
                    ItemStack template = yaml.getItemStack("item");
                    if (template == null || template.getType().isAir()) return false;
                    for (int left = spec.amount(); left > 0;) {
                        int n = Math.min(left, template.getMaxStackSize());
                        ItemStack stack = template.clone(); stack.setAmount(n);
                        for (ItemStack over : p.getInventory().addItem(stack).values()) p.getWorld().dropItemNaturally(p.getLocation(), over);
                        left -= n;
                    }
                    return true;
                } catch (Exception e) { return false; }
            }
            Material m = Material.matchMaterial(spec.item());
            if (m == null && spec.item().startsWith("minecraft:")) m = Material.matchMaterial(spec.item().substring(10));
            if (m == null || !m.isItem() || m == Material.AIR) return false;
            for (int left = Math.max(1, spec.amount()); left > 0; ) {
                int n = Math.min(left, m.getMaxStackSize());
                ItemStack stack = new ItemStack(m, n);
                if (!spec.isPlain()) style(stack, spec);
                for (ItemStack over : p.getInventory().addItem(stack).values()) p.getWorld().dropItemNaturally(p.getLocation(), over);
                left -= n;
            }
            return true;
        }

        @Override public void heal(UUID id, double amount) {
            Player p = Bukkit.getPlayer(id);
            if (p == null || p.getGameMode() == GameMode.CREATIVE || p.getGameMode() == GameMode.SPECTATOR) return;
            @SuppressWarnings("deprecation") double max = p.getMaxHealth();
            p.setHealth(amount <= 0 ? max : Math.min(max, p.getHealth() + amount));
            p.setFireTicks(0);
        }

        @Override public void feed(UUID id, int amount) {
            Player p = Bukkit.getPlayer(id);
            if (p == null) return;
            p.setFoodLevel(Math.min(20, p.getFoodLevel() + Math.max(0, amount)));
            p.setSaturation(Math.min(20f, p.getSaturation() + Math.max(0, amount)));
        }

        @Override public void setFlight(UUID id, boolean allowed) {
            Player p = Bukkit.getPlayer(id);
            if (p == null || p.getGameMode() == GameMode.CREATIVE || p.getGameMode() == GameMode.SPECTATOR) return;
            p.setAllowFlight(allowed);
            if (allowed) {
                // Leave the ground so it is obvious flight is on.
                p.setFlying(!p.isOnGround());
            } else {
                if (p.isFlying()) softLanding.put(id, System.currentTimeMillis() + 15_000);
                p.setFlying(false);
            }
        }

        @Override public void openEnderChest(UUID id) {
            Player p = Bukkit.getPlayer(id);
            if (p != null) p.openInventory(p.getEnderChest());
        }

        @Override public void openVault(UUID id, int number, int rows) {
            Player p = Bukkit.getPlayer(id);
            if (p == null) return;
            VaultHolder holder = new VaultHolder(id, number);
            Inventory inv = Bukkit.createInventory(holder, rows * 9, "Vault " + number);
            YamlConfiguration yaml = YamlConfiguration.loadConfiguration(vaultFile(id));
            List<ItemStack> overflow = new ArrayList<>();
            var section = yaml.getConfigurationSection("v" + number);
            if (section != null) {
                for (String key : section.getKeys(false)) {
                    ItemStack item = section.getItemStack(key);
                    int slot;
                    try { slot = Integer.parseInt(key); } catch (NumberFormatException e) { continue; }
                    if (item == null) continue;
                    if (slot < inv.getSize()) inv.setItem(slot, item); else overflow.add(item);
                }
            }
            // The vault got smaller since these were stored: hand them back rather than lose them.
            for (ItemStack item : overflow) for (ItemStack over : p.getInventory().addItem(item).values()) p.getWorld().dropItemNaturally(p.getLocation(), over);
            if (!overflow.isEmpty()) {
                save(holder, inv);
                p.sendMessage("§eThis vault is smaller than before; " + overflow.size() + " stack(s) were moved to your inventory.");
            }
            p.openInventory(inv);
        }

        @Override public boolean depositToVault(UUID id, Item item, int vaults, int rows) {
            ItemStack stack = null;
            if (item.data() != null && !item.data().isBlank()) {
                try {
                    YamlConfiguration y = new YamlConfiguration();
                    y.loadFromString(item.data());
                    stack = y.getItemStack("item");
                } catch (Exception ignored) { /* fall back to id and count below */ }
            }
            if (stack == null) {
                Material m = Material.matchMaterial(item.id());
                if (m == null || !m.isItem() || m == Material.AIR) return false;
                stack = new ItemStack(m, Math.max(1, Math.min(item.count(), m.getMaxStackSize())));
            }
            int size = Math.max(1, Math.min(6, rows)) * 9;
            Player online = Bukkit.getPlayer(id);
            // A vault the player has open is the live copy: put it there and the close handler saves it.
            if (online != null && online.getOpenInventory().getTopInventory().getHolder() instanceof VaultHolder open && open.owner().equals(id)) {
                if (online.getOpenInventory().getTopInventory().addItem(stack.clone()).isEmpty()) return true;
            }
            File file = vaultFile(id);
            YamlConfiguration yaml = YamlConfiguration.loadConfiguration(file);
            for (int n = 1; n <= Math.max(1, vaults); n++) {
                if (online != null && online.getOpenInventory().getTopInventory().getHolder() instanceof VaultHolder open && open.owner().equals(id) && open.number() == n) continue;
                String base = "v" + n;
                ItemStack[] slots = new ItemStack[size];
                var section = yaml.getConfigurationSection(base);
                if (section != null) {
                    for (String key : section.getKeys(false)) {
                        try { int slot = Integer.parseInt(key); if (slot >= 0 && slot < size) slots[slot] = section.getItemStack(key); } catch (NumberFormatException ignored) { /* skip */ }
                    }
                }
                int free = -1;
                int left = stack.getAmount();
                for (int i = 0; i < size && left > 0; i++) {
                    ItemStack there = slots[i];
                    if (there == null || there.getType() == Material.AIR) { if (free < 0) free = i; continue; }
                    if (there.isSimilar(stack) && there.getAmount() < there.getMaxStackSize()) {
                        int add = Math.min(left, there.getMaxStackSize() - there.getAmount());
                        there.setAmount(there.getAmount() + add);
                        yaml.set(base + "." + i, there);
                        left -= add;
                    }
                }
                if (left > 0 && free >= 0) {
                    ItemStack rest = stack.clone();
                    rest.setAmount(left);
                    yaml.set(base + "." + free, rest);
                    left = 0;
                }
                if (left == 0) {
                    try { vaultDir.mkdirs(); yaml.save(file); return true; }
                    catch (IOException e) { plugin.getLogger().severe("Could not save vault " + n + " of " + id + ": " + e.getMessage()); return false; }
                }
            }
            return false;
        }

        @Override public void actionbar(UUID id, String text) {
            Player p = Bukkit.getPlayer(id);
            if (p != null) p.spigot().sendMessage(net.md_5.bungee.api.ChatMessageType.ACTION_BAR, net.md_5.bungee.api.chat.TextComponent.fromLegacyText(text));
        }
    }

    // ---- styling a designed item -------------------------------------------------------------------------

    private void style(ItemStack stack, ItemSpec spec) {
        ItemMeta meta = stack.getItemMeta();
        if (meta == null) return;
        if (!spec.name().isBlank()) meta.setDisplayName("§r" + ChatLayout.colorize(spec.name()));
        if (!spec.lore().isEmpty()) {
            List<String> lore = new ArrayList<>();
            for (String line : spec.lore()) lore.add("§r" + ChatLayout.colorize(line));
            meta.setLore(lore);
        }
        for (Map.Entry<String, Integer> e : spec.enchants().entrySet()) {
            String key = e.getKey().contains(":") ? e.getKey() : "minecraft:" + e.getKey();
            NamespacedKey nk = NamespacedKey.fromString(key);
            Enchantment ench = nk == null ? null : Enchantment.getByKey(nk);
            if (ench != null) meta.addEnchant(ench, e.getValue(), true);
        }
        if (spec.glow() && spec.enchants().isEmpty()) {
            Enchantment luck = Enchantment.getByKey(NamespacedKey.minecraft("unbreaking"));
            if (luck != null) meta.addEnchant(luck, 1, true);
            meta.addItemFlags(ItemFlag.HIDE_ENCHANTS);
        }
        if (spec.unbreakable()) meta.setUnbreakable(true);
        if (spec.hideFlags()) meta.addItemFlags(ItemFlag.values());
        if (spec.customModelData() >= 0) meta.setCustomModelData(spec.customModelData());
        for (ItemSpec.Attribute a : spec.attributes()) addAttribute(meta, a);
        stack.setItemMeta(meta);
    }

    /** Attribute constants were renamed in newer Minecraft versions (GENERIC_ATTACK_DAMAGE → ATTACK_DAMAGE), so find them by name. */
    private static void addAttribute(ItemMeta meta, ItemSpec.Attribute a) {
        try {
            String base = a.attribute().replace("generic.", "").toUpperCase(Locale.ROOT).replace('.', '_');
            org.bukkit.attribute.Attribute attribute = null;
            for (String candidate : new String[] {"GENERIC_" + base, base}) {
                try { attribute = (org.bukkit.attribute.Attribute) org.bukkit.attribute.Attribute.class.getField(candidate).get(null); break; }
                catch (ReflectiveOperationException ignored) { /* try the other spelling */ }
            }
            if (attribute == null) return;
            org.bukkit.attribute.AttributeModifier.Operation op = switch (a.operation()) {
                case "multiply_base" -> org.bukkit.attribute.AttributeModifier.Operation.ADD_SCALAR;
                case "multiply_total" -> org.bukkit.attribute.AttributeModifier.Operation.MULTIPLY_SCALAR_1;
                default -> org.bukkit.attribute.AttributeModifier.Operation.ADD_NUMBER;
            };
            org.bukkit.inventory.EquipmentSlot slot = switch (a.slot()) {
                case "offhand" -> org.bukkit.inventory.EquipmentSlot.OFF_HAND;
                case "head" -> org.bukkit.inventory.EquipmentSlot.HEAD;
                case "chest" -> org.bukkit.inventory.EquipmentSlot.CHEST;
                case "legs" -> org.bukkit.inventory.EquipmentSlot.LEGS;
                case "feet" -> org.bukkit.inventory.EquipmentSlot.FEET;
                case "any" -> null;
                default -> org.bukkit.inventory.EquipmentSlot.HAND;
            };
            org.bukkit.attribute.AttributeModifier mod = slot == null
                    ? new org.bukkit.attribute.AttributeModifier("scopenet." + base.toLowerCase(Locale.ROOT), a.amount(), op)
                    : new org.bukkit.attribute.AttributeModifier(UUID.randomUUID(), "scopenet." + base.toLowerCase(Locale.ROOT), a.amount(), op, slot);
            meta.addAttributeModifier(attribute, mod);
        } catch (RuntimeException | LinkageError ignored) { /* an attribute this server version doesn't know is skipped */ }
    }

    /** Just enough of a platform to show a chat message to a player on the server thread. */
    static final class ChatPlatform implements Platform {
        private final ScopenetPlugin plugin;
        ChatPlatform(ScopenetPlugin plugin) { this.plugin = plugin; }
        @Override public Optional<CorePlayer> player(UUID uuid) { return Optional.ofNullable(Bukkit.getPlayer(uuid)).map(PaperHubPlayer::new); }
        @Override public Optional<CorePlayer> playerByName(String name) { return Optional.empty(); }
        @Override public Collection<? extends CorePlayer> online() { return List.of(); }
        @Override public Optional<Pos> safeSurface(String world, int x, int z) { return Optional.empty(); }
        @Override public Pos worldSpawn(String world) { return new Pos(world, 0, 64, 0, 0, 0); }
        @Override public void runMain(Runnable task) { if (Bukkit.isPrimaryThread()) task.run(); else Bukkit.getScheduler().runTask(plugin, task); }
        @Override public void runAsync(Runnable task) { Bukkit.getScheduler().runTaskAsynchronously(plugin, task); }
    }

    // ---- a Bukkit player as the shared code sees them ---------------------------------------------------------

    private record PaperHubPlayer(Player p) implements CorePlayer {
        @Override public UUID uuid() { return p.getUniqueId(); }
        @Override public String name() { return p.getName(); }
        @Override public Pos pos() {
            org.bukkit.Location l = p.getLocation();
            return new Pos(ScopenetPlugin.dimension(l.getWorld()), l.getX(), l.getY(), l.getZ(), l.getYaw(), l.getPitch());
        }
        @Override public void send(String message) { p.sendMessage(message); }
        @Override public void teleport(Pos destination) {
            for (org.bukkit.World w : Bukkit.getWorlds()) {
                if (ScopenetPlugin.dimension(w).equals(destination.world())) {
                    p.teleport(new org.bukkit.Location(w, destination.x(), destination.y(), destination.z(), destination.yaw(), destination.pitch()));
                    return;
                }
            }
        }
        @Override public boolean hasPermission(String node) { return p.hasPermission(node); }
        @Override public long playtimeSeconds() { return p.getStatistic(org.bukkit.Statistic.PLAY_ONE_MINUTE) / 20L; }
        @Override public Optional<Item> heldItem() { return Optional.empty(); }
        @Override public void clearHeld() { }
        @Override public void give(Item item) { }
    }
}
