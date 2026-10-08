package net.scopenet.worldmap;

import java.util.ArrayList;
import java.util.List;

/** Reads one chunk's blocks and finds, for each of its 256 columns, the block a top-down map would show. */
public final class ChunkSampler {
    private ChunkSampler() {}

    public enum Kind { OVERWORLD, NETHER, END }

    /** Per-column results, indexed {@code z * 16 + x}. */
    public static final class Columns {
        public final int[] color = new int[256];
        public final int[] height = new int[256];
        /** For water: how many blocks of water sit above the floor. */
        public final int[] depth = new int[256];
        /** Biome table index (see {@link BiomeColors}) at the surface block. */
        public final int[] biome = new int[256];
        /** {@link BlockColors#TINT_NONE}, {@code TINT_GRASS} or {@code TINT_FOLIAGE}: whether the biome recolours the surface block. */
        public final int[] tint = new int[256];
        /** For water: the colour and height of the block under it, which shows through. */
        public final int[] floorColor = new int[256];
        public final int[] floorY = new int[256];
    }

    private record Section(int y, int[] colors, byte[] tints, long[] data, int bits, int[] biomes, long[] biomeData, int biomeBits) {
        private int paletteIndex(int x, int y, int z) {
            if (data == null) return 0;
            int index = y << 8 | z << 4 | x;
            int perLong = 64 / bits;
            int word = index / perLong;
            if (word >= data.length) return -1;
            return (int) (data[word] >>> (index % perLong * bits) & ((1L << bits) - 1));
        }

        int colorAt(int x, int y, int z) {
            int v = paletteIndex(x, y, z);
            return v >= 0 && v < colors.length ? colors[v] : 0;
        }

        int tintAt(int x, int y, int z) {
            int v = paletteIndex(x, y, z);
            return v >= 0 && v < tints.length ? tints[v] : 0;
        }

        /** Biomes are stored per 4 x 4 x 4 cells. */
        int biomeAt(int x, int y, int z) {
            if (biomes.length == 0) return 0;
            if (biomeData == null || biomes.length == 1) return biomes[0];
            int index = (y >> 2) << 4 | (z >> 2) << 2 | (x >> 2);
            int perLong = 64 / biomeBits;
            int word = index / perLong;
            if (word >= biomeData.length) return biomes[0];
            int v = (int) (biomeData[word] >>> (index % perLong * biomeBits) & ((1L << biomeBits) - 1));
            return v < biomes.length ? biomes[v] : biomes[0];
        }
    }

    /**
     * A short description of a chunk's structure (tag names and types, block names, palette and data sizes). Sent along when a world
     * is read but nothing can be drawn, so an unfamiliar chunk format can be understood from a log line instead of guessed at.
     */
    public static String describe(Nbt.Compound chunk) {
        StringBuilder out = new StringBuilder("chunk tags {");
        for (var e : new java.util.TreeMap<>(chunk).entrySet()) out.append(e.getKey()).append(':').append(kind(e.getValue())).append(' ');
        out.append("} Status=").append(chunk.get("Status")).append(" sections=").append(chunk.list("sections").size());
        List<Object> sections = chunk.list("sections");
        int shown = 0;
        for (Object o : sections) {
            if (!(o instanceof Nbt.Compound s) || shown >= 2) continue;
            Nbt.Compound states = s.compound("block_states");
            if (states != null && states.list("palette").size() < 2 && shown == 0 && sections.size() > 3) continue; // skip empty sky sections
            shown++;
            out.append(" | section Y=").append(s.get("Y")).append(" tags {");
            for (var e : new java.util.TreeMap<>(s).entrySet()) out.append(e.getKey()).append(':').append(kind(e.getValue())).append(' ');
            out.append('}');
            if (states == null) continue;
            out.append(" block_states {");
            for (var e : new java.util.TreeMap<>(states).entrySet()) out.append(e.getKey()).append(':').append(kind(e.getValue())).append(' ');
            out.append("} palette=[");
            List<Object> palette = states.list("palette");
            for (int i = 0; i < Math.min(6, palette.size()); i++) {
                Object p = palette.get(i);
                out.append(i == 0 ? "" : ", ").append(blockName(p)).append('=').append(BlockColors.of(blockName(p)));
            }
            out.append(palette.size() > 6 ? ", ... " + palette.size() + " total]" : "]");
        }
        return out.length() > 1400 ? out.substring(0, 1400) + "..." : out.toString();
    }

    private static String kind(Object v) {
        if (v == null) return "null";
        if (v instanceof long[] a) return "long[" + a.length + "]";
        if (v instanceof int[] a) return "int[" + a.length + "]";
        if (v instanceof byte[] a) return "byte[" + a.length + "]";
        if (v instanceof List<?> l) return "list[" + l.size() + "]";
        if (v instanceof Nbt.Compound) return "compound";
        return v.getClass().getSimpleName();
    }

