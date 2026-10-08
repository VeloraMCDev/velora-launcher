package net.scopenet.client.hud;

import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.scopenet.client.ClientConfig;
import net.scopenet.client.ClientState;

/** One draggable piece of the HUD. */
public abstract class HudWidget {
    public final String id;
    public final String label;

    protected HudWidget(String id, String label) { this.id = id; this.label = label; }

    /** False when the server says this system is off, or there's nothing to show. The editor ignores this. */
    public abstract boolean available(ClientState s);

    public abstract int width(Font font, ClientState s, ClientConfig.WidgetConfig c);

    public abstract int height(Font font, ClientState s, ClientConfig.WidgetConfig c);

    /** Draw with the top-left corner at (x, y). */
    public abstract void draw(GuiGraphics g, Font font, ClientState s, ClientConfig.WidgetConfig c, int x, int y, float opacity);
}
