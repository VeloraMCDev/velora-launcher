package net.velora.core;

import java.util.List;

/** One slash command, the same on every platform. */
public interface CoreCommand {
    String name();
    default List<String> aliases() { return List.of(); }
    /** Node checked before {@link #run}; platforms apply LuckPerms or their default. */
    String permission();
    void run(CorePlayer player, String[] args);
    /** Tab-completion candidates for the last argument. */
    default List<String> complete(CorePlayer player, String[] args) { return List.of(); }
}
