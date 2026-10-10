package net.scopenet.paper.commands;

import net.scopenet.integration.Integration;
import net.scopenet.integration.Settings;
import net.scopenet.paper.ScopenetPlugin;
import org.bukkit.ChatColor;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;

public final class ScopenetCommandHandler implements CommandExecutor {
    private final ScopenetPlugin plugin;
    private final Integration integration;

    private final java.util.function.Supplier<net.scopenet.paper.compat.CompatManager> compat;

    public ScopenetCommandHandler(ScopenetPlugin plugin, Integration integration, java.util.function.Supplier<net.scopenet.paper.compat.CompatManager> compat) {
        this.plugin = plugin;
        this.integration = integration;
        this.compat = compat;
    }

    @Override
    public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (command.getName().equalsIgnoreCase("map")) {
            sendMap(sender);
            return true;
        }
        if (args.length == 0 || args[0].equalsIgnoreCase("help")) {
            if (!sender.hasPermission("scopenet.command.scopenet.help")) {
                sender.sendMessage(ChatColor.RED + "You do not have permission to view Velora help.");
                return true;
            }
            sendHelp(sender);
            return true;
        }

        String sub = args[0].toLowerCase();
        switch (sub) {
            case "panel", "web", "app" -> sendPanel(sender);
            case "status" -> sendStatus(sender);
            case "map" -> sendMap(sender);
            case "reload" -> handleReload(sender);
            default -> {
                sender.sendMessage(ChatColor.RED + "Unknown command. Use " + ChatColor.YELLOW + "/scopenet help");
            }
        }
        return true;
    }

    /** The link to the website's player panel: market, casino, friends, guilds and more, from any phone or browser. */
    private void sendPanel(CommandSender sender) {
        if (!sender.hasPermission("scopenet.command.scopenet.panel")) {
            sender.sendMessage(ChatColor.RED + "You do not have permission to use /scopenet panel.");
            return;
        }
        if (integration == null) {
            sender.sendMessage(ChatColor.RED + "Velora is not connected to a panel yet.");
            return;
        }
        String url = integration.settings().panel().toString().replaceAll("/+$", "") + "/#/play";
        sender.sendMessage(ChatColor.GOLD + "Player panel: " + ChatColor.AQUA + url);
        sender.sendMessage(ChatColor.GRAY + "Market, casino, friends, guilds, quests and more, from any phone or browser. Sign in with your launcher account.");
    }

    private void sendHelp(CommandSender sender) {
        Settings s = integration != null ? integration.settings() : null;
        sender.sendMessage(ChatColor.GOLD + "=================== " + ChatColor.YELLOW + "Velora Commands" + ChatColor.GOLD + " ===================");
        sender.sendMessage(ChatColor.AQUA + "[Utilities & custom content]");
        for (String name : java.util.List.of("heal", "feed", "fly", "vault", "echest", "kit", "quests", "hand", "nick", "shopkeeper", "autoclaim", "reqwarp", "warprequests", "warpapprove", "warpdeny", "setwarp", "delwarp", "customitem", "adminclaim")) {
            org.bukkit.command.PluginCommand c = plugin.getCommand(name);
            if (c != null && c.testPermissionSilent(sender)) sender.sendMessage(ChatColor.YELLOW + " /" + name + ChatColor.GRAY + " - " + c.getDescription());
        }
        if (sender.hasPermission("scopenet.admin.kits")) sender.sendMessage(ChatColor.YELLOW + " /kit create <name>" + ChatColor.GRAY + " - Save your inventory as a kit");
        if (sender.hasPermission("scopenet.admin.customitem")) sender.sendMessage(ChatColor.YELLOW + " /customitem give <player> <id> [amount]" + ChatColor.GRAY + " - Give a custom item");
        if (s == null || s.guildsEnabled()) sender.sendMessage(ChatColor.YELLOW + " /guild " + String.join("|", net.scopenet.core.GuildManage.COMPLETIONS) + ChatColor.GRAY + " - Guild management");
        if (s == null || s.economyEnabled()) sender.sendMessage(ChatColor.YELLOW + " /orders; /market auction|bid|buy|cancel|claim|point" + ChatColor.GRAY + " - Marketplace commands");

        // Essentials commands
        if (s == null || s.essentialsEnabled()) {
            sender.sendMessage(ChatColor.AQUA + "[Essentials Commands]");
            sender.sendMessage(ChatColor.YELLOW + " /spawn" + ChatColor.GRAY + " - Teleport to server spawn");
            sender.sendMessage(ChatColor.YELLOW + " /home [name]" + ChatColor.GRAY + " - Teleport to home or open homes GUI");
            sender.sendMessage(ChatColor.YELLOW + " /sethome [name]" + ChatColor.GRAY + " - Set home location");
            sender.sendMessage(ChatColor.YELLOW + " /delhome [name]" + ChatColor.GRAY + " - Delete home");
            sender.sendMessage(ChatColor.YELLOW + " /back" + ChatColor.GRAY + " - Return to location before teleport/death");
            sender.sendMessage(ChatColor.YELLOW + " /tpa <player>" + ChatColor.GRAY + " - Request to teleport to a player");
            sender.sendMessage(ChatColor.YELLOW + " /tpaccept / /tpdeny" + ChatColor.GRAY + " - Accept or deny teleport");
            sender.sendMessage(ChatColor.YELLOW + " /rtp" + ChatColor.GRAY + " - Random wilderness teleport");
            sender.sendMessage(ChatColor.YELLOW + " /warp [name]" + ChatColor.GRAY + " - Teleport to warp or open warps GUI");
            sender.sendMessage(ChatColor.YELLOW + " /playtime [player]" + ChatColor.GRAY + " - View total playtime stats");
        }

        // Economy commands
        if (s == null || s.economyEnabled()) {
            sender.sendMessage(ChatColor.GREEN + "[Economy Commands]");
            sender.sendMessage(ChatColor.YELLOW + " /balance (or /bal)" + ChatColor.GRAY + " - Check your wallet balance");
            sender.sendMessage(ChatColor.YELLOW + " /pay <player> <amount>" + ChatColor.GRAY + " - Send money to a player");
            sender.sendMessage(ChatColor.YELLOW + " /baltop" + ChatColor.GRAY + " - Leaderboard of richest players");
            sender.sendMessage(ChatColor.YELLOW + " /shop" + ChatColor.GRAY + " - Open interactive Server Shop GUI");
            sender.sendMessage(ChatColor.YELLOW + " /sell (or /sell hand)" + ChatColor.GRAY + " - Sell items for cash");
            sender.sendMessage(ChatColor.YELLOW + " /market" + ChatColor.GRAY + " - Open player marketplace GUI");
            sender.sendMessage(ChatColor.YELLOW + " /market sell <price>" + ChatColor.GRAY + " - List held item on market");
            sender.sendMessage(ChatColor.YELLOW + " /trade <player>" + ChatColor.GRAY + " - Secure two-player trade GUI");
            sender.sendMessage(ChatColor.YELLOW + " /transactions" + ChatColor.GRAY + " - View recent transaction history");
        }

        // Guilds commands
        if (s == null || s.guildsEnabled()) {
            sender.sendMessage(ChatColor.BLUE + "[Guilds & Claims]");
            sender.sendMessage(ChatColor.YELLOW + " /guild" + ChatColor.GRAY + " - Open interactive Guild Dashboard GUI");
            sender.sendMessage(ChatColor.YELLOW + " /guild create <name> <tag>" + ChatColor.GRAY + " - Create a new guild");
            sender.sendMessage(ChatColor.YELLOW + " /guild leave" + ChatColor.GRAY + " - Leave your current guild");
            sender.sendMessage(ChatColor.YELLOW + " /guild members" + ChatColor.GRAY + " - List online guild members");
            sender.sendMessage(ChatColor.YELLOW + " /guild chat <msg>" + ChatColor.GRAY + " - Send guild chat message");
            sender.sendMessage(ChatColor.YELLOW + " /claim" + ChatColor.GRAY + " - Claim current chunk for your guild");
            sender.sendMessage(ChatColor.YELLOW + " /unclaim" + ChatColor.GRAY + " - Unclaim current chunk");
            sender.sendMessage(ChatColor.YELLOW + " /guild map" + ChatColor.GRAY + " - Show nearby land claims radar map");
        }

        // On the web and in the launcher
        sender.sendMessage(ChatColor.DARK_AQUA + "[Launcher & Web]");
        sender.sendMessage(ChatColor.YELLOW + " /scopenet panel" + ChatColor.GRAY + " - Link to the player panel (market, casino, friends, guilds on any device)");
        sender.sendMessage(ChatColor.GRAY + " The casino, friends and DMs live in the launcher and the player panel, not in game commands.");
        sender.sendMessage(ChatColor.GRAY + " Open the launcher's Command guide (Ctrl+K) for what every command does.");

        // Admin commands
        if (sender.hasPermission("scopenet.admin")) {
            sender.sendMessage(ChatColor.LIGHT_PURPLE + "[Admin Commands]");
            sender.sendMessage(ChatColor.YELLOW + " /scopenet status" + ChatColor.GRAY + " - Integration health and module statuses");
            sender.sendMessage(ChatColor.YELLOW + " /map" + ChatColor.GRAY + " - Map rendering and upload progress");
            sender.sendMessage(ChatColor.YELLOW + " /scopenet reload" + ChatColor.GRAY + " - Reload config and feature flags");
        }
    }

    private void sendStatus(CommandSender sender) {
        if (!sender.hasPermission("scopenet.command.scopenet.status")) {
            sender.sendMessage(ChatColor.RED + "You do not have permission to view Velora status.");
            return;
        }

        Settings s = integration != null ? integration.settings() : null;
        sender.sendMessage(ChatColor.GOLD + "=== Velora Integration Status ===");
        if (s == null) {
            sender.sendMessage(ChatColor.RED + "Status: NOT INITIALIZED or OFFLINE");
            return;
        }

        sender.sendMessage(ChatColor.GRAY + "Panel URL: " + ChatColor.WHITE + s.panel());
        sender.sendMessage(ChatColor.GRAY + "Server Token: " + ChatColor.GREEN + (s.token().isEmpty() ? "MISSING" : "CONFIGURED"));
        sender.sendMessage(ChatColor.GRAY + "Leveling: " + formatFlag(s.levelingEnabled()));
        sender.sendMessage(ChatColor.GRAY + "Quests: " + formatFlag(s.questsEnabled()));
        sender.sendMessage(ChatColor.GRAY + "Achievements: " + formatFlag(s.achievementsEnabled()));
        sender.sendMessage(ChatColor.GRAY + "Guilds: " + formatFlag(s.guildsEnabled()));
        sender.sendMessage(ChatColor.GRAY + "Land Claiming: " + formatFlag(s.landClaimingEnabled()));
        sender.sendMessage(ChatColor.GRAY + "Social: " + formatFlag(s.socialEnabled()));
        sender.sendMessage(ChatColor.GRAY + "Essentials: " + formatFlag(s.essentialsEnabled()));
        sender.sendMessage(ChatColor.GRAY + "Economy: " + formatFlag(s.economyEnabled()));
        net.scopenet.paper.compat.CompatManager manager = compat.get();
        if (manager != null) {
            sender.sendMessage(ChatColor.GOLD + "=== Plugin Integrations ===");
            for (net.scopenet.paper.compat.CompatManager.Status status : manager.statuses()) {
                String mark = switch (status.state()) {
                    case ACTIVE -> ChatColor.GREEN + "ACTIVE";
                    case FAILED -> ChatColor.RED + "FAILED";
                    case DISABLED_IN_CONFIG -> ChatColor.YELLOW + "OFF (config)";
                    case NOT_INSTALLED -> ChatColor.DARK_GRAY + "not installed";
                };
                String detail = status.state() == net.scopenet.paper.compat.CompatManager.State.FAILED
                        ? ChatColor.GRAY + " " + status.message() : "";
                String version = status.version() == null || status.version().isEmpty() ? "" : ChatColor.DARK_GRAY + " v" + status.version();
                sender.sendMessage(ChatColor.GRAY + status.name() + ": " + mark + version + detail);
            }
        }
    }

    private void sendMap(CommandSender sender) {
        if (!sender.hasPermission("scopenet.command.scopenet.status")) {
            sender.sendMessage(ChatColor.RED + "You do not have permission to view map status.");
            return;
        }
        sender.sendMessage(ChatColor.GOLD + "Velora Map: " + ChatColor.WHITE + integration.mapStatus());
    }

    private String formatFlag(boolean enabled) {
        return enabled ? ChatColor.GREEN + "ENABLED" : ChatColor.RED + "DISABLED";
    }

    private void handleReload(CommandSender sender) {
        if (!sender.hasPermission("scopenet.command.scopenet.reload")) {
            sender.sendMessage(ChatColor.RED + "You do not have permission to reload Velora.");
            return;
        }

        try {
            plugin.reloadIntegrationSettings();
            sender.sendMessage(ChatColor.GREEN + "Velora configuration reloaded successfully.");
        } catch (Exception e) { sender.sendMessage(ChatColor.RED + "Reload failed; existing integration settings retained: " + e.getMessage()); }
    }
}
