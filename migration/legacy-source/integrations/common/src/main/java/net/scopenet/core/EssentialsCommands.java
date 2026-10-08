package net.scopenet.core;

import java.util.*;

/** /spawn /home /sethome /delhome /back /tpa /tpaccept /tpdeny /rtp /warp /playtime. */
final class EssentialsCommands {
    private EssentialsCommands() {}

    static List<CoreCommand> build(Env env, EssentialsService ess) {
        List<CoreCommand> out = new ArrayList<>();
        out.add(Cmd.of("spawn", List.of("hub", "lobby"), (p, a) -> {
            Pos spawn = env.platform.worldSpawn(p.pos().world());
            ess.teleportTo(p, new Pos(spawn.world(), spawn.x() + 0.5, spawn.y(), spawn.z() + 0.5, spawn.yaw(), spawn.pitch()));
            p.send(Format.GREEN + "Teleported to spawn.");
        }));
        out.add(Cmd.of("home", List.of("homes"), (p, a) -> {
            List<String> choices = ess.home(p, a);
            if (!choices.isEmpty()) p.send(Format.GOLD + "Your homes: " + Format.YELLOW + String.join(Format.GRAY + ", " + Format.YELLOW, choices)
                    + Format.GRAY + "  (use /home <name>)");
        }).completing((p, a) -> a.length <= 1 ? ess.homeNames(p.uuid()) : List.of()));
        out.add(Cmd.of("sethome", List.of("createshome"), ess::setHome));
        out.add(Cmd.of("delhome", List.of("rmhome"), ess::delHome).completing((p, a) -> a.length <= 1 ? ess.homeNames(p.uuid()) : List.of()));
        out.add(Cmd.of("back", List.of("return"), (p, a) -> ess.back(p)));
        out.add(Cmd.of("tpa", List.of(), ess::tpa).completing((p, a) -> a.length <= 1 ? names(env, p) : List.of()));
        out.add(Cmd.of("tpaccept", List.of("tpyes"), (p, a) -> ess.tpAccept(p)));
        out.add(Cmd.of("tpdeny", List.of("tpno"), (p, a) -> ess.tpDeny(p)));
        out.add(Cmd.of("rtp", List.of("wild", "randomtp"), (p, a) -> ess.rtp(p)));
        out.add(Cmd.of("warp", List.of("warps"), (p, a) -> {
            List<String> choices = ess.warp(p, a);
            if (!choices.isEmpty()) p.send(Format.AQUA + "Warps: " + Format.YELLOW + String.join(Format.GRAY + ", " + Format.YELLOW, choices)
                    + Format.GRAY + "  (use /warp <name>)");
        }).completing((p, a) -> a.length <= 1 ? ess.warpNames() : List.of()));
        out.add(Cmd.of("playtime", List.of("ontime"), (p, a) -> {
            CorePlayer target = p;
            if (a.length > 0) {
                Optional<CorePlayer> found = env.platform.playerByName(a[0]);
                if (found.isEmpty()) { p.send(Format.RED + "Player not found."); return; }
                target = found.get();
            }
            p.send(Format.GOLD + "=== Playtime: " + Format.YELLOW + target.name() + Format.GOLD + " ===");
            p.send(Format.GRAY + "Total Time: " + Format.GREEN + Format.duration(target.playtimeSeconds()));
        }).completing((p, a) -> a.length <= 1 ? names(env, p) : List.of()));
        return out;
    }

    static List<String> names(Env env, CorePlayer except) {
        List<String> names = new ArrayList<>();
        for (CorePlayer p : env.platform.online()) if (!p.uuid().equals(except.uuid())) names.add(p.name());
        return names;
    }
}
