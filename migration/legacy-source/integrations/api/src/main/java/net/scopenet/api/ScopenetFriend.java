package net.scopenet.api;

import java.util.UUID;

/** An accepted friend, and where they are playing right now (if anywhere). */
public record ScopenetFriend(UUID uuid, String name, boolean online, String playingOn) {}
