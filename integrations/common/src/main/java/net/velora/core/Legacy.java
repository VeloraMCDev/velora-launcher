package net.velora.core;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

/**
 * Carries files created under the pre-rename (SCOPENET) names over to the Velora Core names, so an existing server keeps its
 * configuration, homes, warps, shops and local vault backups. Nothing is ever overwritten, and the old config is copied (not moved)
 * so rolling back to an older jar still works.
 */
public final class Legacy {
    private Legacy() {}

    /** Copies config/scopenet.properties to {@code target} when {@code target} does not exist yet. True if it copied. */
    public static boolean copyServerConfig(Path target) throws IOException {
        Path old = target.resolveSibling("scopenet.properties");
        if (Files.exists(target) || !Files.isRegularFile(old)) return false;
        Files.copy(old, target, StandardCopyOption.COPY_ATTRIBUTES);
        System.err.println("[Velora] Copied " + old + " to " + target + ". The old file is left in place and is no longer read.");
        return true;
    }

    /** Moves config/scopenet (data directory) to config/velora-core when the new one does not exist yet. */
    public static void moveDataDirectory(Path configDir) {
        Path old = configDir.resolve("scopenet");
        Path next = configDir.resolve("velora-core");
        if (Files.exists(next) || !Files.isDirectory(old)) return;
        try {
            Files.move(old, next, StandardCopyOption.ATOMIC_MOVE);
            System.err.println("[Velora] Moved " + old + " to " + next + " (homes, warps, shops and vault backups carried over).");
        } catch (IOException e) {
            System.err.println("[Velora] Could not move " + old + " to " + next + ": " + e.getMessage() + ". Move it by hand before starting the server.");
            throw new IllegalStateException("Velora data directory migration failed", e);
        }
    }

    /** Copies a legacy sibling file (for example scopenet-client.json) to {@code target} when it does not exist yet. */
    public static void copyFile(Path target, String legacyName) {
        Path old = target.resolveSibling(legacyName);
        if (Files.exists(target) || !Files.isRegularFile(old)) return;
        try { Files.copy(old, target); } catch (IOException e) { System.err.println("[Velora] Could not copy " + old + ": " + e.getMessage()); }
    }
}
