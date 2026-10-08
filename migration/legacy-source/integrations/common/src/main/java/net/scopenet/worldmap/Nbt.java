package net.scopenet.worldmap;

import java.io.DataInputStream;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** A small reader for Minecraft's NBT format: just enough to read chunks. Values are plain Java objects. */
public final class Nbt {
    private Nbt() {}

    /** A compound tag: names to values (Byte, Short, Integer, Long, Float, Double, String, byte[], int[], long[], List, Compound). */
    public static final class Compound extends HashMap<String, Object> {
        public Compound compound(String key) { return get(key) instanceof Compound c ? c : null; }

        @SuppressWarnings("unchecked")
        public List<Object> list(String key) { return get(key) instanceof List<?> l ? (List<Object>) l : List.of(); }

        public String string(String key, String fallback) { return get(key) instanceof String s ? s : fallback; }

        public long[] longs(String key) { return get(key) instanceof long[] a ? a : null; }

        public int integer(String key, int fallback) { return get(key) instanceof Number n ? n.intValue() : fallback; }
    }

    private static final int MAX_DEPTH = 64;
    private static final int MAX_ARRAY = 8 * 1024 * 1024;

    /** Reads one named root compound (the form chunks are stored in). */
    public static Compound readRoot(DataInputStream in) throws IOException {
        byte type = in.readByte();
        if (type != 10) throw new IOException("NBT root must be a compound, found tag " + type);
        in.readUTF(); // root name, usually empty
        return readCompound(in, 0);
    }

    private static Compound readCompound(DataInputStream in, int depth) throws IOException {
        if (depth > MAX_DEPTH) throw new IOException("NBT nested too deeply");
        Compound out = new Compound();
        while (true) {
            byte type = in.readByte();
            if (type == 0) return out;
            String name = in.readUTF();
            out.put(name, readValue(in, type, depth + 1));
        }
    }

    private static Object readValue(DataInputStream in, byte type, int depth) throws IOException {
        if (depth > MAX_DEPTH) throw new IOException("NBT nested too deeply");
        switch (type) {
            case 1: return in.readByte();
            case 2: return in.readShort();
            case 3: return in.readInt();
            case 4: return in.readLong();
            case 5: return in.readFloat();
            case 6: return in.readDouble();
            case 7: { byte[] a = new byte[length(in)]; in.readFully(a); return a; }
            case 8: return in.readUTF();
            case 9: {
                byte elementType = in.readByte();
                int n = length(in);
                List<Object> list = new ArrayList<>(Math.min(n, 4096));
                for (int i = 0; i < n; i++) list.add(readValue(in, elementType, depth + 1));
                return list;
            }
            case 10: return readCompound(in, depth);
            case 11: { int[] a = new int[length(in)]; for (int i = 0; i < a.length; i++) a[i] = in.readInt(); return a; }
            case 12: { long[] a = new long[length(in)]; for (int i = 0; i < a.length; i++) a[i] = in.readLong(); return a; }
            default: throw new IOException("unknown NBT tag " + type);
        }
    }

    private static int length(DataInputStream in) throws IOException {
        int n = in.readInt();
        if (n < 0 || n > MAX_ARRAY) throw new IOException("NBT array too large: " + n);
        return n;
    }
}
