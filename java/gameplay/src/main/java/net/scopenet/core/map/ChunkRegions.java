package net.scopenet.core.map;

import java.util.*;

/**
 * Turns a set of claimed chunks into clean outlines for a map: one outer ring per connected area, plus a ring for
 * every hole inside it. Coordinates are block coordinates (chunk x 16). Pure and fast: each chunk adds at most four edges.
 */
public final class ChunkRegions {
    private ChunkRegions() {}

    public record Point(double x, double z) {}

    /** A closed loop; the last point connects back to the first. */
    public record Ring(List<Point> points) {
        public double area() {
            double sum = 0;
            for (int i = 0; i < points.size(); i++) {
                Point a = points.get(i), b = points.get((i + 1) % points.size());
                sum += a.x() * b.z() - b.x() * a.z();
            }
            return sum / 2;
        }

        public boolean contains(double x, double z) {
            boolean inside = false;
            for (int i = 0, j = points.size() - 1; i < points.size(); j = i++) {
                Point a = points.get(i), b = points.get(j);
                if ((a.z() > z) != (b.z() > z) && x < (b.x() - a.x()) * (z - a.z()) / (b.z() - a.z()) + a.x()) inside = !inside;
            }
            return inside;
        }
    }

    /** One connected area: its outline, the holes in it, how many chunks it has and a good spot for its label. */
    public record Region(Ring outer, List<Ring> holes, int chunks, double labelX, double labelZ) {}

    public static long key(int x, int z) { return ((long) x << 32) | (z & 0xffffffffL); }
    static int kx(long key) { return (int) (key >> 32); }
    static int kz(long key) { return (int) key; }

    /** Split into areas that touch along an edge. */
    static List<Set<Long>> components(Set<Long> chunks) {
        List<Set<Long>> out = new ArrayList<>();
        Set<Long> seen = new HashSet<>();
        List<Long> sorted = new ArrayList<>(chunks);
        Collections.sort(sorted);
        for (long start : sorted) {
            if (!seen.add(start)) continue;
            Set<Long> group = new HashSet<>();
            Deque<Long> queue = new ArrayDeque<>();
            queue.add(start);
            group.add(start);
            while (!queue.isEmpty()) {
                long c = queue.poll();
                int x = kx(c), z = kz(c);
                for (long n : new long[]{key(x + 1, z), key(x - 1, z), key(x, z + 1), key(x, z - 1)}) {
                    if (chunks.contains(n) && seen.add(n)) { group.add(n); queue.add(n); }
                }
            }
            out.add(group);
        }
        return out;
    }

    public static List<Region> regions(Set<Long> chunks) {
        List<Region> out = new ArrayList<>();
        for (Set<Long> group : components(chunks)) {
            List<Ring> outers = new ArrayList<>(), holes = new ArrayList<>();
            for (Ring r : outline(group)) (r.area() > 0 ? outers : holes).add(r);
            double[] label = labelSpot(group);
            for (Ring o : outers) {
                List<Ring> mine = new ArrayList<>();
                for (Ring h : holes) if (outers.size() == 1 || o.contains(average(h, true), average(h, false))) mine.add(h);
                out.add(new Region(o, mine, group.size(), label[0], label[1]));
            }
        }
        out.sort(Comparator.comparingInt(Region::chunks).reversed().thenComparingDouble(r -> r.labelX()).thenComparingDouble(r -> r.labelZ()));
        return out;
    }

    private static double average(Ring ring, boolean x) {
        double sum = 0;
        for (Point p : ring.points()) sum += x ? p.x() : p.z();
        return sum / ring.points().size();
    }

