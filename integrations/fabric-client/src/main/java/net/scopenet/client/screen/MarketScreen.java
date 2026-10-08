package net.scopenet.client.screen;

import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;
import net.scopenet.client.ClientState;
import net.scopenet.client.Link;
import net.scopenet.client.ScopenetClient;
import net.scopenet.client.Ui;

import java.util.List;

/** The player market: browse what people are selling, buy with one click, and list the item in your hand. */
public final class MarketScreen extends Screen {
    private static final int W = 300, ROW = 30, LIST_TOP = 44;
    private final ClientState state = ScopenetClient.state();
    private int scroll;
    private int age;
    private int refreshIn = -1;
    private EditBox price;
    private Button list;
    private String notice = "";

    public MarketScreen() { super(Component.literal("Market")); }

    private int left() { return (width - W) / 2; }
    private int top() { return 12; }
    private int listHeight() { return Math.max(ROW * 3, height - 24 - LIST_TOP - 64); }

    @Override protected void init() {
        ScopenetClient.link().send("market");
        int y = top() + LIST_TOP + listHeight() + 12;
        price = new EditBox(font, left() + 10, y, 70, 18, Component.literal("Price"));
        price.setMaxLength(12);
        price.setHint(Component.literal("Price"));
        price.setFilter(s -> s.matches("[0-9]{0,9}(\\.[0-9]{0,2})?"));
        addRenderableWidget(price);
        list = Button.builder(Component.literal("List held item"), b -> listHeld()).bounds(left() + 86, y - 1, 100, 20).build();
        addRenderableWidget(list);
        addRenderableWidget(Button.builder(Component.literal("Refresh"), b -> { ScopenetClient.link().send("market"); notice = ""; }).bounds(left() + W - 70, y - 1, 60, 20).build());
        addRenderableWidget(Button.builder(Component.literal("Back"), b -> minecraft.setScreen(new MenuScreen())).bounds(left() + 10, y + 26, 60, 18).build());
    }

    private void listHeld() {
        String text = price.getValue();
        if (text.isEmpty() || text.equals(".")) { notice = "Type a price first."; return; }
        Link.command("market sell " + text);
        price.setValue("");
        notice = "Listing sent…";
        refreshIn = 30;
    }

    @Override public void tick() {
        if (++age >= 200) { age = 0; ScopenetClient.link().send("market"); }
        if (refreshIn >= 0 && --refreshIn < 0) ScopenetClient.link().send("market");
    }

    @Override public boolean mouseScrolled(double mx, double my, double delta) {
        int max = Math.max(0, state.market.size() * ROW - listHeight());
        scroll = (int) Math.max(0, Math.min(max, scroll - delta * ROW / 2));
        return true;
    }

    @Override public boolean mouseClicked(double mx, double my, int button) {
        int left = left(), top = top(), lh = listHeight();
        if (button == 0 && Ui.inside(mx, my, left, top + LIST_TOP, W, lh)) {
            List<ClientState.Listing> items = state.market;
            for (int i = 0; i < items.size(); i++) {
                int y = top + LIST_TOP + i * ROW - scroll;
                if (Ui.inside(mx, my, left + W - 60, y + 4, 48, ROW - 10)) {
                    ClientState.Listing l = items.get(i);
                    Link.command("market buy " + l.id());
                    notice = "Buying #" + l.id() + "…";
                    refreshIn = 30;
                    return true;
                }
            }
        }
        return super.mouseClicked(mx, my, button);
    }

    @Override public void render(GuiGraphics g, int mouseX, int mouseY, float delta) {
        renderBackground(g);
        int left = left(), top = top(), lh = listHeight();
        Ui.window(g, left, top, W, LIST_TOP + lh + 76);
        g.fill(left, top, left + W, top + 3, Ui.CYAN);
        g.drawString(font, "Player Market", left + 10, top + 10, Ui.CYAN, true);
        String bal = "Balance " + Ui.money(state.currency, state.balance);
        g.drawString(font, bal, left + W - 10 - font.width(bal), top + 10, Ui.GOLD, true);
        g.drawString(font, state.market.size() + " listing" + (state.market.size() == 1 ? "" : "s"), left + 10, top + 26, Ui.MUTED, false);

        List<ClientState.Listing> items = state.market;
        g.enableScissor(left + 1, top + LIST_TOP, left + W - 1, top + LIST_TOP + lh);
        for (int i = 0; i < items.size(); i++) {
            int y = top + LIST_TOP + i * ROW - scroll;
            if (y + ROW < top + LIST_TOP || y > top + LIST_TOP + lh) continue;
            ClientState.Listing l = items.get(i);
            boolean affordable = state.balance >= l.price() || l.guildTag() != null;
            g.fill(left + 6, y, left + W - 6, y + ROW - 2, Ui.alpha(Ui.PANEL_2, 0.9f));
            g.renderItem(Ui.stackFor(l.itemId(), l.amount()), left + 10, y + 5);
            g.renderItemDecorations(font, Ui.stackFor(l.itemId(), l.amount()), left + 10, y + 5);
            g.drawString(font, Ui.clip(font, l.itemName() + " x" + l.amount(), 150), left + 32, y + 4, Ui.TEXT, true);
            String seller = l.guildTag() != null ? "[" + l.guildTag() + "] guild" : l.seller();
            g.drawString(font, Ui.clip(font, seller + "  ·  #" + l.id(), 150), left + 32, y + 15, Ui.MUTED, false);
            String p = Ui.money(state.currency, l.price());
            g.drawString(font, p, left + W - 68 - font.width(p), y + 9, Ui.GREEN, true);
            boolean hover = Ui.inside(mouseX, mouseY, left + W - 60, y + 4, 48, ROW - 10);
            g.fill(left + W - 60, y + 4, left + W - 12, y + ROW - 6, affordable ? (hover ? Ui.alpha(Ui.ACCENT, 1f) : Ui.alpha(Ui.ACCENT, 0.75f)) : Ui.alpha(Ui.RED, 0.35f));
            g.drawCenteredString(font, "Buy", left + W - 36, y + 10, Ui.TEXT);
        }
        g.disableScissor();
        if (items.isEmpty()) g.drawCenteredString(font, state.marketAt == 0 ? "Loading listings…" : "No listings. Be the first to sell something!", left + W / 2, top + LIST_TOP + 24, Ui.MUTED);
        if (!notice.isEmpty()) g.drawString(font, notice, left + 10, top + LIST_TOP + lh + 2, Ui.MUTED, false);
        super.render(g, mouseX, mouseY, delta);
    }
}
