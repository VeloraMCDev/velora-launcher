package net.scopenet.paper.commands;

import net.scopenet.paper.ScopenetPlugin;
import net.scopenet.paper.gui.GuiHelper;
import org.bukkit.*;
import org.bukkit.block.Block;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.entity.PlayerDeathEvent;
import org.bukkit.event.inventory.InventoryClickEvent;
import org.bukkit.event.player.PlayerTeleportEvent;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.ItemStack;

import java.io.File;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

public final class EssentialsHandler implements CommandExecutor, Listener, org.bukkit.command.TabCompleter {
    @Override public List<String> onTabComplete(CommandSender sender, Command command, String alias, String[] args) {
        if (!(sender instanceof Player player) || !command.testPermissionSilent(sender) || !plugin.getConfig().getBoolean("essentials.enabled", true)) return List.of();
        String name = command.getName().toLowerCase(java.util.Locale.ROOT);
        List<String> choices = List.of();
        if (args.length == 1 && Set.of("home", "delhome", "sethome").contains(name)) choices = new ArrayList<>(homes.getOrDefault(player.getUniqueId(), Map.of()).keySet());
        else if (args.length == 1 && Set.of("warp", "delwarp").contains(name)) choices = new ArrayList<>(warps.keySet());
        else if (args.length == 1 && Set.of("warpapprove", "warpdeny").contains(name) && player.hasPermission("scopenet.command.warp.manage")) choices = new ArrayList<>(warpRequestsConfig.getKeys(false));
        else choices = net.scopenet.core.CommandSuggestions.choices(name, args, Bukkit.getOnlinePlayers().stream().filter(player::canSee).map(Player::getName).toList(), player::hasPermission);
        return net.scopenet.core.CommandSuggestions.filter(choices, args);
    }
    private final ScopenetPlugin plugin;
    private final File homesFile;
    private final File warpsFile;
    private final File warpRequestsFile;
    private final YamlConfiguration homesConfig;
    private final YamlConfiguration warpsConfig;
    private final YamlConfiguration warpRequestsConfig;

    private final Map<UUID, Map<String, Location>> homes = new ConcurrentHashMap<>();
    private final Map<String, Location> warps = new ConcurrentHashMap<>();
    private final Map<UUID, Location> lastLocations = new ConcurrentHashMap<>();
    private final Map<UUID, UUID> pendingTpa = new ConcurrentHashMap<>(); // target -> requester
    private final Map<UUID, Long> tpaTimestamp = new ConcurrentHashMap<>();

    private static final String GUI_HOMES_TITLE = ChatColor.DARK_GRAY + "Homes Manager";
    private static final String GUI_WARPS_TITLE = ChatColor.DARK_GRAY + "Server Warps";

    public EssentialsHandler(ScopenetPlugin plugin) {
        this.plugin = plugin;
        this.homesFile = new File(plugin.getDataFolder(), "homes.yml");
        this.warpsFile = new File(plugin.getDataFolder(), "warps.yml");
        this.warpRequestsFile = new File(plugin.getDataFolder(), "warp_requests.yml");
        this.homesConfig = YamlConfiguration.loadConfiguration(homesFile);
        this.warpsConfig = YamlConfiguration.loadConfiguration(warpsFile);
        this.warpRequestsConfig = YamlConfiguration.loadConfiguration(warpRequestsFile);
        loadHomes();
        loadWarps();
    }

    private void loadHomes() {
        if (!homesFile.exists()) return;
        for (String uuidStr : homesConfig.getKeys(false)) {
            try {
                UUID uuid = UUID.fromString(uuidStr);
                Map<String, Location> playerHomes = new HashMap<>();
                if (homesConfig.isConfigurationSection(uuidStr)) {
                    for (String homeName : homesConfig.getConfigurationSection(uuidStr).getKeys(false)) {
                        Location loc = homesConfig.getLocation(uuidStr + "." + homeName);
                        if (loc != null) playerHomes.put(homeName.toLowerCase(), loc);
                    }
                }
                homes.put(uuid, playerHomes);
            } catch (Exception ignored) {}
        }
    }