    /** The middle of the chunk nearest to the area's average, so a label never lands outside a bent shape. */
    static double[] labelSpot(Set<Long> group) {
        double mx = 0, mz = 0;
        for (long c : group) { mx += kx(c) + 0.5; mz += kz(c) + 0.5; }
        mx /= group.size();
        mz /= group.size();
        long best = group.iterator().next();
        double bestD = Double.MAX_VALUE;
        for (long c : group) {
            double d = Math.pow(kx(c) + 0.5 - mx, 2) + Math.pow(kz(c) + 0.5 - mz, 2);
            if (d < bestD || (d == bestD && c < best)) { bestD = d; best = c; }
        }
        return new double[]{(kx(best) + 0.5) * 16, (kz(best) + 0.5) * 16};
    }

    private record Edge(int fx, int fz, int tx, int tz) {}

    /** Boundary loops of one connected area. Outer loops have positive area, holes negative. */
    static List<Ring> outline(Set<Long> group) {
        // Directed edges with the chunk on the right-hand side (clockwise around solid land).
        Map<Long, List<Edge>> from = new HashMap<>();
        for (long c : group) {
            int x = kx(c), z = kz(c);
            if (!group.contains(key(x, z - 1))) add(from, new Edge(x, z, x + 1, z));
            if (!group.contains(key(x + 1, z))) add(from, new Edge(x + 1, z, x + 1, z + 1));
            if (!group.contains(key(x, z + 1))) add(from, new Edge(x + 1, z + 1, x, z + 1));
            if (!group.contains(key(x - 1, z))) add(from, new Edge(x, z + 1, x, z));
        }
        List<Ring> rings = new ArrayList<>();
        while (!from.isEmpty()) {
            // Start from a stable edge so results don't depend on hash order.
            long startKey = Collections.min(from.keySet());
            Edge first = from.get(startKey).get(0);
            List<int[]> loop = new ArrayList<>();
            Edge e = first;
            do {
                remove(from, e);
                loop.add(new int[]{e.fx(), e.fz()});
                e = next(from, e);
                if (e == null) break;
            } while (!(e.fx() == first.fx() && e.fz() == first.fz()));
            rings.add(new Ring(simplify(loop)));
        }
        return rings;
    }

    private static void add(Map<Long, List<Edge>> m, Edge e) { m.computeIfAbsent(key(e.fx(), e.fz()), k -> new ArrayList<>()).add(e); }

    private static void remove(Map<Long, List<Edge>> m, Edge e) {
        List<Edge> list = m.get(key(e.fx(), e.fz()));
        list.remove(e);
        if (list.isEmpty()) m.remove(key(e.fx(), e.fz()));
    }

    /** The edge leaving this edge's end. Where two loops touch at a corner, turn right to keep them apart. */
    private static Edge next(Map<Long, List<Edge>> m, Edge e) {
        List<Edge> options = m.get(key(e.tx(), e.tz()));
        if (options == null || options.isEmpty()) return null;
        if (options.size() == 1) return options.get(0);
        int dx = e.tx() - e.fx(), dz = e.tz() - e.fz();
        Edge best = options.get(0);
        int bestScore = Integer.MIN_VALUE;
        for (Edge o : options) {
            int ox = o.tx() - o.fx(), oz = o.tz() - o.fz();
            int cross = dx * oz - dz * ox; // >0: turns right in (x right, z down) coordinates
            int score = cross > 0 ? 2 : cross == 0 ? 1 : 0;
            if (score > bestScore) { bestScore = score; best = o; }
        }
        return best;
    }

    /** Drop corner points that sit in a straight line, then scale chunks to blocks. */
    private static List<Point> simplify(List<int[]> loop) {
        List<int[]> kept = new ArrayList<>();
        int n = loop.size();
        for (int i = 0; i < n; i++) {
            int[] prev = loop.get((i + n - 1) % n), cur = loop.get(i), nxt = loop.get((i + 1) % n);
            int cross = (cur[0] - prev[0]) * (nxt[1] - cur[1]) - (cur[1] - prev[1]) * (nxt[0] - cur[0]);
            if (cross != 0) kept.add(cur);
        }
        List<Point> out = new ArrayList<>();
        for (int[] p : kept) out.add(new Point(p[0] * 16.0, p[1] * 16.0));
        return out;
    }
}
