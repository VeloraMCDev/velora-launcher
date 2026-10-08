package net.scopenet.paper;

import net.scopenet.core.*;
import net.scopenet.core.map.MapService;
import net.scopenet.core.map.MapSource;
import net.scopenet.paper.commands.EconomyHandler;
import net.scopenet.paper.commands.EssentialsHandler;
import net.scopenet.paper.commands.GuildHandler;
import net.scopenet.integration.Integration;
import org.bukkit.Bukkit;
import org.bukkit.Material;
import org.bukkit.entity.Player;
import org.bukkit.inventory.ItemStack;
import org.bukkit.plugin.messaging.PluginMessageListener;

import java.nio.file.Path;
import java.util.*;
import java.util.logging.Logger;

/**
 * Talks to the optional Velora client mod (HUD, claim overlay, shop and market windows) through plugin messaging,
 * using shared link logic. Player actions use ordinary commands or a restricted panel dispatcher;
 * Paper owns inventories, identity and command permissions.
 */
final class ClientLinkBridge implements PluginMessageListener {
    private final ScopenetPlugin plugin;
    private final Env env;
    private final PlayerCache cache;
    private final ClientLink link;
    private final Placeholders placeholders;
    private final Integration integration;
    private final ShopPoints shops;
    private ActionPoller poller;
    private net.scopenet.core.RewardPoller rewards;
    private MapService mapService;
    private EconomyHandler economy;
    private CosmeticRuntime cosmetics;

    ClientLinkBridge(ScopenetPlugin plugin, Integration integration) {
        this.plugin = plugin;
        this.integration = integration;
        PaperPlatform platform = new PaperPlatform(plugin);
        Features features = new Features(true, plugin.getConfig().getBoolean("economy.enabled", true), integration.settings().guildsEnabled(),
                integration.settings().socialEnabled(), integration.settings().landClaimingEnabled(),
                plugin.getConfig().getBoolean("clientlink.enabled", true), plugin.getConfig().getString("economy.currency-symbol", "$"));
        Logger log = plugin.getLogger();
        env = new Env(platform, new PanelAdapter(integration), features, Path.of(plugin.getDataFolder().getPath()), System::currentTimeMillis, new Random(), log);
        cache = new PlayerCache(env);
        placeholders = new Placeholders(env, cache);
        shops = new ShopPoints(Path.of(plugin.getDataFolder().getPath()).resolve("shops.json"));
        link = new ClientLink(env, cache, Bukkit.getServer().getName(), (p, bytes) -> {
            Player target = Bukkit.getPlayer(p.uuid());
            if (target != null) target.sendPluginMessage(plugin, Wire.S2C, bytes);
        });
    }

    ClientLink link() { return link; }

    /** The player just changed what they wear: fetch their profile now and show it, rather than at the next refresh. */
    void refreshCosmetics(Player player) {
        if (cosmetics == null) return;
        cache.refresh(new PaperPlatform.PaperPlayer(player), info -> {
            if (info != null && player.isOnline()) cosmetics.apply(player, info);
        });
    }

    /** Modelled cosmetics (Cosmetics Studio) are drawn from each profile refresh. Null when switched off. */
    void cosmetics(CosmeticRuntime runtime) { cosmetics = runtime; }

    void dialogue(Player player, ContentDef def) {
        link.dialogue(new PaperPlatform.PaperPlayer(player), def.name().isBlank() ? def.title() : def.name(),
                def.messages().stream().map(line -> line.replace("{player}", player.getName())).toList());
    }

    void start() {
        Bukkit.getMessenger().registerOutgoingPluginChannel(plugin, Wire.S2C);
        Bukkit.getMessenger().registerIncomingPluginChannel(plugin, Wire.C2S, this);
        Bukkit.getScheduler().runTaskTimer(plugin, () -> {
            cache.tick(p -> {
                link.pushState(p);
                if (cosmetics == null) return;
                Player online = Bukkit.getPlayer(p.uuid());
                if (online != null) cosmetics.apply(online, cache.info(p.uuid()));
            });
            link.tick();
        }, 40, 20);
    }

