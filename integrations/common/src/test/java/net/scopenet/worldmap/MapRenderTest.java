package net.scopenet.worldmap;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import javax.imageio.ImageIO;
import java.io.*;
import java.nio.ByteBuffer;
import java.nio.file.*;
import java.util.*;
import java.util.zip.Deflater;
import java.util.zip.DeflaterOutputStream;

import static org.junit.jupiter.api.Assertions.*;

class MapRenderTest {
    // ---- a tiny NBT writer for building chunks ----

    static final class W {
        final ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        final DataOutputStream out = new DataOutputStream(bytes);

        W compound(String name) throws IOException { out.writeByte(10); out.writeUTF(name); return this; }
        W end() throws IOException { out.writeByte(0); return this; }
        W string(String name, String v) throws IOException { out.writeByte(8); out.writeUTF(name); out.writeUTF(v); return this; }
        W i(String name, int v) throws IOException { out.writeByte(3); out.writeUTF(name); out.writeInt(v); return this; }
        W b(String name, int v) throws IOException { out.writeByte(1); out.writeUTF(name); out.writeByte(v); return this; }
        W list(String name, int type, int n) throws IOException { out.writeByte(9); out.writeUTF(name); out.writeByte(type); out.writeInt(n); return this; }
        W longs(String name, long[] v) throws IOException {
            out.writeByte(12); out.writeUTF(name); out.writeInt(v.length);
            for (long l : v) out.writeLong(l);
            return this;
        }
    }

    /** A chunk with one section (y = sectionY) whose blocks come from {@code blockAt}(x,y,z) -> palette index. */
    static byte[] chunk(int sectionY, List<String> palette, TriFunction blockAt) throws IOException {
        W w = new W().compound("");
        w.string("Status", "minecraft:full");
        w.list("sections", 10, 1);
        w.b("Y", sectionY);
        w.compound("block_states");
        w.list("palette", 10, palette.size());
        for (String name : palette) w.string("Name", name).end();
        int bits = Math.max(4, 32 - Integer.numberOfLeadingZeros(palette.size() - 1));
        int per = 64 / bits;
        long[] data = new long[(4096 + per - 1) / per];
        for (int y = 0; y < 16; y++) for (int z = 0; z < 16; z++) for (int x = 0; x < 16; x++) {
            int idx = y << 8 | z << 4 | x;
            data[idx / per] |= (long) blockAt.apply(x, y, z) << (idx % per * bits);
        }
        w.longs("data", data);
        w.end(); // block_states
        w.end(); // section
        w.end(); // root
        return w.bytes.toByteArray();
    }

    interface TriFunction { int apply(int x, int y, int z); }

    /** Writes a region file with the given chunks at (cx, cz) -> NBT bytes, zlib compressed. */
    static Path region(Path dir, Map<Integer, byte[]> chunks) throws IOException {
        Path file = dir.resolve("r.0.0.mca");
        ByteBuffer header = ByteBuffer.allocate(8192);
        ByteArrayOutputStream body = new ByteArrayOutputStream();
        int sector = 2;
        for (var e : chunks.entrySet()) {
            ByteArrayOutputStream z = new ByteArrayOutputStream();
            try (DeflaterOutputStream d = new DeflaterOutputStream(z, new Deflater())) { d.write(e.getValue()); }
            byte[] comp = z.toByteArray();
            int length = comp.length + 1;
            int sectors = (length + 4 + 4095) / 4096;
            header.putInt(e.getKey() * 4, sector << 8 | sectors);
            header.putInt(4096 + e.getKey() * 4, 1_700_000_000);
            ByteBuffer rec = ByteBuffer.allocate(sectors * 4096);
            rec.putInt(length).put((byte) 2).put(comp);
            body.writeBytes(rec.array());
            sector += sectors;
        }
        Files.createDirectories(dir);
        try (OutputStream out = Files.newOutputStream(file)) {
            out.write(header.array());
            out.write(body.toByteArray());
        }
        return file;
    }

