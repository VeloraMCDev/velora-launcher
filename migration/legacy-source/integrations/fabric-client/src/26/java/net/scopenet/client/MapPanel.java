package net.scopenet.client;

import com.google.gson.JsonObject;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.ItemStack;
import java.util.*;

/**
 * The Live Map inside the game: the same tile pyramid, zoom levels, claims, pins and player skins as the panel and the launcher
 * (shared/map/engine.ts), drawn with Minecraft's own GUI calls. The maths lives in {@link MapCamera} and {@link MapScene}.
 */
final class MapPanel implements AutoCloseable {
    private static final int BACKGROUND = 0xff0d1220, LABEL_BG = 0xd10a0e1a, SELF = 0xff34d399, ACCENT = 0xff8b6cff;

    final MapCamera cam = new MapCamera();
    MapScene scene = new MapScene();
    boolean claims = true, pins = true, players = true, skins = true;
    private final Atlas atlas = new Atlas();
    private final Skins heads = new Skins();
    private String slug = "";
    private String hoverId, selectedId, following;
    private Object selected, hover;
    private boolean dragging, positioned;
    private double moved;
    private int x, y, w, h;
    private double selfX, selfZ;
    private String selfSlug = "";

    // ---- data ---------------------------------------------------------------------------------------------------

    void setData(JsonObject data, double sx, double sz, String sslug) {
        selfX = sx; selfZ = sz; selfSlug = sslug;
        scene = MapScene.parse(data);
        if (scene.dimensions.isEmpty()) return;
        if (selectedId != null) {
            Object fresh = null;
            for (MapScene.Player p : scene.players) if (p.uuid().equals(selectedId)) fresh = p;
            for (MapScene.Pin p : scene.pins) if (p.id().equals(selectedId)) fresh = p;
            for (MapScene.Claim c : scene.claims) if (c.id().equals(selectedId)) fresh = c;
            selected = fresh; if (fresh == null) selectedId = null;
        }
        MapScene.Dimension current = scene.dimension(slug);
        if (current == null) {
            slug = scene.dimension(selfSlug) != null ? selfSlug : scene.dimensions.get(0).slug();
            positioned = false;
        }
        if (!positioned && w > 0) place(selfX, selfZ, selfSlug);
        if (following != null) for (MapScene.Player p : scene.players) if (p.uuid().equals(following)) {
            if (!p.slug().equals(slug)) setDimension(p.slug());
            cam.flyTo(p.x(), p.z(), cam.scale, System.currentTimeMillis());
        }
    }

    /** First view: centred on you when you are in this dimension, otherwise the framed dimension, exactly like the panel. */
    private void place(double selfX, double selfZ, String selfSlug) {
        positioned = true;
        cam.frame(dimensionBounds());
        if (slug.equals(selfSlug) && (selfX != 0 || selfZ != 0)) { cam.cx = selfX; cam.cz = selfZ; cam.scale = Math.max(cam.scale, 0.5); }
    }

    private int[] dimensionBounds() { MapScene.Dimension d = scene.dimension(slug); return d == null ? null : d.core(); }

    String slug() { return slug; }

    void setDimension(String id) {
        String next = MapScene.slug(id);
        if (next.equals(slug) || scene.dimension(next) == null) return;
        slug = next; selected = null; selectedId = null; following = null;
        cam.frame(dimensionBounds());
    }

    void cycleDimension() {
        if (scene.dimensions.size() < 2) return;
        for (int i = 0; i < scene.dimensions.size(); i++) if (scene.dimensions.get(i).slug().equals(slug)) { setDimension(scene.dimensions.get((i + 1) % scene.dimensions.size()).slug()); return; }
    }

    void findMe(double px, double pz, String pslug) {
        if (!pslug.equals(slug)) setDimension(pslug);
        cam.flyTo(px, pz, Math.max(cam.scale, 1.5), System.currentTimeMillis());
    }

    void zoom(double factor) { cam.zoomAround(w / 2.0, h / 2.0, factor, true, System.currentTimeMillis()); following = null; }

    // ---- input (coordinates are screen pixels) -----------------------------------------------------------------

    boolean contains(double mx, double my) { return mx >= x && mx < x + w && my >= y && my < y + h; }

    boolean mouseClicked(double mx, double my, boolean doubled) {
        if (!contains(mx, my)) return false;
        dragging = true; moved = 0;
        if (doubled && hover == null) cam.zoomAround(mx - x, my - y, 2, true, System.currentTimeMillis());
        return true;
    }

