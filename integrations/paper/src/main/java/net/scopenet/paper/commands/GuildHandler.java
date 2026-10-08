package net.scopenet.paper.commands;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import net.scopenet.integration.ChunkCheckResult;
import net.scopenet.integration.Integration;
import net.scopenet.paper.ScopenetPlugin;
import net.scopenet.paper.gui.GuiHelper;
import org.bukkit.Bukkit;
import org.bukkit.ChatColor;
import org.bukkit.Chunk;
import org.bukkit.Location;
import org.bukkit.Material;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.event.inventory.InventoryClickEvent;
import org.bukkit.event.player.PlayerMoveEvent;
import org.bukkit.event.player.PlayerQuitEvent;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.ItemStack;

import java.io.File;
import java.io.IOException;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

public final class GuildHandler implements CommandExecutor, Listener {
    private final ScopenetPlugin plugin;
    private final Integration integration;
    private final File guildHomesFile;
    private final YamlConfiguration guildHomesConfig;
    private final Map<String, Location> guildHomes = new ConcurrentHashMap<>();
    private final Set<UUID> autoClaim = ConcurrentHashMap.newKeySet();
    private final Map<UUID, String> lastAutoClaim = new ConcurrentHashMap<>();

    private static final String GUI_GUILD_TITLE = ChatColor.DARK_AQUA + "Guild Dashboard";

    private final EconomyHandler economy;
    private final GuildBank bank;

    public GuildHandler(ScopenetPlugin plugin, Integration integration, EconomyHandler economy) {
        this.plugin = plugin;
        this.integration = integration;
        this.economy = economy;
        this.bank = new GuildBank(plugin, integration, economy.operations());
        this.guildHomesFile = new File(plugin.getDataFolder(), "guild_homes.yml");
        this.guildHomesConfig = YamlConfiguration.loadConfiguration(guildHomesFile);
        loadGuildHomes();
    }

    private void loadGuildHomes() {
        if (!guildHomesFile.exists()) return;
        for (String gid : guildHomesConfig.getKeys(false)) {
            Location loc = guildHomesConfig.getLocation(gid);
            if (loc != null) guildHomes.put(gid, loc);
        }
    }

    private void saveGuildHomes() {
        try {
            for (Map.Entry<String, Location> entry : guildHomes.entrySet()) {
                guildHomesConfig.set(entry.getKey(), entry.getValue());
            }
            guildHomesConfig.save(guildHomesFile);
        } catch (IOException e) {
            plugin.getLogger().warning("Failed to save guild homes: " + e.getMessage());
        }
    }

    @Override
    public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (!(sender instanceof Player player)) {
            sender.sendMessage(ChatColor.RED + "Only players can execute guild commands.");
            return true;
        }

        if (!integration.settings().guildsEnabled()) { player.sendMessage(ChatColor.RED + "Guilds are disabled."); return true; }
        String cmd = command.getName().toLowerCase();
        if (cmd.equals("claim")) {
            handleClaim(player);
            return true;
        }
        if (cmd.equals("unclaim")) {
            handleUnclaim(player);
            return true;
        }
        if (cmd.equals("autoclaim")) {
            if (!player.hasPermission("scopenet.command.claim")) { player.sendMessage(ChatColor.RED + "You cannot claim land."); return true; }
            if (args.length != 1 || (!args[0].equalsIgnoreCase("on") && !args[0].equalsIgnoreCase("off"))) {
                player.sendMessage(ChatColor.YELLOW + "Usage: /autoclaim on|off"); return true;
            }
            if (args[0].equalsIgnoreCase("on")) {
                autoClaim.add(player.getUniqueId());
                lastAutoClaim.remove(player.getUniqueId());
                player.sendMessage(ChatColor.GREEN + "Auto-claim is on. New chunks you walk into will be claimed for your guild.");
                claimOnMove(player, player.getLocation());
            } else {
                autoClaim.remove(player.getUniqueId());
                lastAutoClaim.remove(player.getUniqueId());
                player.sendMessage(ChatColor.YELLOW + "Auto-claim is off.");
            }
            return true;
        }

