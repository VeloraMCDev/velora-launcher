package net.scopenet.worldmap;

import java.io.*;
import java.nio.ByteBuffer;
import java.nio.file.*;
import java.util.*;
import java.util.zip.*;

/**
 * The local cache of rendered tiles (raw pixels, compressed), plus the zoom pyramid built from them. Zoom 0 is one pixel per
 * block; every level up halves the resolution, down to {@code maxZoom}.
 */
public final class TileStore {
    public record Key(String dim, int zoom, int x, int y) {
        @Override public String toString() { return dim + "/" + zoom + "/" + x + "/" + y; }

        public static Key parse(String s) {
            String[] p = s.split("/");
            return new Key(p[0], Integer.parseInt(p[1]), Integer.parseInt(p[2]), Integer.parseInt(p[3]));
        }
    }

    private static final int SIZE = RegionRenderer.TILE;
    private static final int PIXELS = SIZE * SIZE;

    private final Path root;
    private final Map<Key, int[]> memory = new LinkedHashMap<>(64, 0.75f, true) {
        @Override protected boolean removeEldestEntry(Map.Entry<Key, int[]> e) { return size() > 96; }
    };

    public TileStore(Path root) { this.root = root; }

    private Path file(Key k) { return root.resolve(k.dim()).resolve(Integer.toString(k.zoom())).resolve(k.x() + "_" + k.y() + ".tile"); }

    public boolean exists(Key k) { return Files.isRegularFile(file(k)); }

    /** Every stored tile of one dimension at one zoom level. */
    public synchronized List<Key> keys(String dim, int zoom) {
        List<Key> out = new ArrayList<>();
        Path dir = root.resolve(dim).resolve(Integer.toString(zoom));
        if (!Files.isDirectory(dir)) return out;
        try (DirectoryStream<Path> stream = Files.newDirectoryStream(dir, "*.tile")) {
            for (Path p : stream) {
                String name = p.getFileName().toString();
                String[] xy = name.substring(0, name.length() - ".tile".length()).split("_");
                try {
                    out.add(new Key(dim, zoom, Integer.parseInt(xy[0]), Integer.parseInt(xy[1])));
                } catch (RuntimeException ignored) {
                    // not one of ours
                }
            }
        } catch (IOException ignored) {
            // an unreadable folder just has no tiles
        }
        return out;
    }

    /** The tile's pixels, or null if it was never rendered. */
    public synchronized int[] load(Key k) {
        int[] cached = memory.get(k);
        if (cached != null) return cached;
        Path f = file(k);
        if (!Files.isRegularFile(f)) return null;
        try (InflaterInputStream in = new InflaterInputStream(new BufferedInputStream(Files.newInputStream(f)))) {
            byte[] raw = in.readAllBytes();
            if (raw.length != PIXELS * 4) return null;
            int[] pixels = new int[PIXELS];
            ByteBuffer.wrap(raw).asIntBuffer().get(pixels);
            memory.put(k, pixels);
            return pixels;
        } catch (IOException e) {
            return null;
        }
    }

    public synchronized void save(Key k, int[] pixels) throws IOException {
        ByteBuffer raw = ByteBuffer.allocate(PIXELS * 4);
        raw.asIntBuffer().put(pixels);
        Path f = file(k);
        Files.createDirectories(f.getParent());
        Path tmp = f.resolveSibling(f.getFileName() + ".tmp");
        try (DeflaterOutputStream out = new DeflaterOutputStream(Files.newOutputStream(tmp), new Deflater(3))) {
            out.write(raw.array());
        }
        Files.move(tmp, f, StandardCopyOption.REPLACE_EXISTING);
        memory.put(k, pixels);
    }

    public synchronized void clear() throws IOException {
        memory.clear();
        if (!Files.isDirectory(root)) return;
        try (var walk = Files.walk(root)) {
            for (Path p : (Iterable<Path>) walk.sorted(Comparator.reverseOrder())::iterator) if (!p.equals(root)) Files.deleteIfExists(p);
        }
    }

    public static boolean blank(int[] pixels) {
        for (int p : pixels) if ((p >>> 24) != 0) return false;
        return true;
    }

    /** Stores a freshly rendered tile and returns true if it differs from what was there. */
    public boolean update(Key k, int[] pixels) throws IOException {
        int[] old = load(k);
        if (old == null ? blank(pixels) : Arrays.equals(old, pixels)) return false;
        save(k, pixels);
        return true;
    }

    /**
     * Rebuilds the parent tile of {@code child} from its four children. Returns the parent's key if it changed, otherwise null.
     */
    public Key rebuildParent(Key child) throws IOException {
        int px = child.x() >> 1, py = child.y() >> 1, pz = child.zoom() + 1;
        int[] out = new int[PIXELS];
        for (int qy = 0; qy < 2; qy++) {
            for (int qx = 0; qx < 2; qx++) {
                int[] src = load(new Key(child.dim(), child.zoom(), px * 2 + qx, py * 2 + qy));
                if (src == null) continue;
                int ox = qx * (SIZE / 2), oy = qy * (SIZE / 2);
                for (int y = 0; y < SIZE / 2; y++) {
                    for (int x = 0; x < SIZE / 2; x++) {
                        out[(oy + y) * SIZE + ox + x] = average(src[(y * 2) * SIZE + x * 2], src[(y * 2) * SIZE + x * 2 + 1],
                                src[(y * 2 + 1) * SIZE + x * 2], src[(y * 2 + 1) * SIZE + x * 2 + 1]);
                    }
                }
            }
        }
        Key parent = new Key(child.dim(), pz, px, py);
        return update(parent, out) ? parent : null;
    }

    /** The mean of the opaque pixels among four (transparent ones don't dilute it); 0 if all four are transparent. */
    static int average(int a, int b, int c, int d) {
        int n = 0, r = 0, g = 0, bl = 0;
        for (int px : new int[]{a, b, c, d}) {
            if ((px >>> 24) == 0) continue;
            n++;
            r += px >> 16 & 0xFF;
            g += px >> 8 & 0xFF;
            bl += px & 0xFF;
        }
        return n == 0 ? 0 : 0xFF000000 | (r / n) << 16 | (g / n) << 8 | bl / n;
    }
}