    /**
     * Shop points, actions from the map, and the claims and pins for the Velora Map. Called once the command handlers exist; any of them may be
     * null when that system is switched off.
     */
    void startMap(EssentialsHandler essentials, GuildHandler guilds, EconomyHandler economy) {
        this.economy = economy;
        link.localRequests((p, request) -> {
            String id = PlayerCache.str(request, "id", ""), op = PlayerCache.str(request, "operation", "");
            Player player = Bukkit.getPlayer(p.uuid());
            if (player == null) return;
            com.google.gson.JsonObject data = new com.google.gson.JsonObject();
            if (op.equals("travel")) {
                com.google.gson.JsonArray places = new com.google.gson.JsonArray();
                if (essentials != null && player.hasPermission("scopenet.command.home")) {
                    essentials.homesForMap().getOrDefault(p.uuid(), Map.of()).forEach((name, pos) -> places.add(place("home", name, pos)));
                }
                if (essentials != null && player.hasPermission("scopenet.command.warp")) essentials.warpsForMap().forEach((name, pos) -> places.add(place("warp", name, pos)));
                data.add("places", places);
            } else if (op.equals("storage")) {
                com.google.gson.JsonArray vaults = new com.google.gson.JsonArray();
                var u = integration.utilities();
                if (u.vault.enabled() && player.hasPermission("scopenet.command.vault")) for (int n = 1; n <= u.vault.count(); n++) {
                    if (n <= u.vault.freeCount() || player.hasPermission("scopenet.vault." + n) || player.hasPermission("scopenet.vault.*")) vaults.add(n);
                }
                data.add("vaults", vaults); data.addProperty("echest", u.echest && player.hasPermission("scopenet.command.echest"));
                com.google.gson.JsonArray kits = new com.google.gson.JsonArray();
                for (var kit : u.kits) { var k = new com.google.gson.JsonObject(); k.addProperty("id", kit.id()); k.addProperty("name", kit.name()); k.addProperty("description", kit.description()); kits.add(k); }
                data.add("kits", kits);
            }
            link.result(p, id, data, null);
        });
        link.panelUrl(integration.settings().panel().toString());
        Bukkit.getPluginManager().registerEvents(new ShopListener(), plugin);
        // Shopkeepers: villagers (or any mob) that open the market or shop when right-clicked.
        ShopkeeperHandler keepers = new ShopkeeperHandler(plugin, shops, (player, kind) -> {
            if (economy == null) { player.sendMessage("§cThe market is switched off on this server."); return; }
            if (!link.open(new PaperPlatform.PaperPlayer(player), kind)) {
                if (kind.equals("market")) economy.openMarket(player); else economy.openShop(player);
            }
        });
        Bukkit.getPluginManager().registerEvents(keepers, plugin);
        org.bukkit.command.PluginCommand shopkeeperCommand = plugin.getCommand("shopkeeper");
        if (shopkeeperCommand != null) { shopkeeperCommand.setExecutor(keepers); shopkeeperCommand.setTabCompleter(keepers); }
        poller = new ActionPoller(env, (from, to) -> {
            Player a = Bukkit.getPlayer(from.uuid()), b = Bukkit.getPlayer(to.uuid());
            if (a != null && b != null && essentials != null) essentials.requestTpa(a, b);
        });
        MapSource source = new MapSource() {
            @Override public Map<String, Pos> warps() { return essentials == null ? Map.of() : essentials.warpsForMap(); }
            @Override public Map<UUID, Map<String, Pos>> homes() { return essentials == null ? Map.of() : essentials.homesForMap(); }
            @Override public Map<String, Pos> guildHomes() { return guilds == null ? Map.of() : guilds.guildHomesForMap(); }
            @Override public Collection<ShopPoints.Shop> shops() { return shops.all(); }
        };
        mapService = new MapService(env, integration.client().claims(), source, cache, integration.settings().panel().toString(),
                plugin.getConfig().getBoolean("map.show-homes", false), Bukkit.getServer().getName());
        integration.setMapOverlay(() -> net.scopenet.core.map.MapOverlay.toJson(mapService.snapshot()).toString());
        var cfg = plugin.getConfig();
        var defaults = net.scopenet.core.RewardRules.defaults();
        rewards = new net.scopenet.core.RewardPoller(env, new net.scopenet.core.RewardRules(cfg.getBoolean("rewards.enabled", true), cfg.getBoolean("rewards.allow-commands", false),
                cfg.getString("rewards.permission-command", defaults.permissionCommand()), cfg.getString("rewards.permission-deny-command", defaults.permissionDenyCommand()),
                cfg.getString("rewards.permission-temp-command", defaults.permissionTempCommand()), cfg.getString("rewards.group-command", defaults.groupCommand())));
        Bukkit.getScheduler().runTaskTimer(plugin, () -> { poller.tick(); rewards.tick(); mapService.tick(); }, 60, 20);
    }

