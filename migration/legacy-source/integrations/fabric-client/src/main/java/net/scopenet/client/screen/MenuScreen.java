package net.scopenet.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.ClientState;
import net.scopenet.client.ScopenetClient;
import net.scopenet.client.Ui;

/** The hub (press K): who you are on this server, and the way into the market and settings. */
public final class MenuScreen extends Screen {
    private static final int W = 236, H = 176;
    private final ClientState state = ScopenetClient.state();

    public MenuScreen() { super(Component.literal("SCOPENET")); }

    @Override protected void init() {
        int left = (width - W) / 2, top = (height - H) / 2;
        int bw = 104, gap = 8, x1 = left + 10, x2 = left + W - 10 - bw;
        boolean eco = state.connected && state.featEconomy;
        Button market = Button.builder(Component.literal("Market"), b -> minecraft.setScreen(new MarketScreen())).bounds(x1, top + 86, W - 20, 20).build();
        market.active = eco;
        addRenderableWidget(market);
        addRenderableWidget(Button.builder(Component.literal("HUD layout"), b -> minecraft.setScreen(new HudEditorScreen(this))).bounds(x1, top + 86 + 20 + gap, bw, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Settings"), b -> minecraft.setScreen(new SettingsScreen(this))).bounds(x2, top + 86 + 20 + gap, bw, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Close"), b -> onClose()).bounds(left + (W - 80) / 2, top + H - 26, 80, 18).build());
    }

    @Override public void render(GuiGraphics g, int mouseX, int mouseY, float delta) {
        renderBackground(g);
        int left = (width - W) / 2, top = (height - H) / 2;
        Ui.window(g, left, top, W, H);
        g.fill(left, top, left + W, top + 3, Ui.ACCENT);
        g.drawString(font, "SCOPENET", left + 10, top + 10, Ui.ACCENT, true);
        if (!state.connected) {
            g.drawCenteredString(font, "This server isn't running SCOPENET", left + W / 2, top + 50, Ui.TEXT);
            g.drawCenteredString(font, "or its client link is turned off.", left + W / 2, top + 62, Ui.MUTED);
        } else {
            g.drawString(font, state.server.isEmpty() ? "Connected" : state.server, left + W - 10 - font.width(state.server), top + 10, Ui.MUTED, false);
            g.drawString(font, state.name + (state.title.isEmpty() ? "" : "  ·  " + state.title), left + 10, top + 28, Ui.TEXT, true);
            g.drawString(font, "Level " + state.level, left + 10, top + 42, Ui.MUTED, false);
            Ui.bar(g, left + 10, top + 54, W - 20, 5, state.xpPct, Ui.ACCENT, 1f);
            if (state.featEconomy) g.drawString(font, Ui.money(state.currency, state.balance), left + 10, top + 66, Ui.GOLD, true);
            if (state.guild != null) {
                String guild = "[" + state.guild.tag() + "] " + state.guild.name();
                g.drawString(font, guild, left + W - 10 - font.width(guild), top + 66, Ui.CYAN, true);
            }
        }
        super.render(g, mouseX, mouseY, delta);
    }
}
