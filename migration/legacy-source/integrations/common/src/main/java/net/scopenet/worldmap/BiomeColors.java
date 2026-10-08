package net.scopenet.worldmap;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Per-biome grass, foliage and water colours (vanilla values), so forests, deserts, swamps and oceans each get their own tint the
 * way they look in game. Unknown biomes (new or modded) fall back to plains.
 */
public final class BiomeColors {
    private BiomeColors() {}

    /** The colours the block table's grass and leaf entries were averaged for; tints are applied relative to these. */
    public static final int PLAINS_GRASS = 0x91BD59, PLAINS_FOLIAGE = 0x77AB2F, PLAINS_WATER = 0x3F76E4;

    private static final List<int[]> TABLE = new ArrayList<>();
    private static final Map<String, Integer> INDEX = new HashMap<>();

    static {
        add(PLAINS_GRASS, PLAINS_FOLIAGE, PLAINS_WATER, "plains", "sunflower_plains", "beach", "river", "lush_caves", "dripstone_caves", "deep_dark");
        add(0x80B497, 0x60A17B, 0x3D57D6, "snowy_plains", "ice_spikes", "snowy_taiga", "grove", "snowy_slopes", "jagged_peaks", "frozen_peaks");
        add(0xBFB755, 0xAEA42A, PLAINS_WATER, "desert", "savanna", "savanna_plateau", "windswept_savanna");
        add(0x6A7039, 0x6A7039, 0x617B64, "swamp");
        add(0x8DB127, 0x8DB127, 0x3A7A6A, "mangrove_swamp");
        add(0x79C05A, 0x59AE30, PLAINS_WATER, "forest", "flower_forest");
        add(0x88BB67, 0x6BA941, PLAINS_WATER, "birch_forest", "old_growth_birch_forest");
        add(0x507A32, 0x59AE30, PLAINS_WATER, "dark_forest");
        add(0x86B783, 0x68A464, PLAINS_WATER, "taiga");
        add(0x86B87F, 0x68A464, PLAINS_WATER, "old_growth_pine_taiga", "old_growth_spruce_taiga");
        add(0x59C93C, 0x30BB0B, PLAINS_WATER, "jungle", "bamboo_jungle");
        add(0x64C73F, 0x3EB80F, PLAINS_WATER, "sparse_jungle");
        add(0x90814D, 0x9E814D, PLAINS_WATER, "badlands", "eroded_badlands", "wooded_badlands");
        add(0x8AB689, 0x6DA36B, PLAINS_WATER, "windswept_hills", "windswept_gravelly_hills", "windswept_forest", "stony_shore", "stony_peaks");
        add(0x83BB6D, 0x63A948, 0x0E4ECF, "meadow");
        add(0xB6DB61, 0xB6DB61, 0x5DB7EF, "cherry_grove");
        add(0x83B593, 0x64A278, 0x3D57D6, "snowy_beach");
        add(0x8EB971, 0x71A74D, PLAINS_WATER, "ocean", "deep_ocean");
        add(0x8EB971, 0x71A74D, 0x45ADF2, "lukewarm_ocean", "deep_lukewarm_ocean");
        add(0x8EB971, 0x71A74D, 0x43D5EE, "warm_ocean");
        add(0x8EB971, 0x71A74D, 0x3D57D6, "cold_ocean", "deep_cold_ocean");
        add(0x80B497, 0x60A17B, 0x3938C9, "frozen_ocean", "deep_frozen_ocean", "frozen_river");
        add(0x55C93F, 0x2BBB0F, PLAINS_WATER, "mushroom_fields");
        add(0x7C8471, 0x878D76, 0x76889D, "pale_garden");
    }

    private static void add(int grass, int foliage, int water, String... names) {
        int index = TABLE.size();
        TABLE.add(new int[]{grass, foliage, water});
        for (String n : names) INDEX.put(n, index);
    }

    /** The table index for a biome id such as {@code minecraft:forest}. */
    public static int index(String biome) {
        String plain = biome.startsWith("minecraft:") ? biome.substring(10) : biome.substring(biome.indexOf(':') + 1);
        return INDEX.getOrDefault(plain, 0);
    }

    public static int grass(int index) { return TABLE.get(index)[0]; }

    public static int foliage(int index) { return TABLE.get(index)[1]; }

    public static int water(int index) { return TABLE.get(index)[2]; }
}
