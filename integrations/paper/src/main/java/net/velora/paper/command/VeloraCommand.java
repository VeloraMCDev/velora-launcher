package net.velora.paper.command;

import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.format.NamedTextColor;
import net.kyori.adventure.text.format.TextDecoration;
import net.velora.paper.VeloraPlugin;
import net.velora.paper.analytics.AnalyticsTracker;
import net.velora.paper.config.VeloraConfig;
import net.velora.paper.panel.HelloResult;
import net.velora.paper.panel.PanelClient;
import org.bukkit.Bukkit;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.command.TabCompleter;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

public final class VeloraCommand implements CommandExecutor, TabCompleter {
    private final VeloraPlugin plugin;
    private final VeloraConfig config;
    private final PanelClient panelClient;
    private final AnalyticsTracker tracker;

    public VeloraCommand(VeloraPlugin plugin, VeloraConfig config, PanelClient panelClient, AnalyticsTracker tracker) {
        this.plugin = plugin;
        this.config = config;
        this.panelClient = panelClient;
        this.tracker = tracker;
    }

    @Override
    public boolean onCommand(@NotNull CommandSender sender, @NotNull Command command, @NotNull String label, @NotNull String[] args) {
        if (!sender.hasPermission("velora.admin")) {
            sender.sendMessage(Component.text("You do not have permission to use this command.", NamedTextColor.RED));
            return true;
        }

        if (args.length == 0 || args[0].equalsIgnoreCase("status")) {
            showStatus(sender);
            return true;
        }

        String sub = args[0].toLowerCase();
        switch (sub) {
            case "link" -> {
                if (args.length < 2) {
                    sender.sendMessage(Component.text("Usage: /" + label + " link <token>", NamedTextColor.YELLOW));
                    return true;
                }
                String token = args[1].trim();
                sender.sendMessage(Component.text("Updating token and connecting to Velora panel...", NamedTextColor.GRAY));
                config.setToken(token);

                Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
                    HelloResult result = plugin.performHandshake();
                    if (result.ok()) {
                        sender.sendMessage(Component.text("✔ Successfully linked to Velora Panel as '", NamedTextColor.GREEN)
                                .append(Component.text(result.serverName(), NamedTextColor.AQUA))
                                .append(Component.text("' (ID: " + result.serverId() + ")!", NamedTextColor.GREEN)));
                    } else {
                        sender.sendMessage(Component.text("✖ Failed to link with panel: ", NamedTextColor.RED)
                                .append(Component.text(result.errorMessage(), NamedTextColor.YELLOW)));
                    }
                });
                return true;
            }
            case "reload" -> {
                sender.sendMessage(Component.text("Reloading Velora configuration...", NamedTextColor.GRAY));
                config.load();
                Bukkit.getScheduler().runTaskAsynchronously(plugin, () -> {
                    HelloResult result = plugin.performHandshake();
                    if (result.ok()) {
                        sender.sendMessage(Component.text("✔ Configuration reloaded and linked to panel ('" + result.serverName() + "')!", NamedTextColor.GREEN));
                    } else {
                        sender.sendMessage(Component.text("⚠ Configuration reloaded, but panel link warning: " + result.errorMessage(), NamedTextColor.YELLOW));
                    }
                });
                return true;
            }
            default -> {
                sender.sendMessage(Component.text("Unknown subcommand. Use /" + label + " [status | link <token> | reload]", NamedTextColor.RED));
                return true;
            }
        }
    }

    private void showStatus(CommandSender sender) {
        HelloResult hello = panelClient.getLastHello();
        boolean linked = panelClient.isLinked();

        sender.sendMessage(Component.text("---------------- [ Velora Paper Status ] ----------------", NamedTextColor.DARK_GRAY, TextDecoration.BOLD));
        sender.sendMessage(Component.text("Panel URL: ", NamedTextColor.GRAY).append(Component.text(config.getPanelUrl(), NamedTextColor.WHITE)));
        sender.sendMessage(Component.text("Server Token: ", NamedTextColor.GRAY).append(Component.text(config.hasToken() ? (config.getToken().substring(0, Math.min(6, config.getToken().length())) + "...") : "<not set>", NamedTextColor.WHITE)));

        if (linked) {
            sender.sendMessage(Component.text("Connection: ", NamedTextColor.GRAY).append(Component.text("✔ Linked", NamedTextColor.GREEN))
                    .append(Component.text(" (Server: ", NamedTextColor.GRAY))
                    .append(Component.text(hello.serverName(), NamedTextColor.AQUA))
                    .append(Component.text(" #", NamedTextColor.GRAY))
                    .append(Component.text(String.valueOf(hello.serverId()), NamedTextColor.AQUA))
                    .append(Component.text(")", NamedTextColor.GRAY)));
        } else {
            sender.sendMessage(Component.text("Connection: ", NamedTextColor.GRAY).append(Component.text("✖ Not linked", NamedTextColor.RED))
                    .append(Component.text(" (" + hello.errorMessage() + ")", NamedTextColor.YELLOW)));
        }

        sender.sendMessage(Component.text("Custom Auth: ", NamedTextColor.GRAY).append(config.isAuthEnabled() ? Component.text("Enabled (Fail-Closed)", NamedTextColor.GREEN) : Component.text("Disabled", NamedTextColor.RED)));
        sender.sendMessage(Component.text("Analytics: ", NamedTextColor.GRAY).append(config.isAnalyticsEnabled() ? Component.text("Enabled (" + config.getSyncIntervalSeconds() + "s interval)", NamedTextColor.GREEN) : Component.text("Disabled", NamedTextColor.RED)));
        sender.sendMessage(Component.text("Chat Formatting: ", NamedTextColor.GRAY).append(config.isChatEnabled() ? Component.text("Enabled", NamedTextColor.GREEN) : Component.text("Disabled", NamedTextColor.RED)));
        sender.sendMessage(Component.text("TPS: ", NamedTextColor.GRAY).append(Component.text(String.format("%.2f", tracker.getCurrentTps()), NamedTextColor.YELLOW)));
        sender.sendMessage(Component.text("Online Players: ", NamedTextColor.GRAY).append(Component.text(String.valueOf(Bukkit.getOnlinePlayers().size()), NamedTextColor.WHITE)));
        sender.sendMessage(Component.text("---------------------------------------------------------", NamedTextColor.DARK_GRAY, TextDecoration.BOLD));
    }

    @Override
    public @Nullable List<String> onTabComplete(@NotNull CommandSender sender, @NotNull Command command, @NotNull String alias, @NotNull String[] args) {
        if (!sender.hasPermission("velora.admin")) return Collections.emptyList();
        if (args.length == 1) {
            List<String> subs = Arrays.asList("status", "link", "reload");
            List<String> results = new ArrayList<>();
            for (String s : subs) {
                if (s.startsWith(args[0].toLowerCase())) {
                    results.add(s);
                }
            }
            return results;
        }
        return Collections.emptyList();
    }
}
