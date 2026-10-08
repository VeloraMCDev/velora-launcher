package net.scopenet.worldmap;

import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

import static net.scopenet.worldmap.MapColors.*;

/**
 * Which map colour a block shows as. Vanilla blocks follow the real table; anything else (modded blocks) is matched by
 * name keywords, then falls back to a stable neutral colour so the same block always looks the same.
 */
public final class BlockColors {
    private BlockColors() {}

    private static final Map<String, Integer> CACHE = new ConcurrentHashMap<>();

    private static final Set<String> CLEAR = Set.of("air", "cave_air", "void_air", "barrier", "light", "structure_void", "glass", "glass_pane",
            "tinted_glass", "ladder", "lever", "tripwire", "tripwire_hook", "redstone_wire", "comparator", "repeater", "flower_pot",
            "moving_piston", "cobweb", "end_rod", "scaffolding", "vine_air");
    private static final String[] CLEAR_PARTS = {"torch", "rail", "button", "pressure_plate", "potted_", "banner", "head", "skull"};
    private static final Set<String> WATERY = Set.of("water", "bubble_column", "kelp", "kelp_plant", "seagrass", "tall_seagrass");

    private static final String[] DYES = {"light_blue", "light_gray", "white", "orange", "magenta", "yellow", "lime", "pink", "gray", "cyan",
            "purple", "blue", "brown", "green", "red", "black"};
    private static final int[] DYE_WOOL = {LIGHT_BLUE, LIGHT_GRAY, SNOW, ORANGE, MAGENTA, YELLOW, LIGHT_GREEN, PINK, GRAY, CYAN, PURPLE, BLUE,
            BROWN, GREEN, RED, BLACK};
    private static final int[] DYE_TERRACOTTA = {39, 44, 36, 37, 38, 40, 41, 42, 43, 45, 46, 47, 48, 49, 50, 51};
    private static final String[] DYED = {"wool", "carpet", "concrete", "stained_glass", "glazed_terracotta", "shulker_box", "bed"};

    private static final String[] STRIP_PREFIX = {"waxed_", "polished_", "chiseled_", "cut_", "smooth_", "cracked_", "mossy_", "stripped_",
            "infested_", "weathered_cut_", "budding_", "reinforced_", "large_", "small_", "tall_"};
    private static final String[] STRIP_SUFFIX = {"_stairs", "_slab", "_wall", "_fence_gate", "_fence", "_trapdoor", "_door", "_sign",
            "_wall_sign", "_hanging_sign", "_wall_hanging_sign", "_bricks", "_brick", "_tiles", "_tile", "_block", "_pillar"};

    private static final int[] UNKNOWN = {STONE, DIRT, WOOD, GRAY, BROWN, LIGHT_GRAY, DEEPSLATE};

    /** Water is special-cased by the renderer, which shades it by depth. */
    public static final int WATER_COLOR = 0xFF3F76E4;

    private static final Map<String, Integer> TEXTURES = loadTextures();
    private static final Map<String, Integer> COLORS = new ConcurrentHashMap<>();

    /**
     * The colour this block shows as on the map (opaque ARGB), or 0 for blocks a map looks straight through (air, glass, rails).
     * Vanilla blocks use the average colour of their real top texture; anything without one falls back to the map colour table.
     */
    public static int of(String blockName) {
        return COLORS.computeIfAbsent(blockName, BlockColors::resolve);
    }

    /** The nearest vanilla map colour id for a block (0 = see-through); also what the fallback palette uses. */
    public static int mapColor(String blockName) {
        return CACHE.computeIfAbsent(blockName, BlockColors::classify);
    }

