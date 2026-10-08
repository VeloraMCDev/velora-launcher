package net.scopenet.worldmap;

import java.io.*;
import java.nio.ByteBuffer;
import java.nio.channels.FileChannel;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.zip.GZIPInputStream;
import java.util.zip.InflaterInputStream;

/**
 * Read-only access to one Anvil region file (r.X.Z.mca): 32 x 32 chunks. Safe to use while the server is running:
 * a chunk caught mid-write simply fails to read and is retried on the next pass.
 */
public final class RegionFile implements AutoCloseable {
    private static final int SECTOR = 4096;

    private final FileChannel channel;
    private final int[] locations = new int[1024];
    private final int[] timestamps = new int[1024];

    public RegionFile(Path file) throws IOException {
        channel = FileChannel.open(file, StandardOpenOption.READ);
        try {
            ByteBuffer header = ByteBuffer.allocate(2 * SECTOR);
            while (header.hasRemaining()) if (channel.read(header) < 0) break;
            header.flip();
            if (header.remaining() == 2 * SECTOR) {
                header.asIntBuffer().get(locations, 0, 1024);
                header.position(SECTOR);
                header.asIntBuffer().get(timestamps, 0, 1024);
            }
        } catch (IOException | RuntimeException e) {
            channel.close();
            throw e;
        }
    }

    public static int index(int chunkX, int chunkZ) { return (chunkX & 31) + (chunkZ & 31) * 32; }

    /** When this chunk was last saved (seconds since 1970), or 0 if the region has no such chunk. */
    public int timestamp(int index) { return locations[index] == 0 ? 0 : timestamps[index]; }

    /**
     * The chunk's NBT, or null if it is missing, uses an unsupported format (external .mcc file or LZ4) or couldn't be read.
     * Region files written with LZ4 (a server option since 1.20.5) are skipped.
     */
    public Nbt.Compound read(int index) {
        int loc = locations[index];
        if (loc == 0) return null;
        long offset = (long) (loc >>> 8) * SECTOR;
        int sectors = loc & 0xFF;
        if (offset < 2 * SECTOR || sectors == 0) return null;
        try {
            ByteBuffer head = ByteBuffer.allocate(5);
            if (read(head, offset) < 5) return null;
            head.flip();
            int length = head.getInt();
            int type = head.get() & 0xFF;
            if (length < 2 || length - 1 > (long) sectors * SECTOR || type > 3 || type == 0) return null; // 4 = LZ4, 0x80+ = external file
            ByteBuffer body = ByteBuffer.allocate(length - 1);
            if (read(body, offset + 5) < length - 1) return null;
            InputStream raw = new ByteArrayInputStream(body.array());
            InputStream stream = type == 1 ? new GZIPInputStream(raw) : type == 2 ? new InflaterInputStream(raw) : raw;
            try (DataInputStream in = new DataInputStream(new BufferedInputStream(stream))) {
                return Nbt.readRoot(in);
            }
        } catch (IOException | RuntimeException e) {
            return null;
        }
    }

    private int read(ByteBuffer buffer, long position) throws IOException {
        int total = 0;
        while (buffer.hasRemaining()) {
            int n = channel.read(buffer, position + total);
            if (n < 0) break;
            total += n;
        }
        return total;
    }

    @Override public void close() throws IOException { channel.close(); }
}