    private static com.google.gson.JsonObject place(String kind, String name, Pos pos) {
        var p = new com.google.gson.JsonObject(); p.addProperty("kind", kind); p.addProperty("name", name);
        p.addProperty("world", pos.world()); p.addProperty("x", pos.x()); p.addProperty("z", pos.z()); return p;
    }

    /** /market point add|remove|list, on the block the player is looking at (or standing on). */
    void shopCommand(Player player, String[] args) {
        org.bukkit.block.Block looked = player.getTargetBlockExact(6);
        org.bukkit.Location l = looked != null ? looked.getLocation() : player.getLocation().subtract(0, 1, 0);
        shops.command(new PaperPlatform.PaperPlayer(player), args, new Pos(ScopenetPlugin.dimension(l.getWorld()), l.getBlockX(), l.getBlockY(), l.getBlockZ(), 0, 0));
    }

    /** Right-clicking a shop point opens the market or shop: the client mod's window if they have it, otherwise the chest GUI. */
    private final class ShopListener implements org.bukkit.event.Listener {
        @org.bukkit.event.EventHandler(ignoreCancelled = true)
        public void onInteract(org.bukkit.event.player.PlayerInteractEvent event) {
            if (event.getAction() != org.bukkit.event.block.Action.RIGHT_CLICK_BLOCK || event.getHand() != org.bukkit.inventory.EquipmentSlot.HAND) return;
            org.bukkit.block.Block block = event.getClickedBlock();
            if (block == null) return;
            Optional<ShopPoints.Shop> shop = shops.at(ScopenetPlugin.dimension(block.getWorld()), block.getX(), block.getY(), block.getZ());
            if (shop.isEmpty() || economy == null) return;
            event.setCancelled(true);
            Player player = event.getPlayer();
            String kind = shop.get().kind().equals("market") ? "market" : "shop";
            if (!link.open(new PaperPlatform.PaperPlayer(player), kind)) {
                if (kind.equals("market")) economy.openMarket(player); else economy.openShop(player);
            }
        }
    }

    void stop() {
        if (cosmetics != null) cosmetics.stop();
        Bukkit.getMessenger().unregisterOutgoingPluginChannel(plugin, Wire.S2C);
        Bukkit.getMessenger().unregisterIncomingPluginChannel(plugin, Wire.C2S, this);
    }

    void quit(Player player) {
        link.disconnected(player.getUniqueId());
        if (cosmetics != null) cosmetics.remove(player);
    }

    @Override public void onPluginMessageReceived(String channel, Player player, byte[] message) {
        if (!Wire.C2S.equals(channel)) return;
        link.receive(new PaperPlatform.PaperPlayer(player), message);
    }

    /** Panel events (level-ups, achievements, guild changes) become toasts on clients. Any thread. */
    void panelEvents(com.google.gson.JsonArray events) { link.onPanelEvents(events); }

