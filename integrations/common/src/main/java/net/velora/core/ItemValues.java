package net.velora.core;

import java.util.Locale;
import java.util.Map;
import java.util.TreeMap;

/** What the server shop pays per item (same table as the Paper plugin). Keys are panel item ids. */
public final class ItemValues {
    private static final Map<String, Double> VALUES = new TreeMap<>();
    static {
        Object[][] table = {
            {"COBBLESTONE", 0.1}, {"STONE", 0.2}, {"OAK_LOG", 0.5}, {"BIRCH_LOG", 0.5}, {"SPRUCE_LOG", 0.5}, {"COAL", 1.0},
            {"RAW_COPPER", 1.5}, {"COPPER_INGOT", 2.0}, {"RAW_IRON", 3.0}, {"IRON_INGOT", 4.0}, {"RAW_GOLD", 6.0},
            {"GOLD_INGOT", 8.0}, {"REDSTONE", 1.5}, {"LAPIS_LAZULI", 2.0}, {"DIAMOND", 35.0}, {"EMERALD", 20.0},
            {"NETHERITE_SCRAP", 150.0}, {"NETHERITE_INGOT", 650.0}, {"WHEAT", 0.5}, {"CARROT", 0.5}, {"POTATO", 0.5},
            {"BEEF", 1.0}, {"COOKED_BEEF", 2.0}, {"PORKCHOP", 1.0}, {"COOKED_PORKCHOP", 2.0},
        };
        for (Object[] row : table) VALUES.put((String) row[0], (Double) row[1]);
    }

    private ItemValues() {}

    public static Double unitPrice(String panelId) { return VALUES.get(panelId.toUpperCase(Locale.ROOT)); }

    public static Map<String, Double> all() { return java.util.Collections.unmodifiableMap(VALUES); }
}
