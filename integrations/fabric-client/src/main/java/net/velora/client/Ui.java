package net.velora.client;

import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;

import java.util.Locale;

/** Shared look: the Velora palette and a few drawing helpers, so every widget and window feels like one app. */
public final class Ui {
    public static final int ACCENT = 0xFF8B6CFF, CYAN = 0xFF22D3EE, GOLD = 0xFFFBBF24, GREEN = 0xFF34D399, RED = 0xFFFB7185;
    public static final int TEXT = 0xFFE6E8EE, MUTED = 0xFF9AA0AE, PANEL = 0x0E1014, PANEL_2 = 0x1A1D25, LINE = 0xFF2B2F3A;

    private Ui() {}

    public static int alpha(int rgb, float opacity) {
        return (Math.max(0, Math.min(255, (int) (opacity * 255))) << 24) | (rgb & 0xFFFFFF);
    }

    public static String money(String symbol, double value) { return symbol + String.format(Locale.US, "%,.2f", value); }

    /** A card: dark rounded-looking panel with a coloured edge on the left. */
    public static void card(GuiGraphics g, int x, int y, int w, int h, int accent, float opacity) {
        g.fill(x + 1, y, x + w - 1, y + h, alpha(PANEL, opacity));
        g.fill(x, y + 1, x + w, y + h - 1, alpha(PANEL, opacity));
        g.fill(x, y + 1, x + 2, y + h - 1, accent);
    }

    /** A flat panel with a 1px border, for windows. */
    public static void window(GuiGraphics g, int x, int y, int w, int h) {
        g.fill(x - 1, y - 1, x + w + 1, y + h + 1, LINE);
        g.fill(x, y, x + w, y + h, 0xF0101319);
    }

    public static void bar(GuiGraphics g, int x, int y, int w, int h, double pct, int colour, float opacity) {
        g.fill(x, y, x + w, y + h, alpha(0x2B2F3A, Math.max(opacity, 0.6f)));
        int filled = (int) Math.round(w * Math.max(0, Math.min(100, pct)) / 100.0);
        if (filled > 0) g.fill(x, y, x + filled, y + h, colour);
    }

    /** The item for a panel id ("DIAMOND" or "modid:item"); barrier if the client doesn't know it. */
    public static net.minecraft.world.item.ItemStack stackFor(String panelId, int count) {
        String registry = panelId.contains(":") ? panelId : "minecraft:" + panelId.toLowerCase(Locale.ROOT);
        net.minecraft.resources.ResourceLocation id = net.minecraft.resources.ResourceLocation.tryParse(registry);
        net.minecraft.world.item.Item item = id == null ? net.minecraft.world.item.Items.AIR : net.minecraft.core.registries.BuiltInRegistries.ITEM.get(id);
        if (item == net.minecraft.world.item.Items.AIR) item = net.minecraft.world.item.Items.BARRIER;
        return new net.minecraft.world.item.ItemStack(item, Math.max(1, count));
    }

    public static boolean inside(double mx, double my, int x, int y, int w, int h) { return mx >= x && mx < x + w && my >= y && my < y + h; }

    public static String clip(Font font, String text, int maxWidth) {
        if (font.width(text) <= maxWidth) return text;
        String ellipsis = "…";
        int end = text.length();
        while (end > 0 && font.width(text.substring(0, end) + ellipsis) > maxWidth) end--;
        return text.substring(0, end) + ellipsis;
    }
}
