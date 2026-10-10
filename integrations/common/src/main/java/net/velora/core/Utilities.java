package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** The utility-command settings and custom items the admin set in the panel (received with every sync). */
public final class Utilities {
    public record Heal(boolean enabled, double amount, int cooldownSecs) {}
    public record Feed(boolean enabled, int amount, int cooldownSecs) {}
    public record Vault(boolean enabled, int count, int rows, int freeCount) {}

    /** One entry in a kit: either a literal item or a reference to a custom item. */
    public record KitItem(String custom, ItemSpec spec, int amount) {}

    public record Kit(String id, String name, String description, long cooldownSecs, boolean oneTime, List<String> groups,
                      List<KitItem> items, List<String> commands) {}

    public static final Utilities DEFAULT = new Utilities(new Heal(true, 0, 300), new Feed(true, 20, 120), true,
            new Vault(true, 3, 6, 1), true, List.of(), Map.of());

    public final Heal heal;
    public final Feed feed;
    public final boolean fly;
    public final Vault vault;
    public final boolean echest;
    public final List<Kit> kits;
    public final Map<String, ItemSpec> customItems;
    /** Placeable content from the Content Studio by id, and by the {@code material#modelData} of the item that places it. */
    public final Map<String, ContentDef> content;
    public final Map<String, ContentDef> contentByItem;
    /** Custom food: what eating the item identified by {@code material#modelData} restores. */
    public final Map<String, ContentDef.Food> foods;

    public Utilities(Heal heal, Feed feed, boolean fly, Vault vault, boolean echest, List<Kit> kits, Map<String, ItemSpec> customItems) {
        this(heal, feed, fly, vault, echest, kits, customItems, Map.of(), Map.of());
    }

    public Utilities(Heal heal, Feed feed, boolean fly, Vault vault, boolean echest, List<Kit> kits, Map<String, ItemSpec> customItems,
                     Map<String, ContentDef> content, Map<String, ContentDef.Food> foods) {
        this.heal = heal; this.feed = feed; this.fly = fly; this.vault = vault; this.echest = echest;
        this.kits = List.copyOf(kits); this.customItems = Map.copyOf(customItems);
        this.content = Map.copyOf(content);
        Map<String, ContentDef> byItem = new java.util.HashMap<>();
        for (ContentDef c : content.values()) {
            if (c.look().customModelData() >= 0) byItem.put(c.look().key(), c);
        }
        this.contentByItem = Map.copyOf(byItem);
        this.foods = Map.copyOf(foods);
    }

    public Kit kit(String id) {
        for (Kit k : kits) if (k.id().equalsIgnoreCase(id)) return k;
        return null;
    }

    /** Reads the {@code utilities} and {@code custom_items} parts of a sync response; anything missing keeps its default. */
    public static Utilities fromJson(JsonObject utilities, JsonArray customItems) { return fromJson(utilities, customItems, null); }

    public static Utilities fromJson(JsonObject utilities, JsonArray customItems, JsonArray contentArray) {
        Map<String, ItemSpec> custom = new ConcurrentHashMap<>();
        Map<String, ContentDef.Food> foods = new ConcurrentHashMap<>();
        if (customItems != null) {
            for (JsonElement e : customItems) {
                try {
                    JsonObject o = e.getAsJsonObject();
                    JsonObject spec = o.getAsJsonObject("spec");
                    ItemSpec item = ItemSpec.fromJson(spec);
                    custom.put(o.get("id").getAsString(), item);
                    if (spec.has("food") && spec.get("food").isJsonObject() && item.customModelData() >= 0) {
                        JsonObject f = spec.getAsJsonObject("food");
                        foods.put(ContentDef.keyOf(item.item(), item.customModelData()),
                                new ContentDef.Food(f.has("nutrition") ? f.get("nutrition").getAsInt() : 4, f.has("saturation") ? f.get("saturation").getAsDouble() : 2));
                    }
                } catch (RuntimeException ignored) { /* a malformed item is skipped, the rest still work */ }
            }
        }
        Map<String, ContentDef> content = ContentDef.listFrom(contentArray);
        if (utilities == null) return new Utilities(DEFAULT.heal, DEFAULT.feed, DEFAULT.fly, DEFAULT.vault, DEFAULT.echest, List.of(), custom, content, foods);
        JsonObject h = obj(utilities, "heal"), f = obj(utilities, "feed"), v = obj(utilities, "vault");
        List<Kit> kits = new ArrayList<>();
        if (utilities.has("kits") && utilities.get("kits").isJsonArray()) {
            for (JsonElement e : utilities.getAsJsonArray("kits")) {
                try { kits.add(kit(e.getAsJsonObject())); } catch (RuntimeException ignored) { /* skip a broken kit */ }
            }
        }
        return new Utilities(
                new Heal(bool(h, "enabled", true), num(h, "amount", 0), (int) num(h, "cooldown_secs", 300)),
                new Feed(bool(f, "enabled", true), (int) num(f, "amount", 20), (int) num(f, "cooldown_secs", 120)),
                bool(obj(utilities, "fly"), "enabled", true),
                new Vault(bool(v, "enabled", true), (int) num(v, "count", 3), (int) num(v, "rows", 6), (int) num(v, "free_count", 1)),
                bool(obj(utilities, "echest"), "enabled", true), kits, custom, content, foods);
    }

    private static Kit kit(JsonObject k) {
        List<String> groups = new ArrayList<>(), commands = new ArrayList<>();
        if (k.has("groups")) for (JsonElement g : k.getAsJsonArray("groups")) groups.add(g.getAsString().toLowerCase());
        if (k.has("commands")) for (JsonElement c : k.getAsJsonArray("commands")) commands.add(c.getAsString());
        List<KitItem> items = new ArrayList<>();
        if (k.has("items")) {
            for (JsonElement e : k.getAsJsonArray("items")) {
                JsonObject o = e.getAsJsonObject();
                int amount = o.has("amount") ? o.get("amount").getAsInt() : 1;
                if (o.has("custom")) items.add(new KitItem(o.get("custom").getAsString(), null, amount));
                else items.add(new KitItem(null, ItemSpec.fromJson(o), amount));
            }
        }
        return new Kit(k.get("id").getAsString(), k.has("name") ? k.get("name").getAsString() : k.get("id").getAsString(),
                k.has("description") ? k.get("description").getAsString() : "", (long) num(k, "cooldown_secs", 0),
                bool(k, "one_time", false), groups, items, commands);
    }

    private static JsonObject obj(JsonObject o, String key) { return o.has(key) && o.get(key).isJsonObject() ? o.getAsJsonObject(key) : new JsonObject(); }
    private static boolean bool(JsonObject o, String key, boolean d) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsBoolean() : d; }
    private static double num(JsonObject o, String key, double d) { return o.has(key) && o.get(key).isJsonPrimitive() ? o.get(key).getAsDouble() : d; }
}