    /**
     * A palette entry's block name. Up to Minecraft 1.21 every entry is a compound ({@code Name}, optional {@code Properties});
     * newer worlds (DataVersion 5000 and up) write a plain string for blocks without properties, and a compound with {@code id} for the rest.
     */
    static String blockName(Object entry) {
        if (entry instanceof String name) return name;
        // Older worlds name the block "Name". Minecraft 26.x uses "id" (and a lower case "properties"), and writes a block that has
        // no properties as a compound whose only value sits under an empty key when its palette also holds blocks with properties.
        if (entry instanceof Nbt.Compound c) {
            for (String key : new String[]{"Name", "id", ""}) if (c.get(key) instanceof String name) return name;
            for (Object v : c.values()) if (v instanceof String name && name.indexOf(':') > 0) return name;
        }
        return "minecraft:air";
    }

    /** True if the chunk was fully generated and could be read (the columns are then filled in). */
    public static boolean sample(Nbt.Compound chunk, Kind kind, Columns out) {
        String status = chunk.string("Status", "minecraft:full");
        if (!status.endsWith("full")) return false;
        List<Section> sections = new ArrayList<>();
        for (Object o : chunk.list("sections")) {
            if (!(o instanceof Nbt.Compound s)) continue;
            Nbt.Compound states = s.compound("block_states");
            if (states == null) continue;
            List<Object> palette = states.list("palette");
            if (palette.isEmpty()) continue;
            int[] colors = new int[palette.size()];
            byte[] tints = new byte[palette.size()];
            for (int i = 0; i < colors.length; i++) {
                String name = blockName(palette.get(i));
                colors[i] = BlockColors.of(name);
                tints[i] = (byte) BlockColors.tintOf(name);
            }
            long[] data = states.longs("data");
            int bits = Math.max(4, 32 - Integer.numberOfLeadingZeros(colors.length - 1));
            Nbt.Compound biomeStates = s.compound("biomes");
            List<Object> biomePalette = biomeStates == null ? List.of() : biomeStates.list("palette");
            int[] biomes = new int[biomePalette.size()];
            for (int i = 0; i < biomes.length; i++) biomes[i] = BiomeColors.index(blockName(biomePalette.get(i)));
            long[] biomeData = biomeStates == null ? null : biomeStates.longs("data");
            int biomeBits = Math.max(1, 32 - Integer.numberOfLeadingZeros(Math.max(1, biomes.length - 1)));
            sections.add(new Section(s.integer("Y", 0), colors, tints, colors.length == 1 ? null : data, bits, biomes,
                    biomes.length == 1 ? null : biomeData, biomeBits));
        }
        if (sections.isEmpty()) return false;
        sections.sort((a, b) -> Integer.compare(b.y(), a.y()));

        for (int z = 0; z < 16; z++) {
            for (int x = 0; x < 16; x++) {
                int i = z * 16 + x;
                out.color[i] = 0;
                out.height[i] = 0;
                out.depth[i] = 0;
                out.biome[i] = 0;
                out.tint[i] = 0;
                out.floorColor[i] = 0;
                out.floorY[i] = 0;
                if (kind == Kind.NETHER) column(sections, x, z, out, i, true);
                else column(sections, x, z, out, i, false);
            }
        }
        return true;
    }

    private static void column(List<Section> sections, int x, int z, Columns out, int i, boolean roofed) {
        boolean pastRoof = !roofed;
        int roofColor = 0, roofY = 0;
        for (int si = 0; si < sections.size(); si++) {
            Section sec = sections.get(si);
            for (int ly = 15; ly >= 0; ly--) {
                int worldY = sec.y() * 16 + ly;
                if (roofed && worldY > 126) continue;
                int c = sec.colorAt(x, ly, z);
                if (!pastRoof) {
                    // Under the nether ceiling: skip solid rock until the first open air.
                    if (c == 0) pastRoof = true;
                    else if (roofColor == 0) { roofColor = c; roofY = worldY; }
                    continue;
                }
                if (c == 0) continue;
                out.color[i] = c;
                out.height[i] = worldY;
                out.floorY[i] = worldY;
                out.biome[i] = sec.biomeAt(x, ly, z);
                out.tint[i] = sec.tintAt(x, ly, z);
                if (c == BlockColors.WATER_COLOR) findFloor(sections, si, ly, x, z, out, i);
                return;
            }
        }
        if (roofed && roofColor != 0) { // a solid column: show the ceiling instead
            out.color[i] = roofColor;
            out.height[i] = roofY;
            out.floorY[i] = roofY;
        }
    }

    /** Under water: how deep it is, and the first solid block below it (which the map shows through the water). */
    private static void findFloor(List<Section> sections, int si, int ly, int x, int z, Columns out, int i) {
        int depth = 1;
        for (int s = si; s < sections.size(); s++) {
            Section sec = sections.get(s);
            for (int y = s == si ? ly - 1 : 15; y >= 0; y--) {
                int c = sec.colorAt(x, y, z);
                if (c == BlockColors.WATER_COLOR) {
                    if (++depth >= 64) { out.depth[i] = depth; return; }
                    continue;
                }
                if (c == 0) { out.depth[i] = depth; return; } // open air or glass under the water: no floor to show
                out.depth[i] = depth;
                out.floorColor[i] = c;
                out.floorY[i] = sec.y() * 16 + y;
                return;
            }
        }
        out.depth[i] = depth;
    }
}