    boolean mouseDragged(double dx, double dy) {
        if (!dragging) return false;
        moved += Math.abs(dx) + Math.abs(dy);
        if (moved > 3) { cam.pan(dx, dy); following = null; }
        return true;
    }

    boolean mouseReleased(double mx, double my) {
        if (!dragging) return false;
        dragging = false;
        if (moved <= 3) select(hit(mx - x, my - y), System.currentTimeMillis());
        return true;
    }

    boolean mouseScrolled(double mx, double my, double vertical) {
        if (!contains(mx, my)) return false;
        cam.wheel(mx - x, my - y, vertical);
        following = null;
        return true;
    }

    private void select(Object what, long now) {
        selected = what; selectedId = what == null ? null : idOf(what); following = null;
        if (what instanceof MapScene.Player p) { following = p.uuid(); cam.flyTo(p.x(), p.z(), Math.max(cam.scale, 1), now); }
        else if (what instanceof MapScene.Pin p) cam.flyTo(p.x(), p.z(), Math.max(cam.scale, 1), now);
        else if (what instanceof MapScene.Claim c) cam.flyTo(c.labelX(), c.labelZ(), cam.scale, now);
    }

    private static String idOf(Object o) {
        return o instanceof MapScene.Player p ? p.uuid() : o instanceof MapScene.Pin p ? p.id() : o instanceof MapScene.Claim c ? c.id() : "";
    }

    private static String labelOf(Object o) {
        return o instanceof MapScene.Player p ? p.name() : o instanceof MapScene.Pin p ? p.label() : o instanceof MapScene.Claim c ? c.title() : "";
    }

    /** Players, then pins (15 px reach), then claims, like the panel's hit test. */
    private Object hit(double sx, double sy) {
        if (players) {
            MapScene.Player best = null; double bd = 15 * 15;
            for (MapScene.Player p : scene.players) {
                if (!p.slug().equals(slug)) continue;
                double d = Math.pow(cam.sx(p.x()) - sx, 2) + Math.pow(cam.sy(p.z()) - sy, 2);
                if (d < bd) { best = p; bd = d; }
            }
            if (best != null) return best;
        }
        if (pins) {
            MapScene.Pin best = null; double bd = 15 * 15;
            for (MapScene.Pin p : scene.pins) {
                if (!p.slug().equals(slug)) continue;
                double d = Math.pow(cam.sx(p.x()) - sx, 2) + Math.pow(cam.sy(p.z()) - sy, 2);
                if (d < bd) { best = p; bd = d; }
            }
            if (best != null) return best;
        }
        if (claims) {
            double wx = cam.wx(sx), wz = cam.wz(sy);
            for (MapScene.Claim c : scene.claims) if (c.slug().equals(slug) && MapScene.contains(c, wx, wz)) return c;
        }
        return null;
    }

    // ---- drawing ------------------------------------------------------------------------------------------------

    void render(GuiGraphicsExtractor g, Font font, int px, int py, int pw, int ph, int mx, int my, String selfUuid) {
        boolean resized = pw != w || ph != h;
        x = px; y = py; w = pw; h = ph;
        if (resized) cam.resize(pw, ph);
        if (!positioned && !scene.dimensions.isEmpty() && scene.dimension(slug) != null) place(selfX, selfZ, selfSlug);
        cam.tick(System.currentTimeMillis());
        Look.well(g, x, y, w, h);
        g.enableScissor(x, y, x + w, y + h);
        g.fill(x, y, x + w, y + h, BACKGROUND);
        MapScene.Dimension dim = scene.dimension(slug);
        if (dim == null || !scene.ready && scene.token.isBlank()) {
            g.centeredText(font, scene.dimensions.isEmpty() ? (scene.message.isBlank() ? "Loading map..." : scene.message) : "No map for this dimension", x + w / 2, y + h / 2 - 4, Look.MUTED);
            g.disableScissor();
            return;
        }
        boolean over = !dragging && contains(mx, my);
        hover = over ? hit(mx - x, my - y) : null;
        hoverId = hover == null ? null : idOf(hover);
        tiles(g, dim);
        if (cam.scale >= 6) blocks(g, mx, my, over);
        if (claims) drawClaims(g, font);
        if (pins) drawPins(g, font);
        if (players) drawPlayers(g, font, selfUuid);
        g.disableScissor();
        if (hover != null && selected == null) g.setTooltipForNextFrame(net.minecraft.network.chat.Component.literal(labelOf(hover)), mx, my);
        card(g, font);
        if (over) {
            String coords = "X " + (int) Math.floor(cam.wx(mx - x)) + "   Z " + (int) Math.floor(cam.wz(my - y));
            g.text(font, coords, x + w - font.width(coords) - 6, y + h - 12, Look.MUTED);
        }
    }

