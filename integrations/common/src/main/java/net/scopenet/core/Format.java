package net.scopenet.core;

import java.util.Locale;

/** Text helpers shared by every command. Colour codes are legacy section signs. */
public final class Format {
    public static final String GOLD = "§6", YELLOW = "§e", GREEN = "§a", RED = "§c", GRAY = "§7",
            WHITE = "§f", AQUA = "§b", DARK_GRAY = "§8";
    private Format() {}

    public static String money(String symbol, double value) { return symbol + String.format(Locale.US, "%,.2f", value); }

    /** Positive amount with at most two decimals, or null. */
    public static Double amount(String text) {
        try {
            double value = Double.parseDouble(text.replace(",", ""));
            if (Double.isNaN(value) || Double.isInfinite(value) || value < 0.01) return null;
            return Math.round(value * 100.0) / 100.0;
        } catch (NumberFormatException e) {
            return null;
        }
    }

    public static String plain(String withColours) { return withColours.replaceAll("§.", ""); }

    public static String duration(long seconds) {
        return (seconds / 3600) + "h " + ((seconds % 3600) / 60) + "m " + (seconds % 60) + "s";
    }
}
