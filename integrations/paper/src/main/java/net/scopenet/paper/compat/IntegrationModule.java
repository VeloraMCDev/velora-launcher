package net.scopenet.paper.compat;

import java.util.Map;

/**
 * One optional connection to another plugin. The {@link CompatManager} finds out
 * which plugins are installed and switches the matching modules on and off, so
 * {@code ScopenetPlugin} never has to know about any of them.
 */
public interface IntegrationModule {
    /** Short id used in config ({@code integrations.<id>.enabled}) and in the panel. */
    String id();

    /** Human name, e.g. "LuckPerms". */
    String displayName();

    /** The plugin this integrates with, exactly as it calls itself in plugin.yml. */
    String pluginName();

    /** Start working. Called on the server thread once the other plugin is enabled. */
    void enable(CompatContext context) throws Exception;

    /** Stop and undo everything {@link #enable} did. Must be safe to call twice. */
    void disable();

    /** Extra facts for /scopenet status and the panel, e.g. how many placeholders were registered. */
    default Map<String, Object> details() {
        return Map.of();
    }
}
