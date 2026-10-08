package net.scopenet.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.ClientConfig;
import net.scopenet.client.ClientConfig.WidgetConfig;
import net.scopenet.client.ClientState;
import net.scopenet.client.ScopenetClient;
import net.scopenet.client.Ui;
import net.scopenet.client.hud.Hud;
import net.scopenet.client.hud.HudWidget;
import net.scopenet.client.hud.Widgets;

/**
 * Drag the widgets where you want them. Right-click a widget to hide or show it, scroll to resize everything.
 * Positions snap to the nearest screen anchor, so the layout holds on any window size.
 */
public final class HudEditorScreen extends Screen {
    private final Screen parent;
    private final ClientConfig config = ScopenetClient.config();
    private final ClientState preview = ClientState.preview();
    private HudWidget dragging;
    private double grabDx, grabDy;

    public HudEditorScreen(Screen parent) { super(Component.literal("HUD layout")); this.parent = parent; }

    private float scale() { return config.hud.scale; }
    private int sw() { return (int) (width / scale()); }
    private int sh() { return (int) (height / scale()); }

    private int[] size(HudWidget w) {
        WidgetConfig c = config.widget(w.id);
        return new int[]{w.width(font, preview, c), w.height(font, preview, c)};
    }

    private int[] origin(HudWidget w) {
        int[] s = size(w);
        return Hud.place(config.widget(w.id), sw(), sh(), s[0], s[1]);
    }

    @Override protected void init() {
        addRenderableWidget(Button.builder(Component.literal("Reset"), b -> { config.resetHud(); }).bounds(width / 2 - 104, height - 28, 100, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Done"), b -> onClose()).bounds(width / 2 + 4, height - 28, 100, 20).build());
    }

    @Override public void render(GuiGraphics g, int mouseX, int mouseY, float delta) {
        g.fill(0, 0, width, height, 0x99000000);
        // Faint guides at the thirds, showing where anchors switch.
        for (int i = 1; i <= 2; i++) {
            g.fill(width * i / 3, 0, width * i / 3 + 1, height, 0x22FFFFFF);
            g.fill(0, height * i / 3, width, height * i / 3 + 1, 0x22FFFFFF);
        }
        g.drawCenteredString(font, "Drag to move  ·  Right-click to show or hide  ·  Scroll to resize", width / 2, 8, Ui.TEXT);
        g.pose().pushPose();
        g.pose().scale(scale(), scale(), 1f);
        for (HudWidget w : Widgets.ALL) {
            WidgetConfig c = config.widget(w.id);
            int[] s = size(w), at = origin(w);
            float opacity = c.enabled ? config.hud.opacity : 0.25f;
            w.draw(g, font, preview, c, at[0], at[1], opacity);
            boolean hover = Ui.inside(mouseX / scale(), mouseY / scale(), at[0], at[1], s[0], s[1]);
            int outline = w == dragging ? Ui.ACCENT : hover ? Ui.TEXT : Ui.alpha(0xFFFFFF, 0.3f);
            g.renderOutline(at[0] - 1, at[1] - 1, s[0] + 2, s[1] + 2, outline);
            if (!c.enabled) g.drawString(font, "hidden", at[0] + 4, at[1] + s[1] + 2, Ui.RED, false);
        }
        g.pose().popPose();
        super.render(g, mouseX, mouseY, delta);
    }

    private HudWidget at(double mx, double my) {
        HudWidget found = null;
        for (HudWidget w : Widgets.ALL) {
            int[] s = size(w), o = origin(w);
            if (Ui.inside(mx / scale(), my / scale(), o[0], o[1], s[0], s[1])) found = w; // later ones draw on top
        }
        return found;
    }

    @Override public boolean mouseClicked(double mx, double my, int button) {
        if (super.mouseClicked(mx, my, button)) return true;
        HudWidget w = at(mx, my);
        if (w == null) return false;
        if (button == 1) { WidgetConfig c = config.widget(w.id); c.enabled = !c.enabled; return true; }
        if (button == 0) {
            int[] o = origin(w);
            dragging = w;
            grabDx = mx / scale() - o[0];
            grabDy = my / scale() - o[1];
            return true;
        }
        return false;
    }

    @Override public boolean mouseDragged(double mx, double my, int button, double dx, double dy) {
        if (dragging == null || button != 0) return super.mouseDragged(mx, my, button, dx, dy);
        int[] s = size(dragging);
        int left = (int) Math.round(mx / scale() - grabDx), top = (int) Math.round(my / scale() - grabDy);
        left = Math.max(0, Math.min(sw() - s[0], left));
        top = Math.max(0, Math.min(sh() - s[1], top));
        Hud.anchorTo(config.widget(dragging.id), sw(), sh(), s[0], s[1], left, top);
        return true;
    }

    @Override public boolean mouseReleased(double mx, double my, int button) {
        dragging = null;
        return super.mouseReleased(mx, my, button);
    }

    @Override public boolean mouseScrolled(double mx, double my, double delta) {
        config.hud.scale = Math.max(0.5f, Math.min(2.0f, config.hud.scale + (float) delta * 0.05f));
        return true;
    }

    @Override public void onClose() {
        config.save();
        minecraft.setScreen(parent);
    }
}