    private void tiles(GuiGraphicsExtractor g, MapScene.Dimension dim) {
        int zoom = MapCamera.usableZoom(cam.zoomLevel(scene.maxZoom), dim.levels());
        double span = MapCamera.TILE * (double) (1L << zoom), px = span * cam.scale;
        int x0 = (int) Math.floor(cam.wx(0) / span), x1 = (int) Math.floor(cam.wx(w) / span), y0 = (int) Math.floor(cam.wz(0) / span), y1 = (int) Math.floor(cam.wz(h) / span);
        // Zoomed far out on a level that was never built, thousands of tiles would be in view: draw what is cached, ask for nothing.
        boolean crowded = (long) (x1 - x0 + 1) * (y1 - y0 + 1) > 1200;
        if (cam.scale >= 3 && cam.scale < 6) grid(g, 0x14ffffff);
        for (int ty = y0; ty <= y1; ty++) {
            for (int tx = x0; tx <= x1; tx++) {
                if (!MapCamera.inBounds(zoom, tx, ty, dim.bounds())) continue;
                int dx = x + (int) Math.floor(cam.sx(tx * span)), dy = y + (int) Math.floor(cam.sy(ty * span)), size = (int) Math.ceil(px) + 1;
                Identifier tile = crowded ? atlas.peek(slug, zoom, tx, ty) : atlas.tile(Companion.model.panel, scene.serverId, scene.token, slug, zoom, tx, ty);
                if (tile != null) { Look.blit(g, tile, dx, dy, size, size, 0, 0, 1, 1); continue; }
                // Missing at this level: sharper tiles underneath may still exist.
                if (zoom > 0 && atlas.missing(slug, zoom, tx, ty) && fromChildren(g, zoom, tx, ty, dx, dy, size)) continue;
                // Not here yet: show the blurry parent underneath until the real one arrives.
                for (int up = 1; zoom + up <= scene.maxZoom; up++) {
                    int ptx = tx >> up, pty = ty >> up;
                    Identifier parent = up == 1 && !crowded && !atlas.missing(slug, zoom, tx, ty) ? atlas.tile(Companion.model.panel, scene.serverId, scene.token, slug, zoom + up, ptx, pty) : atlas.peek(slug, zoom + up, ptx, pty);
                    if (parent == null) continue;
                    float part = 1f / (1 << up), u = (tx & ((1 << up) - 1)) * part, v = (ty & ((1 << up) - 1)) * part;
                    Look.blit(g, parent, dx, dy, size, size, u, v, u + part, v + part);
                    break;
                }
            }
        }
    }

    private boolean fromChildren(GuiGraphicsExtractor g, int zoom, int tx, int ty, int dx, int dy, int size) {
        boolean drew = false;
        int half = size / 2;
        for (int j = 0; j < 2; j++) for (int i = 0; i < 2; i++) {
            int cx = tx * 2 + i, cy = ty * 2 + j;
            Identifier child = atlas.tile(Companion.model.panel, scene.serverId, scene.token, slug, zoom - 1, cx, cy);
            if (child == null) continue;
            Look.blit(g, child, dx + i * half, dy + j * half, half + 1, half + 1, 0, 0, 1, 1);
            drew = true;
        }
        return drew;
    }

    /** A chunk grid: faint when it only starts to matter, stronger once blocks are big. */
    private void grid(GuiGraphicsExtractor g, int color) {
        double step = 16 * cam.scale;
        for (double wx = Math.floor(cam.wx(0) / 16) * 16; cam.sx(wx) < w + step; wx += 16) { int sx = x + (int) Math.round(cam.sx(wx)); if (sx >= x && sx < x + w) g.fill(sx, y, sx + 1, y + h, color); }
        for (double wz = Math.floor(cam.wz(0) / 16) * 16; cam.sy(wz) < h + step; wz += 16) { int sy = y + (int) Math.round(cam.sy(wz)); if (sy >= y && sy < y + h) g.fill(x, sy, x + w, sy + 1, color); }
    }

    /** Close up every block is its own cell: chunk borders, and an outline on the block under the cursor. */
    private void blocks(GuiGraphicsExtractor g, int mx, int my, boolean over) {
        grid(g, 0x57ffffff);
        if (over && cam.scale >= 8) {
            int bx = (int) Math.floor(cam.wx(mx - x)), bz = (int) Math.floor(cam.wz(my - y));
            int sx = x + (int) Math.round(cam.sx(bx)), sy = y + (int) Math.round(cam.sy(bz)), s = (int) Math.round(cam.scale);
            g.outline(sx, sy, s, s, 0xffffffff);
        }
    }

