package net.velora.core;

import com.google.gson.Gson;
import com.google.gson.JsonObject;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.HashMap;
import java.util.Map;
import java.util.UUID;

/** When each player last used a cooldown-limited command or claimed a kit; survives restarts. */
public final class UtilityStore {
    private final Path file;
    private final Map<String, Long> times = new HashMap<>();

    public UtilityStore(Path file) {
        this.file = file;
        try {
            if (Files.exists(file)) {
                JsonObject o = new Gson().fromJson(Files.readString(file, StandardCharsets.UTF_8), JsonObject.class);
                if (o != null) o.entrySet().forEach(e -> times.put(e.getKey(), e.getValue().getAsLong()));
            }
        } catch (IOException | RuntimeException ignored) { /* start fresh if the file is unreadable */ }
    }

    private static String key(UUID player, String what) { return player + "|" + what; }

    /** Epoch millis of the last use, or -1 if never. */
    public synchronized long last(UUID player, String what) { return times.getOrDefault(key(player, what), -1L); }

    /** Milliseconds still to wait, 0 if ready. */
    public synchronized long remainingMs(UUID player, String what, long cooldownMs, long now) {
        long last = last(player, what);
        return last < 0 || cooldownMs <= 0 ? 0 : Math.max(0, last + cooldownMs - now);
    }

    public synchronized void stamp(UUID player, String what, long now) {
        times.put(key(player, what), now);
        save();
    }

    public synchronized void clear(UUID player, String what) {
        if (times.remove(key(player, what)) != null) save();
    }

    private void save() {
        try {
            Files.createDirectories(file.toAbsolutePath().getParent());
            JsonObject o = new JsonObject();
            times.forEach(o::addProperty);
            Path tmp = file.resolveSibling(file.getFileName() + ".tmp");
            Files.writeString(tmp, o.toString(), StandardCharsets.UTF_8);
            Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException ignored) { /* a lost cooldown stamp only means a free reuse */ }
    }
}
