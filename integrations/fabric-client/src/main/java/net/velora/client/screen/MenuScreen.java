package net.velora.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.velora.client.ClientState;
import net.velora.client.VeloraClient;
import net.velora.client.Ui;

/** The hub (press K): who you are on this server, and the way into the market and settings. */
public final class MenuScreen extends Screen {
    private static final int W = 236, H = 236;
    private final ClientState state = VeloraClient.state();

    public MenuScreen() { super(Component.literal("Velora")); }

    @Override protected void init() {
        int left = (width - W) / 2, top = (height - H) / 2;
        int bw = 104, gap = 8, x1 = left + 10, x2 = left + W - 10 - bw;
        boolean eco = state.connected && state.featEconomy && state.module("economy") && VeloraClient.config().module("economy");
        Button market = Button.builder(Component.literal("Market"), b -> minecraft.setScreen(new MarketScreen())).bounds(x1, top + 86, bw, 20).build();
        market.active = eco;
        addRenderableWidget(market);
        Button darknet=Button.builder(Component.literal("Darknet"),b->minecraft.setScreen(new DarknetScreen())).bounds(x2,top+86,bw,20).build();darknet.active=eco;addRenderableWidget(darknet);
        Button casino=Button.builder(Component.literal("Casino"),b->minecraft.setScreen(new CasinoScreen())).bounds(x1,top+114,bw,20).build();casino.active=eco&&state.module("casino")&&VeloraClient.config().module("casino");addRenderableWidget(casino);
        Button faction=Button.builder(Component.literal("Faction"),b->minecraft.setScreen(new FactionScreen())).bounds(x2,top+114,bw,20).build();faction.active=state.connected&&state.module("factions")&&VeloraClient.config().module("factions");addRenderableWidget(faction);
        Button map=Button.builder(Component.literal("Map"),b->minecraft.setScreen(new MapScreen())).bounds(x1,top+142,bw,20).build();map.active=state.connected&&state.module("map")&&VeloraClient.config().module("map");addRenderableWidget(map);
        Button vault=Button.builder(Component.literal("Vault 1"),b->{net.velora.client.Link.command("vault 1");onClose();}).bounds(x2,top+142,bw,20).build();vault.active=state.connected&&state.module("vaults")&&VeloraClient.config().module("vaults");addRenderableWidget(vault);
        addRenderableWidget(Button.builder(Component.literal("HUD layout"), b -> minecraft.setScreen(new HudEditorScreen(this))).bounds(x1, top + 170, bw, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Settings"), b -> minecraft.setScreen(new SettingsScreen(this))).bounds(x2, top + 170, bw, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Close"), b -> onClose()).bounds(left + (W - 80) / 2, top + H - 26, 80, 18).build());
    }

    @Override public void render(GuiGraphics g, int mouseX, int mouseY, float delta) {
        renderBackground(g);
        int left = (width - W) / 2, top = (height - H) / 2;
        Ui.window(g, left, top, W, H);
        g.fill(left, top, left + W, top + 3, Ui.ACCENT);
        g.drawString(font, "Velora", left + 10, top + 10, Ui.ACCENT, true);
        if (!state.connected) {
            g.drawCenteredString(font, "This server isn't running Velora", left + W / 2, top + 50, Ui.TEXT);
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
