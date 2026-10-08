package net.scopenet.core;

import java.util.*;
import java.util.function.Predicate;

/** Argument choices shared by command adapters. Free text and numeric values have no invented placeholders. */
public final class CommandSuggestions {
    private CommandSuggestions() {}
    public static final List<String> GUILD = List.of("info", "create", "leave", "rename", "disband", "claim", "unclaim", "map", "chat", "sethome", "home", "members", "bank", "sell", "market", "pay", "invite", "accept", "decline", "list", "join", "requests", "approve", "reject", "kick", "promote", "demote", "role", "roles", "transfer", "motd", "desc", "post", "posts", "flags");
    /** The rule ids {@code /guild flags <rule> <on|off>} accepts (the panel decides which a guild may actually change). */
    public static final List<String> GUILD_RULES = List.of("build", "interact", "containers", "entry", "pvp", "mob_spawning", "mob_griefing", "explosions", "fire_spread", "fluid_flow");
    public static List<String> choices(String command, String[] args, List<String> players, Predicate<String> permission) {
        if (args.length == 0) return List.of();
        String name = command.toLowerCase(Locale.ROOT), sub = args[0].toLowerCase(Locale.ROOT);
        if (name.equals("scopenet")) return args.length == 1 ? List.of("help", "panel", "status", "map", "reload").stream()
                .filter(s -> permission.test("scopenet.command.scopenet." + (s.equals("map") ? "status" : s))).toList() : List.of();
        if (name.equals("guild")) {
            if (args.length == 1) return GUILD.stream().filter(s -> permission.test("scopenet.command.guild." + s)).toList();
            if (args.length == 2 && Set.of("invite", "accept", "decline", "approve", "reject", "kick", "promote", "demote", "role", "transfer", "pay").contains(sub)) return players;
            if (args.length == 2 && sub.equals("bank")) return List.of("deposit", "withdraw");
            if (args.length == 2 && sub.equals("flags")) return GUILD_RULES;
            if (args.length == 3 && sub.equals("flags")) return List.of("on", "off");
            if (args.length == 2 && sub.equals("sell")) return List.of("hand");
            if (sub.equals("market")) return choices("market", Arrays.copyOfRange(args, 1, args.length), players, permission);
        }
        if (name.equals("market")) {
            if (args.length == 1) return List.of("sell", "auction", "bid", "buy", "cancel", "claim", "point").stream()
                    .filter(s -> !s.equals("point") || permission.test("scopenet.command.market.point")).toList();
            if (args.length == 2 && sub.equals("point")) return List.of("add", "remove", "list");
            if (args.length == 3 && sub.equals("point") && args[1].equalsIgnoreCase("add")) return List.of("shop", "market");
        }
        if (args.length == 1 && Set.of("tpa", "playtime", "pay", "trade", "quests").contains(name)) return players;
        if (args.length == 1 && name.equals("sell")) return List.of("hand");
        if (args.length == 1 && name.equals("nick")) return List.of("off");
        if (args.length == 1 && name.equals("autoclaim")) return List.of("on", "off");
        return List.of();
    }
    public static List<String> filter(List<String> choices, String[] args) {
        String partial = args.length == 0 ? "" : args[args.length - 1].toLowerCase(Locale.ROOT);
        return choices.stream().filter(s -> s.toLowerCase(Locale.ROOT).startsWith(partial)).distinct().sorted().toList();
    }
}
