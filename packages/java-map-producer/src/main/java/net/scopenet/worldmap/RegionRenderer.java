package net.scopenet.worldmap;

import java.io.IOException;
import java.nio.file.Path;

/**
 * Turns one region file (512 x 512 blocks) into four 256 x 256 map tiles in the style of Pl3xMap (MIT licensed), which in turn
 * follows the look of the game: block colours averaged from the real textures, grass, leaves and water tinted by biome and blended
 * over neighbouring columns, terrain shaded by how it steps up or down from the block to its west and north, and water that is
 * see-through and deepens in colour so the sea floor shows in the shallows.
 */
public final class RegionRenderer {
    private RegionRenderer() {}

    public static final int TILE = 256;
    private static final int SIDE = 512;
    /** How many columns each side the biome tint is averaged over (2 = 5 x 5), so biome borders fade instead of cutting. */
    private static final int BLEND = 2;

    /** Set when the last region had readable chunks but not one drawable column; says what the chunks looked like. */
    public static volatile String lastEmptyReport;

    /** Everything known about a region's columns, indexed {@code z * 512 + x}. */
    static final class Surface {
        final int[] colors = new int[SIDE * SIDE], heights = new int[SIDE * SIDE], depths = new int[SIDE * SIDE];
        final int[] biomes = new int[SIDE * SIDE], tints = new int[SIDE * SIDE];
        final int[] floorColors = new int[SIDE * SIDE], floorYs = new int[SIDE * SIDE];
    }

    /** Four tile images in the order (0,0) (1,0) (0,1) (1,1) of {@code [tileX][tileY]}, or null if nothing renderable was found. */
    public static int[][] render(Path file, ChunkSampler.Kind kind) throws IOException {
        Surface surface = new Surface();
        ChunkSampler.Columns cols = new ChunkSampler.Columns();
        boolean any = false;
        int readable = 0, drawable = 0, present = 0, unreadable = 0;
        Nbt.Compound first = null, rejected = null;
        lastEmptyReport = null;
        try (RegionFile region = new RegionFile(file)) {
            for (int i = 0; i < 1024; i++) {
                if (region.timestamp(i) == 0) continue;
                present++;
                Nbt.Compound chunk = region.read(i);
                if (chunk == null) { unreadable++; continue; }
                if (!ChunkSampler.sample(chunk, kind, cols)) { if (rejected == null) rejected = chunk; continue; }
                readable++;
                if (first == null) first = chunk;
                for (int c : cols.color) if (c != 0) drawable++;
                int cx = (i & 31) * 16, cz = (i >> 5) * 16;
                for (int z = 0; z < 16; z++) {
                    int dst = (cz + z) * SIDE + cx, src = z * 16;
                    System.arraycopy(cols.color, src, surface.colors, dst, 16);
                    System.arraycopy(cols.height, src, surface.heights, dst, 16);
                    System.arraycopy(cols.depth, src, surface.depths, dst, 16);
                    System.arraycopy(cols.biome, src, surface.biomes, dst, 16);
                    System.arraycopy(cols.tint, src, surface.tints, dst, 16);
                    System.arraycopy(cols.floorColor, src, surface.floorColors, dst, 16);
                    System.arraycopy(cols.floorY, src, surface.floorYs, dst, 16);
                }
                any = true;
            }
        }
        if (!any) {
            lastEmptyReport = present + " chunks listed, " + unreadable + " unreadable (compression or format), " + (present - unreadable)
                    + " read but not accepted" + (rejected == null ? "" : "; " + ChunkSampler.describe(rejected));
            return null;
        }
        if (drawable == 0) lastEmptyReport = readable + " chunks read but no block to draw; " + ChunkSampler.describe(first);
        int[] pixels = shade(surface);
        int[][] tiles = new int[4][TILE * TILE];
        for (int ty = 0; ty < 2; ty++) {
            for (int tx = 0; tx < 2; tx++) {
                int[] tile = tiles[tx + ty * 2];
                for (int row = 0; row < TILE; row++) {
                    System.arraycopy(pixels, (ty * TILE + row) * SIDE + tx * TILE, tile, row * TILE, TILE);
                }
            }
        }
        return tiles;
    }

