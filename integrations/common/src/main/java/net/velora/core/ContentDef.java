package net.velora.core;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Random;

/**
 * One piece of placeable content from the panel's Content Studio: a block, chest, decoration, NPC, vehicle, crop or mob. Pure data
 * plus the small rules that do not need Bukkit (crop growth, drop rolls), so they can be unit tested; platforms draw and place it.
 */
public record ContentDef(String id, String kind, String title, String name, List<String> lore, Look look, List<Look> stages,
                         double hardness, int light, List<Drop> drops, int rows, String chestTitle, boolean seat, boolean solid,
                         double hitWidth, double hitHeight, boolean nameVisible, List<String> commands, List<String> messages,
                         String mount, double speed, int growthSeconds, boolean replant,
                         String entity, double health, double damage, double armor) {
    /** How it looks: the base item, the model-data number the resource pack re-skins it with, and the size multiplier. */
    public record Look(String item, int customModelData, double scale) {
        public String key() { return keyOf(item, customModelData); }
    }

    /** {@code item} is a custom item/content id or a vanilla material name; {@code chance} runs 0..1. */
    public record Drop(String item, int min, int max, double chance) {}

    /** What eating a custom food does on top of the base item. */
    public record Food(int nutrition, double saturation) {}

    public static final List<String> KINDS = List.of("block", "chest", "decoration", "npc", "vehicle", "crop", "mob");

    public ContentDef {
        lore = List.copyOf(lore);
        stages = List.copyOf(stages);
        drops = List.copyOf(drops);
        commands = List.copyOf(commands);
        messages = List.copyOf(messages);
    }

    /** {@code material#modelData}, the identity of a re-skinned item. */
    public static String keyOf(String material, int customModelData) {
        String m = material.toLowerCase(Locale.ROOT);
        return (m.contains(":") ? m : "minecraft:" + m) + "#" + customModelData;
    }

    /** The item players hold to place this. */
    public ItemSpec toItemSpec() {
        String shown = name == null || name.isBlank() ? title : name;
        return new ItemSpec(look.item(), 1, shown, lore, java.util.Map.of(), false, false, false, look.customModelData(), List.of());
    }

    public Look stageLook(int stage) {
        return stages.isEmpty() ? look : stages.get(Math.max(0, Math.min(stage, stages.size() - 1)));
    }

    public int lastStage() { return Math.max(0, stages.size() - 1); }

    public boolean is(String k) { return kind.equals(k); }

    // ---- rules -------------------------------------------------------------------------------------------

    /** Which growth stage a crop planted at {@code plantedAtMs} is in: evenly spaced over {@code growthSeconds}, ending on the last. */
    public static int cropStage(long plantedAtMs, long nowMs, int growthSeconds, int stageCount) {
        if (stageCount <= 1) return 0;
        long total = Math.max(1, growthSeconds) * 1000L;
        long elapsed = Math.max(0, nowMs - plantedAtMs);
        if (elapsed >= total) return stageCount - 1;
        return (int) Math.min(stageCount - 1, elapsed * (stageCount - 1) / total);
    }

    /** Milliseconds until the next stage begins (0 when fully grown). */
    public static long millisToNextStage(long plantedAtMs, long nowMs, int growthSeconds, int stageCount) {
        if (stageCount <= 1) return 0;
        long total = Math.max(1, growthSeconds) * 1000L;
        long elapsed = Math.max(0, nowMs - plantedAtMs);
        if (elapsed >= total) return 0;
        int current = cropStage(plantedAtMs, nowMs, growthSeconds, stageCount);
        long nextStart = (long) Math.ceil((current + 1) * (double) total / (stageCount - 1));
        return Math.max(0, nextStart - elapsed);
    }

    public record Rolled(String item, int amount) {}

    /** Rolls every drop: skipped by its chance, otherwise a random amount between min and max (0 amounts are dropped). */
    public static List<Rolled> rollDrops(List<Drop> drops, Random random) {
        List<Rolled> out = new ArrayList<>();
        for (Drop d : drops) {
            if (d.chance() < 1.0 && random.nextDouble() >= d.chance()) continue;
            int amount = d.max() <= d.min() ? d.min() : d.min() + random.nextInt(d.max() - d.min() + 1);
            if (amount > 0) out.add(new Rolled(d.item(), amount));
        }
        return out;
    }

    /** Punches a block needs before it breaks: instant for 0, then roughly one per 0.6 s of mining time, at most 20. */
    public int punchesToBreak() { return hardness <= 0 ? 1 : (int) Math.max(1, Math.min(20, Math.round(hardness / 0.6))); }

    // ---- JSON --------------------------------------------------------------------------------------------

    public static ContentDef fromJson(JsonObject o) {
        JsonObject s = obj(o, "spec");
        JsonObject d = obj(s, "display");
        Look look = look(d, new Look("minecraft:paper", -1, 1.0));
        List<Look> stages = new ArrayList<>();
        if (s.has("stages") && s.get("stages").isJsonArray()) for (JsonElement e : s.getAsJsonArray("stages")) stages.add(look(e.getAsJsonObject(), look));
        JsonObject hit = obj(s, "hitbox");
        return new ContentDef(
                str(o, "id", ""), str(o, "kind", ""), str(o, "title", str(o, "id", "")), str(s, "name", ""), strings(s, "lore"), look, stages,
                num(s, "hardness", 1.5), (int) num(s, "light", 0), drops(s), (int) Math.max(1, Math.min(6, num(s, "rows", 3))), str(s, "title", ""),
                bool(s, "seat", false), bool(s, "solid", false), num(hit, "width", 1.0), num(hit, "height", 1.0), bool(s, "name_visible", true),
                strings(s, "commands"), strings(s, "messages"), str(s, "mount", "horse"), num(s, "speed", 0.25),
                (int) num(s, "growth_seconds", 600), bool(s, "replant", true),
                str(s, "entity", "ZOMBIE"), num(s, "health", 20), num(s, "damage", 2), num(s, "armor", 0));
    }

    private static Look look(JsonObject d, Look fallback) {
        return new Look(str(d, "item", fallback.item()), d.has("custom_model_data") && d.get("custom_model_data").isJsonPrimitive() ? d.get("custom_model_data").getAsInt() : fallback.customModelData(),
                num(d, "scale", fallback.scale()));
    }

    private static List<Drop> drops(JsonObject s) {
        List<Drop> out = new ArrayList<>();
        if (s.has("drops") && s.get("drops").isJsonArray()) {
            for (JsonElement e : s.getAsJsonArray("drops")) {
                try {
                    JsonObject d = e.getAsJsonObject();
                    int min = (int) num(d, "min", 1);
                    out.add(new Drop(str(d, "item", "").toLowerCase(Locale.ROOT), min, (int) Math.max(min, num(d, "max", min)), num(d, "chance", 1.0)));
                } catch (RuntimeException ignored) { /* skip a broken drop */ }
            }
        }
        return out;
    }

    public static java.util.Map<String, ContentDef> listFrom(JsonArray array) {
        java.util.Map<String, ContentDef> out = new java.util.LinkedHashMap<>();
        if (array == null) return out;
        for (JsonElement e : array) {
            try {
                ContentDef c = fromJson(e.getAsJsonObject());
                if (!c.id().isBlank() && KINDS.contains(c.kind())) out.put(c.id(), c);
            } catch (RuntimeException ignored) { /* a malformed entry is skipped, the rest still work */ }
        }
        return out;
    }

    private static JsonObject obj(JsonObject o, String k) { return o.has(k) && o.get(k).isJsonObject() ? o.getAsJsonObject(k) : new JsonObject(); }
    private static String str(JsonObject o, String k, String d) { return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsString() : d; }
    private static double num(JsonObject o, String k, double d) { return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsDouble() : d; }
    private static boolean bool(JsonObject o, String k, boolean d) { return o.has(k) && o.get(k).isJsonPrimitive() ? o.get(k).getAsBoolean() : d; }
    private static List<String> strings(JsonObject o, String k) {
        List<String> out = new ArrayList<>();
        if (o.has(k) && o.get(k).isJsonArray()) for (JsonElement e : o.getAsJsonArray(k)) if (e.isJsonPrimitive()) out.add(e.getAsString());
        return out;
    }
}
