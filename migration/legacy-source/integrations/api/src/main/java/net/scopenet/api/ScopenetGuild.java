package net.scopenet.api;

import java.util.List;
import java.util.UUID;

/** A guild and its members. Balances and claim counts are for the server the plugin runs on. */
public record ScopenetGuild(String id, String name, String tag, String description, UUID leader, int claims, double balance, List<Member> members) {
    public record Member(UUID uuid, String name, String role) {}

    public boolean isMember(UUID player) {
        return members.stream().anyMatch(m -> m.uuid().equals(player));
    }
}
