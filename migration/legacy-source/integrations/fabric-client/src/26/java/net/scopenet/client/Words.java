package net.scopenet.client;

import java.util.Locale;

/** Display text: every label a player reads goes through here so capitalisation is the same everywhere. */
final class Words {
    private Words() {}

    /** "ender_chest" -> "Ender Chest", "guild_home" -> "Guild Home", "pending_incoming" -> "Pending Incoming". */
    static String title(String s) {
        if (s == null || s.isBlank()) return "";
        StringBuilder out = new StringBuilder();
        boolean start = true;
        for (char c : s.trim().replace('_', ' ').replace('-', ' ').toCharArray()) {
            if (c == ' ') { out.append(' '); start = true; continue; }
            out.append(start ? Character.toUpperCase(c) : c);
            start = false;
        }
        return out.toString().replaceAll(" {2,}", " ");
    }

    /** Strips colour codes ("&6", "§a") from server-authored text. */
    static String plain(String s) { return s.replaceAll("(?i)[&§][0-9a-fk-or]", ""); }

    /** First letter up, the rest untouched: "claim this chunk" -> "Claim this chunk". */
    static String sentence(String s) {
        if (s == null || s.isBlank()) return "";
        return Character.toUpperCase(s.charAt(0)) + s.substring(1);
    }

    /** "minecraft:the_nether", "world_nether", "DIAMOND_PICKAXE" -> readable names. */
    static String world(String id) {
        String s = id == null ? "" : id.toLowerCase(Locale.ROOT);
        if (s.startsWith("minecraft:")) s = s.substring(10);
        return switch (s) {
            case "", "world", "overworld" -> "Overworld";
            case "world_nether", "the_nether", "nether" -> "The Nether";
            case "world_the_end", "the_end", "end" -> "The End";
            default -> title(s.replace(':', ' '));
        };
    }

    /** "DIAMOND_PICKAXE" or "minecraft:diamond_pickaxe" -> "Diamond Pickaxe". */
    static String item(String id) {
        String s = id == null ? "" : id;
        int colon = s.indexOf(':');
        if (colon >= 0) s = s.substring(colon + 1);
        return title(s.toLowerCase(Locale.ROOT));
    }

    /** 90 -> "1m 30s", 7300 -> "2h 1m", 0 -> "now". */
    static String duration(long seconds) {
        if (seconds <= 0) return "Now";
        long d = seconds / 86400, h = seconds % 86400 / 3600, m = seconds % 3600 / 60, s = seconds % 60;
        if (d > 0) return d + "d " + h + "h";
        if (h > 0) return h + "h " + m + "m";
        if (m > 0) return m + "m " + (seconds < 600 ? s + "s" : "").trim();
        return s + "s";
    }

    /** Seconds from now until an ISO-8601 instant, or -1 when it cannot be read. */
    static long secondsUntil(String iso, long nowMillis) {
        try { return Math.max(0, (java.time.Instant.parse(iso).toEpochMilli() - nowMillis) / 1000); }
        catch (Exception e) { return -1; }
    }

    static String number(double n) {
        return n == Math.rint(n) ? String.format(Locale.ROOT, "%,d", (long) n) : String.format(Locale.ROOT, "%,.2f", n);
    }
}
