package net.velora.fabric.features;

import net.velora.core.CommandSet;
import net.velora.core.CoreCommand;
import net.velora.core.CorePlayer;
import net.velora.core.Format;

import java.util.List;
import java.util.Locale;

/** /velora help | panel | status | reload (aliases /sn and /scopenet), with the same nodes as the Paper plugin. */
final class VeloraCommand implements CoreCommand {
    private final FabricFeatures features;

    VeloraCommand(FabricFeatures features) { this.features = features; }

    @Override public String name() { return "velora"; }
    @Override public List<String> aliases() { return List.of("sn", "scopenet"); }
    @Override public String permission() { return "velora.command.velora"; }

    @Override public List<String> complete(CorePlayer player, String[] args) {
        return net.velora.core.CommandSuggestions.choices("velora", args, List.of(), player::hasPermission);
    }

    @Override public void run(CorePlayer p, String[] args) {
        String sub = args.length == 0 ? "help" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "help" -> {
                if (!p.hasPermission("velora.command.velora.help")) { p.send(Format.RED + "You do not have permission to view Velora help."); return; }
                help(p);
            }
            case "panel", "web", "app" -> {
                if (!p.hasPermission("velora.command.velora.panel")) { p.send(Format.RED + "You do not have permission to use /velora panel."); return; }
                net.scopenet.integration.Integration integration = net.velora.minecraft.Bridge.integration();
                if (integration == null) { p.send(Format.RED + "Velora is not connected to a panel yet."); return; }
                p.send(Format.GOLD + "Player panel: " + Format.AQUA + integration.settings().panel().toString().replaceAll("/+$", "") + "/#/play");
                p.send(Format.GRAY + "Market, casino, friends, guilds, quests and more, from any phone or browser. Sign in with your launcher account.");
            }
            case "status" -> {
                if (!p.hasPermission("velora.command.velora.status")) { p.send(Format.RED + "You do not have permission to use /velora status."); return; }
                features.sendStatus(p);
            }
            case "map" -> {
                if (!p.hasPermission("velora.command.velora.status")) { p.send(Format.RED + "You do not have permission to view map status."); return; }
                net.scopenet.integration.Integration integration = net.velora.minecraft.Bridge.integration();
                p.send(Format.GOLD + "Velora Map: " + Format.WHITE + (integration == null ? "Integration unavailable" : integration.mapStatus()));
            }
            case "reload" -> {
                if (!p.hasPermission("velora.command.velora.reload")) { p.send(Format.RED + "You do not have permission to reload Velora."); return; }
                features.reload(p);
            }
            default -> p.send(Format.RED + "Unknown command. Use " + Format.YELLOW + "/velora help");
        }
    }

    private void help(CorePlayer p) {
        p.send(Format.GOLD + "=================== " + Format.YELLOW + "Velora Commands" + Format.GOLD + " ===================");
        CommandSet set = features.commandSet();
        if (set == null) return;
        for (CoreCommand c : set.all()) {
            if (!c.permission().isBlank() && !p.hasPermission(c.permission())) continue;
            String aliases = c.aliases().isEmpty() ? "" : Format.DARK_GRAY + " (" + String.join(", ", c.aliases().stream().map(a -> "/" + a).toList()) + ")";
            p.send(Format.YELLOW + " /" + c.name() + aliases);
        }
        p.send(Format.GRAY + "Open the launcher's Command guide for what each one does.");
        if (p.hasPermission("velora.admin.kits")) p.send(Format.YELLOW + " /kit create <name>" + Format.GRAY + " - Save your inventory as a kit");
        if (p.hasPermission("velora.admin.customitem")) p.send(Format.YELLOW + " /customitem give <player> <id> [amount]" + Format.GRAY + " - Give a custom item");
        if (p.hasPermission("velora.admin.claims")) p.send(Format.YELLOW + " /adminclaim create|add|remove|delete|rename|describe|color|list|info" + Format.GRAY + " - Protect server land");
        if (p.hasPermission("velora.command.guild")) p.send(Format.YELLOW + " /guild " + String.join("|", net.velora.core.GuildManage.COMPLETIONS) + Format.GRAY + " - Guild management");
        p.send(Format.YELLOW + " /velora panel" + Format.GRAY + " - Link to the player panel (market, casino, friends, guilds on any device)");
        p.send(Format.YELLOW + " /velora map" + Format.GRAY + " - Map rendering and upload progress");
    }
}
