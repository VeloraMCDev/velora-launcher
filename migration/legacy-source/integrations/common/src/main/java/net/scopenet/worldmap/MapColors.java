package net.scopenet.worldmap;

/** Minecraft's map colour table (1.20) and the four brightness steps a map applies to each colour. */
public final class MapColors {
    private MapColors() {}

    public static final int NONE = 0, GRASS = 1, SAND = 2, WOOL = 3, FIRE = 4, ICE = 5, METAL = 6, PLANT = 7, SNOW = 8, CLAY = 9, DIRT = 10,
            STONE = 11, WATER = 12, WOOD = 13, QUARTZ = 14, ORANGE = 15, MAGENTA = 16, LIGHT_BLUE = 17, YELLOW = 18, LIGHT_GREEN = 19,
            PINK = 20, GRAY = 21, LIGHT_GRAY = 22, CYAN = 23, PURPLE = 24, BLUE = 25, BROWN = 26, GREEN = 27, RED = 28, BLACK = 29,
            GOLD = 30, DIAMOND = 31, LAPIS = 32, EMERALD = 33, PODZOL = 34, NETHER = 35, TERRACOTTA_WHITE = 36, TERRACOTTA_ORANGE = 37,
            TERRACOTTA_GRAY = 43, TERRACOTTA_LIGHT_GRAY = 44, TERRACOTTA_CYAN = 45, TERRACOTTA_BROWN = 48, CRIMSON_NYLIUM = 52,
            CRIMSON_STEM = 53, CRIMSON_HYPHAE = 54, WARPED_NYLIUM = 55, WARPED_STEM = 56, WARPED_HYPHAE = 57, WARPED_WART = 58,
            DEEPSLATE = 59, RAW_IRON = 60, GLOW_LICHEN = 61;

    private static final int[] RGB = {
            0x000000, 0x7FB238, 0xF7E9A3, 0xC7C7C7, 0xFF0000, 0xA0A0FF, 0xA7A7A7, 0x007C00, 0xFFFFFF, 0xA4A8B8, 0x976D4D, 0x707070,
            0x4040FF, 0x8F7748, 0xFFFCF5, 0xD87F33, 0xB24CD8, 0x6699D8, 0xE5E533, 0x7FCC19, 0xF27FA5, 0x4C4C4C, 0x999999, 0x4C7F99,
            0x7F3FB2, 0x334CB2, 0x664C33, 0x667F33, 0x993333, 0x191919, 0xFAEE4D, 0x5CDBD5, 0x4A80FF, 0x00D93A, 0x815631, 0x700200,
            0xD1B1A1, 0x9F5224, 0x95576C, 0x706C8A, 0xBA8524, 0x677535, 0xA04D4E, 0x392923, 0x876B62, 0x575C5C, 0x7A4958, 0x4C3E5C,
            0x4C3223, 0x4C522A, 0x8E3C2E, 0x251610, 0xBD3031, 0x943F61, 0x5C191D, 0x167E86, 0x3A8E8C, 0x562C3E, 0x14B485, 0x646464,
            0xD8AF93, 0x7FA796};

    public static final int COUNT = RGB.length;

    /** Brightness factors for the four shades: darkest .. brightest, as a map shows low, normal, high and lowest ground. */
    private static final int[] SHADE = {180, 220, 255, 135};

    /** The colour's base RGB (opaque ARGB), or 0 for none. */
    public static int base(int id) { return id <= 0 || id >= COUNT ? 0 : 0xFF000000 | RGB[id]; }

    /** Opaque ARGB for a colour id at a shade (0 = low, 1 = normal, 2 = high, 3 = lowest); 0 for no colour. */
    public static int argb(int id, int shade) {
        if (id <= 0 || id >= COUNT) return 0;
        int base = RGB[id], m = SHADE[shade & 3];
        int r = (base >> 16 & 0xFF) * m / 255, g = (base >> 8 & 0xFF) * m / 255, b = (base & 0xFF) * m / 255;
        return 0xFF000000 | r << 16 | g << 8 | b;
    }
}