    @Test void blockNamesBecomeMapColours() {
        assertEquals(MapColors.NONE, BlockColors.mapColor("minecraft:air"));
        assertEquals(MapColors.NONE, BlockColors.mapColor("minecraft:glass"));
        assertEquals(MapColors.GRASS, BlockColors.mapColor("minecraft:grass_block"));
        assertEquals(MapColors.WATER, BlockColors.mapColor("minecraft:water"));
        assertEquals(MapColors.STONE, BlockColors.mapColor("minecraft:stone"));
        assertEquals(MapColors.STONE, BlockColors.mapColor("minecraft:mossy_cobblestone_stairs"));
        assertEquals(MapColors.WOOD, BlockColors.mapColor("minecraft:oak_planks"));
        assertEquals(MapColors.WOOD, BlockColors.mapColor("minecraft:oak_stairs"));
        assertEquals(MapColors.PLANT, BlockColors.mapColor("minecraft:oak_leaves"));
        assertEquals(MapColors.SAND, BlockColors.mapColor("minecraft:cut_sandstone_slab"));
        assertEquals(MapColors.NETHER, BlockColors.mapColor("minecraft:netherrack"));
        assertEquals(MapColors.RED, BlockColors.mapColor("minecraft:red_wool") == MapColors.RED ? MapColors.RED : -1);
        assertEquals(MapColors.LIGHT_BLUE, BlockColors.mapColor("minecraft:light_blue_concrete"));
        assertEquals(MapColors.DEEPSLATE, BlockColors.mapColor("minecraft:deepslate_iron_ore"));
        assertEquals(MapColors.FIRE, BlockColors.mapColor("minecraft:lava"));
        assertEquals(0, BlockColors.of("minecraft:air"));
        assertEquals(BlockColors.WATER_COLOR, BlockColors.of("minecraft:water"));
        int grass = BlockColors.of("minecraft:grass_block");
        assertTrue((grass >> 8 & 0xFF) > (grass >> 16 & 0xFF) && (grass >> 8 & 0xFF) > (grass & 0xFF), "grass is green");
        assertEquals(BlockColors.of("minecraft:oak_planks"), BlockColors.of("minecraft:oak_stairs"), "shapes share their material's colour");
        assertTrue(BlockColors.of("create:brass_casing") != 0, "modded blocks still get a colour");
        int modded = BlockColors.mapColor("create:brass_casing");
        assertEquals(modded, BlockColors.mapColor("create:brass_casing"), "stable for unknown blocks");
        assertTrue(modded > 0);
    }

    @Test void nbtAndRegionFilesRoundTrip(@TempDir Path dir) throws IOException {
        byte[] nbt = chunk(4, List.of("minecraft:air", "minecraft:stone"), (x, y, z) -> y < 8 ? 1 : 0);
        Path file = region(dir, Map.of(RegionFile.index(3, 5), nbt));
        try (RegionFile r = new RegionFile(file)) {
            assertEquals(1_700_000_000, r.timestamp(RegionFile.index(3, 5)));
            assertEquals(0, r.timestamp(RegionFile.index(0, 0)));
            assertNull(r.read(RegionFile.index(0, 0)));
            Nbt.Compound c = r.read(RegionFile.index(3, 5));
            assertNotNull(c);
            assertEquals("minecraft:full", c.string("Status", ""));
            assertEquals(1, c.list("sections").size());
        }
    }

    /** Minecraft 26.x (DataVersion 5000+) writes palette entries without properties as plain strings. */
    @Test void stringPaletteEntriesAreRead() {
        Nbt.Compound states = new Nbt.Compound();
        states.put("palette", new ArrayList<Object>(List.of("minecraft:air", "minecraft:stone")));
        long[] data = new long[256];
        for (int i = 0; i < 4096; i++) if ((i >> 8) < 4) data[i / 16] |= 1L << (i % 16 * 4); // y < 4 is stone
        states.put("data", data);
        Nbt.Compound section = new Nbt.Compound();
        section.put("Y", (byte) 0);
        section.put("block_states", states);
        Nbt.Compound chunk = new Nbt.Compound();
        chunk.put("Status", "minecraft:full");
        chunk.put("sections", new ArrayList<Object>(List.of(section)));
        ChunkSampler.Columns cols = new ChunkSampler.Columns();
        assertTrue(ChunkSampler.sample(chunk, ChunkSampler.Kind.OVERWORLD, cols));
        assertEquals(BlockColors.of("minecraft:stone"), cols.color[0]);
        assertEquals(3, cols.height[0]);
        // a palette that mixes both forms (blocks with properties stay compounds)
        Nbt.Compound slab = new Nbt.Compound();
        slab.put("Name", "minecraft:stone");
        assertEquals("minecraft:stone", ChunkSampler.blockName(slab));
        assertEquals("minecraft:stone", ChunkSampler.blockName("minecraft:stone"));
    }