    private void drawClaims(GuiGraphicsExtractor g, Font font) {
        for (MapScene.Claim c : scene.claims) {
            if (!c.slug().equals(slug)) continue;
            if (c.maxX() < cam.wx(0) || c.minX() > cam.wx(w) || c.maxZ() < cam.wz(0) || c.minZ() > cam.wz(h)) continue;
            boolean hot = c.id().equals(hoverId) || c.id().equals(selectedId);
            int rgb = c.color(), fill = Look.alpha(rgb, hot ? .46f : .28f), edge = Look.alpha(rgb, .95f);
            int top = Math.max(0, (int) Math.floor(cam.sy(c.minZ()))), bottom = Math.min(h, (int) Math.ceil(cam.sy(c.maxZ())));
            for (int sy = top; sy < bottom; sy += 2) {
                double[] spans = MapScene.spans(c, cam.wz(sy + 1));
                for (int i = 0; i + 1 < spans.length; i += 2) {
                    int a = Math.max(0, (int) Math.round(cam.sx(spans[i]))), b = Math.min(w, (int) Math.round(cam.sx(spans[i + 1])));
                    if (b > a) g.fill(x + a, y + sy, x + b, y + Math.min(h, sy + 2), fill);
                }
            }
            outline(g, c.outer(), hot ? 3 : 2, edge);
            for (double[] hole : c.holes()) outline(g, hole, hot ? 3 : 2, edge);
        }
        // Labels on top of every shape, once the claim is big enough to read.
        for (MapScene.Claim c : scene.claims) {
            if (!c.slug().equals(slug)) continue;
            double cw = (c.maxX() - c.minX()) * cam.scale, ch = (c.maxZ() - c.minZ()) * cam.scale;
            if (cw < 46 || ch < 30) continue;
            int sx = (int) cam.sx(c.labelX()), sy = (int) cam.sy(c.labelZ());
            if (sx < -80 || sy < -40 || sx > w + 80 || sy > h + 40) continue;
            chip(g, font, x + sx, y + sy, cw > 150 && ch > 50 ? c.title() : c.tag().isEmpty() ? c.name() : c.tag(), 0xff000000 | c.color());
        }
    }

    private void outline(GuiGraphicsExtractor g, double[] ring, int thick, int color) {
        int n = ring.length / 2;
        for (int i = 0; i < n; i++) {
            int j = (i + 1) % n;
            double ax = x + cam.sx(ring[i * 2]), ay = y + cam.sy(ring[i * 2 + 1]), bx = x + cam.sx(ring[j * 2]), by = y + cam.sy(ring[j * 2 + 1]);
            if (Math.max(ax, bx) < x || Math.min(ax, bx) > x + w || Math.max(ay, by) < y || Math.min(ay, by) > y + h) continue;
            Look.line(g, ax, ay, bx, by, thick, color);
        }
    }

    private void drawPins(GuiGraphicsExtractor g, Font font) {
        for (MapScene.Pin p : scene.pins) {
            if (!p.slug().equals(slug)) continue;
            int sx = x + (int) cam.sx(p.x()), sy = y + (int) cam.sy(p.z());
            if (sx < x - 30 || sy < y - 30 || sx > x + w + 30 || sy > y + h + 30) continue;
            boolean hot = p.id().equals(hoverId) || p.id().equals(selectedId);
            int color = 0xff000000 | MapScene.PIN_COLORS.getOrDefault(p.kind(), 0x94a3b8), r = hot ? 11 : 10;
            g.fill(sx - r - 1, sy - r - 1, sx + r + 1, sy + r + 1, 0xff000000);
            g.fill(sx - r, sy - r, sx + r, sy + r, color);
            g.outline(sx - r, sy - r, r * 2, r * 2, 0xffffffff);
            ItemStack stack = Presentation.icon(MapScene.PIN_ITEMS.getOrDefault(p.kind(), "minecraft:chest"));
            if (!stack.isEmpty()) g.item(stack, sx - 8, sy - 8);
            if (hot || cam.scale >= 0.9) tag(g, font, sx, sy + r + 8, p.label(), 0);
        }
    }