    /** Paints the region: biome tint, then relief shading, then (over water) the sea floor under tinted, depth-darkened water. */
    static int[] shade(Surface s) {
        int[] present = new int[SIDE * SIDE];
        int[] grass = new int[SIDE * SIDE], foliage = new int[SIDE * SIDE], water = new int[SIDE * SIDE];
        for (int i = 0; i < present.length; i++) {
            if (s.colors[i] == 0) continue;
            present[i] = 1;
            grass[i] = BiomeColors.grass(s.biomes[i]);
            foliage[i] = BiomeColors.foliage(s.biomes[i]);
            water[i] = BiomeColors.water(s.biomes[i]);
        }
        grass = blur(grass, present);
        foliage = blur(foliage, present);
        water = blur(water, present);

        int[] out = new int[SIDE * SIDE];
        for (int z = 0; z < SIDE; z++) {
            for (int x = 0; x < SIDE; x++) {
                int i = z * SIDE + x;
                int c = s.colors[i];
                if (c == 0) continue;
                boolean wet = c == BlockColors.WATER_COLOR;
                int y = wet ? s.floorYs[i] : s.heights[i];

                // Relief, as Pl3xMap's "modern" heightmap: stepping up from the west/north neighbour lightens, down darkens.
                int step = 0x22;
                if (x > 0 && present[i - 1] == 1) step = stepFrom(y, neighbourY(s, i - 1), step);
                if (z > 0 && present[i - SIDE] == 1) step = stepFrom(y, neighbourY(s, i - SIDE), step);
                float light = 1f - Math.max(0, Math.min(0x44, step)) / 255f;

                if (!wet) {
                    int base = s.tints[i] == BlockColors.TINT_GRASS ? tint(c, grass[i], BiomeColors.PLAINS_GRASS)
                            : s.tints[i] == BlockColors.TINT_FOLIAGE ? tint(c, foliage[i], BiomeColors.PLAINS_FOLIAGE) : c;
                    out[i] = scale(base, light);
                    continue;
                }
                float depth = s.depths[i] * 0.025f;
                int sea = 0xFF000000 | water[i];
                sea = lerp(sea, 0xFF000000, Math.min(0.45f, cubicOut(depth / 1.5f)));
                int floor = s.floorColors[i];
                if (floor == 0) { out[i] = sea; continue; }
                out[i] = lerp(scale(floor, light), sea, quinticOut(Math.min(1f, depth * 5f)));
            }
        }
        return out;
    }

    private static int neighbourY(Surface s, int i) {
        return s.colors[i] == BlockColors.WATER_COLOR ? s.floorYs[i] : s.heights[i];
    }

    private static int stepFrom(int y, int other, int step) {
        return y > other ? step - 0x22 : y < other ? step + 0x22 : step;
    }

    /** Box average of the colours around each column that has terrain (empty columns don't pull the colour towards black). */
    private static int[] blur(int[] rgb, int[] present) {
        int n = SIDE * SIDE;
        int[] r = new int[n], g = new int[n], b = new int[n], w = new int[n];
        for (int z = 0; z < SIDE; z++) {
            for (int x = 0; x < SIDE; x++) {
                int sr = 0, sg = 0, sb = 0, sw = 0;
                for (int dx = -BLEND; dx <= BLEND; dx++) {
                    int xx = x + dx;
                    if (xx < 0 || xx >= SIDE) continue;
                    int j = z * SIDE + xx;
                    if (present[j] == 0) continue;
                    sr += rgb[j] >> 16 & 0xFF; sg += rgb[j] >> 8 & 0xFF; sb += rgb[j] & 0xFF; sw++;
                }
                int i = z * SIDE + x;
                r[i] = sr; g[i] = sg; b[i] = sb; w[i] = sw;
            }
        }
        int[] out = new int[n];
        for (int z = 0; z < SIDE; z++) {
            for (int x = 0; x < SIDE; x++) {
                int sr = 0, sg = 0, sb = 0, sw = 0;
                for (int dz = -BLEND; dz <= BLEND; dz++) {
                    int zz = z + dz;
                    if (zz < 0 || zz >= SIDE) continue;
                    int j = zz * SIDE + x;
                    sr += r[j]; sg += g[j]; sb += b[j]; sw += w[j];
                }
                out[z * SIDE + x] = sw == 0 ? rgb[z * SIDE + x] : (sr / sw) << 16 | (sg / sw) << 8 | (sb / sw);
            }
        }
        return out;
    }

    /** Recolours a block (averaged for plains) for a biome by the ratio of the biome's colour to the plains colour. */
    private static int tint(int block, int biome, int plains) {
        int r = Math.min(255, (block >> 16 & 0xFF) * Math.max(1, biome >> 16 & 0xFF) / (plains >> 16 & 0xFF));
        int g = Math.min(255, (block >> 8 & 0xFF) * Math.max(1, biome >> 8 & 0xFF) / (plains >> 8 & 0xFF));
        int b = Math.min(255, (block & 0xFF) * Math.max(1, biome & 0xFF) / (plains & 0xFF));
        return 0xFF000000 | r << 16 | g << 8 | b;
    }

    static int scale(int argb, float factor) {
        int r = Math.min(255, (int) ((argb >> 16 & 0xFF) * factor)), g = Math.min(255, (int) ((argb >> 8 & 0xFF) * factor)),
                b = Math.min(255, (int) ((argb & 0xFF) * factor));
        return 0xFF000000 | r << 16 | g << 8 | b;
    }

    private static int lerp(int from, int to, float t) {
        int r = (int) ((from >> 16 & 0xFF) + ((to >> 16 & 0xFF) - (from >> 16 & 0xFF)) * t);
        int g = (int) ((from >> 8 & 0xFF) + ((to >> 8 & 0xFF) - (from >> 8 & 0xFF)) * t);
        int b = (int) ((from & 0xFF) + ((to & 0xFF) - (from & 0xFF)) * t);
        return 0xFF000000 | r << 16 | g << 8 | b;
    }

    private static float cubicOut(float t) { t = Math.max(0f, Math.min(1f, t)); return 1f - (1f - t) * (1f - t) * (1f - t); }

    private static float quinticOut(float t) { t = Math.max(0f, Math.min(1f, t)); float u = 1f - t; return 1f - u * u * u * u * u; }
}