    @Test void topSurfaceAndWaterDepthAreFound() throws IOException {
        // y 0..9 stone, 10..12 water in the left half.
        byte[] nbt = chunk(4, List.of("minecraft:air", "minecraft:stone", "minecraft:water"),
                (x, y, z) -> y < 6 ? 1 : (x < 8 && y < 9) ? 2 : y < 6 ? 1 : 0);
        Nbt.Compound c = new Nbt.Compound();
        try (var in = new DataInputStream(new ByteArrayInputStream(nbt))) { c = Nbt.readRoot(in); }
        ChunkSampler.Columns cols = new ChunkSampler.Columns();
        assertTrue(ChunkSampler.sample(c, ChunkSampler.Kind.OVERWORLD, cols));
        assertEquals(BlockColors.of("minecraft:stone"), cols.color[15]);
        assertEquals(4 * 16 + 5, cols.height[15]);
        assertEquals(BlockColors.WATER_COLOR, cols.color[0]);
        assertEquals(4 * 16 + 8, cols.height[0]);
        assertEquals(3, cols.depth[0]);
    }

    @Test void theNetherIsShownBeneathItsCeiling() throws IOException {
        // Section 7 covers y 112..127: bedrock up top (y 124..127 -> local 12..15), open air below, floor of netherrack at local y 3.
        byte[] nbt = chunk(7, List.of("minecraft:air", "minecraft:bedrock", "minecraft:netherrack"),
                (x, y, z) -> y >= 12 ? 1 : y <= 3 ? 2 : 0);
        Nbt.Compound c;
        try (var in = new DataInputStream(new ByteArrayInputStream(nbt))) { c = Nbt.readRoot(in); }
        ChunkSampler.Columns cols = new ChunkSampler.Columns();
        assertTrue(ChunkSampler.sample(c, ChunkSampler.Kind.NETHER, cols));
        assertEquals(BlockColors.of("minecraft:netherrack"), cols.color[0], "the floor, not the bedrock roof");
        assertEquals(7 * 16 + 3, cols.height[0]);
    }

    @Test void regionsRenderIntoPositionedTiles(@TempDir Path dir) throws IOException {
        byte[] flat = chunk(0, List.of("minecraft:air", "minecraft:grass_block"), (x, y, z) -> y < 5 ? 1 : 0);
        // chunk (17, 2): second tile column (chunk x 16..31), first tile row
        Path file = region(dir, Map.of(RegionFile.index(17, 2), flat));
        int[][] tiles = RegionRenderer.render(file, ChunkSampler.Kind.OVERWORLD);
        assertNotNull(tiles);
        int[] right = tiles[1];
        int px = (17 - 16) * 16 + 4, py = 2 * 16 + 4; // inside the chunk
        assertEquals(0xFF, right[py * 256 + px] >>> 24);
        int green = right[py * 256 + px];
        assertTrue((green >> 8 & 0xFF) > (green >> 16 & 0xFF), "grass reads as green");
        assertEquals(0, tiles[0][py * 256 + px], "other tiles stay empty");
        assertEquals(0, right[0], "outside the chunk is transparent");
    }

    private static RegionRenderer.Surface flatSurface(int color, int height) {
        RegionRenderer.Surface s = new RegionRenderer.Surface();
        Arrays.fill(s.colors, color);
        Arrays.fill(s.heights, height);
        Arrays.fill(s.floorYs, height);
        return s;
    }

