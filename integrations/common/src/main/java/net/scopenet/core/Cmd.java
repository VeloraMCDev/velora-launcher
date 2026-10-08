package net.scopenet.core;

import java.util.List;
import java.util.function.BiFunction;
import java.util.function.BiConsumer;

/** A {@link CoreCommand} built from lambdas. */
final class Cmd implements CoreCommand {
    private final String name;
    private final List<String> aliases;
    private final String permission;
    private final BiConsumer<CorePlayer, String[]> run;
    private final BiFunction<CorePlayer, String[], List<String>> complete;

    Cmd(String name, List<String> aliases, String permission, BiConsumer<CorePlayer, String[]> run,
        BiFunction<CorePlayer, String[], List<String>> complete) {
        this.name = name; this.aliases = aliases; this.permission = permission; this.run = run; this.complete = complete;
    }

    static Cmd of(String name, List<String> aliases, BiConsumer<CorePlayer, String[]> run) {
        return new Cmd(name, aliases, "scopenet.command." + name, run, (p, a) -> List.of());
    }

    Cmd completing(BiFunction<CorePlayer, String[], List<String>> c) { return new Cmd(name, aliases, permission, run, c); }

    @Override public String name() { return name; }
    @Override public List<String> aliases() { return aliases; }
    @Override public String permission() { return permission; }
    @Override public void run(CorePlayer player, String[] args) { run.accept(player, args); }
    @Override public List<String> complete(CorePlayer player, String[] args) { return complete.apply(player, args); }
}
