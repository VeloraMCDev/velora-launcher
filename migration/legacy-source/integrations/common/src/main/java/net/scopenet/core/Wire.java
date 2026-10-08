package net.scopenet.core;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;

/** Encoding of client-link messages: a Minecraft "UTF string" (VarInt byte length, then UTF-8), same as FriendlyByteBuf.writeUtf. */
public final class Wire {
    public static final String S2C = "scopenet:s2c";
    public static final String C2S = "scopenet:c2s";
    public static final int VERSION = 1;
    private static final int MAX_BYTES = 32767 * 3;

    private Wire() {}

    public static byte[] encode(String json) {
        byte[] body = json.getBytes(StandardCharsets.UTF_8);
        if (body.length > MAX_BYTES) throw new IllegalArgumentException("message too large");
        ByteArrayOutputStream out = new ByteArrayOutputStream(body.length + 5);
        int v = body.length;
        while ((v & ~0x7F) != 0) { out.write((v & 0x7F) | 0x80); v >>>= 7; }
        out.write(v);
        out.writeBytes(body);
        return out.toByteArray();
    }

    public static String decode(byte[] data) {
        int length = 0, shift = 0, at = 0;
        while (true) {
            if (at >= data.length || shift > 28) throw new IllegalArgumentException("bad length");
            int b = data[at++];
            length |= (b & 0x7F) << shift;
            if ((b & 0x80) == 0) break;
            shift += 7;
        }
        if (length < 0 || length > MAX_BYTES || at + length > data.length) throw new IllegalArgumentException("bad length");
        return new String(data, at, length, StandardCharsets.UTF_8);
    }
}