    @Test void slopesAreShadedAgainstTheWestAndNorth() {
        int n = 512;
        RegionRenderer.Surface s = flatSurface(BlockColors.of("minecraft:stone"), 64);
        for (int z = 0; z < n; z++) Arrays.fill(s.heights, z * n, z * n + n, 64 + z / 8 * 3);
        int[] px = RegionRenderer.shade(s);
        int uphill = px[16 * n + 3];           // a step up from the north
        int flat = px[17 * n + 3];             // same height as its north neighbour
        assertTrue(brightness(uphill) > brightness(flat));
        for (int z = 0; z < n; z++) Arrays.fill(s.heights, z * n, z * n + n, 200 - z / 8 * 3);
        int[] down = RegionRenderer.shade(s);
        assertTrue(brightness(down[16 * n + 3]) < brightness(down[17 * n + 3]));
    }

    @Test void biomesTintGrassAndLeaves() {
        int grass = BlockColors.of("minecraft:grass_block");
        RegionRenderer.Surface s = flatSurface(grass, 70);
        Arrays.fill(s.tints, BlockColors.TINT_GRASS);
        Arrays.fill(s.biomes, BiomeColors.index("minecraft:plains"));
        int plains = RegionRenderer.shade(s)[100 * 512 + 100];
        Arrays.fill(s.biomes, BiomeColors.index("minecraft:desert"));
        int desert = RegionRenderer.shade(s)[100 * 512 + 100];
        Arrays.fill(s.biomes, BiomeColors.index("minecraft:swamp"));
        int swamp = RegionRenderer.shade(s)[100 * 512 + 100];
        assertTrue(desert != plains && swamp != plains && swamp != desert, "each biome has its own grass");
        assertTrue((desert >> 16 & 0xFF) > (plains >> 16 & 0xFF), "desert grass is yellower than plains grass");
        // blocks that are not tinted keep their colour in every biome
        Arrays.fill(s.colors, BlockColors.of("minecraft:stone"));
        Arrays.fill(s.tints, BlockColors.TINT_NONE);
        Arrays.fill(s.biomes, BiomeColors.index("minecraft:desert"));
        int a = RegionRenderer.shade(s)[100 * 512 + 100];
        Arrays.fill(s.biomes, BiomeColors.index("minecraft:forest"));
        assertEquals(a, RegionRenderer.shade(s)[100 * 512 + 100]);
    }

    @Test void waterShowsTheSeaFloorInTheShallowsAndDarkensWithDepth() {
        RegionRenderer.Surface s = flatSurface(BlockColors.WATER_COLOR, 62);
        Arrays.fill(s.floorColors, BlockColors.of("minecraft:sand"));
        Arrays.fill(s.floorYs, 61);
        Arrays.fill(s.depths, 1);
        int shallow = RegionRenderer.shade(s)[100 * 512 + 100];
        Arrays.fill(s.depths, 12);
        int deep = RegionRenderer.shade(s)[100 * 512 + 100];
        Arrays.fill(s.depths, 40);
        int abyss = RegionRenderer.shade(s)[100 * 512 + 100];
        assertTrue(brightness(shallow) > brightness(deep), "shallows are lighter");
        assertTrue(brightness(deep) >= brightness(abyss), "and never gets lighter again (the darkening is capped, as in Pl3xMap)");
        assertTrue((shallow >> 16 & 0xFF) > (deep >> 16 & 0xFF), "sand shines through the shallows");
    }

    @Test void biomePalettesAreReadFromChunks() {
        Nbt.Compound states = new Nbt.Compound();
        states.put("palette", new ArrayList<Object>(List.of("minecraft:air", "minecraft:grass_block")));
        long[] data = new long[256];
        for (int i = 0; i < 4096; i++) if ((i >> 8) < 4) data[i / 16] |= 1L << (i % 16 * 4);
        states.put("data", data);
        Nbt.Compound biomes = new Nbt.Compound();
        biomes.put("palette", new ArrayList<Object>(List.of("minecraft:desert")));
        Nbt.Compound section = new Nbt.Compound();
        section.put("Y", (byte) 0);
        section.put("block_states", states);
        section.put("biomes", biomes);
        Nbt.Compound chunk = new Nbt.Compound();
        chunk.put("Status", "minecraft:full");
        chunk.put("sections", new ArrayList<Object>(List.of(section)));
        ChunkSampler.Columns cols = new ChunkSampler.Columns();
        assertTrue(ChunkSampler.sample(chunk, ChunkSampler.Kind.OVERWORLD, cols));
        assertEquals(BiomeColors.index("minecraft:desert"), cols.biome[0]);
        assertEquals(BlockColors.TINT_GRASS, cols.tint[0]);
    }

