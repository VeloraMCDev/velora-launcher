package net.scopenet.client;

import net.minecraft.client.gui.GuiGraphicsExtractor;

/** The mod's shared look: vanilla-style dark boxes with a thin accent edge, so nothing feels bolted on. */
final class Look {
    static final int TEXT = 0xffffffff, MUTED = 0xffa0a0a0, DIM = 0xff707070, GOOD = 0xff55ff55, BAD = 0xffff5555, WARN = 0xffffaa00, ROW = 0xff1a1d21, ROW_HOVER = 0xff23272c;
    private Look() {}

    static int alpha(int argb, float a) { return ((int) (Math.clamp(a, 0f, 1f) * 255) << 24) | (argb & 0xffffff); }

    /** Like a vanilla tooltip: near-black fill, a two-tone frame tinted with the accent. */
    static void tooltip(GuiGraphicsExtractor g, int x, int y, int w, int h, int accent, float opacity) {
        g.fill(x + 1, y, x + w - 1, y + h, alpha(0x100010, opacity * 0.94f));
        g.fill(x, y + 1, x + w, y + h - 1, alpha(0x100010, opacity * 0.94f));
        int edge = alpha(accent, 0.55f), edge2 = alpha(accent, 0.22f);
        g.fill(x + 1, y + 1, x + w - 1, y + 2, edge); g.fill(x + 1, y + h - 2, x + w - 1, y + h - 1, edge2);
        g.fill(x + 1, y + 2, x + 2, y + h - 2, edge); g.fill(x + w - 2, y + 2, x + w - 1, y + h - 2, edge2);
    }

    /** A screen panel: stone-dark with a light top edge and a dark bottom edge, like a vanilla menu. */
    static void panel(GuiGraphicsExtractor g, int x, int y, int w, int h, float opacity) {
        g.fill(x, y, x + w, y + h, alpha(0x16191d, opacity));
        g.fill(x, y, x + w, y + 1, 0xff55595f); g.fill(x, y, x + 1, y + h, 0xff3b3f44);
        g.fill(x, y + h - 1, x + w, y + h, 0xff08090a); g.fill(x + w - 1, y, x + w, y + h, 0xff08090a);
    }

    /** A recessed area (lists, the map): dark inside, light lower edge. */
    static void well(GuiGraphicsExtractor g, int x, int y, int w, int h) {
        g.fill(x - 1, y - 1, x + w + 1, y + h + 1, 0xff08090a);
        g.fill(x, y, x + w, y + h, 0xff0d0f11);
        g.fill(x - 1, y + h, x + w + 1, y + h + 1, 0xff3b3f44);
    }

    static void bar(GuiGraphicsExtractor g, int x, int y, int w, int h, double fraction, int color) {
        g.fill(x, y, x + w, y + h, 0xff2a2f35);
        int filled = (int) Math.round(w * Math.clamp(fraction, 0, 1));
        if (filled > 0) g.fill(x, y, x + filled, y + h, color);
    }

    /** Draws part of a texture (u0..u1, v0..v1 as fractions) into a w x h box. */
    static void blit(GuiGraphicsExtractor g, net.minecraft.resources.Identifier texture, int x, int y, int w, int h, float u0, float v0, float u1, float v1) {
        g.blit(texture, x, y, x + w, y + h, u0, u1, v0, v1);
    }

    /** A straight line as thin boxes: exact for the axis-aligned edges claims are made of, stepped otherwise. */
    static void line(GuiGraphicsExtractor g, double x0, double y0, double x1, double y1, int thick, int color) {
        int half = thick / 2;
        if (Math.abs(x1 - x0) < 0.5 || Math.abs(y1 - y0) < 0.5) {
            int ax = (int) Math.round(Math.min(x0, x1)), bx = (int) Math.round(Math.max(x0, x1)), ay = (int) Math.round(Math.min(y0, y1)), by = (int) Math.round(Math.max(y0, y1));
            g.fill(ax - half, ay - half, bx + thick - half, by + thick - half, color);
            return;
        }
        int steps = (int) Math.min(600, Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0)));
        for (int i = 0; i <= steps; i++) {
            int px = (int) Math.round(x0 + (x1 - x0) * i / steps), py = (int) Math.round(y0 + (y1 - y0) * i / steps);
            g.fill(px - half, py - half, px + thick - half, py + thick - half, color);
        }
    }
}
