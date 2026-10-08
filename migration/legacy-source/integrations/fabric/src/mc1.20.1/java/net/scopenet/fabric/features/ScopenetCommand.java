package net.scopenet.fabric.features;

import net.scopenet.core.CommandSet;
import net.scopenet.core.CoreCommand;
import net.scopenet.core.CorePlayer;
import net.scopenet.core.Format;

import java.util.List;
import java.util.Locale;

/** /scopenet help | panel | status | reload (alias /sn), with the same nodes as the Paper plugin. */
final class ScopenetCommand implements CoreCommand {
    private final FabricFeatures features;

    ScopenetCommand(FabricFeatures features) { this.features = features; }

    @Override public String name() { return "scopenet"; }
    @Override public List<String> aliases() { return List.of("sn"); }
    @Override public String permission() { return "scopenet.command.scopenet"; }

    @Override public List<String> complete(CorePlayer player, String[] args) {
        return net.scopenet.core.CommandSuggestions.choices("scopenet", args, List.of(), player::hasPermission);
    }

    @Override public void run(CorePlayer p, String[] args) {
        String sub = args.length == 0 ? "help" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "help" -> {
                if (!p.hasPermission("scopenet.command.scopenet.help")) { p.send(Format.RED + "You do not have permission to view SCOPENET help."); return; }
                help(p);
            }
            case "panel", "web", "app" -> {
                if (!p.hasPermission("scopenet.command.scopenet.panel")) { p.send(Format.RED + "You do not have permission to use /scopenet panel."); return; }
                net.scopenet.integration.Integration integration = net.scopenet.minecraft.Bridge.integration();
                if (integration == null) { p.send(Format.RED + "SCOPENET is not connected to a panel yet."); return; }
                p.send(Format.GOLD + "Player panel: " + Format.AQUA + integration.settings().panel().toString().replaceAll("/+$", "") + "/#/play");
                p.send(Format.GRAY + "Market, casino, friends, guilds, quests and more, from any phone or browser. Sign in with your launcher account.");
            }
            case "status" -> {
                if (!p.hasPermission("scopenet.command.scopenet.status")) { p.send(Format.RED + "You do not have permission to use /scopenet status."); return; }
                features.sendStatus(p);
            }
            case "map" -> {
                if (!p.hasPermission("scopenet.command.scopenet.status")) { p.send(Format.RED + "You do not have permission to view map status."); return; }
                net.scopenet.integration.Integration integration = net.scopenet.minecraft.Bridge.integration();
                p.send(Format.GOLD + "SCOPENET Map: " + Format.WHITE + (integration == null ? "Integration unavailable" : integration.mapStatus()));
            }
            case "reload" -> {
                if (!p.hasPermission("scopenet.command.scopenet.reload")) { p.send(Format.RED + "You do not have permission to reload SCOPENET."); return; }
                features.reload(p);
            }
            default -> p.send(Format.RED + "Unknown command. Use " + Format.YELLOW + "/scopenet help");
        }
    }

    private void help(CorePlayer p) {
        p.send(Format.GOLD + "=================== " + Format.YELLOW + "SCOPENET Commands" + Format.GOLD + " ===================");
        CommandSet set = features.commandSet();
        if (set == null) return;
        for (CoreCommand c : set.all()) {
            if (!c.permission().isBlank() && !p.hasPermission(c.permission())) continue;
            String aliases = c.aliases().isEmpty() ? "" : Format.DARK_GRAY + " (" + String.join(", ", c.aliases().stream().map(a -> "/" + a).toList()) + ")";
            p.send(Format.YELLOW + " /" + c.name() + aliases);
        }
        p.send(Format.GRAY + "Open the launcher's Command guide for what each one does.");
        if (p.hasPermission("scopenet.admin.kits")) p.send(Format.YELLOW + " /kit create <name>" + Format.GRAY + " - Save your inventory as a kit");
        if (p.hasPermission("scopenet.admin.customitem")) p.send(Format.YELLOW + " /customitem give <player> <id> [amount]" + Format.GRAY + " - Give a custom item");
        if (p.hasPermission("scopenet.admin.claims")) p.send(Format.YELLOW + " /adminclaim create|add|remove|delete|rename|describe|color|list|info" + Format.GRAY + " - Protect server land");
        if (p.hasPermission("scopenet.command.guild")) p.send(Format.YELLOW + " /guild " + String.join("|", net.scopenet.core.GuildManage.COMPLETIONS) + Format.GRAY + " - Guild management");
        p.send(Format.YELLOW + " /scopenet panel" + Format.GRAY + " - Link to the player panel (market, casino, friends, guilds on any device)");
        p.send(Format.YELLOW + " /scopenet map" + Format.GRAY + " - Map rendering and upload progress");
    }
}
