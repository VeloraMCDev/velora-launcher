package net.scopenet.client;

import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;
import java.util.*;

/**
 * Drag the modules where you want them, size each one on its own (scroll over it, drag its corner, or use the buttons), switch them
 * on and off, and set the overall scale and opacity. Every change is saved straight away.
 */
final class HudEditorScreen extends Screen {
    private static final double MIN_SIZE = .5, MAX_SIZE = 2.5;
    private Preferences.Widget dragging, resizing;
    private String selected = "";
    private double grabX, grabY;
    private int toolbarTop;
    private boolean selectionChanged;

    HudEditorScreen() { super(Component.literal("HUD Manager")); }

    private Preferences prefs() { return Companion.preferences; }
    private void save() { prefs().save(); }

    @Override protected void init() { build(); }

    private Button add(String text, int x, int y, int w, Runnable run) {
        Button b = Button.builder(Component.literal(text), ignored -> { run.run(); save(); rebuild(); }).bounds(x, y, w, 20).build();
        addRenderableWidget(b);
        return b;
    }
    private void rebuild() { clearWidgets(); build(); }

    private void build() {
        Preferences p = prefs();
        List<String[]> buttons = new ArrayList<>();
        for (Preferences.Widget w : p.widgets) buttons.add(new String[]{Hud.name(w.id) + ": " + (w.enabled ? "On" : "Off"), "w:" + w.id});
        Preferences.Widget picked = p.widget(selected);
        if (picked != null) {
            buttons.add(new String[]{"Smaller", "smaller"});
            buttons.add(new String[]{"Larger", "larger"});
            buttons.add(new String[]{"Reset Size", "size"});
        }
        buttons.add(new String[]{"Scale: " + Math.round(p.scale * 100) + "%", "scale"});
        buttons.add(new String[]{"Opacity: " + Math.round(p.opacity * 100) + "%", "opacity"});
        buttons.add(new String[]{"Notifications: " + (p.notifications ? "On" : "Off"), "notify"});
        buttons.add(new String[]{"Claim Borders: " + (p.claimBorders ? "On" : "Off"), "borders"});
        buttons.add(new String[]{"Reset All", "reset"});
        buttons.add(new String[]{"Done", "done"});
        int gap = 4, rowWidth = 0, max = width - 24;
        List<List<String[]>> rows = new ArrayList<>(); List<String[]> row = new ArrayList<>();
        for (String[] b : buttons) {
            int w = font.width(b[0]) + 16;
            if (!row.isEmpty() && rowWidth + gap + w > max) { rows.add(row); row = new ArrayList<>(); rowWidth = 0; }
            rowWidth += (row.isEmpty() ? 0 : gap) + w; row.add(b);
        }
        rows.add(row);
        toolbarTop = height - 14 - rows.size() * 24 - 12;
        int y = toolbarTop + 18;
        for (List<String[]> r : rows) {
            int total = -gap; for (String[] b : r) total += font.width(b[0]) + 16 + gap;
            int x = (width - total) / 2;
            for (String[] b : r) { int w = font.width(b[0]) + 16; String key = b[1]; add(b[0], x, y, w, () -> act(key)); x += w + gap; }
            y += 24;
        }
    }

    private void act(String key) {
        Preferences p = prefs();
        Preferences.Widget picked = p.widget(selected);
        if (key.startsWith("w:")) { Preferences.Widget w = p.widget(key.substring(2)); if (w != null) { w.enabled = !w.enabled; selected = w.id; } return; }
        switch (key) {
            case "smaller" -> { if (picked != null) picked.scale = size(picked.scale - .1); }
            case "larger" -> { if (picked != null) picked.scale = size(picked.scale + .1); }
            case "size" -> { if (picked != null) picked.scale = 1; }
            case "scale" -> p.scale = p.scale >= 2f - 1e-3 ? .5f : Math.round((p.scale + .1f) * 10) / 10f;
            case "opacity" -> p.opacity = p.opacity >= 1f - 1e-3 ? .3f : Math.round((p.opacity + .1f) * 10) / 10f;
            case "notify" -> p.notifications = !p.notifications;
            case "borders" -> p.claimBorders = !p.claimBorders;
            case "reset" -> { p.widgets = Preferences.defaults(); p.scale = 1; p.opacity = .88f; selected = ""; }
            case "done" -> onClose();
            default -> {}
        }
    }

    private static double size(double v) { return Math.round(Math.clamp(v, MIN_SIZE, MAX_SIZE) * 20) / 20.0; }

    private int sw() { return (int) (width / prefs().scale); }
    private int sh() { return (int) (height / prefs().scale); }

    /** A module as placed on screen, in the global-scale coordinate space. */
    private record Box(Preferences.Widget widget, Hud.Content content, int x, int y, int w, int h, double size) {
        int sw() { return (int) Math.round(w * size); }
        int sh() { return (int) Math.round(h * size); }
        boolean hit(double px, double py) { return px >= x && px < x + sw() && py >= y && py < y + sh(); }
        boolean onHandle(double px, double py) { return px >= x + sw() - 9 && px < x + sw() + 3 && py >= y + sh() - 9 && py < y + sh() + 3; }
    }
    private List<Box> boxes() {
        List<Box> out = new ArrayList<>();
        for (Preferences.Widget widget : prefs().widgets) {
            if (!widget.enabled) continue;
            Hud.Content c = Hud.sample(widget.id);
            if (c == null) continue;
            int w = Hud.width(font, c), h = Hud.height(c);
            out.add(new Box(widget, c, (int) (widget.x * Math.max(0, sw() - w * widget.scale)), (int) (widget.y * Math.max(0, sh() - h * widget.scale)), w, h, widget.scale));
        }
        return out;
    }

