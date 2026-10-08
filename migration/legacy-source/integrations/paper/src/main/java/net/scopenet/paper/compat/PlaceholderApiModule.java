package net.scopenet.paper.compat;

import java.util.Map;

/** Registers the {@code %scopenet_…%} placeholders with PlaceholderAPI. */
public final class PlaceholderApiModule implements IntegrationModule {
    private ScopenetExpansion expansion;

    @Override public String id() { return "placeholderapi"; }
    @Override public String displayName() { return "PlaceholderAPI"; }
    @Override public String pluginName() { return "PlaceholderAPI"; }

    @Override public void enable(CompatContext context) {
        String symbol = context.config().getString("economy.currency-symbol", "$");
        expansion = new ScopenetExpansion(context.api(), symbol, context.plugin().getDescription().getVersion());
        if (!expansion.register()) throw new IllegalStateException("PlaceholderAPI refused the expansion");
    }

    @Override public void disable() {
        if (expansion != null) {
            expansion.unregister();
            expansion = null;
        }
    }

    @Override public Map<String, Object> details() {
        return Map.of("placeholders", ScopenetExpansion.PLACEHOLDERS.size());
    }
}
