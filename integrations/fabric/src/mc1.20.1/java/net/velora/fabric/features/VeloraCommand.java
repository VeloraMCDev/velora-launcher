package net.velora.fabric.features;

import net.velora.core.CommandSet;
import net.velora.core.CoreCommand;
import net.velora.core.CorePlayer;
import net.velora.core.Format;

import java.util.List;
import java.util.Locale;

/** The Velora hub command; /help shares the same command guide. */
final class VeloraCommand implements CoreCommand {
    private final FabricFeatures features;

    VeloraCommand(FabricFeatures features) { this.features = features; }

    @Override public String name() { return "velora"; }
    @Override public List<String> aliases() { return List.of(); }
    @Override public String permission() { return "velora.command.velora"; }

    @Override public List<String> complete(CorePlayer player, String[] args) {
        return net.velora.core.CommandSuggestions.choices("velora", args, List.of(), player::hasPermission);
    }

    @Override public void run(CorePlayer p, String[] args) {
        String sub = args.length == 0 ? "help" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "help" -> {
                if (!p.hasPermission("velora.command.velora.help")) { p.send(Format.RED + "You do not have permission to view Velora help."); return; }
                help(p, java.util.Arrays.copyOfRange(args, Math.min(1, args.length), args.length));
            }
            case "panel", "web", "app" -> {
                if (!p.hasPermission("velora.command.velora.panel")) { p.send(Format.RED + "You do not have permission to use /velora panel."); return; }
                net.scopenet.integration.Integration integration = net.velora.minecraft.Bridge.integration();
                if (integration == null) { p.send(Format.RED + "Velora is not connected to a panel yet."); return; }
                p.send(Format.GOLD + "Player panel: " + Format.AQUA + integration.settings().panel().toString().replaceAll("/+$", "") + "/#/play");
                p.send(Format.GRAY + "Market, casino, friends, factions, quests and more, from any phone or browser. Sign in with your launcher account.");
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

    void help(CorePlayer p, String[] args) {
        CommandSet set = features.commandSet();
        if (set == null) return;
        String[] query = args.clone();
        if (query.length > 0) {
            String requested = query[0].replaceFirst("^/", "").toLowerCase(Locale.ROOT);
            set.all().stream().filter(c -> c.aliases().contains(requested)).findFirst().ifPresent(c -> query[0] = c.name());
        }
        net.velora.core.CommandGuide.show(p, name -> name.equals("velora") || name.equals("help") ||
                (name.equals("pshop") && features.physicalShopsEnabled()) ||
                set.all().stream().anyMatch(c -> c.name().equals(name) && c.available()), query);
    }
}
