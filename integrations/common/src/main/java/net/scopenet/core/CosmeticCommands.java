package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Consumer;

/**
 * /cosmetic: see what you have unlocked (titles, badges, particles, pets, messages and modelled cosmetics), and put things on or take
 * them off. The panel holds the collection; the same choices are made in the launcher and the web panel.
 */
final class CosmeticCommands {
    private record Entry(String key, String type, String label, boolean equipped) {}

    private static final List<String> SUBS = List.of("list", "equip", "unequip");

    private final Env env;
    private final Consumer<CorePlayer> changed;
    /** The last list each player saw, so tab completion never waits on the network. */
    private final Map<UUID, List<Entry>> known = new ConcurrentHashMap<>();

    CosmeticCommands(Env env, Consumer<CorePlayer> changed) { this.env = env; this.changed = changed; }

    List<CoreCommand> build() {
        return List.of(new Cmd("cosmetic", List.of("cosmetics", "wardrobe"), "scopenet.command.cosmetic", this::run, this::complete));
    }

    private static String pretty(String type) { return type.replace('_', ' '); }

    private static JsonObject body(CorePlayer p) {
        JsonObject o = new JsonObject();
        o.addProperty("uuid", p.uuid().toString());
        return o;
    }

    private static String str(JsonObject o, String key) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsString() : ""; }

    private void run(CorePlayer p, String[] args) {
        String sub = args.length == 0 ? "list" : args[0].toLowerCase(Locale.ROOT);
        switch (sub) {
            case "equip", "wear", "use", "on" -> change(p, args, true);
            case "unequip", "remove", "off", "takeoff" -> change(p, args, false);
            case "list", "ls" -> list(p, args.length > 1 ? args[1] : "");
            default -> p.send(Format.RED + "Usage: /cosmetic [list [type]] | equip <name> | unequip <name|type|all>");
        }
    }

    /** Ask the panel; {@code then} gets the player's collection (also remembered for completion). */
    private void load(CorePlayer p, Consumer<List<Entry>> then) {
        env.io(() -> env.panel.call("cosmetics/list", body(p)).getAsJsonObject(), r -> {
            List<Entry> out = new ArrayList<>();
            JsonArray list = r.has("cosmetics") && r.get("cosmetics").isJsonArray() ? r.getAsJsonArray("cosmetics") : new JsonArray();
            for (JsonElement e : list) {
                JsonObject o = e.getAsJsonObject();
                out.add(new Entry(str(o, "key"), str(o, "type"), str(o, "label"), o.has("equipped") && o.get("equipped").getAsBoolean()));
            }
            known.put(p.uuid(), out);
            then.accept(out);
        }, e -> p.send(Format.RED + e));
    }

    private void list(CorePlayer p, String type) {
        p.send(Format.GRAY + "Loading your collection...");
        load(p, all -> {
            String wanted = type.toLowerCase(Locale.ROOT).replace(' ', '_');
            List<Entry> shown = all.stream().filter(e -> wanted.isEmpty() || e.type().equals(wanted) || e.type().equals(wanted.replaceAll("s$", ""))).toList();
            p.send(Format.GOLD + "=== Your cosmetics ===");
            if (shown.isEmpty()) {
                p.send(Format.GRAY + (all.isEmpty() ? "You haven't unlocked any yet. Earn them from quests, achievements and levels." : "Nothing of that type. Types: " + Format.YELLOW + types(all)));
                return;
            }
            String last = "";
            for (Entry e : shown) {
                if (!e.type().equals(last)) { p.send(Format.AQUA + capitalise(pretty(e.type()))); last = e.type(); }
                p.send("  " + (e.equipped() ? Format.GREEN + "★ " : Format.DARK_GRAY + "☆ ") + Format.YELLOW + e.label() + Format.DARK_GRAY + " (" + e.key() + ")" + (e.equipped() ? Format.GREEN + " worn" : ""));
            }
            p.send(Format.GRAY + "Wear one with " + Format.YELLOW + "/cosmetic equip <name>" + Format.GRAY + ", take it off with " + Format.YELLOW + "/cosmetic unequip <name>");
        });
    }

    private static String types(List<Entry> all) { return String.join(", ", all.stream().map(e -> e.type()).distinct().map(CosmeticCommands::pretty).toList()); }

    private static String capitalise(String s) { return s.isEmpty() ? s : Character.toUpperCase(s.charAt(0)) + s.substring(1); }

    private void change(CorePlayer p, String[] args, boolean equip) {
        if (args.length < 2) { p.send(Format.RED + "Usage: /cosmetic " + (equip ? "equip <name>" : "unequip <name|type|all>")); return; }
        String wanted = String.join(" ", java.util.Arrays.copyOfRange(args, 1, args.length)).trim();
        load(p, all -> {
            List<Entry> targets = equip ? match(all, wanted) : unequipTargets(all, wanted);
            if (targets.isEmpty()) { p.send(Format.RED + "You don't have a cosmetic called \"" + wanted + "\". Try " + Format.YELLOW + "/cosmetic list"); return; }
            if (targets.size() > 1 && equip) {
                p.send(Format.RED + "That could be " + String.join(", ", targets.stream().map(Entry::label).toList()) + ". Be more specific.");
                return;
            }
            apply(p, targets, 0, equip);
        });
    }

    /** One at a time, so the panel's "one per type" rule and any error show up in order. */
    private void apply(CorePlayer p, List<Entry> targets, int i, boolean equip) {
        if (i >= targets.size()) { changed.accept(p); return; }
        Entry e = targets.get(i);
        JsonObject b = body(p);
        b.addProperty("key", e.key());
        b.addProperty("equipped", equip);
        env.io(() -> env.panel.call("cosmetics/equip", b), r -> {
            p.send(equip ? Format.GREEN + "You put on " + Format.YELLOW + e.label() + Format.GREEN + "." : Format.GRAY + "You took off " + Format.YELLOW + e.label() + Format.GRAY + ".");
            apply(p, targets, i + 1, equip);
        }, err -> p.send(Format.RED + err));
    }

    /** An exact key or name, else a name that starts with (or contains) what was typed. */
    private static List<Entry> match(List<Entry> all, String wanted) {
        String w = wanted.toLowerCase(Locale.ROOT);
        List<Entry> exact = all.stream().filter(e -> e.key().equalsIgnoreCase(w) || e.label().equalsIgnoreCase(w)).toList();
        if (!exact.isEmpty()) return exact;
        List<Entry> starts = all.stream().filter(e -> e.label().toLowerCase(Locale.ROOT).startsWith(w) || e.key().toLowerCase(Locale.ROOT).startsWith(w)).toList();
        if (!starts.isEmpty()) return starts;
        return all.stream().filter(e -> e.label().toLowerCase(Locale.ROOT).contains(w)).toList();
    }

    private static List<Entry> unequipTargets(List<Entry> all, String wanted) {
        String w = wanted.toLowerCase(Locale.ROOT).replace(' ', '_');
        if (w.equals("all") || w.equals("everything")) return all.stream().filter(Entry::equipped).toList();
        List<Entry> byType = all.stream().filter(e -> e.equipped() && (e.type().equals(w) || e.type().equals(w.replaceAll("s$", "")))).toList();
        if (!byType.isEmpty()) return byType;
        return match(all, wanted).stream().filter(Entry::equipped).limit(1).toList();
    }

    private List<String> complete(CorePlayer p, String[] args) {
        if (args.length <= 1) return SUBS;
        List<Entry> mine = known.get(p.uuid());
        if (mine == null) {
            // First time: fetch quietly so the next Tab has names.
            load(p, all -> { });
            return List.of();
        }
        String sub = args[0].toLowerCase(Locale.ROOT);
        if (sub.equals("list")) return mine.stream().map(Entry::type).distinct().toList();
        if (sub.equals("equip")) return mine.stream().filter(e -> !e.equipped()).map(Entry::key).toList();
        if (sub.equals("unequip")) {
            List<String> out = new ArrayList<>(List.of("all"));
            mine.stream().filter(Entry::equipped).forEach(e -> out.add(e.key()));
            return out;
        }
        return List.of();
    }
}
