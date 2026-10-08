package net.scopenet.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * An item as an admin designs it in the panel: which item, how many, and the extras (name, lore, enchantments past vanilla
 * limits, attribute bonuses). Platforms turn it into a real stack. {@code &} colour codes are still raw here.
 */
public record ItemSpec(String item, int amount, String name, List<String> lore, Map<String, Integer> enchants, boolean unbreakable,
                       boolean glow, boolean hideFlags, int customModelData, List<Attribute> attributes, String dataFormat, String dataValue) {
    public ItemSpec(String item, int amount, String name, List<String> lore, Map<String, Integer> enchants, boolean unbreakable,
                    boolean glow, boolean hideFlags, int customModelData, List<Attribute> attributes) {
        this(item, amount, name, lore, enchants, unbreakable, glow, hideFlags, customModelData, attributes, "", "");
    }
    public record Attribute(String attribute, double amount, String operation, String slot) {}

    public ItemSpec {
        lore = List.copyOf(lore);
        enchants = Map.copyOf(enchants);
        attributes = List.copyOf(attributes);
    }

    /** A plain stack with no extras. */
    public static ItemSpec plain(String item, int amount) {
        return new ItemSpec(item, amount, "", List.of(), Map.of(), false, false, false, -1, List.of());
    }

    public boolean isPlain() {
        return name.isBlank() && lore.isEmpty() && enchants.isEmpty() && !unbreakable && !glow && !hideFlags && customModelData < 0 && attributes.isEmpty();
    }

    public ItemSpec withAmount(int n) {
        return new ItemSpec(item, n, name, lore, enchants, unbreakable, glow, hideFlags, customModelData, attributes, dataFormat, dataValue);
    }

    /** The registry id with its namespace, e.g. {@code minecraft:diamond_sword}. */
    public String registryId() { return item.contains(":") ? item : "minecraft:" + item; }

    public static ItemSpec fromJson(JsonObject o) {
        List<String> lore = new ArrayList<>();
        if (o.has("lore") && o.get("lore").isJsonArray()) for (JsonElement l : o.getAsJsonArray("lore")) lore.add(l.getAsString());
        Map<String, Integer> enchants = new LinkedHashMap<>();
        if (o.has("enchants") && o.get("enchants").isJsonObject()) {
            o.getAsJsonObject("enchants").entrySet().forEach(e -> enchants.put(e.getKey(), Math.max(1, Math.min(255, e.getValue().getAsInt()))));
        }
        List<Attribute> attrs = new ArrayList<>();
        if (o.has("attributes") && o.get("attributes").isJsonArray()) {
            for (JsonElement e : o.getAsJsonArray("attributes")) {
                JsonObject a = e.getAsJsonObject();
                attrs.add(new Attribute(a.get("attribute").getAsString(), a.get("amount").getAsDouble(),
                        a.has("operation") ? a.get("operation").getAsString() : "add", a.has("slot") ? a.get("slot").getAsString() : "mainhand"));
            }
        }
        return new ItemSpec(o.get("item").getAsString(), o.has("amount") ? Math.max(1, Math.min(64, o.get("amount").getAsInt())) : 1,
                o.has("name") ? o.get("name").getAsString() : "", lore, enchants,
                flag(o, "unbreakable"), flag(o, "glow"), flag(o, "hide_flags"),
                o.has("custom_model_data") ? o.get("custom_model_data").getAsInt() : -1, attrs,
                o.has("data") ? o.getAsJsonObject("data").get("format").getAsString() : "",
                o.has("data") ? o.getAsJsonObject("data").get("value").getAsString() : "");
    }

    private static boolean flag(JsonObject o, String key) { return o.has(key) && o.get(key).getAsBoolean(); }

    public static List<ItemSpec> listFrom(JsonArray array) {
        List<ItemSpec> out = new ArrayList<>();
        for (JsonElement e : array) out.add(fromJson(e.getAsJsonObject()));
        return out;
    }
}
