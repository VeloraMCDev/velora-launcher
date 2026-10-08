package net.scopenet.core;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/** Builds every shared command, wired to its services, and gates them on feature switches and permission nodes. */
public final class CommandSet {
    public final EssentialsService essentials;
    public final Jobs jobs;
    public final ShopPoints shops;
    private final GuildCommands guildCommands;
    /** /heal /feed /fly /vault /echest /kit /customitem /adminclaim and the claim banner. */
    public final UtilityHub utilityHub;
    private final EconomyCommands economyCommands;
    private final List<CoreCommand> commands = new ArrayList<>();
    private final Limits.Cooldowns cooldowns = new Limits.Cooldowns();

    public CommandSet(Env env, EssentialsConfig essentialsConfig) {
        Path dir = env.dataDir;
        essentials = new EssentialsService(env.platform, dir.resolve("essentials.json"), essentialsConfig, env.clock, env.rng);
        jobs = new Jobs(env, dir.resolve("economy-pending.json"));
        shops = new ShopPoints(dir.resolve("shops.json"));
        EconomyCommands economy = new EconomyCommands(env, jobs, shops);
        GuildCommands guilds = new GuildCommands(env, jobs, economy, essentials, dir.resolve("guild-homes.json"));
        economyCommands = economy;
        guildCommands = guilds;
        add(env, EssentialsCommands.build(env, essentials).stream().<CoreCommand>map(c -> new Cooled(c, essentials, cooldowns, env.clock)).toList(),
                "Essentials", () -> env.features.essentials());
        add(env, economy.build(), "Economy", () -> env.features.economy());
        add(env, new BoardCommands(env, jobs).build(), "Economy", () -> env.features.economy());
        add(env, guilds.build(), "Guilds", () -> env.features.guilds());
        utilityHub = new UtilityHub(env, () -> env.utilities.get(), new UtilityStore(dir.resolve("utility-state.json")));
        add(env, utilityHub.commands(), "Utility commands", () -> true);
    }

    private void add(Env env, List<CoreCommand> list, String system, java.util.function.BooleanSupplier enabled) {
        for (CoreCommand c : list) commands.add(new Gated(c, system, enabled));
    }

    public List<CoreCommand> all() { return List.copyOf(commands); }

    /** Run a command by name as this player, with the same checks as typing it. */
    public void run(String name, CorePlayer player, String... args) {
        for (CoreCommand c : commands) if (c.name().equals(name)) { c.run(player, args); return; }
        throw new IllegalArgumentException("no command " + name);
    }

    /** What this command set knows that the map can show. */
    public net.scopenet.core.map.MapSource mapSource() {
        return new net.scopenet.core.map.MapSource() {
            @Override public java.util.Map<String, Pos> warps() { return essentials.allWarps(); }
            @Override public java.util.Map<java.util.UUID, java.util.Map<String, Pos>> homes() { return essentials.allHomes(); }
            @Override public java.util.Map<String, Pos> guildHomes() { return guildCommands.guildHomes(); }
            @Override public java.util.Collection<ShopPoints.Shop> shops() { return shops.all(); }
        };
    }

    /** Guild homes by guild id (for the map). */
    public java.util.Map<String, Pos> guildHomes() { return guildCommands.guildHomes(); }

    /** Call about once a second from the server thread. */
    public void tick() { jobs.tick(); }

    /**
     * Teleport commands can have a cooldown ({@code essentials.cooldowns}). It starts when the command actually teleports (or sends a
     * request), so a typo doesn't cost the player anything. Players with {@code scopenet.cooldown.bypass} skip it.
     */
    private record Cooled(CoreCommand inner, EssentialsService essentials, Limits.Cooldowns cooldowns, java.util.function.LongSupplier clock) implements CoreCommand {
        @Override public String name() { return inner.name(); }
        @Override public List<String> aliases() { return inner.aliases(); }
        @Override public String permission() { return inner.permission(); }
        @Override public void run(CorePlayer p, String[] args) {
            int seconds = essentials.cooldownSeconds(inner.name());
            boolean cooled = seconds > 0 && !p.hasPermission("scopenet.cooldown.bypass");
            long now = clock.getAsLong();
            if (cooled) {
                long wait = cooldowns.remainingMs(p.uuid(), inner.name(), now);
                if (wait > 0) { p.send(Format.RED + "Please wait " + Limits.Cooldowns.waitText(wait) + " before using /" + inner.name() + " again."); return; }
            }
            long before = essentials.lastAction(p.uuid());
            inner.run(p, args);
            if (cooled && essentials.lastAction(p.uuid()) != before) cooldowns.start(p.uuid(), inner.name(), now, seconds);
        }
        @Override public List<String> complete(CorePlayer p, String[] args) { return inner.complete(p, args); }
    }

    /** Checks the feature switch first, then the permission node, so every platform behaves identically. */
    private record Gated(CoreCommand inner, String system, java.util.function.BooleanSupplier enabled) implements CoreCommand {
        @Override public String name() { return inner.name(); }
        @Override public List<String> aliases() { return inner.aliases(); }
        @Override public String permission() { return inner.permission(); }
        @Override public void run(CorePlayer p, String[] args) {
            if (!enabled.getAsBoolean()) { p.send(Format.RED + system + " are disabled."); return; }
            if (!inner.permission().isBlank() && !p.hasPermission(inner.permission())) { p.send(Format.RED + "You do not have permission to use /" + inner.name() + "."); return; }
            inner.run(p, args);
        }
        @Override public List<String> complete(CorePlayer p, String[] args) {
            if (!enabled.getAsBoolean() || (!permission().isBlank() && !p.hasPermission(permission()))) return List.of();
            return inner.complete(p, args);
        }
    }
}
