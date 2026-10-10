package net.velora.client.hud;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.screens.ChatScreen;
import net.velora.client.ClientConfig;
import net.velora.client.ClientConfig.WidgetConfig;
import net.velora.client.ClientState;

/** Lays out and draws the widgets. The editor uses the same maths, so what you place is what you get. */
public final class Hud {
    public static final String[] ANCHORS = {"top_left", "top_center", "top_right", "middle_left", "middle_center", "middle_right", "bottom_left", "bottom_center", "bottom_right"};

    private final ClientState state;
    private final ClientConfig config;

    public Hud(ClientState state, ClientConfig config) { this.state = state; this.config = config; }

    /** Top-left corner for a widget of size (w, h) on a screen of (sw, sh) (in HUD units, i.e. already divided by the scale). */
    public static int[] place(WidgetConfig c, int sw, int sh, int w, int h) {
        String a = c.anchor == null ? "top_left" : c.anchor;
        int x = a.endsWith("right") ? sw - w : a.endsWith("center") ? (sw - w) / 2 : 0;
        int y = a.startsWith("bottom") ? sh - h : a.startsWith("middle") ? (sh - h) / 2 : 0;
        return new int[]{x + c.x, y + c.y};
    }

    /** The anchor nearest to a spot, and the offsets from it, so dragged widgets keep their place on any window size. */
    public static void anchorTo(WidgetConfig c, int sw, int sh, int w, int h, int left, int top) {
        double cx = left + w / 2.0, cy = top + h / 2.0;
        String col = cx < sw / 3.0 ? "left" : cx > sw * 2 / 3.0 ? "right" : "center";
        String row = cy < sh / 3.0 ? "top" : cy > sh * 2 / 3.0 ? "bottom" : "middle";
        String anchor = row + "_" + col;
        c.anchor = anchor;
        int bx = anchor.endsWith("right") ? sw - w : anchor.endsWith("center") ? (sw - w) / 2 : 0;
        int by = anchor.startsWith("bottom") ? sh - h : anchor.startsWith("middle") ? (sh - h) / 2 : 0;
        c.x = left - bx;
        c.y = top - by;
    }

    public void render(GuiGraphics g, float tickDelta) {
        Minecraft mc = Minecraft.getInstance();
        if (!config.enabled || !state.connected || mc.options.hideGui || mc.player == null) return;
        if (config.hud.hideInF3 && mc.options.renderDebug) return;
        if (config.hud.hideWhileChatOpen && mc.screen instanceof ChatScreen) return;
        Font font = mc.font;
        float scale = config.hud.scale;
        int sw = (int) (g.guiWidth() / scale), sh = (int) (g.guiHeight() / scale);
        g.pose().pushPose();
        g.pose().scale(scale, scale, 1f);
        for (HudWidget widget : Widgets.ALL) {
            String module = switch (widget.id) { case "balance" -> "economy"; case "guild", "claim" -> "factions"; default -> "analytics"; };
            if (!config.module(module) || !state.module(module)) continue;
            if (widget.id.equals("quests") && state.modules != null) continue;
            WidgetConfig c = config.widget(widget.id);
            if (!c.enabled || !widget.available(state)) continue;
            int w = widget.width(font, state, c), h = widget.height(font, state, c);
            int[] at = place(c, sw, sh, w, h);
            widget.draw(g, font, state, c, at[0], at[1], config.hud.opacity);
        }
        g.pose().popPose();
        WidgetConfig minimap = config.widget("minimap");
        if (minimap.enabled && config.module("map") && state.module("map")) {
            int size = minimap.compact ? 88 : 116;
            int[] at = place(minimap, g.guiWidth(), g.guiHeight(), size, size);
            net.velora.client.VeloraClient.link().map.render(g, at[0], at[1], size, size, mc.player.getX(), mc.player.getZ(), config.minimapScale, net.velora.client.CoreMap.dimension(), false);
        }
    }
}