    @Override public boolean mouseClicked(MouseButtonEvent e, boolean doubled) {
        if (super.mouseClicked(e, doubled)) return true;
        double mx = e.x() / prefs().scale, my = e.y() / prefs().scale;
        List<Box> boxes = boxes();
        for (int i = boxes.size() - 1; i >= 0; i--) {
            Box b = boxes.get(i);
            if (!b.hit(mx, my) && !(b.widget.id.equals(selected) && b.onHandle(mx, my))) continue;
            if (!b.widget.id.equals(selected)) { selected = b.widget.id; selectionChanged = true; }
            if (b.widget.id.equals(selected) && b.onHandle(mx, my)) resizing = b.widget;
            else { dragging = b.widget; grabX = mx - b.x; grabY = my - b.y; }
            return true;
        }
        if (!selected.isEmpty()) { selected = ""; selectionChanged = true; }
        return false;
    }

    @Override public boolean mouseDragged(MouseButtonEvent e, double dx, double dy) {
        double gs = prefs().scale;
        if (resizing != null) {
            Hud.Content c = Hud.sample(resizing.id);
            if (c == null) return true;
            int w = Hud.width(font, c), h = Hud.height(c);
            double left = resizing.x * Math.max(0, sw() - w * resizing.scale);
            resizing.scale = size((e.x() / gs - left + 4) / w);
            return true;
        }
        if (dragging == null) return super.mouseDragged(e, dx, dy);
        Hud.Content c = Hud.sample(dragging.id);
        if (c == null) return true;
        double bw = Hud.width(font, c) * dragging.scale, bh = Hud.height(c) * dragging.scale;
        dragging.x = Math.clamp((e.x() / gs - grabX) / Math.max(1, sw() - bw), 0, 1);
        dragging.y = Math.clamp((e.y() / gs - grabY) / Math.max(1, sh() - bh), 0, 1);
        return true;
    }

    @Override public boolean mouseReleased(MouseButtonEvent e) {
        boolean was = dragging != null || resizing != null;
        dragging = null; resizing = null;
        if (was) save();
        if (selectionChanged) { selectionChanged = false; rebuild(); }
        return was || super.mouseReleased(e);
    }

    @Override public boolean mouseScrolled(double x, double y, double horizontal, double vertical) {
        double mx = x / prefs().scale, my = y / prefs().scale;
        for (Box b : boxes()) if (b.hit(mx, my)) {
            b.widget.scale = size(b.widget.scale + Math.signum(vertical) * .1);
            selected = b.widget.id; save(); rebuild();
            return true;
        }
        return super.mouseScrolled(x, y, horizontal, vertical);
    }

    @Override public void extractRenderState(GuiGraphicsExtractor g, int mx, int my, float delta) {
        g.fill(0, 0, width, height, 0x55000000);
        // Faint thirds guide the eye, the way a layout editor would.
        for (int i = 1; i < 3; i++) { g.fill(width * i / 3, 0, width * i / 3 + 1, toolbarTop, 0x22ffffff); g.fill(0, toolbarTop * i / 3, width, toolbarTop * i / 3 + 1, 0x22ffffff); }
        g.pose().pushMatrix(); g.pose().scale(prefs().scale, prefs().scale);
        double smx = mx / prefs().scale, smy = my / prefs().scale;
        String hovered = "";
        for (Box b : boxes()) {
            boolean picked = b.widget.id.equals(selected), over = dragging == b.widget || resizing == b.widget || dragging == null && resizing == null && b.hit(smx, smy);
            if (over) hovered = Hud.name(b.widget.id);
            Hud.drawScaled(g, font, b.content, b.x, b.y, b.w, b.h, b.size, over || picked);
            if (picked) {
                // The corner handle that resizes this module.
                g.fill(b.x + b.sw() - 6, b.y + b.sh() - 6, b.x + b.sw() + 2, b.y + b.sh() + 2, 0xffffffff);
                g.fill(b.x + b.sw() - 5, b.y + b.sh() - 5, b.x + b.sw() + 1, b.y + b.sh() + 1, Look.alpha(prefs().accentColor(), 1));
            }
        }
        g.pose().popMatrix();
        Look.panel(g, 0, toolbarTop, width, height - toolbarTop, .92f);
        Preferences.Widget picked = prefs().widget(selected);
        String line = picked == null ? "HUD Manager  ·  Drag a module to move it, click one to size it" : Hud.name(picked.id) + "  ·  Size " + Math.round(picked.scale * 100) + "%  ·  Scroll, drag the corner, or use the buttons";
        g.centeredText(font, line, width / 2, toolbarTop + 6, picked == null ? Look.TEXT : Look.alpha(prefs().accentColor(), 1));
        if (!hovered.isEmpty() && picked == null) g.centeredText(font, hovered, width / 2, toolbarTop - 12, Look.alpha(prefs().accentColor(), 1));
        super.extractRenderState(g, mx, my, delta);
    }

    @Override public boolean isPauseScreen() { return false; }
}
