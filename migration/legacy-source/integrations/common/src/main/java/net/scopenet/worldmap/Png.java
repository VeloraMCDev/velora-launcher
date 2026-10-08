package net.scopenet.worldmap;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Set;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.zip.CRC32;
import java.util.zip.Deflater;

/** A minimal PNG writer (RGBA, 8 bit), so rendering needs nothing from the JDK's desktop module. */
public final class Png {
    private Png() {}

    /**
     * Encodes the image. Tiles with 256 colours or fewer (flat areas, oceans, the zoomed-out levels) are written as palette PNGs at the
     * smallest bit depth that fits; richer tiles (biome-tinted land, shaded water) are written as RGBA so no colour is ever altered.
     */
    public static byte[] encode(int[] argb, int width, int height) {
        Map<Integer, Integer> palette = new LinkedHashMap<>();
        for (int c : argb) {
            int key = (c >>> 24) == 0 ? 0 : c;
            if (!palette.containsKey(key)) {
                palette.put(key, palette.size());
                if (palette.size() > 256) return rgba(argb, width, height);
            }
        }
        int n = palette.size();
        int depth = n <= 2 ? 1 : n <= 4 ? 2 : n <= 16 ? 4 : 8;
        int rowBytes = (width * depth + 7) / 8;
        byte[] raw = new byte[height * (1 + rowBytes)];
        int p = 0;
        for (int y = 0; y < height; y++) {
            raw[p++] = 0;
            for (int x = 0; x < width; x++) {
                int c = argb[y * width + x];
                int idx = palette.get((c >>> 24) == 0 ? 0 : c);
                if (depth == 8) raw[p + x] = (byte) idx;
                else raw[p + x * depth / 8] |= (byte) (idx << (8 - depth - (x * depth) % 8));
            }
            p += rowBytes;
        }
        byte[] plte = new byte[n * 3];
        List<Integer> alpha = new ArrayList<>();
        for (var e : palette.entrySet()) {
            int c = e.getKey(), i = e.getValue();
            plte[i * 3] = (byte) (c >> 16);
            plte[i * 3 + 1] = (byte) (c >> 8);
            plte[i * 3 + 2] = (byte) c;
            alpha.add(c == 0 ? 0 : c >>> 24);
        }
        int last = -1;
        for (int i = 0; i < alpha.size(); i++) if (alpha.get(i) != 255) last = i;
        byte[] trns = new byte[last + 1];
        for (int i = 0; i <= last; i++) trns[i] = (byte) (int) alpha.get(i);

        ByteArrayOutputStream out = new ByteArrayOutputStream(1024);
        out.writeBytes(SIGNATURE);
        chunk(out, "IHDR", ByteBuffer.allocate(13).putInt(width).putInt(height).put((byte) depth).put((byte) 3).put((byte) 0).put((byte) 0).put((byte) 0).array());
        chunk(out, "PLTE", plte);
        if (trns.length > 0) chunk(out, "tRNS", trns);
        chunk(out, "IDAT", deflate(raw));
        chunk(out, "IEND", new byte[0]);
        return out.toByteArray();
    }

    private static final byte[] SIGNATURE = {(byte) 0x89, 'P', 'N', 'G', '\r', '\n', 0x1A, '\n'};

    private static byte[] deflate(byte[] raw) {
        Deflater deflater = new Deflater(9);
        deflater.setInput(raw);
        deflater.finish();
        ByteArrayOutputStream z = new ByteArrayOutputStream(raw.length / 4 + 64);
        byte[] buf = new byte[16384];
        while (!deflater.finished()) z.write(buf, 0, deflater.deflate(buf));
        deflater.end();
        return z.toByteArray();
    }

    private static byte[] rgba(int[] argb, int width, int height) {
        byte[] raw = new byte[height * (1 + width * 4)];
        int p = 0;
        for (int y = 0; y < height; y++) {
            raw[p++] = 0;
            for (int x = 0; x < width; x++) {
                int c = argb[y * width + x];
                raw[p++] = (byte) (c >> 16);
                raw[p++] = (byte) (c >> 8);
                raw[p++] = (byte) c;
                raw[p++] = (byte) (c >>> 24);
            }
        }
        ByteArrayOutputStream out = new ByteArrayOutputStream(1024);
        out.writeBytes(SIGNATURE);
        chunk(out, "IHDR", ByteBuffer.allocate(13).putInt(width).putInt(height).put((byte) 8).put((byte) 6).put((byte) 0).put((byte) 0).put((byte) 0).array());
        chunk(out, "IDAT", deflate(raw));
        chunk(out, "IEND", new byte[0]);
        return out.toByteArray();
    }

    private static void chunk(ByteArrayOutputStream out, String type, byte[] data) {
        out.writeBytes(ByteBuffer.allocate(4).putInt(data.length).array());
        byte[] t = type.getBytes(java.nio.charset.StandardCharsets.US_ASCII);
        out.writeBytes(t);
        out.writeBytes(data);
        CRC32 crc = new CRC32();
        crc.update(t);
        crc.update(data);
        out.writeBytes(ByteBuffer.allocate(4).putInt((int) crc.getValue()).array());
    }
}