        if (cmd.equals("guild") || cmd.equals("g")) {
            if (args.length == 0 || (args.length == 1 && (args[0].equalsIgnoreCase("info") || args[0].equalsIgnoreCase("gui")))) {
                if (!player.hasPermission("scopenet.command.guild.info")) {
                    player.sendMessage(ChatColor.RED + "You do not have permission to use /guild info.");
                    return true;
                }
                handleGuildDashboard(player);
                return true;
            }

            String sub = args[0].toLowerCase();
            String permission = sub.equals("c") ? "chat" : sub;
            if (java.util.Set.of("create", "leave", "rename", "disband", "claim", "unclaim", "map", "chat", "sethome", "home", "members", "bank", "sell", "market", "pay", "invite", "accept", "decline").contains(permission)
                    && !player.hasPermission("scopenet.command.guild." + permission)) {
                player.sendMessage(ChatColor.RED + "You do not have permission to use /guild " + permission + ".");
                return true;
            }
            // Finer nodes for the money subcommands that do several things.
            String detail = args.length > 1 ? args[1].toLowerCase() : "";
            String fine = (sub.equals("sell") && detail.equals("hand")) ? "sell.hand"
                    : (sub.equals("market") && (detail.equals("sell") || detail.equals("buy"))) ? "market." + detail : null;
            if (fine != null && !player.hasPermission("scopenet.command.guild." + fine)) {
                player.sendMessage(ChatColor.RED + "You do not have permission to use /guild " + sub + " " + detail + ".");
                return true;
            }
            switch (sub) {
                case "create" -> handleCreate(player, args);
                case "leave" -> handleLeave(player);
                case "rename" -> handleRename(player, args);
                case "disband" -> handleDisband(player, args);
                case "claim" -> handleClaim(player);
                case "unclaim" -> handleUnclaim(player);
                case "map" -> handleMap(player);
                case "chat", "c" -> handleChat(player, args);
                case "sethome" -> handleSetHome(player);
                case "home" -> handleHome(player);
                case "members" -> handleMembers(player);
                case "bank" -> bank.bank(player, Arrays.copyOfRange(args, 1, args.length));
                case "pay" -> bank.pay(player, Arrays.copyOfRange(args, 1, args.length));
                case "invite" -> handleInvite(player, args);
                case "accept" -> handleInviteAnswer(player, args, true);
                case "decline", "deny" -> handleInviteAnswer(player, args, false);
                case "sell" -> economy.handleSell(player, Arrays.copyOfRange(args, 1, args.length), true);
                case "market" -> economy.handleMarket(player, Arrays.copyOfRange(args, 1, args.length), true);
                default -> {
                    // list, join, kick, promote, roles, motd, post… are handled by the shared guild management code.
                    if (manager != null && net.scopenet.core.GuildManage.handles(sub, args.length - 1)) manager.accept(player, args);
                    else sendGuildHelp(player);
                }
            }
            return true;
        }