    @Test void paletteEntriesInEveryFormatAreRead() {
        Nbt.Compound emptyKey = new Nbt.Compound();
        emptyKey.put("", "minecraft:water");
        Nbt.Compound id = new Nbt.Compound();
        id.put("id", "minecraft:oak_leaves");
        assertEquals("minecraft:water", ChunkSampler.blockName(emptyKey));
        assertEquals("minecraft:oak_leaves", ChunkSampler.blockName(id));
    }

    private static int brightness(int argb) { return (argb >> 16 & 0xFF) + (argb >> 8 & 0xFF) + (argb & 0xFF); }

    @Test void pngIsAValidImage() throws IOException {
        int[] pixels = new int[256 * 256];
        for (int i = 0; i < pixels.length; i++) pixels[i] = (i % 256 < 128) ? 0xFF336699 : 0;
        byte[] png = Png.encode(pixels, 256, 256);
        var image = ImageIO.read(new ByteArrayInputStream(png));
        assertEquals(256, image.getWidth());
        assertEquals(0xFF336699, image.getRGB(10, 10));
        assertEquals(0, image.getRGB(200, 10) >>> 24);
        assertTrue(png.length < 400, "a two-colour tile is tiny: " + png.length);
    }

    @Test void realisticTilesStaySmall(@TempDir Path dir) throws IOException {
        byte[] hills = chunk(0, List.of("minecraft:air", "minecraft:grass_block", "minecraft:stone", "minecraft:water"),
                (x, y, z) -> y < 3 + (x * 7 + z * 3) % 5 ? 1 : y < 9 && x > 8 ? 3 : 0);
        Map<Integer, byte[]> chunks = new HashMap<>();
        for (int i = 0; i < 256; i++) chunks.put(i, hills);
        int[][] tiles = RegionRenderer.render(region(dir, chunks), ChunkSampler.Kind.OVERWORLD);
        byte[] png = Png.encode(tiles[0], 256, 256);
        byte[] rgba = Png.encode(tiles[0], 256, 256);
        assertTrue(png.length < 20_000, "palette PNG size " + png.length);
        var image = ImageIO.read(new ByteArrayInputStream(png));
        for (int i = 0; i < 256 * 256; i += 997) assertEquals(tiles[0][i], image.getRGB(i % 256, i / 256), "pixel " + i);
    }

    @Test void zoomLevelsAreBuiltFromTheTilesBelow(@TempDir Path dir) throws IOException {
        TileStore store = new TileStore(dir);
        int[] red = new int[256 * 256];
        Arrays.fill(red, 0xFFFF0000);
        TileStore.Key k = new TileStore.Key("overworld", 0, 3, -2);
        assertTrue(store.update(k, red));
        assertFalse(store.update(k, red.clone()), "an identical tile is not a change");
        TileStore.Key parent = store.rebuildParent(k);
        assertEquals(new TileStore.Key("overworld", 1, 1, -1), parent);
        int[] p = store.load(parent);
        // child (3,-2) is the lower-right?? x=3 odd -> right, y=-2 even -> top: quadrant (1,0)
        assertEquals(0xFFFF0000, p[10 * 256 + 200]);
        assertEquals(0, p[10 * 256 + 10]);
        assertEquals(parent, TileStore.Key.parse(parent.toString()));
        // a fresh store sees the cached tile
        assertArrayEquals(red, new TileStore(dir).load(k));
        store.clear();
        assertNull(new TileStore(dir).load(k));
    }
}
