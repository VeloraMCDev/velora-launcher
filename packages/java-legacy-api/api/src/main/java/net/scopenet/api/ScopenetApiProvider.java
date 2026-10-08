package net.scopenet.api;

/** Where other plugins find the API. Also registered with Bukkit's ServicesManager. */
public final class ScopenetApiProvider {
    private static volatile ScopenetApi api;

    private ScopenetApiProvider() {}

    /** @throws IllegalStateException if Velora isn't loaded (add it to your plugin's depend/softdepend) */
    public static ScopenetApi get() {
        ScopenetApi current = api;
        if (current == null) throw new IllegalStateException("Velora is not enabled");
        return current;
    }

    public static boolean isAvailable() { return api != null; }

    /** Called by Velora. */
    public static void register(ScopenetApi implementation) { api = implementation; }

    /** Called by Velora when it shuts down. */
    public static void unregister(ScopenetApi implementation) {
        if (api == implementation) api = null;
    }
}