    private static int resolve(String full) {
        int id = mapColor(full);
        if (id == 0) return 0;
        if (id == WATER) return WATER_COLOR;
        String n = plain(full);
        Integer exact = TEXTURES.get(n);
        if (exact != null) return exact;
        // Short grass, ferns, vines and the like have no texture colour of their own: start from the matching tinted block.
        int tint = tintOf(full);
        if (tint != TINT_NONE) {
            Integer base = TEXTURES.get(tint == TINT_GRASS ? "grass_block" : "oak_leaves");
            if (base != null) return base;
        }
        String s = n;
        for (boolean changed = true; changed; ) {
            changed = false;
            for (String p : STRIP_PREFIX) if (s.startsWith(p) && s.length() > p.length()) { s = s.substring(p.length()); changed = true; }
            for (String p : STRIP_SUFFIX) if (s.endsWith(p) && s.length() > p.length()) { s = s.substring(0, s.length() - p.length()); changed = true; }
            if (!changed) break;
            for (String candidate : new String[]{s, s + "s", s + "_planks", s + "_block"}) {
                Integer hit = TEXTURES.get(candidate);
                if (hit != null) return hit;
            }
        }
        return MapColors.base(id);
    }

    private static String plain(String full) {
        return full.startsWith("minecraft:") ? full.substring(10) : full.substring(full.indexOf(':') + 1);
    }

    private static Map<String, Integer> loadTextures() {
        Map<String, Integer> out = new java.util.HashMap<>();
        try (java.io.InputStream in = BlockColors.class.getResourceAsStream("block_colors.csv")) {
            if (in == null) return out;
            for (String line : new String(in.readAllBytes(), java.nio.charset.StandardCharsets.UTF_8).split("\n")) {
                int comma = line.indexOf(',');
                if (comma <= 0 || comma + 7 > line.length()) continue;
                try {
                    out.put(line.substring(0, comma), 0xFF000000 | Integer.parseInt(line.substring(comma + 1, comma + 7).trim(), 16));
                } catch (NumberFormatException ignored) {
                    // a malformed row just falls back to the map colour table
                }
            }
        } catch (java.io.IOException ignored) {
            // no texture table: the map colour table covers everything
        }
        return out;
    }

    private static int classify(String full) {
        String n = plain(full);
        int c = lookup(n);
        if (c >= 0) return c;
        // Peel shape and finish words (cut_sandstone_stairs -> sandstone) and try again.
        String s = n;
        for (boolean changed = true; changed; ) {
            changed = false;
            for (String p : STRIP_PREFIX) if (s.startsWith(p) && s.length() > p.length()) { s = s.substring(p.length()); changed = true; }
            for (String p : STRIP_SUFFIX) if (s.endsWith(p) && s.length() > p.length()) { s = s.substring(0, s.length() - p.length()); changed = true; }
            if (changed && (c = lookup(s)) >= 0) return c;
        }
        return UNKNOWN[Math.floorMod(n.hashCode(), UNKNOWN.length)];
    }