        return false;
    }

    private java.util.function.BiConsumer<Player, String[]> manager;

    /** Hands the management sub-commands (everything after the guild-specific ones above) to the shared implementation. */
    public void useManager(java.util.function.BiConsumer<Player, String[]> manager) { this.manager = manager; }

    private void handleInvite(Player player, String[] args) {
        if (args.length < 2) { player.sendMessage(ChatColor.RED + "Usage: /guild invite <player>"); return; }
        com.google.gson.JsonObject body = new com.google.gson.JsonObject();
        body.addProperty("uuid", player.getUniqueId().toString());
        body.addProperty("target", args[1]);
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                com.google.gson.JsonObject r = integration.client().post("guilds/invite/send", body);
                player.sendMessage(ChatColor.GREEN + "Invitation sent to " + ChatColor.YELLOW + (r.has("player") ? r.get("player").getAsString() : args[1]) + ChatColor.GREEN + ".");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Could not invite: " + e.getMessage());
            }
        });
    }

    private void handleInviteAnswer(Player player, String[] args, boolean accept) {
        com.google.gson.JsonObject body = new com.google.gson.JsonObject();
        body.addProperty("uuid", player.getUniqueId().toString());
        body.addProperty("tag", args.length > 1 ? args[1] : "");
        body.addProperty("accept", accept);
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                com.google.gson.JsonObject r = integration.client().post("guilds/invite/respond", body);
                if (accept) integration.client().requestClaimRefresh();
                player.sendMessage(accept
                        ? ChatColor.GREEN + "Welcome to " + ChatColor.YELLOW + "[" + r.get("tag").getAsString() + "] " + r.get("guild").getAsString() + ChatColor.GREEN + "!"
                        : ChatColor.YELLOW + "Invitation declined.");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + e.getMessage());
            }
        });
    }

    /** Guild homes by guild id, for the map. */
    public Map<String, net.scopenet.core.Pos> guildHomesForMap() {
        Map<String, net.scopenet.core.Pos> out = new java.util.TreeMap<>();
        guildHomes.forEach((id, l) -> { if (l.getWorld() != null) out.put(id, new net.scopenet.core.Pos(ScopenetPlugin.dimension(l.getWorld()), l.getX(), l.getY(), l.getZ(), l.getYaw(), l.getPitch())); });
        return out;
    }

    private void sendGuildHelp(Player player) {
        player.sendMessage(ChatColor.GOLD + "=== Velora Guild Commands ===");
        player.sendMessage(ChatColor.YELLOW + "/guild" + ChatColor.GRAY + " - Open interactive Guild Dashboard GUI");
        player.sendMessage(ChatColor.YELLOW + "/guild create <name> <tag>" + ChatColor.GRAY + " - Create a new guild");
        player.sendMessage(ChatColor.YELLOW + "/guild leave" + ChatColor.GRAY + " - Leave your current guild");
        player.sendMessage(ChatColor.YELLOW + "/guild rename <name> [tag]" + ChatColor.GRAY + " - Rename your guild (leader)");
        player.sendMessage(ChatColor.YELLOW + "/guild disband" + ChatColor.GRAY + " - Disband your guild (leader)");
        player.sendMessage(ChatColor.YELLOW + "/claim" + ChatColor.GRAY + " (or /guild claim) - Claim current chunk");
        player.sendMessage(ChatColor.YELLOW + "/unclaim" + ChatColor.GRAY + " (or /guild unclaim) - Unclaim current chunk");
        player.sendMessage(ChatColor.YELLOW + "/guild map" + ChatColor.GRAY + " - Show nearby land claims radar map");
        player.sendMessage(ChatColor.YELLOW + "/guild chat <msg>" + ChatColor.GRAY + " - Send message to guild members");
        player.sendMessage(ChatColor.YELLOW + "/guild sethome / /guild home" + ChatColor.GRAY + " - Guild waypoint base");
        player.sendMessage(ChatColor.YELLOW + "/guild bank [deposit|withdraw <amount>]" + ChatColor.GRAY + " - Guild money");
        player.sendMessage(ChatColor.YELLOW + "/guild sell [hand]" + ChatColor.GRAY + " - Sell items to the shop for the guild bank");
        player.sendMessage(ChatColor.YELLOW + "/guild market sell <price> | buy <#id>" + ChatColor.GRAY + " - Trade on the market as a guild");
        player.sendMessage(ChatColor.YELLOW + "/guild pay <tag> <amount>" + ChatColor.GRAY + " - Pay another guild");
        player.sendMessage(ChatColor.YELLOW + "/guild invite <player>" + ChatColor.GRAY + " - Invite a player (leaders and officers)");
        player.sendMessage(ChatColor.YELLOW + "/guild accept [tag] | decline [tag]" + ChatColor.GRAY + " - Answer a guild invitation");
        player.sendMessage(ChatColor.YELLOW + "/guild list" + ChatColor.GRAY + " / " + ChatColor.YELLOW + "info <name>" + ChatColor.GRAY + " - Browse the guilds on this server");
        player.sendMessage(ChatColor.YELLOW + "/guild join <name> [message]" + ChatColor.GRAY + " - Ask to join a guild");
        player.sendMessage(ChatColor.YELLOW + "/guild requests" + ChatColor.GRAY + " / " + ChatColor.YELLOW + "approve <player>" + ChatColor.GRAY + " / " + ChatColor.YELLOW + "reject <player>" + ChatColor.GRAY + " - Review join requests");
        player.sendMessage(ChatColor.YELLOW + "/guild kick <player>" + ChatColor.GRAY + " - Remove a member");
        player.sendMessage(ChatColor.YELLOW + "/guild promote <player>" + ChatColor.GRAY + " / " + ChatColor.YELLOW + "demote <player>" + ChatColor.GRAY + " - Officers (leader)");
        player.sendMessage(ChatColor.YELLOW + "/guild roles" + ChatColor.GRAY + " / " + ChatColor.YELLOW + "role <player> <role>" + ChatColor.GRAY + " - See and hand out roles (leader)");
        player.sendMessage(ChatColor.YELLOW + "/guild transfer <player> confirm" + ChatColor.GRAY + " - Hand over leadership");
        player.sendMessage(ChatColor.YELLOW + "/guild motd <text>" + ChatColor.GRAY + " / " + ChatColor.YELLOW + "desc <text>" + ChatColor.GRAY + " - Message of the day and description");
        player.sendMessage(ChatColor.YELLOW + "/guild post <title> | <text>" + ChatColor.GRAY + " / " + ChatColor.YELLOW + "posts" + ChatColor.GRAY + " - Guild announcements");
    }

    private void handleClaim(Player player) {
        Chunk chunk = player.getLocation().getChunk();
        String dim = ScopenetPlugin.dimension(player.getWorld());
        player.sendMessage(ChatColor.GRAY + "Claiming chunk [" + chunk.getX() + ", " + chunk.getZ() + "]...");

        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().claimChunk(dim, chunk.getX(), chunk.getZ(), player.getUniqueId());
                String gname = res.has("guild_name") ? res.get("guild_name").getAsString() : "your guild";
                player.sendMessage(ChatColor.GREEN + "Chunk claimed successfully for " + ChatColor.YELLOW + gname + ChatColor.GREEN + "!");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Claim failed: " + e.getMessage());
            }
        });
    }

    @EventHandler
    public void onMove(PlayerMoveEvent event) {
        if (event.getTo() == null || !autoClaim.contains(event.getPlayer().getUniqueId())) return;
        if (event.getFrom().getWorld() == event.getTo().getWorld()
                && event.getFrom().getChunk().getX() == event.getTo().getChunk().getX()
                && event.getFrom().getChunk().getZ() == event.getTo().getChunk().getZ()) return;
        claimOnMove(event.getPlayer(), event.getTo());
    }

    @EventHandler public void onQuit(PlayerQuitEvent event) {
        autoClaim.remove(event.getPlayer().getUniqueId());
        lastAutoClaim.remove(event.getPlayer().getUniqueId());
    }

    private void claimOnMove(Player player, Location location) {
        if (!integration.settings().landClaimingEnabled()) return;
        int x = location.getBlockX() >> 4, z = location.getBlockZ() >> 4;
        String dim = ScopenetPlugin.dimension(location.getWorld());
        String key = dim + ':' + x + ':' + z;
        if (key.equals(lastAutoClaim.put(player.getUniqueId(), key))) return;
        if (integration.isClaimed(dim, x, z)) return;
        UUID uuid = player.getUniqueId();
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                integration.client().claimChunk(dim, x, z, uuid);
                player.sendMessage(ChatColor.GREEN + "Auto-claimed chunk [" + x + ", " + z + "].");
            } catch (Exception e) {
                autoClaim.remove(uuid);
                player.sendMessage(ChatColor.RED + "Auto-claim stopped: " + e.getMessage());
            }
        });
    }

    private void handleUnclaim(Player player) {
        Chunk chunk = player.getLocation().getChunk();
        String dim = ScopenetPlugin.dimension(player.getWorld());
        player.sendMessage(ChatColor.GRAY + "Unclaiming chunk [" + chunk.getX() + ", " + chunk.getZ() + "]...");

        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                integration.client().unclaimChunk(dim, chunk.getX(), chunk.getZ(), player.getUniqueId());
                player.sendMessage(ChatColor.GREEN + "Chunk [" + chunk.getX() + ", " + chunk.getZ() + "] has been unclaimed.");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Unclaim failed: " + e.getMessage());
            }
        });
    }

    private void handleCreate(Player player, String[] args) {
        if (args.length < 3) {
            player.sendMessage(ChatColor.RED + "Usage: /guild create <name> <tag>");
            return;
        }

        String name = args[1];
        String tag = args[2];
        player.sendMessage(ChatColor.GRAY + "Founding guild " + name + " [" + tag + "]...");

        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().createGuild(player.getUniqueId(), player.getName(), name, tag);
                player.sendMessage(ChatColor.GREEN + "Guild " + ChatColor.YELLOW + name + ChatColor.GOLD + " [" + tag + "]"
                        + ChatColor.GREEN + " founded successfully! You are the Guild Leader.");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Failed to create guild: " + e.getMessage());
            }
        });
    }

    private void handleRename(Player player, String[] args) {
        if (args.length < 2) { player.sendMessage(ChatColor.RED + "Usage: /guild rename <name> [tag]"); return; }
        com.google.gson.JsonObject body = new com.google.gson.JsonObject();
        body.addProperty("uuid", player.getUniqueId().toString());
        body.addProperty("name", args[1]);
        if (args.length > 2) body.addProperty("tag", args[2]);
        player.sendMessage(ChatColor.GRAY + "Renaming guild...");
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject r = integration.client().post("guilds/rename", body);
                player.sendMessage(ChatColor.GREEN + "Your guild is now " + ChatColor.YELLOW + r.get("name").getAsString() + ChatColor.GOLD + " [" + r.get("tag").getAsString() + "]" + ChatColor.GREEN + ".");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Could not rename the guild: " + e.getMessage());
            }
        });
    }

    private void handleDisband(Player player, String[] args) {
        if (args.length < 2 || !args[1].equalsIgnoreCase("confirm")) {
            player.sendMessage(ChatColor.RED + "This permanently deletes your guild, frees all its land and pays the treasury to you.");
            player.sendMessage(ChatColor.YELLOW + "Type " + ChatColor.GOLD + "/guild disband confirm" + ChatColor.YELLOW + " to go ahead.");
            return;
        }
        com.google.gson.JsonObject body = new com.google.gson.JsonObject();
        body.addProperty("uuid", player.getUniqueId().toString());
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject r = integration.client().post("guilds/disband", body);
                double refunded = r.has("refunded") ? r.get("refunded").getAsDouble() : 0;
                player.sendMessage(ChatColor.YELLOW + "Your guild was disbanded." + (refunded > 0 ? ChatColor.GREEN + " " + refunded + " from the treasury was paid to you." : ""));
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Could not disband the guild: " + e.getMessage());
            }
        });
    }

    private void handleLeave(Player player) {
        player.sendMessage(ChatColor.GRAY + "Leaving guild...");
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().leaveGuild(player.getUniqueId());
                boolean disbanded = res.has("disbanded") && res.get("disbanded").getAsBoolean();
                if (disbanded) {
                    player.sendMessage(ChatColor.YELLOW + "You were the last member. The guild was disbanded.");
                } else {
                    player.sendMessage(ChatColor.GREEN + "You left your guild.");
                }
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Error leaving guild: " + e.getMessage());
            }
        });
    }

    private void handleGuildDashboard(Player player) {
        player.sendMessage(ChatColor.GRAY + "Loading guild info...");
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().getPlayerGuild(player.getUniqueId());
                if (!res.has("in_guild") || !res.get("in_guild").getAsBoolean()) {
                    player.sendMessage(ChatColor.YELLOW + "You are not in a guild! Create one with "
                            + ChatColor.GOLD + "/guild create <name> <tag>");
                    return;
                }

                JsonObject g = res.getAsJsonObject("guild");
                String name = g.get("name").getAsString();
                String tag = g.get("tag").getAsString();
                String role = g.get("role").getAsString();
                int level = g.has("level") ? g.get("level").getAsInt() : 1;
                long claims = g.has("claims_count") ? g.get("claims_count").getAsLong() : 0;
                long maxClaims = g.has("max_claims") ? g.get("max_claims").getAsLong() : 16;
                JsonArray members = g.has("members") ? g.getAsJsonArray("members") : new JsonArray();

                Bukkit.getScheduler().runTask(plugin, () -> {
                    Inventory inv = Bukkit.createInventory(null, 36, GUI_GUILD_TITLE);
                    ItemStack border = GuiHelper.createBorder(Material.CYAN_STAINED_GLASS_PANE);
                    for (int i = 0; i < 9; i++) inv.setItem(i, border);
                    for (int i = 27; i < 36; i++) inv.setItem(i, border);

                    // Guild banner
                    inv.setItem(11, GuiHelper.createItem(Material.CYAN_BANNER,
                            "&bGuild: &f" + name + " &e[" + tag + "]",
                            "&7Your Role: &a" + role.toUpperCase(),
                            "&7Guild Level: &6" + level,
                            "&7Members: &f" + members.size()));

                    // Claims status
                    inv.setItem(13, GuiHelper.createItem(Material.EMERALD_BLOCK,
                            "&aTerritory Claims",
                            "&7Claimed Chunks: &e" + claims + " &7/ &a" + maxClaims,
                            "",
                            "&7Click &e/claim &7to claim nearby area."));

                    // Quick claim button
                    inv.setItem(14, GuiHelper.createItem(Material.LIME_DYE,
                            "&aClaim Current Chunk",
                            "&7Click to claim the chunk you are standing on!"));

                    // Quick unclaim button
                    inv.setItem(15, GuiHelper.createItem(Material.RED_DYE,
                            "&cUnclaim Current Chunk",
                            "&7Click to unclaim the current chunk."));

                    // Minimap button
                    inv.setItem(22, GuiHelper.createItem(Material.MAP,
                            "&eView Land Map",
                            "&7Shows nearby territory claims radar in chat."));

                    player.openInventory(inv);
                });
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Failed to load guild: " + e.getMessage());
            }
        });
    }

    private void handleMap(Player player) {
        Location pLoc = player.getLocation();
        Chunk center = pLoc.getChunk();
        int cx = center.getX();
        int cz = center.getZ();
        String dim = ScopenetPlugin.dimension(pLoc.getWorld());

        player.sendMessage(ChatColor.GOLD + "========= " + ChatColor.YELLOW + "Territory Map (" + cx + ", " + cz + ")" + ChatColor.GOLD + " =========");
        player.sendMessage(ChatColor.GRAY + "   N (-Z)");

        StringBuilder header = new StringBuilder("  ");
        for (int dx = -4; dx <= 4; dx++) {
            header.append(" ");
        }
        player.sendMessage(header.toString());

        for (int dz = -3; dz <= 3; dz++) {
            StringBuilder row = new StringBuilder(" ");
            for (int dx = -4; dx <= 4; dx++) {
                int targetX = cx + dx;
                int targetZ = cz + dz;
                boolean isPlayer = (dx == 0 && dz == 0);

                ChunkCheckResult res = integration.checkChunk(dim, targetX, targetZ, player.getUniqueId());
                if (isPlayer) {
                    row.append(ChatColor.GOLD).append("[P]");
                } else if (res.claimed() && res.allowed()) {
                    row.append(ChatColor.GREEN).append(" + ");
                } else if (res.claimed() && !res.allowed()) {
                    row.append(ChatColor.RED).append(" x ");
                } else {
                    row.append(ChatColor.DARK_GRAY).append(" . ");
                }
            }
            player.sendMessage(row.toString());
        }

        player.sendMessage(ChatColor.GRAY + "   S (+Z)");
        player.sendMessage(ChatColor.GOLD + "[P]" + ChatColor.YELLOW + " You  "
                + ChatColor.GREEN + "+ " + ChatColor.GRAY + "Your Guild  "
                + ChatColor.RED + "x " + ChatColor.GRAY + "Other Guild  "
                + ChatColor.DARK_GRAY + ". " + ChatColor.GRAY + "Wilderness");
    }

    private void handleChat(Player player, String[] args) {
        if (args.length < 2) {
            player.sendMessage(ChatColor.RED + "Usage: /guild chat <message>");
            return;
        }

        String msg = String.join(" ", Arrays.copyOfRange(args, 1, args.length));
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().getPlayerGuild(player.getUniqueId());
                if (!res.has("in_guild") || !res.get("in_guild").getAsBoolean()) {
                    player.sendMessage(ChatColor.RED + "You are not in a guild.");
                    return;
                }
                String tag = res.getAsJsonObject("guild").get("tag").getAsString();
                String formatted = ChatColor.GREEN + "[Guild " + tag + "] " + ChatColor.YELLOW + player.getName() + ": " + ChatColor.WHITE + msg;
                for (Player p : Bukkit.getOnlinePlayers()) {
                    p.sendMessage(formatted);
                }
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Guild chat error: " + e.getMessage());
            }
        });
    }

    private void handleSetHome(Player player) {
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().getPlayerGuild(player.getUniqueId());
                if (!res.has("in_guild") || !res.get("in_guild").getAsBoolean()) {
                    player.sendMessage(ChatColor.RED + "You must be in a guild to set a guild home.");
                    return;
                }
                String role = res.getAsJsonObject("guild").get("role").getAsString();
                if (!role.equals("leader") && !role.equals("officer")) {
                    player.sendMessage(ChatColor.RED + "Only guild leaders and officers can set the guild home.");
                    return;
                }
                String gid = res.getAsJsonObject("guild").get("id").getAsString();
                guildHomes.put(gid, player.getLocation());
                saveGuildHomes();
                player.sendMessage(ChatColor.GREEN + "Guild home waypoint set at your location!");
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Error: " + e.getMessage());
            }
        });
    }

    private void handleHome(Player player) {
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().getPlayerGuild(player.getUniqueId());
                if (!res.has("in_guild") || !res.get("in_guild").getAsBoolean()) {
                    player.sendMessage(ChatColor.RED + "You are not in a guild.");
                    return;
                }
                String gid = res.getAsJsonObject("guild").get("id").getAsString();
                Location loc = guildHomes.get(gid);
                if (loc == null) {
                    player.sendMessage(ChatColor.RED + "Your guild has not set a guild home yet! Use /guild sethome");
                    return;
                }
                Bukkit.getScheduler().runTask(plugin, () -> {
                    player.teleport(loc);
                    player.sendMessage(ChatColor.GREEN + "Teleported to guild home base.");
                });
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Error: " + e.getMessage());
            }
        });
    }

    private void handleMembers(Player player) {
        Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
            try {
                JsonObject res = integration.client().getPlayerGuild(player.getUniqueId());
                if (!res.has("in_guild") || !res.get("in_guild").getAsBoolean()) {
                    player.sendMessage(ChatColor.RED + "You are not in a guild.");
                    return;
                }
                JsonObject g = res.getAsJsonObject("guild");
                String name = g.get("name").getAsString();
                JsonArray members = g.getAsJsonArray("members");

                player.sendMessage(ChatColor.GOLD + "=== Members of " + ChatColor.YELLOW + name + ChatColor.GOLD + " ===");
                for (JsonElement el : members) {
                    JsonObject m = el.getAsJsonObject();
                    String mname = m.get("name").getAsString();
                    String role = m.get("role").getAsString();
                    Player onlineP = Bukkit.getPlayer(mname);
                    String status = onlineP != null && onlineP.isOnline() ? ChatColor.GREEN + "[Online]" : ChatColor.GRAY + "[Offline]";
                    player.sendMessage(ChatColor.WHITE + " - " + mname + " (" + ChatColor.AQUA + role + ChatColor.WHITE + ") " + status);
                }
            } catch (Exception e) {
                player.sendMessage(ChatColor.RED + "Error fetching members: " + e.getMessage());
            }
        });
    }

    @EventHandler
    public void onInventoryClick(InventoryClickEvent event) {
        if (!(event.getWhoClicked() instanceof Player player)) return;
        if (!event.getView().getTitle().equals(GUI_GUILD_TITLE)) return;

        event.setCancelled(true);
        int slot = event.getRawSlot();
        if (slot == 14) { // Claim Current Chunk
            player.closeInventory();
            handleClaim(player);
        } else if (slot == 15) { // Unclaim Current Chunk
            player.closeInventory();
            handleUnclaim(player);
        } else if (slot == 22) { // Minimap
            player.closeInventory();
            handleMap(player);
        }
    }
}