    /** Bukkit adapters for the shared logic. Only what the client link needs is real. */
    static final class PaperPlatform implements Platform {
        private final ScopenetPlugin plugin;
        PaperPlatform(ScopenetPlugin plugin) { this.plugin = plugin; }

        @Override public Optional<CorePlayer> player(UUID uuid) { return Optional.ofNullable(Bukkit.getPlayer(uuid)).map(PaperPlayer::new); }
        @Override public Optional<CorePlayer> playerByName(String name) { return Optional.ofNullable(Bukkit.getPlayerExact(name)).map(PaperPlayer::new); }
        @Override public Collection<? extends CorePlayer> online() {
            List<CorePlayer> out = new ArrayList<>();
            for (Player p : Bukkit.getOnlinePlayers()) out.add(new PaperPlayer(p));
            return out;
        }
        @Override public Optional<Pos> safeSurface(String world, int x, int z) { return Optional.empty(); }
        @Override public Pos worldSpawn(String world) {
            for (org.bukkit.World w : Bukkit.getWorlds()) {
                if (ScopenetPlugin.dimension(w).equals(world)) { org.bukkit.Location l = w.getSpawnLocation(); return new Pos(world, l.getX(), l.getY(), l.getZ(), 0, 0); }
            }
            org.bukkit.Location l = Bukkit.getWorlds().get(0).getSpawnLocation();
            return new Pos(ScopenetPlugin.dimension(Bukkit.getWorlds().get(0)), l.getX(), l.getY(), l.getZ(), 0, 0);
        }
        @Override public void runMain(Runnable task) {
            if (Bukkit.isPrimaryThread()) task.run(); else Bukkit.getScheduler().runTask(plugin, task);
        }
        @Override public void runAsync(Runnable task) { Bukkit.getScheduler().runTaskAsynchronously(plugin, task); }

        @Override public boolean giveSpec(UUID id, ItemSpec spec) { return plugin.giveSpec(id, spec); }
        @Override public boolean giveItem(UUID id, String item, int amount) {
            Player p = Bukkit.getPlayer(id);
            if (p == null) return false;
            Material m = Material.matchMaterial(item);
            if (m == null && item.startsWith("minecraft:")) m = Material.matchMaterial(item.substring(10));
            if (m == null || !m.isItem() || m == Material.AIR) return false;
            for (int left = amount; left > 0; ) {
                int n = Math.min(left, m.getMaxStackSize());
                for (ItemStack over : p.getInventory().addItem(new ItemStack(m, n)).values()) p.getWorld().dropItemNaturally(p.getLocation(), over);
                left -= n;
            }
            return true;
        }

        @Override public boolean console(String command) { return Bukkit.dispatchCommand(Bukkit.getConsoleSender(), command); }

        record PaperPlayer(Player p) implements CorePlayer {
            @Override public UUID uuid() { return p.getUniqueId(); }
            @Override public String name() { return p.getName(); }
            @Override public Pos pos() {
                org.bukkit.Location l = p.getLocation();
                return new Pos(ScopenetPlugin.dimension(l.getWorld()), l.getX(), l.getY(), l.getZ(), l.getYaw(), l.getPitch());
            }
            @Override public void send(String message) { p.sendMessage(message); }
            @Override public void teleport(Pos destination) { }
            @Override public boolean hasPermission(String node) { return p.hasPermission(node); }
            @Override public long playtimeSeconds() { return p.getStatistic(org.bukkit.Statistic.PLAY_ONE_MINUTE) / 20L; }
            @Override public Optional<Item> heldItem() {
                ItemStack hand = p.getInventory().getItemInMainHand();
                if (hand.getType() == Material.AIR) return Optional.empty();
                String name = hand.hasItemMeta() && hand.getItemMeta().hasDisplayName() ? hand.getItemMeta().getDisplayName() : hand.getType().name().replace('_', ' ');
                return Optional.of(new Item(hand.getType().name(), name, hand.getAmount(), ""));
            }
            @Override public void clearHeld() { }
            @Override public void give(Item item) { }
        }
    }
}
