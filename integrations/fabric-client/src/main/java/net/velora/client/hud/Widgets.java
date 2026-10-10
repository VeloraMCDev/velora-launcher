package net.velora.client.hud;

import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.velora.client.ClientConfig.WidgetConfig;
import net.velora.client.ClientState;
import net.velora.client.Ui;

import java.util.List;

/** The built-in HUD widgets: level, balance, guild, claim and quests. */
public final class Widgets {
    private Widgets() {}

    public static final List<HudWidget> ALL = List.of(new Level(), new Balance(), new Guild(), new Claim(), new Quests());

    static final class Level extends HudWidget {
        Level() { super("level", "Level"); }
        @Override public boolean available(ClientState s) { return s.hasState; }
        @Override public int width(Font f, ClientState s, WidgetConfig c) { return c.compact ? 64 : 118; }
        @Override public int height(Font f, ClientState s, WidgetConfig c) { return c.compact ? 22 : 34; }
        @Override public void draw(GuiGraphics g, Font f, ClientState s, WidgetConfig c, int x, int y, float o) {
            int w = width(f, s, c), h = height(f, s, c);
            Ui.card(g, x, y, w, h, Ui.ACCENT, o);
            g.drawString(f, "Lv " + s.level, x + 7, y + 5, Ui.TEXT, true);
            if (!c.compact) {
                String title = s.title.isEmpty() ? "" : Ui.clip(f, s.title, w - 52);
                g.drawString(f, title, x + w - 6 - f.width(title), y + 5, Ui.MUTED, true);
                Ui.bar(g, x + 7, y + 18, w - 14, 5, s.xpPct, Ui.ACCENT, o);
                String text = Math.round(s.xpPct) + "%  ·  SMP Lv " + s.serverLevel;
                g.drawString(f, text, x + 7, y + 25, Ui.MUTED, false);
            } else {
                Ui.bar(g, x + 7, y + 15, w - 14, 4, s.xpPct, Ui.ACCENT, o);
            }
        }
    }

    static final class Balance extends HudWidget {
        Balance() { super("balance", "Balance"); }
        @Override public boolean available(ClientState s) { return s.hasState && s.featEconomy; }
        private String text(ClientState s) { return Ui.money(s.currency, s.balance); }
        @Override public int width(Font f, ClientState s, WidgetConfig c) { return Math.max(c.compact ? 50 : 76, f.width(text(s)) + 18); }
        @Override public int height(Font font, ClientState s, WidgetConfig c) { return c.compact ? 18 : 28; }
        @Override public void draw(GuiGraphics g, Font f, ClientState s, WidgetConfig c, int x, int y, float o) {
            int w = width(f, s, c), h = height(f, s, c);
            Ui.card(g, x, y, w, h, Ui.GOLD, o);
            if (!c.compact) {
                g.drawString(f, "BALANCE", x + 7, y + 4, Ui.MUTED, false);
                g.drawString(f, text(s), x + 7, y + 15, Ui.GOLD, true);
            } else {
                g.drawString(f, text(s), x + 7, y + 5, Ui.GOLD, true);
            }
        }
    }

    static final class Guild extends HudWidget {
        Guild() { super("guild", "Guild"); }
        @Override public boolean available(ClientState s) { return s.hasState && s.featGuilds && s.guild != null; }
        @Override public int width(Font f, ClientState s, WidgetConfig c) { return c.compact ? 86 : 124; }
        @Override public int height(Font f, ClientState s, WidgetConfig c) { return c.compact ? 18 : 30; }
        @Override public void draw(GuiGraphics g, Font f, ClientState s, WidgetConfig c, int x, int y, float o) {
            ClientState.Guild guild = s.guild;
            int w = width(f, s, c), h = height(f, s, c);
            Ui.card(g, x, y, w, h, Ui.CYAN, o);
            String head = "[" + guild.tag() + "] " + guild.name();
            g.drawString(f, Ui.clip(f, head, w - 14), x + 7, y + 4, Ui.TEXT, true);
            if (!c.compact) g.drawString(f, guild.role() + "  ·  " + guild.claims() + " chunks", x + 7, y + 17, Ui.MUTED, false);
        }
    }

    static final class Claim extends HudWidget {
        Claim() { super("claim", "Claim"); }
        @Override public boolean available(ClientState s) { return s.featClaims && !s.claimCells.isEmpty(); }
        private String text(ClientState s) {
            ClientState.Owner o = s.claimOwner;
            if (o == null) return "Wilderness";
            String name = o.tag().isEmpty() ? o.name() : "[" + o.tag() + "] " + o.name();
            return (o.mine() ? "Your land: " : "Claimed: ") + name;
        }
        @Override public int width(Font f, ClientState s, WidgetConfig c) { return f.width(text(s)) + 18; }
        @Override public int height(Font f, ClientState s, WidgetConfig c) { return 18; }
        @Override public void draw(GuiGraphics g, Font f, ClientState s, WidgetConfig c, int x, int y, float o) {
            int w = width(f, s, c);
            ClientState.Owner owner = s.claimOwner;
            int colour = owner == null ? Ui.MUTED : owner.mine() ? Ui.GREEN : Ui.RED;
            Ui.card(g, x, y, w, 18, colour, o);
            g.drawString(f, text(s), x + 9, y + 5, colour == Ui.MUTED ? Ui.TEXT : colour, true);
        }
    }

    static final class Quests extends HudWidget {
        Quests() { super("quests", "Quests"); }
        @Override public boolean available(ClientState s) { return s.hasState && (s.daily.total() + s.weekly.total()) > 0; }
        @Override public int width(Font f, ClientState s, WidgetConfig c) { return 96; }
        @Override public int height(Font f, ClientState s, WidgetConfig c) { return 38; }
        @Override public void draw(GuiGraphics g, Font f, ClientState s, WidgetConfig c, int x, int y, float o) {
            int w = width(f, s, c);
            Ui.card(g, x, y, w, 38, Ui.GREEN, o);
            row(g, f, "Daily", s.daily, x + 7, y + 4, w - 14);
            row(g, f, "Weekly", s.weekly, x + 7, y + 21, w - 14);
        }
        private void row(GuiGraphics g, Font f, String label, ClientState.Quest q, int x, int y, int w) {
            g.drawString(f, label, x, y, Ui.MUTED, false);
            String n = q.done() + "/" + q.total();
            g.drawString(f, n, x + w - f.width(n), y, Ui.TEXT, true);
            Ui.bar(g, x, y + 10, w, 3, q.total() == 0 ? 0 : 100.0 * q.done() / q.total(), Ui.GREEN, 1f);
        }
    }
}