    private void drawPlayers(GuiGraphicsExtractor g, Font font, String selfUuid) {
        for (MapScene.Player p : scene.players) {
            if (!p.slug().equals(slug)) continue;
            int sx = x + (int) cam.sx(p.x()), sy = y + (int) cam.sy(p.z());
            if (sx < x - 30 || sy < y - 30 || sx > x + w + 30 || sy > y + h + 30) continue;
            boolean me = selfUuid != null && p.uuid().equalsIgnoreCase(selfUuid), hot = p.uuid().equals(hoverId) || p.uuid().equals(selectedId);
            int ring = me ? SELF : ACCENT, r = hot ? 9 : 8;
            // Facing: Minecraft yaw 0 = south (+z), 90 = west (-x). Three shrinking marks point the way.
            double a = Math.toRadians(p.yaw()), fx = -Math.sin(a), fz = Math.cos(a);
            for (int i = 0; i < 3; i++) {
                int d = r + 4 + i * 3, s = 3 - i;
                int mxk = sx + (int) Math.round(fx * d), myk = sy + (int) Math.round(fz * d);
                g.fill(mxk - s / 2 - 1, myk - s / 2 - 1, mxk + s / 2 + 1, myk + s / 2 + 1, ring);
            }
            g.fill(sx - r - 2, sy - r - 2, sx + r + 2, sy + r + 2, 0xff000000);
            g.fill(sx - r - 1, sy - r - 1, sx + r + 1, sy + r + 1, ring);
            Identifier head = skins ? heads.head(Companion.model.panel, p.uuid()) : null;
            if (head != null) Look.blit(g, head, sx - r, sy - r, r * 2, r * 2, 0, 0, 1, 1);
            else { g.fill(sx - r, sy - r, sx + r, sy + r, 0xff1e293b); g.centeredText(font, p.name().isEmpty() ? "?" : p.name().substring(0, 1).toUpperCase(Locale.ROOT), sx, sy - 4, 0xffffffff); }
            tag(g, font, sx, sy - r - 10, p.name(), me ? SELF : 0);
        }
    }

    private void chip(GuiGraphicsExtractor g, Font font, int cx, int cy, String text, int color) {
        int tw = font.width(text), bw = tw + 12;
        g.fill(cx - bw / 2 - 1, cy - 9, cx + bw / 2 + 1, cy + 9, color);
        g.fill(cx - bw / 2, cy - 8, cx + bw / 2, cy + 8, LABEL_BG);
        g.text(font, text, cx - tw / 2, cy - 4, 0xfff4f6ff);
    }

    private void tag(GuiGraphicsExtractor g, Font font, int cx, int cy, String text, int color) {
        int tw = font.width(text), bw = tw + 8;
        g.fill(cx - bw / 2, cy - 6, cx + bw / 2, cy + 6, LABEL_BG);
        if (color != 0) g.outline(cx - bw / 2, cy - 6, bw, 12, color);
        g.text(font, text, cx - tw / 2, cy - 4, 0xfff4f6ff);
    }

    /** The details card for the selected claim, pin or player. */
    private void card(GuiGraphicsExtractor g, Font font) {
        if (selected == null) return;
        List<String> lines = new ArrayList<>();
        String title = labelOf(selected), kind;
        if (selected instanceof MapScene.Claim c) { kind = "Claim"; lines.addAll(c.lines()); }
        else if (selected instanceof MapScene.Pin p) { kind = Words.title(p.kind()); lines.addAll(p.lines()); lines.add("X " + (int) p.x() + "  Y " + (int) p.y() + "  Z " + (int) p.z()); }
        else { MapScene.Player p = (MapScene.Player) selected; kind = "Player"; lines.add("X " + (int) p.x() + "  Y " + (int) p.y() + "  Z " + (int) p.z()); lines.add(Words.world(p.slug())); lines.add("Following · click empty map to stop"); }
        int cw = Math.min(w - 12, Math.max(150, font.width(title) + 24)), shown = Math.min(lines.size(), 5), ch = 26 + shown * 11 + 4;
        for (int i = 0; i < shown; i++) cw = Math.min(w - 12, Math.max(cw, font.width(lines.get(i)) + 16));
        int cx = x + 6, cy = y + h - ch - 6;
        Look.tooltip(g, cx, cy, cw, ch, 0xff000000 | (selected instanceof MapScene.Claim c ? c.color() : ACCENT), 1f);
        g.text(font, font.plainSubstrByWidth(title, cw - 12), cx + 7, cy + 6, 0xffffffff);
        g.text(font, kind, cx + 7, cy + 16, Look.DIM);
        for (int i = 0; i < shown; i++) g.text(font, font.plainSubstrByWidth(lines.get(i), cw - 14), cx + 7, cy + 28 + i * 11, Look.MUTED);
    }

    @Override public void close() { atlas.close(); heads.close(); }
}