    /** The colour for a plain name, or -1 when nothing matches. */
    private static int lookup(String n) {
        if (CLEAR.contains(n)) return NONE;
        for (String p : CLEAR_PARTS) if (n.contains(p)) return NONE;
        if (WATERY.contains(n)) return WATER;
        if (n.equals("lava")) return FIRE;
        for (int i = 0; i < DYES.length; i++) {
            String d = DYES[i] + "_";
            if (!n.startsWith(d)) continue;
            String rest = n.substring(d.length());
            if (rest.startsWith("terracotta")) return DYE_TERRACOTTA[i];
            for (String k : DYED) if (rest.startsWith(k)) return DYE_WOOL[i];
        }
        if (n.equals("terracotta")) return ORANGE;
        if (n.equals("grass_block") || n.equals("slime_block")) return GRASS;
        if (n.startsWith("crimson_")) return n.contains("nylium") ? CRIMSON_NYLIUM : n.contains("hyphae") ? CRIMSON_HYPHAE
                : n.contains("stem") || n.contains("planks") ? CRIMSON_STEM : n.contains("fungus") || n.contains("roots") ? CRIMSON_NYLIUM : -1;
        if (n.startsWith("warped_")) return n.contains("nylium") ? WARPED_NYLIUM : n.contains("hyphae") ? WARPED_HYPHAE
                : n.contains("wart") ? WARPED_WART : n.contains("stem") || n.contains("planks") ? WARPED_STEM : n.contains("fungus") || n.contains("roots") ? WARPED_NYLIUM : -1;
        if (n.contains("deepslate")) return DEEPSLATE;
        if (n.contains("tuff")) return TERRACOTTA_GRAY;
        if (n.contains("calcite")) return TERRACOTTA_WHITE;
        if (n.contains("dripstone")) return TERRACOTTA_BROWN;
        if (n.contains("blackstone") || n.contains("basalt") || n.contains("obsidian") || n.equals("coal_block")) return BLACK;
        if (n.contains("netherrack") || n.contains("nether_brick") || n.contains("nether_gold") || n.contains("nether_quartz") || n.contains("magma")) return NETHER;
        if (n.equals("nether_wart_block") || n.equals("shroomlight")) return RED;
        if (n.contains("soul_sand") || n.contains("soul_soil") || n.equals("ancient_debris")) return BROWN;
        if (n.equals("glowstone")) return SAND;
        if (n.equals("bedrock")) return STONE;
        if (n.contains("red_sand")) return ORANGE;
        if (n.contains("end_stone")) return SAND;
        if (n.contains("sandstone") || n.contains("sand") || n.equals("bone_block")) return SAND;
        if (n.contains("gravel")) return STONE;
        if (n.contains("podzol")) return PODZOL;
        if (n.contains("mycelium")) return PURPLE;
        if (n.equals("mud") || n.equals("muddy_mangrove_roots")) return TERRACOTTA_CYAN;
        if (n.contains("dirt") || n.contains("farmland") || n.contains("granite") || n.equals("packed_mud")) return DIRT;
        if (n.equals("clay")) return CLAY;
        if (n.contains("snow")) return SNOW;
        if (n.equals("ice") || n.endsWith("_ice")) return ICE;
        if (n.contains("copper")) return n.startsWith("exposed") ? TERRACOTTA_LIGHT_GRAY : n.startsWith("weathered") ? WARPED_STEM
                : n.startsWith("oxidized") ? WARPED_NYLIUM : ORANGE;
        if (n.equals("raw_iron_block")) return RAW_IRON;
        if (n.equals("raw_gold_block") || n.equals("gold_block")) return GOLD;
        if (n.equals("raw_copper_block")) return ORANGE;
        if (n.equals("diamond_block")) return DIAMOND;
        if (n.equals("emerald_block")) return EMERALD;
        if (n.equals("lapis_block")) return LAPIS;
        if (n.equals("redstone_block") || n.equals("tnt") || n.equals("fire")) return FIRE;
        if (n.contains("amethyst")) return PURPLE;
        if (n.contains("quartz") || n.contains("diorite") || n.equals("sea_lantern")) return QUARTZ;
        if (n.contains("andesite")) return STONE;
        if (n.contains("purpur") || n.equals("chorus_plant") || n.equals("chorus_flower")) return n.contains("chorus") ? PURPLE : MAGENTA;
        if (n.equals("dark_prismarine") || n.contains("prismarine_brick")) return DIAMOND;
        if (n.contains("prismarine")) return CYAN;
        if (n.contains("_ore")) return STONE;
        if (n.contains("iron") || n.equals("chain") || n.equals("lantern") || n.equals("soul_lantern") || n.equals("anvil") || n.contains("cauldron")
                || n.equals("hopper") || n.equals("bell") || n.equals("grindstone")) return METAL;
        if (n.contains("stone") || n.contains("furnace") || n.contains("dispenser") || n.contains("dropper") || n.contains("observer")
                || n.contains("piston") || n.contains("smoker") || n.equals("spawner") || n.equals("lodestone") || n.contains("brick")) return STONE;
        if (n.equals("hay_block")) return YELLOW;
        if (n.contains("bamboo_block") || n.contains("bamboo_planks") || n.contains("bamboo_mosaic") || n.equals("sponge") || n.equals("wet_sponge")) return YELLOW;
        if (n.contains("melon")) return LIGHT_GREEN;
        if (n.contains("pumpkin") || n.equals("honey_block") || n.contains("honeycomb")) return ORANGE;
        if (n.equals("glow_lichen")) return GLOW_LICHEN;
        if (n.equals("moss_block") || n.equals("moss_carpet")) return GREEN;
        if (n.contains("red_mushroom")) return RED;
        if (n.contains("brown_mushroom")) return DIRT;
        if (n.equals("mushroom_stem")) return WOOL;
        if (n.equals("sculk") || n.startsWith("sculk_") || n.equals("dragon_egg")) return BLACK;
        if (n.equals("beacon")) return DIAMOND;
        if (n.contains("cherry_leaves")) return PINK;
        if (n.contains("leaves") || n.contains("cactus") || n.equals("sugar_cane") || n.contains("vine") || n.equals("lily_pad") || n.contains("azalea")
                || n.contains("bush") || n.contains("fern") || n.contains("grass") || n.contains("sapling") || n.contains("flower") || n.contains("tulip")
                || n.contains("poppy") || n.contains("dandelion") || n.contains("orchid") || n.equals("allium") || n.equals("azure_bluet")
                || n.contains("daisy") || n.equals("cornflower") || n.equals("lily_of_the_valley") || n.contains("rose") || n.equals("peony")
                || n.equals("lilac") || n.equals("sunflower") || n.contains("petals") || n.equals("wheat") || n.equals("carrots") || n.equals("potatoes")
                || n.equals("beetroots") || n.equals("bamboo") || n.contains("roots") || n.contains("fungus") || n.contains("wart") || n.equals("dead_bush")
                || n.contains("sprouts") || n.contains("pitcher") || n.contains("torchflower")) return PLANT;
        if (n.contains("dark_oak") || n.contains("mangrove_roots")) return n.contains("dark_oak") ? BROWN : WOOD;
        if (n.contains("mangrove")) return RED;
        if (n.contains("spruce")) return PODZOL;
        if (n.contains("birch")) return SAND;
        if (n.contains("jungle")) return DIRT;
        if (n.contains("acacia")) return ORANGE;
        if (n.contains("cherry")) return TERRACOTTA_WHITE;
        if (n.contains("oak") || n.contains("planks") || n.endsWith("_log") || n.endsWith("_wood") || n.endsWith("_stem") || n.contains("chest")
                || n.contains("barrel") || n.contains("crafting") || n.contains("bookshelf") || n.contains("table") || n.contains("loom")
                || n.contains("composter") || n.contains("lectern") || n.equals("jukebox") || n.equals("note_block")) return WOOD;
        if (n.contains("coal")) return BLACK;
        if (n.equals("campfire") || n.equals("soul_campfire")) return PODZOL;
        if (n.contains("wool") || n.contains("carpet")) return WOOL;
        return -1;
    }

    public static boolean isWater(int colour) { return colour == WATER_COLOR; }

    public static final int TINT_NONE = 0, TINT_GRASS = 1, TINT_FOLIAGE = 2;

    private static final Set<String> GRASS_TINTED = Set.of("grass_block", "short_grass", "grass", "tall_grass", "fern", "large_fern", "sugar_cane");
    private static final Set<String> FOLIAGE_TINTED = Set.of("oak_leaves", "jungle_leaves", "acacia_leaves", "dark_oak_leaves", "mangrove_leaves", "vine");

    /** Whether the biome recolours this block (grass and leaves do; birch and spruce leaves, stone and the rest don't). */
    public static int tintOf(String blockName) {
        String n = plain(blockName);
        return GRASS_TINTED.contains(n) ? TINT_GRASS : FOLIAGE_TINTED.contains(n) ? TINT_FOLIAGE : TINT_NONE;
    }
}