    private void saveHomes() {
        try {
            for (Map.Entry<UUID, Map<String, Location>> entry : homes.entrySet()) {
                String uuidStr = entry.getKey().toString();
                homesConfig.set(uuidStr, null);
                for (Map.Entry<String, Location> home : entry.getValue().entrySet()) {
                    homesConfig.set(uuidStr + "." + home.getKey(), home.getValue());
                }
            }
            homesConfig.save(homesFile);
        } catch (IOException e) {
            plugin.getLogger().warning("Failed to save homes: " + e.getMessage());
        }
    }

    private void loadWarps() {
        if (!warpsFile.exists()) return;
        for (String warpName : warpsConfig.getKeys(false)) {
            Location loc = warpsConfig.getLocation(warpName);
            if (loc != null) warps.put(warpName.toLowerCase(), loc);
        }
    }

    private void saveWarps() {
        try {
            for (Map.Entry<String, Location> entry : warps.entrySet()) {
                warpsConfig.set(entry.getKey(), entry.getValue());
            }
            warpsConfig.save(warpsFile);
        } catch (IOException e) {
            plugin.getLogger().warning("Failed to save warps: " + e.getMessage());
        }
    }

    private void saveWarpRequests() {
        try { warpRequestsConfig.save(warpRequestsFile); }
        catch (IOException e) { plugin.getLogger().warning("Failed to save warp requests: " + e.getMessage()); }
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onTeleport(PlayerTeleportEvent event) {
        if (event.getFrom() != null && event.getCause() != PlayerTeleportEvent.TeleportCause.UNKNOWN) {
            lastLocations.put(event.getPlayer().getUniqueId(), event.getFrom());
        }
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void onDeath(PlayerDeathEvent event) {
        lastLocations.put(event.getEntity().getUniqueId(), event.getEntity().getLocation());
    }

    @EventHandler
    public void onInventoryClick(InventoryClickEvent event) {
        if (!(event.getWhoClicked() instanceof Player player)) return;
        String title = event.getView().getTitle();
        if (title.equals(GUI_HOMES_TITLE)) {
            event.setCancelled(true);
            ItemStack item = event.getCurrentItem();
            if (item == null || !item.hasItemMeta() || item.getItemMeta().getDisplayName() == null) return;
            String homeName = ChatColor.stripColor(item.getItemMeta().getDisplayName()).replace("Home: ", "").trim().toLowerCase();
            Map<String, Location> playerHomes = homes.get(player.getUniqueId());
            if (playerHomes != null && playerHomes.containsKey(homeName)) {
                player.closeInventory();
                if (coolingDown(player, "home")) return;
                player.teleport(playerHomes.get(homeName));
                startCooldown(player, "home");
                player.sendMessage(ChatColor.GREEN + "Teleported to home: " + ChatColor.YELLOW + homeName);
            }
        } else if (title.equals(GUI_WARPS_TITLE)) {
            event.setCancelled(true);
            ItemStack item = event.getCurrentItem();
            if (item == null || !item.hasItemMeta() || item.getItemMeta().getDisplayName() == null) return;
            String warpName = ChatColor.stripColor(item.getItemMeta().getDisplayName()).replace("Warp: ", "").trim().toLowerCase();
            if (warps.containsKey(warpName)) {
                player.closeInventory();
                if (coolingDown(player, "warp")) return;
                player.teleport(warps.get(warpName));
                startCooldown(player, "warp");
                player.sendMessage(ChatColor.GREEN + "Teleported to warp: " + ChatColor.YELLOW + warpName);
            }
        }
    }

    @Override
    public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (!(sender instanceof Player player)) {
            sender.sendMessage(ChatColor.RED + "Only in-game players can execute essentials commands.");
            return true;
        }

        if (!plugin.getConfig().getBoolean("essentials.enabled", true)) { player.sendMessage(ChatColor.RED + "Essentials are disabled."); return true; }
        String cmd = command.getName().toLowerCase();
        boolean cooled = cooldownSeconds(cmd) > 0 && !player.hasPermission("scopenet.cooldown.bypass");
        if (cooled && coolingDown(player, cmd)) return true;
        Location before = player.getLocation();
        switch (cmd) {
            case "spawn" -> handleSpawn(player);
            case "home" -> handleHome(player, args);
            case "sethome" -> handleSetHome(player, args);
            case "delhome" -> handleDelHome(player, args);
            case "back" -> handleBack(player);
            case "tpa" -> handleTpa(player, args);
            case "tpaccept" -> handleTpAccept(player);
            case "tpdeny" -> handleTpDeny(player);
            case "rtp" -> handleRtp(player);
            case "warp" -> handleWarp(player, args);
            case "reqwarp" -> requestWarp(player, args);
            case "warprequests" -> listWarpRequests(player);
            case "warpapprove" -> reviewWarp(player, args, true);
            case "warpdeny" -> reviewWarp(player, args, false);
            case "setwarp" -> setWarp(player, args);
            case "delwarp" -> deleteWarp(player, args);
            case "playtime" -> handlePlaytime(player, args);
            default -> { return false; }
        }
        if (cooled && ((cmd.equals("tpa") && args.length > 0) || cmd.equals("rtp") || !before.equals(player.getLocation()))) startCooldown(player, cmd);
        return true;
    }

    private void handleSpawn(Player player) {
        Location spawnLoc = player.getWorld().getSpawnLocation().add(0.5, 0, 0.5);
        player.teleport(spawnLoc);
        player.sendMessage(ChatColor.GREEN + "Teleported to spawn.");
    }

    private static net.scopenet.core.Pos toPos(Location l) {
        return new net.scopenet.core.Pos(l.getWorld() == null ? "minecraft:overworld" : ScopenetPlugin.dimension(l.getWorld()), l.getX(), l.getY(), l.getZ(), l.getYaw(), l.getPitch());
    }

    /** Warps as shown on the map. A copy. */
    public Map<String, net.scopenet.core.Pos> warpsForMap() {
        Map<String, net.scopenet.core.Pos> out = new TreeMap<>();
        warps.forEach((name, loc) -> { if (loc.getWorld() != null) out.put(name, toPos(loc)); });
        return out;
    }

    /** Every player's homes, for the map (only shown if the admin allows it). A copy. */
    public Map<UUID, Map<String, net.scopenet.core.Pos>> homesForMap() {
        Map<UUID, Map<String, net.scopenet.core.Pos>> out = new HashMap<>();
        homes.forEach((id, byName) -> {
            Map<String, net.scopenet.core.Pos> mine = new TreeMap<>();
            byName.forEach((name, loc) -> { if (loc.getWorld() != null) mine.put(name, toPos(loc)); });
            out.put(id, mine);
        });
        return out;
    }

    private void handleHome(Player player, String[] args) {
        Map<String, Location> playerHomes = homes.computeIfAbsent(player.getUniqueId(), k -> new HashMap<>());
        if (args.length == 0) {
            if (playerHomes.isEmpty()) {
                player.sendMessage(ChatColor.RED + "You have no homes set! Set one with " + ChatColor.YELLOW + "/sethome [name]");
                return;
            }
            if (playerHomes.size() == 1) {
                Location loc = playerHomes.values().iterator().next();
                player.teleport(loc);
                player.sendMessage(ChatColor.GREEN + "Teleported to home.");
                return;
            }
            openHomePicker(player, playerHomes);
            return;
        }

        String homeName = args[0].toLowerCase();
        Location loc = playerHomes.get(homeName);
        if (loc == null) {
            player.sendMessage(ChatColor.RED + "Home '" + homeName + "' not found. Your homes: " + ChatColor.YELLOW + String.join(", ", playerHomes.keySet()));
            return;
        }
        player.teleport(loc);
        player.sendMessage(ChatColor.GREEN + "Teleported to home " + ChatColor.YELLOW + homeName + ChatColor.GREEN + ".");
    }

    private void openHomePicker(Player player, Map<String, Location> playerHomes) {
        Inventory inv = Bukkit.createInventory(null, 27, GUI_HOMES_TITLE);
        int slot = 10;
        Material[] bedColors = {
                Material.RED_BED, Material.BLUE_BED, Material.LIME_BED,
                Material.YELLOW_BED, Material.PURPLE_BED, Material.ORANGE_BED, Material.CYAN_BED
        };
        int idx = 0;
        for (Map.Entry<String, Location> entry : playerHomes.entrySet()) {
            if (slot > 16) break;
            Material mat = bedColors[idx % bedColors.length];
            Location l = entry.getValue();
            String world = l.getWorld() != null ? l.getWorld().getName() : "world";
            ItemStack item = GuiHelper.createItem(
                    mat,
                    "&aHome: &f" + entry.getKey(),
                    "&7World: &f" + world,
                    "&7Coords: &f" + l.getBlockX() + ", " + l.getBlockY() + ", " + l.getBlockZ(),
                    "",
                    "&eClick to teleport!"
            );
            inv.setItem(slot++, item);
            idx++;
        }
        player.openInventory(inv);
    }

    // ---- limits and cooldowns (the same rules as the Fabric mod; see config.yml) ----

    private final net.scopenet.core.Limits.Cooldowns cooldowns = new net.scopenet.core.Limits.Cooldowns();

    private int tpaTimeoutSeconds() { return Math.max(5, plugin.getConfig().getInt("essentials.tpa-timeout-seconds", 60)); }

    private int cooldownSeconds(String cmd) { return Math.max(0, plugin.getConfig().getInt("essentials.cooldowns." + cmd, 0)); }

    /** True (and tells the player) if they must still wait before using this command. */
    private boolean coolingDown(Player player, String cmd) {
        if (cooldownSeconds(cmd) <= 0 || player.hasPermission("scopenet.cooldown.bypass")) return false;
        long wait = cooldowns.remainingMs(player.getUniqueId(), cmd, System.currentTimeMillis());
        if (wait <= 0) return false;
        player.sendMessage(ChatColor.RED + "Please wait " + net.scopenet.core.Limits.Cooldowns.waitText(wait) + " before using /" + cmd + " again.");
        return true;
    }

    private void startCooldown(Player player, String cmd) {
        if (!player.hasPermission("scopenet.cooldown.bypass")) cooldowns.start(player.getUniqueId(), cmd, System.currentTimeMillis(), cooldownSeconds(cmd));
    }

    private void handleSetHome(Player player, String[] args) {
        String homeName = args.length > 0 ? args[0].toLowerCase() : "home";
        int nameMax = Math.max(1, plugin.getConfig().getInt("essentials.home-name-max-length", 32));
        if (homeName.length() > nameMax) { player.sendMessage(ChatColor.RED + "Home names can be at most " + nameMax + " characters."); return; }
        int maxHomes = net.scopenet.core.Limits.tier(player::hasPermission, "scopenet.homes.", plugin.getConfig().getInt("essentials.max-homes", 5), 500);
        Map<String, Location> playerHomes = homes.computeIfAbsent(player.getUniqueId(), k -> new HashMap<>());

        if (!playerHomes.containsKey(homeName) && playerHomes.size() >= maxHomes) {
            player.sendMessage(ChatColor.RED + "You have reached your limit of " + net.scopenet.core.Limits.show(maxHomes) + " homes.");
            return;
        }

        playerHomes.put(homeName, player.getLocation());
        saveHomes();
        player.sendMessage(ChatColor.GREEN + "Home " + ChatColor.YELLOW + homeName + ChatColor.GREEN + " set successfully.");
    }

    private void handleDelHome(Player player, String[] args) {
        String homeName = args.length > 0 ? args[0].toLowerCase() : "home";
        Map<String, Location> playerHomes = homes.get(player.getUniqueId());
        if (playerHomes == null || playerHomes.remove(homeName) == null) {
            player.sendMessage(ChatColor.RED + "Home '" + homeName + "' does not exist.");
            return;
        }
        saveHomes();
        player.sendMessage(ChatColor.GREEN + "Home " + ChatColor.YELLOW + homeName + ChatColor.GREEN + " deleted.");
    }

    private void handleBack(Player player) {
        Location last = lastLocations.get(player.getUniqueId());
        if (last == null) {
            player.sendMessage(ChatColor.RED + "No previous location found to return to.");
            return;
        }
        player.teleport(last);
        player.sendMessage(ChatColor.GREEN + "Teleported back to your previous location.");
    }

    private void handleTpa(Player player, String[] args) {
        if (args.length < 1) {
            player.sendMessage(ChatColor.RED + "Usage: /tpa <player>");
            return;
        }
        Player target = Bukkit.getPlayer(args[0]);
        if (target == null || !target.isOnline()) {
            player.sendMessage(ChatColor.RED + "Player '" + args[0] + "' is not online.");
            return;
        }
        if (target.getUniqueId().equals(player.getUniqueId())) {
            player.sendMessage(ChatColor.RED + "You cannot teleport to yourself.");
            return;
        }

        requestTpa(player, target);
    }

    /** Ask {@code target} to accept a teleport of {@code player} to them. Also used for requests made from the map. */
    public void requestTpa(Player player, Player target) {
        pendingTpa.put(target.getUniqueId(), player.getUniqueId());
        tpaTimestamp.put(target.getUniqueId(), System.currentTimeMillis());

        player.sendMessage(ChatColor.GREEN + "Teleport request sent to " + ChatColor.YELLOW + target.getName() + ChatColor.GREEN + ".");
        target.sendMessage(ChatColor.YELLOW + player.getName() + ChatColor.GREEN + " has sent you a teleport request.");
        target.sendMessage(ChatColor.GRAY + "Type " + ChatColor.YELLOW + "/tpaccept" + ChatColor.GRAY + " to accept or " + ChatColor.RED + "/tpdeny" + ChatColor.GRAY + " to deny (expires in " + tpaTimeoutSeconds() + "s).");
    }

    private void handleTpAccept(Player player) {
        UUID requesterId = pendingTpa.get(player.getUniqueId());
        Long time = tpaTimestamp.get(player.getUniqueId());
        if (requesterId == null || time == null || System.currentTimeMillis() - time > tpaTimeoutSeconds() * 1000L) {
            pendingTpa.remove(player.getUniqueId());
            tpaTimestamp.remove(player.getUniqueId());
            player.sendMessage(ChatColor.RED + "You have no active teleport requests.");
            return;
        }

        Player requester = Bukkit.getPlayer(requesterId);
        if (requester == null || !requester.isOnline()) {
            player.sendMessage(ChatColor.RED + "The player who requested to teleport is no longer online.");
            pendingTpa.remove(player.getUniqueId());
            return;
        }

        requester.teleport(player.getLocation());
        requester.sendMessage(ChatColor.GREEN + "Teleport request accepted by " + ChatColor.YELLOW + player.getName() + ChatColor.GREEN + "!");
        player.sendMessage(ChatColor.GREEN + "Accepted teleport request from " + ChatColor.YELLOW + requester.getName() + ChatColor.GREEN + ".");
        pendingTpa.remove(player.getUniqueId());
        tpaTimestamp.remove(player.getUniqueId());
    }

    private void handleTpDeny(Player player) {
        UUID requesterId = pendingTpa.remove(player.getUniqueId());
        tpaTimestamp.remove(player.getUniqueId());
        if (requesterId == null) {
            player.sendMessage(ChatColor.RED + "You have no active teleport requests.");
            return;
        }
        Player requester = Bukkit.getPlayer(requesterId);
        if (requester != null && requester.isOnline()) {
            requester.sendMessage(ChatColor.RED + player.getName() + " denied your teleport request.");
        }
        player.sendMessage(ChatColor.YELLOW + "Teleport request denied.");
    }

    private void handleRtp(Player player) {
        int radius = Math.max(10, plugin.getConfig().getInt("essentials.rtp-radius", 2500));
        int minRadius = Math.min(plugin.getConfig().getInt("essentials.rtp-min-radius", 0), radius - 1);
        int attempts = Math.max(1, plugin.getConfig().getInt("essentials.rtp-attempts", 15));
        World world = player.getWorld();
        Random rng = new Random();

        player.sendMessage(ChatColor.GRAY + "Searching for a safe wilderness destination...");
        Bukkit.getScheduler().runTask(plugin, () -> {
            for (int i = 0; i < attempts; i++) {
                int x = rng.nextInt(radius * 2) - radius;
                int z = rng.nextInt(radius * 2) - radius;
                if (Math.max(Math.abs(x), Math.abs(z)) < minRadius) continue;
                int y = world.getHighestBlockYAt(x, z);
                Block block = world.getBlockAt(x, y - 1, z);
                if (block.getType().isSolid() && block.getType() != Material.LAVA && block.getType() != Material.WATER) {
                    Location target = new Location(world, x + 0.5, y, z + 0.5);
                    player.teleport(target);
                    player.sendMessage(ChatColor.GREEN + "Wilderness RTP: Teleported to " + ChatColor.YELLOW + x + ", " + y + ", " + z + ChatColor.GREEN + "!");
                    return;
                }
            }
            player.sendMessage(ChatColor.RED + "Could not find a safe RTP spot. Please try again.");
        });
    }

    private void handleWarp(Player player, String[] args) {
        if (args.length == 0) {
            openWarpPicker(player);
            return;
        }

        String warpName = args[0].toLowerCase();
        Location loc = warps.get(warpName);
        if (loc == null) {
            player.sendMessage(ChatColor.RED + "Warp '" + warpName + "' does not exist.");
            return;
        }
        player.teleport(loc);
        player.sendMessage(ChatColor.GREEN + "Teleported to warp " + ChatColor.YELLOW + warpName + ChatColor.GREEN + ".");
    }

    private String warpName(Player player, String[] args, String usage) {
        if (args.length != 1 || !args[0].matches("[A-Za-z0-9_-]{2,24}")) {
            player.sendMessage(ChatColor.RED + "Usage: " + usage + " (2–24 letters, numbers, _ or -)");
            return null;
        }
        return args[0].toLowerCase(Locale.ROOT);
    }

    private void requestWarp(Player player, String[] args) {
        String name = warpName(player, args, "/reqwarp <name>");
        if (name == null) return;
        if (warps.containsKey(name)) { player.sendMessage(ChatColor.RED + "That warp already exists."); return; }
        if (warpRequestsConfig.contains(name) && !player.getUniqueId().toString().equals(warpRequestsConfig.getString(name + ".uuid"))) {
            player.sendMessage(ChatColor.RED + "That warp name has already been requested."); return;
        }
        warpRequestsConfig.set(name + ".location", player.getLocation());
        warpRequestsConfig.set(name + ".uuid", player.getUniqueId().toString());
        warpRequestsConfig.set(name + ".requester", player.getName());
        warpRequestsConfig.set(name + ".created_at", System.currentTimeMillis());
        saveWarpRequests();
        player.sendMessage(ChatColor.GREEN + "Warp request sent. A server admin can approve it with /warpapprove " + name + ".");
        Bukkit.getOnlinePlayers().stream().filter(p -> p.hasPermission("scopenet.command.warp.manage"))
                .forEach(p -> p.sendMessage(ChatColor.YELLOW + player.getName() + " requested warp '" + name + "'. Use /warprequests to review."));
    }

    private boolean warpAdmin(Player player) {
        if (player.hasPermission("scopenet.command.warp.manage")) return true;
        player.sendMessage(ChatColor.RED + "Only server admins can manage warps.");
        return false;
    }

    private void listWarpRequests(Player player) {
        if (!warpAdmin(player)) return;
        if (warpRequestsConfig.getKeys(false).isEmpty()) { player.sendMessage(ChatColor.YELLOW + "No pending warp requests."); return; }
        player.sendMessage(ChatColor.GOLD + "Pending warp requests:");
        for (String name : warpRequestsConfig.getKeys(false)) {
            Location at = warpRequestsConfig.getLocation(name + ".location");
            player.sendMessage(ChatColor.YELLOW + name + ChatColor.GRAY + " by " + warpRequestsConfig.getString(name + ".requester", "unknown")
                    + (at == null || at.getWorld() == null ? "" : " at " + at.getWorld().getName() + " " + at.getBlockX() + ", " + at.getBlockY() + ", " + at.getBlockZ()));
        }
    }

    private void reviewWarp(Player player, String[] args, boolean approve) {
        if (!warpAdmin(player)) return;
        String name = warpName(player, args, approve ? "/warpapprove <name>" : "/warpdeny <name>");
        if (name == null) return;
        Location at = warpRequestsConfig.getLocation(name + ".location");
        if (at == null || at.getWorld() == null) { player.sendMessage(ChatColor.RED + "No pending request with a loaded world has that name."); return; }
        if (approve) {
            if (warps.containsKey(name)) { player.sendMessage(ChatColor.RED + "Warp already exists. Deny this request or choose another name."); return; }
            warps.put(name, at);
            saveWarps();
        }
        String uuid = warpRequestsConfig.getString(name + ".uuid", "");
        warpRequestsConfig.set(name, null);
        saveWarpRequests();
        player.sendMessage(ChatColor.GREEN + "Warp request " + (approve ? "approved" : "denied") + ": " + name);
        try {
            Player requester = Bukkit.getPlayer(UUID.fromString(uuid));
            if (requester != null) requester.sendMessage(ChatColor.YELLOW + "Your warp request '" + name + "' was " + (approve ? "approved" : "denied") + ".");
        } catch (IllegalArgumentException ignored) {}
    }

    private void setWarp(Player player, String[] args) {
        if (!warpAdmin(player)) return;
        String name = warpName(player, args, "/setwarp <name>");
        if (name == null) return;
        int maxWarps = plugin.getConfig().getInt("essentials.max-warps", 0);
        if (maxWarps > 0 && !warps.containsKey(name) && warps.size() >= maxWarps) {
            player.sendMessage(ChatColor.RED + "The server already has " + maxWarps + " warps. Delete one first.");
            return;
        }
        warps.put(name, player.getLocation());
        saveWarps();
        player.sendMessage(ChatColor.GREEN + "Warp set: " + name);
    }

    private void deleteWarp(Player player, String[] args) {
        if (!warpAdmin(player)) return;
        String name = warpName(player, args, "/delwarp <name>");
        if (name == null) return;
        if (warps.remove(name) == null) { player.sendMessage(ChatColor.RED + "Warp not found."); return; }
        warpsConfig.set(name, null);
        saveWarps();
        player.sendMessage(ChatColor.GREEN + "Warp deleted: " + name);
    }

    private void openWarpPicker(Player player) {
        Inventory inv = Bukkit.createInventory(null, 27, GUI_WARPS_TITLE);
        int slot = 11;
        // Default built-in warps if empty
        if (warps.isEmpty()) {
            warps.put("spawn", player.getWorld().getSpawnLocation());
            saveWarps();
        }

        for (Map.Entry<String, Location> entry : warps.entrySet()) {
            if (slot > 15) break;
            ItemStack item = GuiHelper.createItem(
                    Material.ENDER_PEARL,
                    "&bWarp: &f" + entry.getKey(),
                    "&7World: &f" + (entry.getValue().getWorld() != null ? entry.getValue().getWorld().getName() : "world"),
                    "",
                    "&eClick to teleport!"
            );
            inv.setItem(slot++, item);
        }
        player.openInventory(inv);
    }

    private void handlePlaytime(Player player, String[] args) {
        Player target = args.length > 0 ? Bukkit.getPlayer(args[0]) : player;
        if (target == null) {
            player.sendMessage(ChatColor.RED + "Player not found.");
            return;
        }

        int ticksPlayed = target.getStatistic(Statistic.PLAY_ONE_MINUTE);
        long seconds = ticksPlayed / 20;
        long hours = seconds / 3600;
        long minutes = (seconds % 3600) / 60;
        long secs = seconds % 60;

        player.sendMessage(ChatColor.GOLD + "=== Playtime: " + ChatColor.YELLOW + target.getName() + ChatColor.GOLD + " ===");
        player.sendMessage(ChatColor.GRAY + "Total Time: " + ChatColor.GREEN + hours + "h " + minutes + "m " + secs + "s");
    }
}
