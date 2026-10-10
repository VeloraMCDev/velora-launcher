package net.scopenet.client;

/** The Live Map's camera, ported 1:1 from the panel's engine (shared/map/engine.ts): same limits, zoom levels and easing. */
final class MapCamera {
    static final int TILE = 256;
    static final double MIN_SCALE = 1 / 128.0, MAX_SCALE = 48;

    double cx, cz, scale = 1;
    int width = 300, height = 200;
    private double fromX, fromZ, fromS, toX, toZ, toS;
    private long start, ms;
    private boolean flying;

    static double clamp(double v, double lo, double hi) { return Math.max(lo, Math.min(hi, v)); }

    double sx(double x) { return (x - cx) * scale + width / 2.0; }
    double sy(double z) { return (z - cz) * scale + height / 2.0; }
    double wx(double sx) { return (sx - width / 2.0) / scale + cx; }
    double wz(double sy) { return (sy - height / 2.0) / scale + cz; }

    void resize(int w, int h) { width = Math.max(1, w); height = Math.max(1, h); }

    void pan(double dxPixels, double dyPixels) { cx -= dxPixels / scale; cz -= dyPixels / scale; flying = false; }

    /** Zoom keeping the world point under (sx, sy) where it is; animated for button and double-click zooms. */
    void zoomAround(double sx, double sy, double factor, boolean animate, long now) {
        double bx = wx(sx), bz = wz(sy), target = clamp(scale * factor, MIN_SCALE, MAX_SCALE);
        if (animate) {
            fly(bx - (sx - width / 2.0) / target, bz - (sy - height / 2.0) / target, target, 220, now);
        } else {
            scale = target; cx = bx - (sx - width / 2.0) / scale; cz = bz - (sy - height / 2.0) / scale; flying = false;
        }
    }

    /** One mouse-wheel notch, the way the panel turns wheel deltas into a zoom. */
    void wheel(double sx, double sy, double notches) { zoomAround(sx, sy, Math.exp(notches * 0.0016 * 100), false, 0); }

    void flyTo(double x, double z, double s, long now) { fly(x, z, clamp(s, MIN_SCALE, MAX_SCALE), 520, now); }

    private void fly(double x, double z, double s, long duration, long now) {
        fromX = cx; fromZ = cz; fromS = scale; toX = x; toZ = z; toS = s; start = now; ms = duration; flying = true;
    }

    /** Advances an animation; true while still moving. */
    boolean tick(long now) {
        if (!flying) return false;
        double k = Math.min(1, (now - start) / (double) ms), e = 1 - Math.pow(1 - k, 3);
        scale = fromS * Math.pow(toS / fromS, e);
        cx = fromX + (toX - fromX) * e; cz = fromZ + (toZ - fromZ) * e;
        if (k >= 1) flying = false;
        return true;
    }

    boolean flying() { return flying; }

    /** Frame a dimension: its populated core when it has enough tiles, else all of it (zoom-0 tile bounds min_x, max_x, min_y, max_y). */
    void frame(int[] tileBounds) {
        if (tileBounds == null) return;
        flying = false;
        double minX = tileBounds[0] * TILE, maxX = (tileBounds[1] + 1) * TILE, minZ = tileBounds[2] * TILE, maxZ = (tileBounds[3] + 1) * TILE;
        cx = (minX + maxX) / 2; cz = (minZ + maxZ) / 2;
        scale = clamp(Math.min(width / Math.max(1, maxX - minX), height / Math.max(1, maxZ - minZ)) * 0.9, 0.05, 2);
    }

    /** The tile level drawn at this scale. */
    int zoomLevel(int maxZoom) { return (int) clamp(Math.floor(Math.log(1 / scale) / Math.log(2)), 0, maxZoom); }

    /** The deepest level at or below `wanted` that actually holds tiles (the panel reports how many each level has). */
    static int usableZoom(int wanted, int[] levels) {
        if (levels == null || levels.length == 0) return wanted;
        for (int z = Math.min(wanted, levels.length - 1); z > 0; z--) if (levels[z] > 0) return z;
        return 0;
    }

    static boolean inBounds(int zoom, int x, int y, int[] b) {
        if (b == null) return true;
        long span = 1L << zoom;
        return x * span <= b[1] && (x + 1) * span - 1 >= b[0] && y * span <= b[3] && (y + 1) * span - 1 >= b[2];
    }
}
